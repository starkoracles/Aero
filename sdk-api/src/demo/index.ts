import { prove, uint8ArrayToU64LE } from "../sdk";
import init from "miden-wasm";
import { MidenProgram, MidenProgramInputs } from "../proto-ts/miden_prover";
import { Digest } from "../proto-ts/common";

async function onPageLoad() {
    document.querySelector("body").innerHTML = `<h1>Proving the 10th fib number!</h1><button id="run_proof">Run Proof</button><h2 id="result"></h2>`;
    console.log("Hello!");
    document.addEventListener('DOMContentLoaded', function () {
        const button = document.getElementById('run_proof');

        button.addEventListener('click', async () => {
            await runProof();
        });
    });
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
            const [, outputs,] = prove(program, inputs);

            let result = uint8ArrayToU64LE(outputs.stack[0].element);

            document.getElementById("result").innerHTML = "Result: " + result.toString();
            console.log("Result: ", result);
            resolve();
        }, 3000);
    });
}

onPageLoad();