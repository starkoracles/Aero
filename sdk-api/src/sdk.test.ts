import { prove } from "./sdk";
import { MidenProgram, MidenProgramInputs } from "./proto-ts/miden_prover";

describe('sdk prove', () => {
    test("prove should run on fib", () => {
        let program = MidenProgram.fromJSON({
            program:
                `begin 
                    repeat.10
                        swap dup.1 add
                    end
                end`
        });
        let inputs = MidenProgramInputs.fromJSON({ stackInit: [0, 1], adviceTape: [] });
        prove(program, inputs);
    });
});