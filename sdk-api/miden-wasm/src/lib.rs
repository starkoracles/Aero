#![feature(once_cell)]

use futures::{executor::block_on, Future, FutureExt};
use js_sys::Reflect::get;
use log::info;
use miden::{
    verify, Digest as MidenDigest, ExecutionTrace, Program, ProgramInputs, ProofOptions, StarkProof,
};
use miden_air::{Felt, FieldElement, ProcessorAir, PublicInputs, StarkField};
use miden_core::ProgramOutputs;
use miden_prover::ExecutionProver;
use pool::WorkerPool;
use prost::Message;
use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    sync::Once,
    task::{Context, Poll},
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use web_sys::{
    console::{self, info},
    MessageEvent,
};
use winter_air::Air;
use winter_crypto::{hashers::Blake2s_256, ByteDigest, Digest, MerkleTree};
use winter_prover::{Matrix, Prover, ProverChannel, Serializable, StarkDomain, Trace};

pub mod convert;

macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

pub mod pool;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn logv(x: &JsValue);
}

use crate::convert::sdk::sdk;
use convert::convert_proof::*;

#[wasm_bindgen(getter_with_clone)]
#[derive(serde::Serialize)]
pub struct ProverOutput {
    pub proof: Vec<u8>,
    pub program_outputs: Vec<u8>,
    pub public_inputs: Vec<u8>,
}

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    set_once_logger();
    Ok(())
}

struct ResolvableFuture<T> {
    result: Rc<RefCell<Vec<T>>>,
    exepected_size: usize,
}

impl<T> Future for ResolvableFuture<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let cur_len = (*self.result).borrow();
        info!(
            "data len: {}, expected_size: {}",
            cur_len.len(),
            self.exepected_size
        );
        if cur_len.len() == self.exepected_size {
            info!("resolved future");
            return Poll::Ready(());
        } else {
            // wait every second
            let wait_fn = {
                let waker = Rc::new(cx.waker().clone());
                Closure::wrap(Box::new(move || {
                    waker.as_ref().clone().wake();
                }) as Box<dyn Fn()>)
            };
            let _ = web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    wait_fn.as_ref().unchecked_ref(),
                    100,
                );
            wait_fn.forget();
            return Poll::Pending;
        }
    }
}

#[wasm_bindgen]
pub struct MidenProver {
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
}

#[wasm_bindgen]
impl MidenProver {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<MidenProver, JsValue> {
        Ok(Self {
            trace: None,
            program: None,
            program_inputs: None,
            proof_options: None,
            program_outputs: None,
            channel: None,
            trace_polys: None,
            trace_lde: None,
            worker_pool: WorkerPool::new(8)?,
            trace_row_hashes: Rc::new(RefCell::new(Vec::new())),
            chunk_size: None,
        })
    }

    #[wasm_bindgen]
    pub async fn prove(
        &mut self,
        program: Vec<u8>,
        program_inputs: Vec<u8>,
        proof_options: Vec<u8>,
        chunk_size: usize,
    ) -> Result<(), JsValue> {
        console::time_with_label("preparing_inputs");
        let miden_program =
            sdk::MidenProgram::decode(&program[..]).expect("Cannot decode miden program");
        let miden_program_inputs = sdk::MidenProgramInputs::decode(&program_inputs[..])
            .expect("Cannot decode miden program inputs");
        let proof_options =
            sdk::ProofOptions::decode(&proof_options[..]).expect("Cannot decode proof options");

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
        self.prove_trace_hashes(chunk_size).await?;
        console::time_end_with_label("prove_trace_hashes");
        // build Merkle tree out of hashed rows
        let mut trace_row_hashes = vec![];

        self.trace_row_hashes.borrow_mut().sort_by_key(|v| v.0);

        // Append the vecs to the result vec in order
        for (_, vec) in self.trace_row_hashes.borrow().iter() {
            trace_row_hashes.extend(vec.clone());
        }

        let r: MerkleTree<Blake2s_256<Felt>> =
            MerkleTree::new(trace_row_hashes).expect("failed to construct trace Merkle tree");
        info!("Merkle root: {:?}", r.root().into_js_value());
        Ok(())
        // let proof = self.prove_full()?;
        // console::time_end_with_label("prove_program");

        // let pub_inputs = PublicInputs::new(
        //     self.program.clone().unwrap().hash(),
        //     self.program_inputs.clone().unwrap().stack_init().to_vec(),
        //     self.program_outputs.clone().unwrap(),
        // );
        // let air = ProcessorAir::new(
        //     proof.get_trace_info(),
        //     pub_inputs.clone(),
        //     proof.options().clone(),
        // );

        // let stack_inputs: Vec<u64> = self
        //     .program_inputs
        //     .clone()
        //     .unwrap()
        //     .stack_init()
        //     .iter()
        //     // for whatever reason miden reverses the stack
        //     .rev()
        //     .map(|e| e.as_int())
        //     .collect();

        // console::time_with_label("verify_program");
        // verify(
        //     self.program.clone().unwrap().hash(),
        //     &stack_inputs[..],
        //     &self.program_outputs.clone().unwrap(),
        //     proof.clone(),
        // )
        // .map_err(|e| format!("Could not verify proof due to {}", e))?;
        // console::time_end_with_label("verify_program");

        // info!(
        //     "proof size: {:.1} KB",
        //     proof.to_bytes().len() as f64 / 1024f64
        // );
        // let sdk_proof: sdk::StarkProof = sdk::StarkProof::into_sdk(proof, &air);
        // info!(
        //     "SDK Proof size: {:.1} KB",
        //     sdk_proof.encode_to_vec().len() as f64 / 1024f64
        // );
        // let sdk_outputs: sdk::MidenProgramOutputs = self.program_outputs.clone().unwrap().into();
        // let sdk_pub_inputs: sdk::MidenPublicInputs = pub_inputs.into();
        // let js_output = ProverOutput {
        //     proof: sdk_proof.encode_to_vec(),
        //     program_outputs: sdk_outputs.encode_to_vec(),
        //     public_inputs: sdk_pub_inputs.encode_to_vec(),
        // };
        // Ok(js_output)
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
        let prover = ExecutionProver::new(
            self.proof_options.clone().unwrap(),
            self.program_inputs.clone().unwrap().stack_init().to_vec(),
            self.program_outputs.clone().unwrap(),
        );
        let trace = self.trace.clone().unwrap();
        // 0 ----- instantiate AIR and prover channel ---------------------------------------------

        // serialize public inputs; these will be included in the seed for the public coin
        let pub_inputs = prover.get_pub_inputs(&trace);
        let mut pub_inputs_bytes = Vec::new();
        pub_inputs.write_into(&mut pub_inputs_bytes);

        // create an instance of AIR for the provided parameters. this takes a generic description
        // of the computation (provided via AIR type), and creates a description of a specific
        // execution of the computation for the provided public inputs.
        let air: ProcessorAir = ProcessorAir::new(
            trace.get_info(),
            pub_inputs,
            self.proof_options.clone().unwrap().0,
        );

        // create a channel which is used to simulate interaction between the prover and the
        // verifier; the channel will be used to commit to values and to draw randomness that
        // should come from the verifier.
        self.channel = Some(ProverChannel::<
            <ExecutionProver as Prover>::Air,
            Felt,
            Blake2s_256<Felt>,
        >::new(air.clone(), pub_inputs_bytes));

        // start building the trace commitments
        let domain = StarkDomain::new(&air);
        let main_trace = trace.main_segment();
        let trace_polys = main_trace.interpolate_columns();
        self.trace_lde = Some(trace_polys.evaluate_columns_over(&domain));
        self.trace_polys = Some(trace_polys);

        Ok(())
    }

    async fn prove_trace_hashes(&mut self, chunk_size: usize) -> Result<(), JsValue> {
        let trace_polys = self.trace_polys.as_ref().unwrap();

        info!("trace_polys: {:?}", trace_polys.num_rows());
        self.trace_row_hashes = Rc::new(RefCell::new(vec![]));
        self.chunk_size = Some(chunk_size);
        // this is fine since trace length is a power of 2
        let num_of_batches = trace_polys.num_rows() / chunk_size;
        let mut dispatched_idx = 0;

        for i in 0..num_of_batches {
            let mut batch = vec![];
            for _ in 0..chunk_size {
                let mut row = vec![Felt::ZERO; trace_polys.num_cols()];
                trace_polys.read_row_into(dispatched_idx, &mut row);
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

    #[wasm_bindgen]
    pub fn prove_sequential(
        &mut self,
        program: Vec<u8>,
        program_inputs: Vec<u8>,
        proof_options: Vec<u8>,
    ) -> Result<ProverOutput, JsValue> {
        console::time_with_label("preparing_inputs");
        let miden_program =
            sdk::MidenProgram::decode(&program[..]).expect("Cannot decode miden program");
        let miden_program_inputs = sdk::MidenProgramInputs::decode(&program_inputs[..])
            .expect("Cannot decode miden program inputs");
        let proof_options =
            sdk::ProofOptions::decode(&proof_options[..]).expect("Cannot decode proof options");

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

    /// Create a closure to act on the message returned by the worker
    fn get_on_msg_callback(&self) -> Closure<dyn FnMut(MessageEvent)> {
        let v = self.trace_row_hashes.clone();
        let chunk_size = self.chunk_size.unwrap();
        let callback = Closure::new(move |event: MessageEvent| {
            let obj: js_sys::Object = event.data().unchecked_into();
            let hashes: js_sys::Array = get(&obj, &"elements_table".into())
                .unwrap()
                .unchecked_into();
            let batch_idx: usize =
                get(&obj, &"batch_idx".into()).unwrap().as_f64().unwrap() as usize;
            info!("got event from worker, batch_idx: {}", batch_idx);
            let mut trace_row_hashes = (*v).borrow_mut();
            let mut batch_hashes = vec![];
            for hash in hashes.iter() {
                let s: ByteDigest<32> = ByteDigest::from_js_value(hash);
                batch_hashes.push(s);
            }
            assert!(
                batch_hashes.len() == chunk_size,
                "batch hashes size is not equal to chunk size"
            );
            trace_row_hashes.push((batch_idx, batch_hashes));
        });

        callback
    }
}

#[inline]
fn set_once_logger() {
    static SET_SINGLETONS: Once = Once::new();
    SET_SINGLETONS.call_once(|| {
        log::set_logger(&DEFAULT_LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Info);
    });
}
