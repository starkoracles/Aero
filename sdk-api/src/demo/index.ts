import { prove } from "../sdk";
import init from "miden-wasm";
import { MidenProgram, MidenProgramInputs } from "../proto-ts/miden_prover";

async function onPageLoad() {
    const proofPromise = runProof();
    document.querySelector("body").innerHTML = `<h1>Proving the 10th fib number!</h1>`;
    console.log("Hello!");

    await proofPromise;
}

async function runProof() {
    return new Promise<void>((resolve) => {
        setTimeout(async () => {
            let program = MidenProgram.fromJSON({
                program:
                    `begin 
                        repeat.10
                            swap dup.1 add
                        end
                    end`
            });
            let inputs = MidenProgramInputs.fromJSON({ stackInit: [0, 1], adviceTape: [] });
            await init();
            prove(program, inputs);
            resolve();
        }, 3000);
    });
}

onPageLoad();