import { prove } from "../sdk";
import init from "miden-wasm";
import { MidenProgram, MidenProgramInputs } from "../proto-ts/miden_prover";


await init();

document.querySelector("body").innerHTML = `<h1>Hello World!</h1>`;
console.log("Hello!");

let program = MidenProgram.fromJSON({
    program:
        `begin 
            repeat.10
                swap dup.1 add
            end
        end`
});
let inputs = MidenProgramInputs.fromJSON({ stackInit: [0, 1], adviceTape: [] });
await prove(program, inputs);