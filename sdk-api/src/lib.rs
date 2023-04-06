use std::convert::TryInto;

use miden::{prove, Assembler, FieldExtension, HashFunction, Program, ProgramInputs, ProofOptions};
use miden_stdlib::StdLibrary;
use prost::Message;
use sdk::{MidenProgram, MidenProgramInputs};
use wasm_bindgen::prelude::*;

pub mod sdk {
    include!(concat!(env!("OUT_DIR"), "/sdk.rs"));
}

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

    let proof_bytes = proof.to_bytes();
    println!("Proof size: {:.1} KB", proof_bytes.len() as f64 / 1024f64);
}

impl Into<ProgramInputs> for MidenProgramInputs {
    fn into(self) -> ProgramInputs {
        ProgramInputs::new(
            &self.stack_init.iter().map(|e| e.into()).collect::<Vec<_>>()[..],
            &self
                .advice_tape
                .iter()
                .map(|e| e.into())
                .collect::<Vec<_>>()[..],
            vec![],
        )
        .expect("cannot parse miden program inputs")
    }
}

impl Into<u64> for &sdk::FieldElement {
    fn into(self) -> u64 {
        u64::from_le_bytes(self.element.clone().try_into().unwrap())
    }
}

impl Into<Program> for MidenProgram {
    fn into(self) -> Program {
        Assembler::new()
            .with_module_provider(StdLibrary::default())
            .compile(&self.program)
            .expect("cannot assemble miden program")
    }
}

impl Into<HashFunction> for sdk::HashFunction {
    fn into(self) -> HashFunction {
        match self {
            sdk::HashFunction::Blake2s => HashFunction::Blake2s_256,
        }
    }
}

impl Into<FieldExtension> for sdk::FieldExtension {
    fn into(self) -> FieldExtension {
        match self {
            sdk::FieldExtension::None => FieldExtension::None,
        }
    }
}

impl Into<ProofOptions> for sdk::ProofOptions {
    fn into(self) -> ProofOptions {
        ProofOptions::new(
            self.num_queries as usize,
            self.blowup_factor as usize,
            self.grinding_factor,
            self.hash_fn().into(),
            self.field_extension().into(),
            self.fri_folding_factor as usize,
            self.fri_max_remainder_size as usize,
        )
    }
}
