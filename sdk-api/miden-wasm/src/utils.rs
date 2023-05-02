use std::sync::Once;

use js_sys::Uint8Array;
use miden_air::{Felt, StarkField};
use serde::ser::SerializeSeq;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;

pub trait IntoWorkerPayload {
    fn into_worker_payload(self) -> Uint8Array;
    fn from_worker_payload(payload: Uint8Array) -> Self;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkerPayload {
    pub job_type: WorkerJobType,
    // WorkerJobPayload - keep binary so message passers don't have to know about the payload type
    pub payload: Vec<u8>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum WorkerJobType {
    HashingWorkItem,
    ProvingWorkItem,
    HashingResult,
    ProvingResult,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum WorkerJobPayload {
    HashingWorkItem(HashingWorkItem),
    ProvingWorkItem(ProvingWorkItem),
    HashingResult(HashingResult),
    ProvingResult(ProverOutput),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProvingWorkItem {
    pub program: Vec<u8>,
    pub program_inputs: Vec<u8>,
    pub proof_options: Vec<u8>,
    pub chunk_size: usize,
    pub is_sequential: bool,
}

impl IntoWorkerPayload for ProvingWorkItem {
    fn into_worker_payload(self) -> Uint8Array {
        let job_payload = WorkerJobPayload::ProvingWorkItem(self);
        let work_item_bytes = bincode::serialize(&job_payload).unwrap();
        let job_payload = WorkerPayload {
            job_type: WorkerJobType::ProvingWorkItem,
            payload: work_item_bytes,
        };
        let payload_bytes = bincode::serialize(&job_payload).unwrap();
        Uint8Array::from(&payload_bytes[..])
    }

    fn from_worker_payload(payload: Uint8Array) -> Self {
        let job_payload: WorkerPayload = bincode::deserialize(&payload.to_vec()).unwrap();
        assert!(job_payload.job_type == WorkerJobType::ProvingWorkItem);
        let job_payload: WorkerJobPayload = bincode::deserialize(&job_payload.payload).unwrap();
        if let WorkerJobPayload::ProvingWorkItem(work_item) = job_payload {
            work_item
        } else {
            panic!("Expected ProvingWorkItem")
        }
    }
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HashingWorkItem {
    pub data: Vec<VecWrapper>,
    pub batch_idx: usize,
}

impl IntoWorkerPayload for HashingWorkItem {
    fn into_worker_payload(self) -> Uint8Array {
        let job_payload = WorkerJobPayload::HashingWorkItem(self);
        let work_item_bytes = bincode::serialize(&job_payload).unwrap();
        let job_payload = WorkerPayload {
            job_type: WorkerJobType::HashingWorkItem,
            payload: work_item_bytes,
        };
        let payload_bytes = bincode::serialize(&job_payload).unwrap();
        Uint8Array::from(&payload_bytes[..])
    }

    fn from_worker_payload(payload: Uint8Array) -> Self {
        let job_payload: WorkerPayload = bincode::deserialize(&payload.to_vec()).unwrap();
        assert!(job_payload.job_type == WorkerJobType::HashingWorkItem);
        let job_payload: WorkerJobPayload = bincode::deserialize(&job_payload.payload).unwrap();
        if let WorkerJobPayload::HashingWorkItem(work_item) = job_payload {
            work_item
        } else {
            panic!("Expected HashingWorkItem")
        }
    }
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

        let work_item = HashingWorkItem { data, batch_idx: 0 };
        let serialized = bincode::serialize(&work_item).unwrap();
        let deserialized: HashingWorkItem = bincode::deserialize(&serialized).unwrap();
        assert_eq!(work_item.data, deserialized.data);
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HashingResult {
    pub batch_idx: usize,
    pub hashes: Vec<[u8; 32]>,
}

impl IntoWorkerPayload for HashingResult {
    fn into_worker_payload(self) -> Uint8Array {
        let work_item_bytes = bincode::serialize(&self).unwrap();
        let job_payload = WorkerPayload {
            job_type: WorkerJobType::HashingResult,
            payload: work_item_bytes,
        };
        let payload_bytes = bincode::serialize(&job_payload).unwrap();
        Uint8Array::from(&payload_bytes[..])
    }

    fn from_worker_payload(payload: Uint8Array) -> Self {
        let job_payload: WorkerPayload = bincode::deserialize(&payload.to_vec()).unwrap();
        assert!(job_payload.job_type == WorkerJobType::HashingResult);
        let work_item: HashingResult = bincode::deserialize(&job_payload.payload).unwrap();
        work_item
    }
}

#[wasm_bindgen(getter_with_clone)]
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ProverOutput {
    pub proof: Vec<u8>,
    pub program_outputs: Vec<u8>,
    pub public_inputs: Vec<u8>,
}

impl IntoWorkerPayload for ProverOutput {
    fn into_worker_payload(self) -> Uint8Array {
        let payload = WorkerJobPayload::ProvingResult(self);
        let payload_bytes = bincode::serialize(&payload).unwrap();
        let job_payload = WorkerPayload {
            job_type: WorkerJobType::ProvingResult,
            payload: payload_bytes,
        };
        let job_payload_bytes = bincode::serialize(&job_payload).unwrap();
        Uint8Array::from(&job_payload_bytes[..])
    }

    fn from_worker_payload(payload: Uint8Array) -> Self {
        let job_payload: WorkerPayload = bincode::deserialize(&payload.to_vec()).unwrap();
        assert!(job_payload.job_type == WorkerJobType::ProvingResult);
        let job_payload: WorkerJobPayload = bincode::deserialize(&job_payload.payload).unwrap();
        if let WorkerJobPayload::ProvingResult(work_item) = job_payload {
            work_item
        } else {
            panic!("Expected ProvingResult")
        }
    }
}

#[inline]
pub fn set_once_logger() {
    static SET_SINGLETONS: Once = Once::new();
    SET_SINGLETONS.call_once(|| {
        console_error_panic_hook::set_once();
        log::set_logger(&DEFAULT_LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Debug);
    });
}
