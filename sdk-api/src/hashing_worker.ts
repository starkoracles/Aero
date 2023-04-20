import init from "miden-wasm";
import { blake2_hash_elements } from "miden-wasm";

async function init_wasm() {
    console.log("starting worker ts");
    await init();
    self.onmessage = (event) => {
        const inputs = event.data;
        console.log("calling blake2 with", inputs);
        const result = blake2_hash_elements(inputs);
        console.log("result is ", result);
        self.postMessage(result);
    };
}

init_wasm();
