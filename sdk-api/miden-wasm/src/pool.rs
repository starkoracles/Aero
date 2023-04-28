// Silences warnings from the compiler about Work.func and child_entry_point
// being unused when the target is not wasm.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

//! A small module that's intended to provide an example of creating a pool of
//! web workers which can be used to execute `rayon`-style work.
use js_sys::Array;
use js_sys::Reflect::{get, set};
use log::info;
use miden_air::{Felt, FieldElement};
use std::sync::Once;
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use web_sys::{Event, Worker};
use winter_crypto::hashers::Blake2s_256;
use winter_crypto::Digest;
use winter_crypto::ElementHasher;

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
    pub fn new(concurrency: usize) -> Result<WorkerPool, JsValue> {
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
        elements_table: Vec<Vec<Felt>>,
        callback: Closure<dyn FnMut(MessageEvent)>,
    ) -> Result<(), JsValue> {
        let worker_idx = batch_idx % self.state.concurrency();
        console_log!("running on worker idx: {}", worker_idx);
        let worker = self.worker(worker_idx)?;

        let arg: Array = elements_table
            .iter()
            .map(|r| r.iter().map(|e| e.into_js_value()).collect::<Array>())
            .collect::<Array>();

        let object = js_sys::Object::new();
        set(&object, &"batch_idx".into(), &JsValue::from(batch_idx))?;
        set(&object, &"elements_table".into(), &arg)?;

        worker.post_message(&object)?;
        worker.set_onmessage(Some(callback.as_ref().unchecked_ref()));
        callback.forget();
        Ok(())
    }
}

impl WorkerPool {
    pub fn run(
        &self,
        batch_idx: usize,
        elements_table: Vec<Vec<Felt>>,
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
pub fn child_entry_point(obj: js_sys::Object) -> Result<(), JsValue> {
    set_once_logger();
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    let element_table = get(&obj, &"elements_table".into())?.unchecked_into();
    let batch_idx: usize = get(&obj, &"batch_idx".into())?.as_f64().unwrap() as usize;
    let hashes = blake2_hash_elements(batch_idx, element_table)?;

    let object = js_sys::Object::new();
    set(&object, &"batch_idx".into(), &JsValue::from(batch_idx))?;
    set(&object, &"elements_table".into(), &hashes)?;
    global.post_message(&object)?;
    Ok(())
}

pub fn blake2_hash_elements(batch_idx: usize, element_table: Array) -> Result<Array, JsValue> {
    // we expect a 2d Array of JsValues that would translate into Felts
    let mut converted_table: Vec<Vec<Felt>> = vec![];
    for (i, row) in element_table.iter().enumerate() {
        let row_array = row.dyn_into::<Array>()?;
        converted_table.push(vec![]);
        for column in row_array.iter() {
            let element = Felt::from_js_value(column);
            converted_table[i].push(element);
        }
    }
    let hashes = Array::new();
    for row in converted_table.iter() {
        let r = Blake2s_256::hash_elements(row);
        hashes.push(&r.into_js_value());
    }
    info!("done processing hashes for batch {}", batch_idx);

    Ok(hashes)
}
