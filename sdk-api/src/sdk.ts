import { miden_prove } from "miden-wasm";
import { MidenProgram, MidenProgramInputs } from "./proto-ts/miden_prover";
import { StarkProof } from "./proto-ts/stark_proof";
import { MidenProgramOutputs, MidenPublicInputs } from "./proto-ts/miden_vm";
import { FieldExtension, HashFunction, PrimeField, ProofOptions } from "./proto-ts/context";

export function prove(program: MidenProgram, inputs: MidenProgramInputs, options: ProofOptions = ProofOptions.fromJSON({
    numQueries: 27,
    blowupFactor: 8,
    grindingFactor: 16,
    hashFn: HashFunction.BLAKE2S,
    fieldExtension: FieldExtension.NONE,
    friFoldingFactor: 8,
    friMaxRemainderSize: 256,
    primeField: PrimeField.GOLDILOCKS,
})): [StarkProof, MidenProgramOutputs, MidenPublicInputs] {
    let program_bytes = MidenProgram.encode(program).finish();
    let input_bytes = MidenProgramInputs.encode(inputs).finish();
    let option_bytes = ProofOptions.encode(options).finish();
    let proof_outputs = miden_prove(program_bytes, input_bytes, option_bytes);

    let proof = StarkProof.decode(proof_outputs.proof);
    let outputs = MidenProgramOutputs.decode(proof_outputs.program_outputs);
    let pub_inputs = MidenPublicInputs.decode(proof_outputs.public_inputs);

    return [proof, outputs, pub_inputs];
}

export function uint8ArrayToU64LE(arr: Uint8Array): BigInt {
    if (arr.length !== 8) {
        throw new Error('Uint8Array must have exactly 8 elements to be converted to u64.');
    }

    let result = BigInt(0);
    for (let i = 0; i < arr.length; i++) {
        result |= BigInt(arr[i]) << BigInt(i * 8);
    }

    return result;
}
