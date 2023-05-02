import { prove, prove_sequential, uint8ArrayToU64LE } from "../sdk";
import { MidenProgram, MidenProgramInputs } from "../proto-ts/miden_prover";
import "../hashing_worker";
import "../proving_worker";

const FIB_NUM = 10;

async function onPageLoad() {
  document.querySelector("body").innerHTML = `<h1>Proving the ${FIB_NUM}th fib number!</h1><button id="run_proof">Run Proof</button><button id="run_proof_sequential">Run Proof sequential</button><h2 id="result"></h2>`;
  console.log("Hello!");
  document.addEventListener('DOMContentLoaded', function () {
    const button = document.getElementById('run_proof');

    button.addEventListener('click', async () => {
      await runProof();
    });

    const button_seq = document.getElementById('run_proof_sequential');

    button_seq.addEventListener('click', async () => {
      await runProofSequential();
    });
  });
}

async function runProof() {
  console.log("Running proof");
  console.time("running_proof");
  return new Promise<void>((resolve) => {
    setTimeout(async () => {
      console.log("in proof");
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
      let inputs = MidenProgramInputs.fromJSON({ stackInit: [FIB_NUM], adviceTape: [] });
      const [, outputs,] = await prove(program, inputs);

      let result = uint8ArrayToU64LE(outputs.stack[0].element);

      document.getElementById("result").innerHTML = "Result: " + result.toString();
      console.log("Result: ", result);
      console.timeEnd("running_proof");
      resolve();
    });
  });
}

async function runProofSequential() {
  console.log("Running proof sequential");
  console.time("running_proof_sequential");
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
      let inputs = MidenProgramInputs.fromJSON({ stackInit: [FIB_NUM], adviceTape: [] });
      const [, outputs,] = await prove_sequential(program, inputs);

      let result = uint8ArrayToU64LE(outputs.stack[0].element);

      document.getElementById("result").innerHTML = "Result: " + result.toString();
      console.log("Result: ", result);
      console.timeEnd("running_proof_sequential");
      resolve();
    });
  });
}

onPageLoad();