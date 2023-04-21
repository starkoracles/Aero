#![feature(once_cell)]

use js_sys::Array;
use log::info;
use miden::{verify, ExecutionTrace, Program, ProgramInputs, ProofOptions, StarkProof};
use miden_air::{Felt, FieldElement, ProcessorAir, PublicInputs, StarkField};
use miden_core::ProgramOutputs;
use miden_prover::ExecutionProver;
use prost::Message;
use std::sync::Once;
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use web_sys::{console, MessageEvent, Worker};
use winter_air::Air;
use winter_prover::Prover;

pub mod convert;

use crate::convert::sdk::sdk;
use convert::convert_proof::*;

static mut WORKER: Option<Worker> = None;

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
    setup_worker()?;
    Ok(())
}

#[wasm_bindgen]
pub fn blake2_hash_elements(element_table: Array) -> Result<Array, JsValue> {
    set_once_logger();
    info!("called with {:?}", &element_table);
    // we expect a 2d Array of JsValues that would translate into Felts
    for row in element_table.iter() {
        let row_array = row.dyn_into::<Array>()?;
        for column in row_array.iter() {
            let element = Felt::from_js_value(column);
            info!("element: {}", &element);
        }
    }
    Ok(Array::new_with_length(0))
}

#[wasm_bindgen]
pub struct MidenProver {
    trace: Option<ExecutionTrace>,
    program: Option<Program>,
    program_inputs: Option<ProgramInputs>,
    program_outputs: Option<ProgramOutputs>,
    proof_options: Option<ProofOptions>,
}

#[wasm_bindgen]
impl MidenProver {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            trace: None,
            program: None,
            program_inputs: None,
            proof_options: None,
            program_outputs: None,
        }
    }

    #[wasm_bindgen]
    pub fn prove(
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
        console::time_with_label("generating_trace");

        self.build_execution_trace()?;
        console::time_end_with_label("generating_trace");
        console::time_with_label("prove_program");

        // execute program and generate proof
        let proof = self.prove_stage_1()?;
        console::time_end_with_label("prove_program");

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
    fn prove_stage_1(&self) -> Result<StarkProof, JsValue> {
        let prover = ExecutionProver::new(
            self.proof_options.clone().unwrap(),
            self.program_inputs.clone().unwrap().stack_init().to_vec(),
            self.program_outputs.clone().unwrap(),
        );
        let proof = prover
            .prove(self.trace.clone().unwrap())
            .map_err(|err| format!("Failed to prove program - {:?}", err))?;
        Ok(proof)
    }
}

fn setup_worker() -> Result<(), JsValue> {
    info!("Setting worker up");
    let worker = Worker::new("hashing_worker.js")?;
    let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        let data = e.data();
        let msg = JsValue::from_str("worker result is");
        web_sys::console::log_2(&msg, &data);
    }) as Box<dyn FnMut(MessageEvent)>);
    worker.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    unsafe {
        WORKER = Some(worker);
    }

    onmessage_callback.forget();
    Ok(())
}

#[wasm_bindgen]
pub fn worker_entry_point() -> Result<(), JsValue> {
    info!("calling webworker");
    let param = vec![vec![Felt::new(1), Felt::new(2)]];

    let arg: Array = param
        .iter()
        .map(|r| r.iter().map(|e| e.into_js_value()).collect::<Array>())
        .collect::<Array>();

    info!("before post_message");
    let worker = unsafe {
        WORKER
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Worker not initialized"))?
    };

    worker.post_message(&arg)?;
    Ok(())
}

#[inline]
fn set_once_logger() {
    static SET_SINGLETONS: Once = Once::new();
    SET_SINGLETONS.call_once(|| {
        log::set_logger(&DEFAULT_LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Info);
    });
}
