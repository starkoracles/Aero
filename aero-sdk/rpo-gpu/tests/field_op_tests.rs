use rpo_gpu::{mul_g, BaseElementGpu};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn pass() {
    mul_g(BaseElementGpu(1), BaseElementGpu(2));
}
