import init from "miden-wasm";
import { hashing_entry_point } from "miden-wasm";

self.onmessage = event => {
    console.debug("Hashing received init:", event.data);
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
        hashing_entry_point(event);
    };
};
