// Silences warnings from the compiler about Work.func and child_entry_point
// being unused when the target is not wasm.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

//! A small module that's intended to provide an example of creating a pool of
//! web workers which can be used to execute `rayon`-style work.
use js_sys::Reflect::set;
use js_sys::{Array, Uint8Array};
use log::debug;
use miden_air::{Felt, StarkField};
use serde::ser::SerializeSeq;
use std::sync::Once;
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use web_sys::{Event, Worker};
use winter_crypto::hashers::Blake2s_256;
use winter_crypto::Digest;
use winter_crypto::ElementHasher;

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkItem {
    pub data: Vec<VecWrapper>,
    pub batch_idx: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VecWrapper(pub Vec<Felt>);

impl serde::Serialize for VecWrapper {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        self.0
            .iter()
            .map(|e| e.as_int())
            .for_each(|e| seq.serialize_element(&e).unwrap());
        seq.end()
    }
}

impl<'de> serde::Deserialize<'de> for VecWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct VecWrapperVisitor;

        impl<'de> serde::de::Visitor<'de> for VecWrapperVisitor {
            type Value = VecWrapper;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a vector of bytes")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut vec = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(e) = seq.next_element::<u64>()? {
                    vec.push(Felt::new(e));
                }
                Ok(VecWrapper(vec))
            }
        }

        deserializer.deserialize_seq(VecWrapperVisitor)
    }
}

#[cfg(test)]
mod work_item_test {
    use super::*;

    #[test]
    fn test_work_item_serialization() {
        let data = vec![
            VecWrapper(vec![Felt::from(1u64), Felt::from(2u64)]),
            VecWrapper(vec![Felt::from(3u64), Felt::from(4u64)]),
        ];

        let work_item = WorkItem { data, batch_idx: 0 };
        let serialized = bincode::serialize(&work_item).unwrap();
        let deserialized: WorkItem = bincode::deserialize(&serialized).unwrap();
        assert_eq!(work_item.data, deserialized.data);
    }
}

#[wasm_bindgen]
pub struct WorkerPool {
    state: PoolState,
}

#[inline]
fn set_once_logger() {
    static SET_SINGLETONS: Once = Once::new();
    SET_SINGLETONS.call_once(|| {
        log::set_logger(&DEFAULT_LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Info);
    });
}

struct PoolState {
    workers: Vec<Worker>,
    callback: Closure<dyn FnMut(Event)>,
}

#[wasm_bindgen]
impl WorkerPool {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WorkerPool, JsValue> {
        let window = web_sys::window().unwrap();
        let navigator = window.navigator();
        let concurrency = (navigator.hardware_concurrency() as usize) * 1;
        let mut pool = WorkerPool {
            state: PoolState {
                workers: Vec::with_capacity(concurrency),
                callback: Closure::new(|event: Event| {
                    console_log!("unhandled event: {}", event.type_());
                    crate::logv(&event);
                }),
            },
        };
        for _ in 0..concurrency {
            let worker = pool.spawn()?;
            pool.state.push(worker);
        }

        Ok(pool)
    }

    /// Unconditionally spawns a new worker
    ///
    /// The worker isn't registered with this `WorkerPool` but is capable of
    /// executing work for this wasm module.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    fn spawn(&self) -> Result<Worker, JsValue> {
        console_log!("spawning new worker");
        // TODO: what do do about `./worker.js`:
        //
        // * the path is only known by the bundler. How can we, as a
        //   library, know what's going on?
        // * How do we not fetch a script N times? It internally then
        //   causes another script to get fetched N times...
        let worker = Worker::new("./hashing_worker.js")?;
        worker.post_message(&JsValue::from_str("wake worker up"))?;
        Ok(worker)
    }

    fn worker(&self, worker_idx: usize) -> Result<&Worker, JsValue> {
        let worker = &self.state.workers[worker_idx];
        Ok(worker)
    }

    fn execute(
        &self,
        batch_idx: usize,
        elements_table: Vec<VecWrapper>,
        callback: Closure<dyn FnMut(MessageEvent)>,
    ) -> Result<(), JsValue> {
        let worker_idx = batch_idx % self.state.concurrency();
        debug!("running on worker idx: {}", worker_idx);
        let worker = self.worker(worker_idx)?;

        let work_item = WorkItem {
            data: elements_table,
            batch_idx,
        };

        let work_item_bytes = bincode::serialize(&work_item)
            .map_err(|err| format!("Failed to convert work_item_bytes - {:?}", err))?;
        let uint8array = Uint8Array::from(&work_item_bytes[..]);

        worker.post_message(&uint8array)?;
        worker.set_onmessage(Some(callback.as_ref().unchecked_ref()));
        callback.forget();
        Ok(())
    }
}

impl WorkerPool {
    pub fn run(
        &self,
        batch_idx: usize,
        elements_table: Vec<VecWrapper>,
        callback: Closure<dyn FnMut(MessageEvent)>,
    ) -> Result<(), JsValue> {
        self.execute(batch_idx, elements_table, callback)?;
        Ok(())
    }
}

impl PoolState {
    fn push(&mut self, worker: Worker) {
        worker.set_onerror(Some(self.callback.as_ref().unchecked_ref()));
        self.workers.push(worker);
    }

    fn concurrency(&self) -> usize {
        self.workers.len()
    }
}

/// Entry point invoked by `worker.js`, a bit of a hack but see the "TODO" above
/// about `worker.js` in general.
#[wasm_bindgen]
pub fn child_entry_point(data_array: Uint8Array) -> Result<(), JsValue> {
    set_once_logger();
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();

    let data = data_array.to_vec();
    let work_item: WorkItem = bincode::deserialize(&data)
        .map_err(|err| format!("Could not deserialize work item: {:?}", err))?;
    let hashes = blake2_hash_elements(&work_item)?;
    let object = js_sys::Object::new();
    set(
        &object,
        &"batch_idx".into(),
        &JsValue::from(work_item.batch_idx),
    )?;
    set(&object, &"elements_table".into(), &hashes)?;
    global.post_message(&object)?;
    Ok(())
}

pub fn blake2_hash_elements(work_item: &WorkItem) -> Result<Array, JsValue> {
    let hashes = Array::new();
    for row in work_item.data.iter() {
        let r = Blake2s_256::hash_elements(&row.0[..]);
        hashes.push(&r.into_js_value());
    }
    debug!("done processing hashes for batch {}", work_item.batch_idx);

    Ok(hashes)
}
