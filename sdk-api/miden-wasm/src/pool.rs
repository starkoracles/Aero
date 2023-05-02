// Silences warnings from the compiler about Work.func and child_entry_point
// being unused when the target is not wasm.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
use log::debug;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker};

use crate::utils::{HashingWorkItem, IntoWorkerPayload, VecWrapper};

#[derive(Debug)]
pub struct WorkerPool {
    state: PoolState,
}

#[derive(Debug)]
struct PoolState {
    workers: Vec<Worker>,
}

impl WorkerPool {
    fn get_concurrency() -> Result<usize, JsValue> {
        let window = web_sys::window().ok_or(JsValue::from_str("cannot get window"))?;
        let navigator = window.navigator();
        let concurrency = navigator.hardware_concurrency() as usize;
        Ok(concurrency)
    }

    pub fn new() -> Result<WorkerPool, JsValue> {
        let concurrency = 10;
        let mut pool = WorkerPool {
            state: PoolState {
                workers: Vec::with_capacity(concurrency),
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
        get_on_msg_callback: Closure<dyn FnMut(MessageEvent)>,
    ) -> Result<(), JsValue> {
        debug!(
            "batch_idx: {}, concurrency: {}",
            batch_idx,
            self.state.concurrency()
        );
        let worker_idx = batch_idx % self.state.concurrency();
        debug!("running on worker idx: {}", worker_idx);
        let worker = self.worker(worker_idx)?;

        let work_item = HashingWorkItem {
            data: elements_table,
            batch_idx,
        };
        let payload = work_item.into_worker_payload();
        worker.post_message(&payload)?;
        worker.set_onmessage(Some(get_on_msg_callback.as_ref().unchecked_ref()));
        get_on_msg_callback.forget();
        Ok(())
    }
}

impl WorkerPool {
    pub fn run(
        &self,
        batch_idx: usize,
        elements_table: Vec<VecWrapper>,
        get_on_msg_callback: Closure<dyn FnMut(MessageEvent)>,
    ) -> Result<(), JsValue> {
        self.execute(batch_idx, elements_table, get_on_msg_callback)?;
        Ok(())
    }
}

impl PoolState {
    fn push(&mut self, worker: Worker) {
        self.workers.push(worker);
    }

    fn concurrency(&self) -> usize {
        self.workers.len()
    }
}
