use crate::convert::convert_proof::*;
use crate::convert::sdk::sdk;
use crate::pool::WorkerPool;
use crate::utils::{
    from_uint8array, set_once_logger, to_uint8array, ComputationFragment, ConstraintComputeResult,
    ConstraintComputeWorkItem, HashingResult, ProverOutput, ProvingWorkItem,
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
use winter_air::{Air, AuxTraceRandElements};
use winter_crypto::{hashers::Blake2s_256, ByteDigest, MerkleTree};
use winter_prover::{
    ConstraintEvaluationTable, ConstraintEvaluator, Matrix, Prover, ProverChannel, Serializable,
    StarkDomain, StarkProof, Trace, TraceLde,
};

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
        debug!("Merkle root: {:x}", main_trace_tree.root());

        let prover = ExecutionProver::new(
            self.proof_options.clone().unwrap(),
            self.program_inputs.clone().unwrap().stack_init().to_vec(),
            self.program_outputs.clone().unwrap(),
        );
        console::time_with_label("prove_final_stage");
        let proof = self.prove_epilogue(&prover, main_trace_tree)?;
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
                let mut row = vec![Felt::ZERO; trace_lde.num_cols()];
                trace_lde.read_row_into(dispatched_idx, &mut row);
                let converted = row.iter().map(|f| f.into()).collect();
                batch.push(converted);
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

    fn prove_epilogue(
        &mut self,
        prover: &ExecutionProver,
        main_trace_tree: MerkleTree<Blake2s_256<Felt>>,
    ) -> Result<StarkProof, JsValue> {
        let mut channel = self.channel.take().unwrap();
        let air = self.air.as_ref().unwrap();
        let domain = StarkDomain::new(air);
        let main_trace_lde = self.trace_lde.take().unwrap();
        let main_trace_polys = self.trace_polys.take().unwrap();
        let mut trace = self.trace.take().unwrap();
        let (trace_polys, trace_commitment, aux_trace_rand_elements) = prover
            .commit_to_trace_and_validate(
                &air,
                &mut channel,
                main_trace_tree,
                main_trace_lde,
                main_trace_polys,
                &mut trace,
            )
            .map_err(|err| format!("Cannot run commit_to_trace_and_validate: {:?}", err))?;

        let constraint_evaluations = self.evaluate_constraints(
            &mut channel,
            trace_commitment.trace_table(),
            aux_trace_rand_elements.clone(),
            &domain,
        )?;
        Ok(prover
            .prove_after_constraint_eval(
                &air,
                channel,
                constraint_evaluations,
                trace_polys,
                trace_commitment,
            )
            .map_err(|err| format!("Cannot run prove_after_build_trace_commitment: {:?}", err))?)
    }

    fn evaluate_constraints<'a>(
        &'a self,
        channel: &mut ProverChannel<<ExecutionProver as Prover>::Air, Felt, Blake2s_256<Felt>>,
        trace_table: &TraceLde<Felt>,
        aux_trace_rand_elements: AuxTraceRandElements<Felt>,
        domain: &'a StarkDomain<Felt>,
    ) -> Result<ConstraintEvaluationTable<Felt>, JsValue> {
        let air = self.air.as_ref().unwrap();
        // 2 ----- evaluate constraints -----------------------------------------------------------
        // evaluate constraints specified by the AIR over the constraint evaluation domain, and
        // compute random linear combinations of these evaluations using coefficients drawn from
        // the channel; this step evaluates only constraint numerators, thus, only constraints with
        // identical denominators are merged together. the results are saved into a constraint
        // evaluation table where each column contains merged evaluations of constraints with
        // identical denominators.
        let constraint_coeffs = channel.get_constraint_composition_coeffs();
        // build a list of constraint divisors; currently, all transition constraints have the same
        // divisor which we put at the front of the list; boundary constraint divisors are appended
        // after that
        let evaluator: ConstraintEvaluator<_, Felt> = ConstraintEvaluator::new(
            air,
            aux_trace_rand_elements.clone(),
            constraint_coeffs.clone(),
        );
        // build a list of constraint divisors; currently, all transition constraints have the same
        // divisor which we put at the front of the list; boundary constraint divisors are appended
        // after that
        let mut divisors = vec![evaluator.transition_constraints.divisor().clone()];
        divisors.append(&mut evaluator.boundary_constraints.get_divisors());

        // allocate space for constraint evaluations; when we are in debug mode, we also allocate
        // memory to hold all transition constraint evaluations (before they are merged into a
        // single value) so that we can check their degrees later
        #[cfg(not(debug_assertions))]
        let mut evaluation_table = ConstraintEvaluationTable::<E>::new(domain, divisors);
        #[cfg(debug_assertions)]
        let mut evaluation_table = ConstraintEvaluationTable::<Felt>::new(
            &domain,
            divisors,
            &evaluator.transition_constraints,
        );
        let mut fragments = evaluation_table.fragments(8);
        let frag_num = 8;
        let pub_inputs = PublicInputs::new(
            self.program.clone().unwrap().hash(),
            self.program_inputs.clone().unwrap().stack_init().to_vec(),
            self.program_outputs.clone().unwrap(),
        );
        let proof_options = self.proof_options.as_ref().unwrap().0.clone();
        for i in 0..frag_num {
            let constraint_work_item = ConstraintComputeWorkItem {
                trace_info: air.trace_info().clone(),
                public_inputs: pub_inputs.clone(),
                proof_options: proof_options.clone(),
                trace_lde: trace_table.clone(),
                constraint_coeffs: constraint_coeffs.clone(),
                aux_rand_elements: aux_trace_rand_elements.clone(),
                computation_fragment: ComputationFragment {
                    num_fragments: frag_num,
                    fragment_offset: i,
                },
            };
            self.worker_pool
                .run_constraint(constraint_work_item, self.get_on_msg_callback_constraints())?;
        }
        for i in 0..frag_num {
            let frag = &mut fragments[i];
            evaluator.evaluate_fragment(trace_table, &domain, frag);
        }
        info!("{:?}", &evaluation_table.evaluations);
        Ok(evaluation_table)
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
            let hashing_result: HashingResult = from_uint8array(&data).unwrap();
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

    fn get_on_msg_callback_constraints(&self) -> Closure<dyn FnMut(MessageEvent)> {
        let callback = Closure::new(move |event: MessageEvent| {
            let result =
                from_uint8array::<ConstraintComputeResult>(&Uint8Array::new(&event.data()))
                    .unwrap();
            info!(
                "Constraint get_on_msg_callback thread got message, {:?}",
                result
            );
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
    if let Ok(proving_work_item) = from_uint8array::<ProvingWorkItem>(&data) {
        let prover_output = if proving_work_item.is_sequential {
            prover.prove_sequential(proving_work_item)?
        } else {
            prover.prove(proving_work_item).await?
        };
        let payload = to_uint8array(&prover_output);
        let global_scope = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
        global_scope.post_message(&payload)?;
        debug!("sent payload back to main thread");
    } else {
        debug!("failed to decode proving workload");
    }
    Ok(())
}
