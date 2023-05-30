#![feature(once_cell)]
use futures::Future;
use js_sys::Uint8Array;
use log::debug;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{cell::RefCell, rc::Rc};
use utils::set_once_logger;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker};

macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

pub mod constraints_worker;
pub mod convert;
pub mod hashing_worker;
pub mod pool;
pub mod proving_worker;
pub mod utils;
use crate::convert::sdk::sdk;
use crate::utils::{from_uint8array, to_uint8array, ProverOutput, ProvingWorkItem};

pub struct ResultFuture<T> {
    pub result: Rc<RefCell<Option<T>>>,
}

impl<T> Future for ResultFuture<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let r = (*self.result).borrow();
        if r.is_some() {
            return Poll::Ready(());
        } else {
            // wait every second
            let wait_fn = {
                let waker = Rc::new(cx.waker().clone());
                Closure::wrap(Box::new(move || {
                    waker.as_ref().clone().wake();
                }) as Box<dyn Fn()>)
            };
            let _ = web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    wait_fn.as_ref().unchecked_ref(),
                    200,
                );
            wait_fn.forget();
            return Poll::Pending;
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn logv(x: &JsValue);
}

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    set_once_logger();
    Ok(())
}

#[wasm_bindgen(getter_with_clone)]
pub struct MidenProver {
    prover_worker: Worker,
    prover_output: Rc<RefCell<Option<ProverOutput>>>,
}

#[wasm_bindgen]
impl MidenProver {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<MidenProver, JsValue> {
        let proving_worker = Worker::new("./proving_worker.js")?;
        proving_worker.post_message(&JsValue::from_str("wake worker up"))?;
        Ok(MidenProver {
            prover_worker: proving_worker,
            prover_output: Rc::new(RefCell::new(None)),
        })
    }

    #[wasm_bindgen]
    pub async fn prove(
        &mut self,
        program: Vec<u8>,
        program_inputs: Vec<u8>,
        proof_options: Vec<u8>,
        chunk_size: usize,
    ) -> Result<ProverOutput, JsValue> {
        self.set_onmessage_handler();
        let work_item = ProvingWorkItem {
            program,
            program_inputs,
            proof_options,
            chunk_size,
            is_sequential: false,
        };
        let payload = to_uint8array(&work_item);
        self.prover_worker.post_message(&payload)?;
        ResultFuture {
            result: self.prover_output.clone(),
        }
        .await;

        let output = self.prover_output.borrow_mut().take().unwrap();
        Ok(output)
    }

    #[wasm_bindgen]
    pub async fn prove_sequential(
        &mut self,
        program: Vec<u8>,
        program_inputs: Vec<u8>,
        proof_options: Vec<u8>,
    ) -> Result<ProverOutput, JsValue> {
        self.set_onmessage_handler();
        let work_item = ProvingWorkItem {
            program,
            program_inputs,
            proof_options,
            chunk_size: 1024,
            is_sequential: true,
        };
        let payload = to_uint8array(&work_item);
        self.prover_worker.post_message(&payload)?;
        ResultFuture {
            result: self.prover_output.clone(),
        }
        .await;

        let output = self.prover_output.borrow_mut().take().unwrap();
        Ok(output)
    }

    fn set_onmessage_handler(&mut self) {
        let callback = self.get_on_msg_callback();
        self.prover_worker
            .set_onmessage(Some(callback.as_ref().unchecked_ref()));

        // Clean up closure to prevent memory leak
        callback.forget();
    }

    /// Message passing by the main thread
    fn get_on_msg_callback(&self) -> Closure<dyn FnMut(MessageEvent)> {
        let prover_output = self.prover_output.clone();
        let callback = Closure::new(move |event: MessageEvent| {
            debug!("Main thread got prover output");
            let data: Uint8Array = Uint8Array::new(&event.data());
            let output: ProverOutput = from_uint8array(&data).unwrap();
            prover_output.replace(Some(output));
        });

        callback
    }
}
