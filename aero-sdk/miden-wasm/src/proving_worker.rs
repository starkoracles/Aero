use crate::convert::convert_proof::*;
use crate::convert::sdk::sdk;
use crate::pool::WorkerPool;
use crate::utils::{
    from_uint8array, set_once_logger, to_uint8array, HashingResult, ProverOutput, ProvingWorkItem,
    VecWrapper,
};
use futures::Future;
use js_sys::Uint8Array;
use log::{debug, info};
use miden::{verify, ExecutionTrace, Program, ProgramInputs, ProofOptions};
use miden_air::{Felt, FieldElement, ProcessorAir, PublicInputs, StarkField};
use miden_core::ProgramOutputs;
use miden_prover::ExecutionProver;
use prost::Message;
use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};
use wasm_bindgen::prelude::*;
use web_sys::{console, DedicatedWorkerGlobalScope, MessageEvent};
use winter_air::Air;
use winter_crypto::{hashers::Blake2s_256, ByteDigest, Digest, MerkleTree};
use winter_prover::{Matrix, Prover, ProverChannel, Serializable, StarkDomain, Trace};

pub struct ResolvableFuture<T> {
    pub result: Rc<RefCell<Vec<T>>>,
    pub exepected_size: usize,
}

impl<T> Future for ResolvableFuture<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let cur_len = (*self.result).borrow();
        if cur_len.len() == self.exepected_size {
            return Poll::Ready(());
        } else {
            // wait every second
            let wait_fn = {
                let waker = Rc::new(cx.waker().clone());
                Closure::wrap(Box::new(move || {
                    waker.as_ref().clone().wake();
                }) as Box<dyn Fn()>)
            };
            let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
            let _ = global.set_timeout_with_callback_and_timeout_and_arguments_0(
                wait_fn.as_ref().unchecked_ref(),
                200,
            );
            wait_fn.forget();
            return Poll::Pending;
        }
    }
}

#[wasm_bindgen]
pub struct MidenProverAsyncWorker {
    trace: Option<ExecutionTrace>,
    program: Option<Program>,
    program_inputs: Option<ProgramInputs>,
    program_outputs: Option<ProgramOutputs>,
    proof_options: Option<ProofOptions>,
    channel: Option<ProverChannel<<ExecutionProver as Prover>::Air, Felt, Blake2s_256<Felt>>>,
    trace_polys: Option<Matrix<Felt>>,
    trace_lde: Option<Matrix<Felt>>,
    worker_pool: WorkerPool,
    trace_row_hashes: Rc<RefCell<Vec<(usize, Vec<ByteDigest<32>>)>>>,
    chunk_size: Option<usize>,
    prover: Option<ExecutionProver>,
    air: Option<ProcessorAir>,
}

#[wasm_bindgen]
impl MidenProverAsyncWorker {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<MidenProverAsyncWorker, JsValue> {
        set_once_logger();
        let worker_pool = WorkerPool::new()?;
        Ok(Self {
            trace: None,
            program: None,
            program_inputs: None,
            proof_options: None,
            program_outputs: None,
            channel: None,
            trace_polys: None,
            trace_lde: None,
            worker_pool,
            trace_row_hashes: Rc::new(RefCell::new(Vec::new())),
            chunk_size: None,
            prover: None,
            air: None,
        })
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) -> Result<MidenProverAsyncWorker, JsValue> {
        Ok(Self {
            trace: None,
            program: None,
            program_inputs: None,
            proof_options: None,
            program_outputs: None,
            channel: None,
            trace_polys: None,
            trace_lde: None,
            worker_pool: self.worker_pool.clone(),
            trace_row_hashes: Rc::new(RefCell::new(Vec::new())),
            chunk_size: None,
            prover: None,
            air: None,
        })
    }

    async fn prove(&mut self, proving_work_item: ProvingWorkItem) -> Result<ProverOutput, JsValue> {
        console::time_with_label("preparing_inputs");
        let miden_program = sdk::MidenProgram::decode(&proving_work_item.program[..])
            .expect("Cannot decode miden program");
        let miden_program_inputs =
            sdk::MidenProgramInputs::decode(&proving_work_item.program_inputs[..])
                .expect("Cannot decode miden program inputs");
        let proof_options = sdk::ProofOptions::decode(&proving_work_item.proof_options[..])
            .expect("Cannot decode proof options");

        self.program = Some(miden_program.into());
        self.program_inputs = Some(miden_program_inputs.into());
        self.proof_options = Some(proof_options.into());
        console::time_end_with_label("preparing_inputs");
        console::time_with_label("generating_trace");

        self.build_execution_trace()?;
        console::time_end_with_label("generating_trace");
        console::time_with_label("prove_program_stage1");

        // execute program and generate proof
        self.prove_stage_1()?;
        console::time_end_with_label("prove_program_stage1");
        console::time_with_label("prove_trace_hashes");
        self.prove_trace_hashes(proving_work_item.chunk_size)
            .await?;
        console::time_end_with_label("prove_trace_hashes");
        // build Merkle tree out of hashed rows
        let mut trace_row_hashes = vec![];

        self.trace_row_hashes.borrow_mut().sort_by_key(|v| v.0);

        // Append the vecs to the result vec in order
        for (_, vec) in self.trace_row_hashes.borrow().iter() {
            trace_row_hashes.extend(vec.clone());
        }

        let main_trace_tree: MerkleTree<Blake2s_256<Felt>> =
            MerkleTree::new(trace_row_hashes).expect("failed to construct trace Merkle tree");
        debug!("Merkle root: {:?}", main_trace_tree.root().into_js_value());

        let prover = ExecutionProver::new(
            self.proof_options.clone().unwrap(),
            self.program_inputs.clone().unwrap().stack_init().to_vec(),
            self.program_outputs.clone().unwrap(),
        );
        console::time_with_label("prove_final_stage");
        let channel_unpacked = self.channel.take().unwrap();
        let proof = prover
            .prove_after_build_trace_commitment(
                self.air.clone().unwrap(),
                channel_unpacked,
                main_trace_tree,
                self.trace_lde.take().unwrap(),
                self.trace_polys.take().unwrap(),
                self.trace.take().unwrap(),
            )
            .map_err(|err| format!("Cannot run prove_after_build_trace_commitment: {:?}", err))?;
        console::time_end_with_label("prove_final_stage");

        let pub_inputs = PublicInputs::new(
            self.program.clone().unwrap().hash(),
            self.program_inputs.clone().unwrap().stack_init().to_vec(),
            self.program_outputs.clone().unwrap(),
        );
        let air = ProcessorAir::new(
            proof.get_trace_info(),
            pub_inputs.clone(),
            proof.options().clone(),
        );

        let stack_inputs: Vec<u64> = self
            .program_inputs
            .clone()
            .unwrap()
            .stack_init()
            .iter()
            // for whatever reason miden reverses the stack
            .rev()
            .map(|e| e.as_int())
            .collect();

        console::time_with_label("verify_program");
        verify(
            self.program.clone().unwrap().hash(),
            &stack_inputs[..],
            &self.program_outputs.clone().unwrap(),
            proof.clone(),
        )
        .map_err(|e| format!("Could not verify proof due to {}", e))?;
        console::time_end_with_label("verify_program");

        info!(
            "proof size: {:.1} KB",
            proof.to_bytes().len() as f64 / 1024f64
        );
        let sdk_proof: sdk::StarkProof = sdk::StarkProof::into_sdk(proof, &air);
        info!(
            "SDK Proof size: {:.1} KB",
            sdk_proof.encode_to_vec().len() as f64 / 1024f64
        );
        let sdk_outputs: sdk::MidenProgramOutputs = self.program_outputs.clone().unwrap().into();
        let sdk_pub_inputs: sdk::MidenPublicInputs = pub_inputs.into();
        let js_output = ProverOutput {
            proof: sdk_proof.encode_to_vec(),
            program_outputs: sdk_outputs.encode_to_vec(),
            public_inputs: sdk_pub_inputs.encode_to_vec(),
        };
        Ok(js_output)
    }

    fn build_execution_trace(&mut self) -> Result<(), JsValue> {
        let trace = miden_processor::execute(
            &self.program.clone().unwrap(),
            &self.program_inputs.clone().unwrap(),
        )
        .map_err(|_| "Could not generate miden trace")?;
        self.program_outputs = Some(trace.program_outputs().clone());
        self.trace = Some(trace);
        Ok(())
    }

    // start the proving process, generate the main trace
    // before commitment will be dispatched to workers
    fn prove_stage_1(&mut self) -> Result<(), JsValue> {
        self.prover = Some(ExecutionProver::new(
            self.proof_options.clone().unwrap(),
            self.program_inputs.clone().unwrap().stack_init().to_vec(),
            self.program_outputs.clone().unwrap(),
        ));
        let trace = self.trace.clone().unwrap();
        // 0 ----- instantiate AIR and prover channel ---------------------------------------------

        // serialize public inputs; these will be included in the seed for the public coin
        let pub_inputs = self.prover.as_ref().unwrap().get_pub_inputs(&trace);
        let mut pub_inputs_bytes = Vec::new();
        pub_inputs.write_into(&mut pub_inputs_bytes);

        // create an instance of AIR for the provided parameters. this takes a generic description
        // of the computation (provided via AIR type), and creates a description of a specific
        // execution of the computation for the provided public inputs.
        self.air = Some(ProcessorAir::new(
            trace.get_info(),
            pub_inputs,
            self.proof_options.clone().unwrap().0,
        ));

        // create a channel which is used to simulate interaction between the prover and the
        // verifier; the channel will be used to commit to values and to draw randomness that
        // should come from the verifier.
        self.channel = Some(ProverChannel::<
            <ExecutionProver as Prover>::Air,
            Felt,
            Blake2s_256<Felt>,
        >::new(self.air.clone().unwrap(), pub_inputs_bytes));

        // start building the trace commitments
        let domain = StarkDomain::new(&self.air.clone().unwrap());
        let main_trace = trace.main_segment();
        let trace_polys = main_trace.interpolate_columns();
        self.trace_lde = Some(trace_polys.evaluate_columns_over(&domain));
        self.trace_polys = Some(trace_polys);

        Ok(())
    }

    async fn prove_trace_hashes(&mut self, chunk_size: usize) -> Result<(), JsValue> {
        let trace_lde = self.trace_lde.as_ref().unwrap();

        debug!("trace_lde: {:?}", trace_lde.num_rows());
        self.trace_row_hashes = Rc::new(RefCell::new(vec![]));
        self.chunk_size = Some(chunk_size);
        // this is fine since trace length is a power of 2
        let num_of_batches = trace_lde.num_rows() / chunk_size;
        let mut dispatched_idx = 0;

        for i in 0..num_of_batches {
            let mut batch = vec![];
            for _ in 0..chunk_size {
                let mut row = VecWrapper(vec![Felt::ZERO; trace_lde.num_cols()]);
                trace_lde.read_row_into(dispatched_idx, &mut row.0);
                batch.push(row);
                dispatched_idx += 1;
            }
            self.worker_pool.run(i, batch, self.get_on_msg_callback())?;
        }
        // await all messages to process
        let fut = ResolvableFuture {
            result: self.trace_row_hashes.clone(),
            exepected_size: num_of_batches,
        };

        fut.await;

        Ok(())
    }

    fn prove_sequential(
        &mut self,
        proving_work_item: ProvingWorkItem,
    ) -> Result<ProverOutput, JsValue> {
        console::time_with_label("preparing_inputs");
        let miden_program = sdk::MidenProgram::decode(&proving_work_item.program[..])
            .expect("Cannot decode miden program");
        let miden_program_inputs =
            sdk::MidenProgramInputs::decode(&proving_work_item.program_inputs[..])
                .expect("Cannot decode miden program inputs");
        let proof_options = sdk::ProofOptions::decode(&proving_work_item.proof_options[..])
            .expect("Cannot decode proof options");

        self.program = Some(miden_program.into());
        self.program_inputs = Some(miden_program_inputs.into());
        self.proof_options = Some(proof_options.into());
        console::time_end_with_label("preparing_inputs");
        self.build_execution_trace()?;
        console::time_with_label("prove_full");
        let prover = ExecutionProver::new(
            self.proof_options.clone().unwrap(),
            self.program_inputs.clone().unwrap().stack_init().to_vec(),
            self.program_outputs.clone().unwrap(),
        );
        let proof = prover
            .prove(self.trace.clone().unwrap())
            .map_err(|err| format!("Failed to prove program - {:?}", err))?;
        console::time_end_with_label("prove_full");
        let pub_inputs = PublicInputs::new(
            self.program.clone().unwrap().hash(),
            self.program_inputs.clone().unwrap().stack_init().to_vec(),
            self.program_outputs.clone().unwrap(),
        );
        let air = ProcessorAir::new(
            proof.get_trace_info(),
            pub_inputs.clone(),
            proof.options().clone(),
        );

        let stack_inputs: Vec<u64> = self
            .program_inputs
            .clone()
            .unwrap()
            .stack_init()
            .iter()
            //for whatever reason miden reverses the stack
            .rev()
            .map(|e| e.as_int())
            .collect();

        console::time_with_label("verify_program");
        verify(
            self.program.clone().unwrap().hash(),
            &stack_inputs[..],
            &self.program_outputs.clone().unwrap(),
            proof.clone(),
        )
        .map_err(|e| format!("Could not verify proof due to {}", e))?;
        console::time_end_with_label("verify_program");

        info!(
            "proof size: {:.1} KB",
            proof.to_bytes().len() as f64 / 1024f64
        );
        let sdk_proof: sdk::StarkProof = sdk::StarkProof::into_sdk(proof, &air);
        info!(
            "SDK Proof size: {:.1} KB",
            sdk_proof.encode_to_vec().len() as f64 / 1024f64
        );
        let sdk_outputs: sdk::MidenProgramOutputs = self.program_outputs.clone().unwrap().into();
        let sdk_pub_inputs: sdk::MidenPublicInputs = pub_inputs.into();
        let js_output = ProverOutput {
            proof: sdk_proof.encode_to_vec(),
            program_outputs: sdk_outputs.encode_to_vec(),
            public_inputs: sdk_pub_inputs.encode_to_vec(),
        };
        Ok(js_output)
    }

    /// Message passing by the main thread
    fn get_on_msg_callback(&self) -> Closure<dyn FnMut(MessageEvent)> {
        let trace_row_hashes = self.trace_row_hashes.clone();
        let callback = Closure::new(move |event: MessageEvent| {
            debug!("Proving get_on_msg_callback thread got message");
            let data: Uint8Array = Uint8Array::new(&event.data());
            let hashing_result: HashingResult = from_uint8array(&data);
            let hashes = hashing_result
                .hashes
                .into_iter()
                .map(|d| ByteDigest::new(d))
                .collect();
            trace_row_hashes
                .borrow_mut()
                .push((hashing_result.batch_idx, hashes));
        });

        callback
    }
}

#[wasm_bindgen]
pub async fn proving_entry_point(
    prover: &mut MidenProverAsyncWorker,
    msg: MessageEvent,
) -> Result<(), JsValue> {
    set_once_logger();
    debug!("got proving workload");
    let data: Uint8Array = Uint8Array::new(&msg.data());
    let proving_work_item: ProvingWorkItem = from_uint8array(&data);
    let prover_output = if proving_work_item.is_sequential {
        prover.prove_sequential(proving_work_item)?
    } else {
        prover.prove(proving_work_item).await?
    };
    let payload = to_uint8array(&prover_output);
    let global_scope = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    global_scope.post_message(&payload)?;
    debug!("sent payload back to main thread");
    Ok(())
}
