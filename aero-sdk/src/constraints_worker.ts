import init from "miden-wasm";
import { constraint_entry_point } from "miden-wasm";

self.onmessage = event => {
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
        constraint_entry_point(event);
    };
};
