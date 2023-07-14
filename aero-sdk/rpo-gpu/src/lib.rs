use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct BaseElementGpu(pub u64);

impl BaseElementGpu {
    pub const M: u64 = 0xFFFFFFFF00000001;
}

#[wasm_bindgen]
pub fn mul_g(a: BaseElementGpu, b: BaseElementGpu) -> BaseElementGpu {
    let r = (a.0 * b.0) % BaseElementGpu::M;
    BaseElementGpu(r)
}
