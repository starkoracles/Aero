use crate::utils::{
    from_uint8array, set_once_logger, to_uint8array, HashingResult, HashingWorkItem,
};
use js_sys::Uint8Array;
use log::debug;
use miden_core::Felt;
use wasm_bindgen::prelude::*;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use winter_crypto::hashers::Blake2s_256;
use winter_crypto::ElementHasher;

pub fn blake2_hash_elements(work_item: &HashingWorkItem) -> Result<Uint8Array, JsValue> {
    let mut hashes = vec![];
    for row in work_item.data.iter() {
        let converted_row: Vec<Felt> = row.iter().map(|f| f.clone().into()).collect();
        let r = Blake2s_256::hash_elements(&converted_row[..]);
        hashes.push(r.0);
    }
    debug!("done processing hashes for batch {}", work_item.batch_idx);

    let response = HashingResult {
        batch_idx: work_item.batch_idx,
        hashes,
    };
    Ok(to_uint8array(&response))
}

#[wasm_bindgen]
pub fn hashing_entry_point(msg: MessageEvent) -> Result<(), JsValue> {
    set_once_logger();
    if let Ok(work_item) = from_uint8array::<HashingWorkItem>(&Uint8Array::new(&msg.data())) {
        debug!(
            "Hashing worker received work item: {:?}",
            work_item.batch_idx
        );
        let response = blake2_hash_elements(&work_item)?;
        let global_scope = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
        global_scope.post_message(&response)?;
    } else {
        debug!("Hashing worker received invalid work item");
    }
    Ok(())
}
