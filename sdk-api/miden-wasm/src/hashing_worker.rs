use crate::utils::{set_once_logger, HashingResult, HashingWorkItem, WorkerJobPayload};
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

    let response = WorkerJobPayload::HashingResult(HashingResult {
        batch_idx: work_item.batch_idx,
        hashes,
    });
    Ok(response.to_uint8array())
}

#[wasm_bindgen]
pub fn hashing_entry_point(msg: MessageEvent) -> Result<(), JsValue> {
    set_once_logger();
    let payload: WorkerJobPayload = WorkerJobPayload::from_uint8array(Uint8Array::new(&msg.data()));
    if let WorkerJobPayload::HashingWorkItem(work_item) = payload {
        info!(
            "Hashing worker received work item: {:?}",
            work_item.batch_idx
        );
        let response = blake2_hash_elements(&work_item)?;
        let global_scope = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
        global_scope.post_message(&response)?;
        Ok(())
    } else {
        Err(JsValue::from_str("Hashing worker received invalid payload"))
    }
}
