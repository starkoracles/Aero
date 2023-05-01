import init from "miden-wasm";
import { child_entry_point } from "miden-wasm";

self.onmessage = event => {
    console.log("Worker received init:", event.data);
    let initialised = init().catch(err => {
        // Propagate to main `onerror`:
        setTimeout(() => {
            throw err;
        });
        // Rethrow to keep promise rejected and prevent execution of further commands:
        throw err;
    });

    self.onmessage = async event => {
        // This will queue further commands up until the module is fully initialised:
        await initialised;
        child_entry_point(event.data);
    };
};
