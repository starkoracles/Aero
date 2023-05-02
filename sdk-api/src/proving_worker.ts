import init, { MidenProverAsyncWorker } from "miden-wasm";
import { proving_entry_point } from "miden-wasm";

self.onmessage = event => {
    console.log("Proving worker received init:", event.data);
    let initialised = init().catch(err => {
        // Propagate to main `onerror`:
        setTimeout(() => {
            throw err;
        });
        // Rethrow to keep promise rejected and prevent execution of further commands:
        throw err;
    });
    var prover = null;
    let full_init = initialised.then(() => {
        prover = new MidenProverAsyncWorker();
    });

    self.onmessage = async event => {
        // This will queue further commands up until the module is fully initialised:
        await full_init;
        await proving_entry_point(prover, event);
        postMessage("done");
    };
};
