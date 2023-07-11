import init, { MidenProverAsyncWorker } from "miden-wasm";
import { proving_entry_point } from "miden-wasm";

async function initialize(): Promise<MidenProverAsyncWorker> {
    await init();
    return new MidenProverAsyncWorker();
}

let full_init = initialize();
let prover: MidenProverAsyncWorker = null;

self.onmessage = async event => {
    console.trace("Proving worker received init:", event.data);
    prover = await full_init;
    self.onmessage = async event => {
        // maintain the reference to the worker pool
        let new_prover = prover.reset();
        await proving_entry_point(new_prover, event);
    };
};
