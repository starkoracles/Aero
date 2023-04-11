use miden::prove;
use prost::Message;
use wasm_bindgen::prelude::*;

mod convert;

use crate::convert::sdk::sdk;
use crate::convert::sdk::sdk::{MidenProgram, MidenProgramInputs};
use convert::convert_inputs::*;
use convert::convert_proof::*;

#[wasm_bindgen]
pub fn miden_prove(program: Vec<u8>, program_inputs: Vec<u8>, proof_options: Vec<u8>) -> () {
    let miden_program = MidenProgram::decode(&program[..]).expect("Cannot decode miden program");
    let miden_program_inputs = MidenProgramInputs::decode(&program_inputs[..])
        .expect("Cannot decode miden program inputs");
    let proof_options =
        sdk::ProofOptions::decode(&proof_options[..]).expect("Cannot decode proof options");

    println!("============================================================");
    println!("Prove program");
    println!("============================================================");

    let program = miden_program.into();
    let program_inputs = miden_program_inputs.into();

    // execute program and generate proof
    let (outputs, proof) = prove(&program, &program_inputs, &proof_options.into())
        .map_err(|err| format!("Failed to prove program - {:?}", err))
        .unwrap();

    println!(
        "proof size: {:.1} KB",
        proof.to_bytes().len() as f64 / 1024f64
    );
    // let sdk_proof: sdk::StarkProof = proof.into();
    // println!(
    //     "SDK Proof size: {:.1} KB",
    //     sdk_proof.encode_to_vec().len() as f64 / 1024f64
    // );
}
