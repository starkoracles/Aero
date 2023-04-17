use log::info;
use miden::prove;
use miden_air::{ProcessorAir, PublicInputs};
use prost::Message;
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use web_sys::console;
use winter_air::Air;

pub mod convert;

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
pub fn miden_prove(
    program: Vec<u8>,
    program_inputs: Vec<u8>,
    proof_options: Vec<u8>,
) -> Result<ProverOutput, JsValue> {
    log::set_logger(&DEFAULT_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);

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
