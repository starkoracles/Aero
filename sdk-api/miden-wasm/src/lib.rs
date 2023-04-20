#![feature(once_cell)]

use js_sys::Array;
use log::info;
use miden::prove;
use miden_air::{Felt, FieldElement, ProcessorAir, PublicInputs};
use prost::Message;
use std::sync::Once;
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use web_sys::{console, MessageEvent, Worker};
use winter_air::Air;

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
pub fn miden_prove(
    program: Vec<u8>,
    program_inputs: Vec<u8>,
    proof_options: Vec<u8>,
) -> Result<ProverOutput, JsValue> {
    worker_entry_point()?;
    console::time_with_label("preparing_inputs");
    info!("============================================================");
    info!("Reading program and inputs");
    info!("============================================================");

    let miden_program =
        sdk::MidenProgram::decode(&program[..]).expect("Cannot decode miden program");
    let miden_program_inputs = sdk::MidenProgramInputs::decode(&program_inputs[..])
        .expect("Cannot decode miden program inputs");
    let proof_options =
        sdk::ProofOptions::decode(&proof_options[..]).expect("Cannot decode proof options");

    let program = miden_program.into();
    let program_inputs = miden_program_inputs.into();

    info!("============================================================");
    info!("Prove program");
    info!("============================================================");
    console::time_end_with_label("preparing_inputs");
    console::time_with_label("prove_program");

    // execute program and generate proof
    let (outputs, proof) = prove(&program, &program_inputs, &proof_options.into())
        .map_err(|err| format!("Failed to prove program - {:?}", err))?;

    console::time_end_with_label("prove_program");

    let pub_inputs = PublicInputs::new(
        program.hash(),
        program_inputs.stack_init().to_vec(),
        outputs.clone(),
    );
    let air = ProcessorAir::new(
        proof.get_trace_info(),
        pub_inputs.clone(),
        proof.options().clone(),
    );

    info!(
        "proof size: {:.1} KB",
        proof.to_bytes().len() as f64 / 1024f64
    );
    let sdk_proof: sdk::StarkProof = sdk::StarkProof::into_sdk(proof, &air);
    info!(
        "SDK Proof size: {:.1} KB",
        sdk_proof.encode_to_vec().len() as f64 / 1024f64
    );
    let sdk_outputs: sdk::MidenProgramOutputs = outputs.into();
    let sdk_pub_inputs: sdk::MidenPublicInputs = pub_inputs.into();
    let js_output = ProverOutput {
        proof: sdk_proof.encode_to_vec(),
        program_outputs: sdk_outputs.encode_to_vec(),
        public_inputs: sdk_pub_inputs.encode_to_vec(),
    };
    Ok(js_output)
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
