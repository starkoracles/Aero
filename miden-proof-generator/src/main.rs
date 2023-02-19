use miden::{prove, Assembler, Program, ProgramInputs, ProofOptions};
use miden_air::PublicInputs;
use miden_core::{utils::Serializable, Felt, FieldElement, StarkField};
use miden_stdlib::StdLibrary;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

#[derive(Serialize, Deserialize)]
struct ProofData {
    input_bytes: Vec<u8>,
    proof_bytes: Vec<u8>,
}

fn main() {
    println!("============================================================");
    println!("Prove program");
    println!("============================================================");

    // configure logging
    env_logger::Builder::new()
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .filter_level(log::LevelFilter::Debug)
        .init();

    let n = 10;
    let program = generate_fibonacci_program(n);
    let expected_result = vec![compute_fibonacci(n).as_int()];
    let proof_security = ProofOptions::with_96_bit_security();
    let input_data = ProgramInputs::new(&[0, 1], &[], vec![]).unwrap();
    println!(
        "Generated a program to compute {}-th Fibonacci term; expected result: {}",
        n, expected_result[0]
    );

    // execute program and generate proof
    let (outputs, proof) = prove(&program, &input_data, &proof_security)
        .map_err(|err| format!("Failed to prove program - {:?}", err))
        .unwrap();

    let mut input_bytes = vec![];
    PublicInputs::new(program.hash(), input_data.stack_init().to_vec(), outputs)
        .write_into(&mut input_bytes);
    let proof_bytes = proof.to_bytes();
    println!("Proof size: {:.1} KB", proof_bytes.len() as f64 / 1024f64);

    // Write proof to disk
    let data = ProofData {
        input_bytes,
        proof_bytes,
    };
    let b = bincode::serialize(&data).unwrap();
    let mut f = File::create("proofs/fib.bin").unwrap();
    f.write_all(&b).unwrap();
}

/// Generates a program to compute the `n`-th term of Fibonacci sequence
fn generate_fibonacci_program(n: usize) -> Program {
    // the program is a simple repetition of 4 stack operations:
    // the first operation moves the 2nd stack item to the top,
    // the second operation duplicates the top 2 stack items,
    // the third operation removes the top item from the stack
    // the last operation pops top 2 stack items, adds them, and pushes
    // the result back onto the stack
    let program = format!(
        "begin 
            repeat.{}
                swap dup.1 add
            end
        end",
        n - 1
    );

    Assembler::new()
        .with_module_provider(StdLibrary::default())
        .compile(&program)
        .unwrap()
}

/// Computes the `n`-th term of Fibonacci sequence
fn compute_fibonacci(n: usize) -> Felt {
    let mut t0 = Felt::ZERO;
    let mut t1 = Felt::ONE;

    for _ in 0..n {
        t1 = t0 + t1;
        core::mem::swap(&mut t0, &mut t1);
    }
    t0
}
