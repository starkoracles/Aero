import { miden_prove } from "miden-wasm";
import { MidenProgram, MidenProgramInputs } from "./proto-ts/miden_prover";
import { FieldExtension, HashFunction, PrimeField, ProofOptions } from "./proto-ts/context";

export async function prove(program: MidenProgram, inputs: MidenProgramInputs, options: ProofOptions = ProofOptions.fromJSON({
    numQueries: 27,
    blowupFactor: 8,
    grindingFactor: 16,
    hashFn: HashFunction.BLAKE2S,
    fieldExtension: FieldExtension.NONE,
    friFoldingFactor: 8,
    friMaxRemainderSize: 256,
    primeField: PrimeField.GOLDILOCKS,
})) {
    let program_bytes = MidenProgram.encode(program).finish();
    let input_bytes = MidenProgramInputs.encode(inputs).finish();
    let option_bytes = ProofOptions.encode(options).finish();
    miden_prove(program_bytes, input_bytes, option_bytes);
}