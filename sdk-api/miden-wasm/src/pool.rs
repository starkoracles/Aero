// Silences warnings from the compiler about Work.func and child_entry_point
// being unused when the target is not wasm.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

//! A small module that's intended to provide an example of creating a pool of
//! web workers which can be used to execute `rayon`-style work.
use js_sys::Array;
use js_sys::Reflect::{get, set};
use log::info;
use miden_air::{Felt, FieldElement};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Once;
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use web_sys::DedicatedWorkerGlobalScope;
use web_sys::{Event, Worker};
use winter_crypto::hashers::Blake2s_256;
use winter_crypto::Digest;
use winter_crypto::ElementHasher;

#[wasm_bindgen]
pub struct WorkerPool {
    state: Rc<PoolState>,
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
    workers: RefCell<Vec<Worker>>,
    callback: Closure<dyn FnMut(Event)>,
}

#[wasm_bindgen]
impl WorkerPool {
    /// Creates a new `WorkerPool` which immediately creates `initial` workers.
    ///
    /// The pool created here can be used over a long period of time, and it
    /// will be initially primed with `initial` workers. Currently workers are
    /// never released or gc'd until the whole pool is destroyed.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    #[wasm_bindgen(constructor)]
    pub fn new(initial: usize) -> Result<WorkerPool, JsValue> {
        let pool = WorkerPool {
            state: Rc::new(PoolState {
                workers: RefCell::new(Vec::with_capacity(initial)),
                callback: Closure::new(|event: Event| {
                    console_log!("unhandled event: {}", event.type_());
                    crate::logv(&event);
                }),
            }),
        };
        for _ in 0..initial {
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

    /// Fetches a worker from this pool, spawning one if necessary.
    ///
    /// This will attempt to pull an already-spawned web worker from our cache
    /// if one is available, otherwise it will spawn a new worker and return the
    /// newly spawned worker.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    fn worker(&self) -> Result<Worker, JsValue> {
        match self.state.workers.borrow_mut().pop() {
            Some(worker) => Ok(worker),
            None => self.spawn(),
        }
    }

    /// Executes the work `f` in a web worker, spawning a web worker if
    /// necessary.
    ///
    /// This will acquire a web worker and then send the closure `f` to the
    /// worker to execute. The worker won't be usable for anything else while
    /// `f` is executing, and no callbacks are registered for when the worker
    /// finishes.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    fn execute(&self, batch_idx: usize, elements_table: Vec<Vec<Felt>>) -> Result<Worker, JsValue> {
        let worker = self.worker()?;

        let arg: Array = elements_table
            .iter()
            .map(|r| r.iter().map(|e| e.into_js_value()).collect::<Array>())
            .collect::<Array>();

        let object = js_sys::Object::new();
        set(&object, &"batch_idx".into(), &JsValue::from(batch_idx));
        set(&object, &"elements_table".into(), &arg);

        match worker.post_message(&object) {
            Ok(()) => Ok(worker),
            Err(e) => Err(e),
        }
    }
}

impl WorkerPool {
    /// Executes `f` in a web worker.
    ///
    /// This pool manages a set of web workers to draw from, and `f` will be
    /// spawned quickly into one if the worker is idle. If no idle workers are
    /// available then a new web worker will be spawned.
    ///
    /// Once `f` returns the worker assigned to `f` is automatically reclaimed
    /// by this `WorkerPool`. This method provides no method of learning when
    /// `f` completes, and for that you'll need to use `run_notify`.
    ///
    /// # Errors
    ///
    /// If an error happens while spawning a web worker or sending a message to
    /// a web worker, that error is returned.
    pub fn run(&self, batch_idx: usize, elements_table: Vec<Vec<Felt>>) -> Result<(), JsValue> {
        let worker = self.execute(batch_idx, elements_table)?;
        self.state.push(worker);
        Ok(())
    }
}

impl PoolState {
    fn push(&self, worker: Worker) {
        worker.set_onerror(Some(self.callback.as_ref().unchecked_ref()));
        let mut workers = self.workers.borrow_mut();
        for prev in workers.iter() {
            let prev: &JsValue = prev;
            let worker: &JsValue = &worker;
            assert!(prev != worker);
        }
        workers.push(worker);
    }
}

/// Entry point invoked by `worker.js`, a bit of a hack but see the "TODO" above
/// about `worker.js` in general.
#[wasm_bindgen]
pub fn child_entry_point(obj: js_sys::Object) -> Result<(), JsValue> {
    set_once_logger();
    info!("child_entry_point");
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    let element_table = get(&obj, &"elements_table".into())?.unchecked_into();
    let batch_idx: usize = get(&obj, &"batch_idx".into())?.as_f64().unwrap() as usize;
    let result = blake2_hash_elements(batch_idx, element_table)?;
    global.post_message(&result)?;
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
    // info!("hashes: {:?}", hashes);
    info!("done processing hashes for batch {}", batch_idx);

    Ok(hashes)
}
