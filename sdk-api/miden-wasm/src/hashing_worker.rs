use crate::utils::{set_once_logger, HashingResult, HashingWorkItem, IntoWorkerPayload};
use js_sys::Uint8Array;
use log::{debug, info};
use wasm_bindgen::prelude::*;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use winter_crypto::hashers::Blake2s_256;
use winter_crypto::ElementHasher;

pub fn blake2_hash_elements(work_item: &HashingWorkItem) -> Result<Uint8Array, JsValue> {
    let mut hashes = vec![];
    for row in work_item.data.iter() {
        let r = Blake2s_256::hash_elements(&row.0[..]);
        hashes.push(r.0);
    }
    debug!("done processing hashes for batch {}", work_item.batch_idx);

    let response = HashingResult {
        batch_idx: work_item.batch_idx,
        hashes,
    };
    Ok(response.into_worker_payload())
}

#[wasm_bindgen]
pub fn hashing_entry_point(msg: MessageEvent) -> Result<(), JsValue> {
    set_once_logger();
    let data: Uint8Array = Uint8Array::new(&msg.data());
    let work_item: HashingWorkItem = HashingWorkItem::from_worker_payload(data);
    info!(
        "Hashing worker received work item: {:?}",
        work_item.batch_idx
    );
    let response = blake2_hash_elements(&work_item)?;
    let global_scope = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    global_scope.post_message(&response)?;
    Ok(())
}
