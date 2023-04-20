import { prove, uint8ArrayToU64LE } from "../sdk";
import init, { start } from "miden-wasm";
import { MidenProgram, MidenProgramInputs } from "../proto-ts/miden_prover";
import "../hashing_worker";

async function onPageLoad() {
    document.querySelector("body").innerHTML = `<h1>Proving the 10th fib number!</h1><button id="run_proof">Run Proof</button><h2 id="result"></h2>`;
    console.log("Hello!");
    setTimeout(async () => {
        await init();
        start();
        console.log("after wasm init");
    }, 3000);
    document.addEventListener('DOMContentLoaded', function () {
        const button = document.getElementById('run_proof');

        button.addEventListener('click', async () => {
            await runProof();
        });
    });
}

async function runProof() {
    console.log("Running proof");
    return new Promise<void>((resolve) => {
        setTimeout(async () => {
            let program = MidenProgram.fromJSON({
                program:
                    `
                    # STACK EFFECT
                    # ITERATION-AMOUNT -- FIB-ANSWER #
                    proc.fib_iter
                      push.0
                      push.1
                      dup.2
                      neq.0
                      # Looks about 8 cyles every loop #
                      while.true
                        swap dup.1 add movup.2 sub.1 dup movdn.3 neq.0
                      end
                      drop
                      swap
                      drop
                    end
                    
                    begin
                      exec.fib_iter
                    end`
            });
            let inputs = MidenProgramInputs.fromJSON({ stackInit: [200], adviceTape: [] });
            const [, outputs,] = prove(program, inputs);

            let result = uint8ArrayToU64LE(outputs.stack[0].element);

            document.getElementById("result").innerHTML = "Result: " + result.toString();
            console.log("Result: ", result);
            resolve();
        }, 3000);
    });
}

onPageLoad();