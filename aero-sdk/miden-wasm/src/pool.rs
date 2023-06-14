// Silences warnings from the compiler about Work.func and child_entry_point
// being unused when the target is not wasm.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
use log::debug;
use wasm_bindgen::prelude::*;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, Worker, WorkerNavigator};

use crate::utils::{to_uint8array, ConstraintComputeWorkItem, FeltWrapper, HashingWorkItem};

#[derive(Debug, Clone)]
pub struct WorkerPool {
    state: PoolState,
}

#[derive(Debug, Clone)]
struct PoolState {
    workers: Vec<Worker>,
    constraint_workers: Vec<Worker>,
}

impl WorkerPool {
    fn get_hardware_concurrency() -> usize {
        let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
        let navigator: WorkerNavigator = global.navigator();
        navigator.hardware_concurrency() as usize
    }

    pub fn new() -> Result<WorkerPool, JsValue> {
        let concurrency = Self::get_hardware_concurrency();
        debug!("creating worker pool with concurrency {}", concurrency);
        let mut pool = WorkerPool {
            state: PoolState {
                workers: Vec::with_capacity(concurrency),
                constraint_workers: Vec::with_capacity(concurrency),
            },
        };
        for _ in 0..concurrency {
            let worker = pool.spawn("./hashing_worker.js")?;
            pool.state.push(worker);
            let constraint_worker = pool.spawn("./constraints_worker.js")?;
            pool.state.push_constraint_worker(constraint_worker);
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
    fn spawn(&self, worker_path: &str) -> Result<Worker, JsValue> {
        console_log!("spawning new worker, {}", worker_path);
        // TODO: what do do about `./worker.js`:
        //
        // * the path is only known by the bundler. How can we, as a
        //   library, know what's going on?
        // * How do we not fetch a script N times? It internally then
        //   causes another script to get fetched N times...
        let worker = Worker::new(worker_path)?;
        worker.post_message(&JsValue::from_str("wake worker up"))?;
        Ok(worker)
    }

    fn worker(&self, worker_idx: usize) -> Result<&Worker, JsValue> {
        let worker = &self.state.workers[worker_idx];
        Ok(worker)
    }

    fn constraint_worker(&self, worker_idx: usize) -> Result<&Worker, JsValue> {
        let worker = &self.state.constraint_workers[worker_idx];
        Ok(worker)
    }

    fn execute(
        &self,
        batch_idx: usize,
        elements_table: Vec<Vec<FeltWrapper>>,
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
        let payload = to_uint8array(&work_item);
        worker.post_message(&payload)?;
        worker.set_onmessage(Some(get_on_msg_callback.as_ref().unchecked_ref()));
        get_on_msg_callback.forget();
        Ok(())
    }

    fn execute_constraint(
        &self,
        constraint_work_item: ConstraintComputeWorkItem,
        get_on_msg_callback: Closure<dyn FnMut(MessageEvent)>,
    ) -> Result<(), JsValue> {
        debug!(
            "fragment_offset: {}, concurrency: {}",
            constraint_work_item.computation_fragment.fragment_offset,
            self.state.concurrency()
        );
        let worker_idx =
            constraint_work_item.computation_fragment.fragment_offset % self.state.concurrency();
        debug!("running on worker idx: {}", worker_idx);
        let worker = self.constraint_worker(worker_idx)?;
        let payload = to_uint8array(&constraint_work_item);
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
        elements_table: Vec<Vec<FeltWrapper>>,
        get_on_msg_callback: Closure<dyn FnMut(MessageEvent)>,
    ) -> Result<(), JsValue> {
        self.execute(batch_idx, elements_table, get_on_msg_callback)?;
        Ok(())
    }

    pub fn run_constraint(
        &self,
        constraint_work_item: ConstraintComputeWorkItem,
        get_on_msg_callback: Closure<dyn FnMut(MessageEvent)>,
    ) -> Result<(), JsValue> {
        self.execute_constraint(constraint_work_item, get_on_msg_callback)?;
        Ok(())
    }
}

impl PoolState {
    fn push(&mut self, worker: Worker) {
        self.workers.push(worker);
    }

    fn push_constraint_worker(&mut self, worker: Worker) {
        self.constraint_workers.push(worker);
    }

    fn concurrency(&self) -> usize {
        self.workers.len()
    }
}
