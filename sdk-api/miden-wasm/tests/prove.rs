use miden::ProofOptions;
use miden_wasm::{
    convert::sdk::sdk::{self, FieldExtension, HashFunction, PrimeField},
    miden_prove,
};
use prost::Message;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn prove_fib() {
    let program_10_fib = "
    begin 
        repeat.10
            swap dup.1 add
        end
    end";

    let program = sdk::MidenProgram {
        program: program_10_fib.to_string(),
        ..Default::default()
    };

    let program_inputs = sdk::MidenProgramInputs {
        stack_init: vec![0, 1],
        advice_tape: vec![],
        ..Default::default()
    };

    let proof_options = sdk::ProofOptions {
        num_queries: 27,
        blowup_factor: 8,
        grinding_factor: 16,
        hash_fn: HashFunction::Blake2s.into(),
        field_extension: FieldExtension::None.into(),
        fri_folding_factor: 8,
        fri_max_remainder_size: 256,
        prime_field: PrimeField::Goldilocks.into(),
        ..Default::default()
    };

    miden_prove(
        program.encode_to_vec(),
        program_inputs.encode_to_vec(),
        proof_options.encode_to_vec(),
    )
    .unwrap();
}
