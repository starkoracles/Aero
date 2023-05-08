use std::{marker::PhantomData, sync::Once};

use js_sys::Uint8Array;
use miden_air::{Felt, PublicInputs, StarkField};
use serde::{ser::SerializeSeq, Deserializer, Serializer};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;
use winter_air::{
    AuxTraceRandElements, ConstraintCompositionCoefficients, ProofOptions, TraceInfo, TraceLayout,
};
use winter_utils::{Deserializable, Serializable, SliceReader};

fn serialize_trace_info<S: Serializer>(
    trace_info: &TraceInfo,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut s = serializer.serialize_seq(Some(3))?;
    s.serialize_element(&trace_info.layout().to_bytes())?;
    s.serialize_element(&trace_info.length())?;
    s.serialize_element(&trace_info.meta())?;
    s.end()
}

fn deserialize_trace_info<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<TraceInfo, D::Error> {
    struct TraceInfoVisitor;

    impl<'de> serde::de::Visitor<'de> for TraceInfoVisitor {
        type Value = TraceInfo;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a TraceInfo")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let layout_bytes = seq.next_element::<Vec<u8>>()?.ok_or_else(|| {
                serde::de::Error::custom("expected trace_layout to deser to bytes")
            })?;
            let layout = TraceLayout::read_from(&mut SliceReader::new(layout_bytes.as_slice()))
                .map_err(|_| {
                    serde::de::Error::custom("expected trace_layout to deser to TraceLayout")
                })?;

            let length = seq
                .next_element::<usize>()?
                .ok_or_else(|| serde::de::Error::custom("expected trace_length to deser"))?;
            let meta = seq
                .next_element::<Vec<u8>>()?
                .ok_or_else(|| serde::de::Error::custom("expected trace_meta to deser"))?;
            Ok(TraceInfo::new_multi_segment(layout, length, meta))
        }
    }

    deserializer.deserialize_seq(TraceInfoVisitor)
}

#[macro_export]
macro_rules! winter_serde {
    ($serialize_name:ident, $deserialize_name:ident, $ty:ty) => {
        fn $serialize_name<S: serde::Serializer>(
            input: &$ty,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            let mut s = serializer.serialize_seq(Some(1))?;
            s.serialize_element(&input.to_bytes())?;
            s.end()
        }

        fn $deserialize_name<'de, D: serde::Deserializer<'de>>(
            deserializer: D,
        ) -> Result<$ty, D::Error> {
            struct InputsVisitor {
                _marker: PhantomData<$ty>,
            }

            impl<'de> serde::de::Visitor<'de> for InputsVisitor {
                type Value = $ty;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    let type_name = std::any::type_name::<$ty>();
                    formatter.write_str(&format!("a winter deser type {}", type_name))
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    let bytes = seq.next_element::<Vec<u8>>()?.ok_or_else(|| {
                        serde::de::Error::custom("expected winter input to deser to bytes")
                    })?;
                    <$ty>::read_from(&mut SliceReader::new(bytes.as_slice())).map_err(|_| {
                        serde::de::Error::custom("expected winter input to deserialize to $ty")
                    })
                }
            }

            deserializer.deserialize_seq(InputsVisitor {
                _marker: PhantomData,
            })
        }
    };
}

winter_serde!(
    serialize_public_inputs,
    deserialize_public_inputs,
    PublicInputs
);
winter_serde!(
    serialize_proof_options,
    deserialize_proof_options,
    ProofOptions
);

fn serialize_aux_rand_elements<S: Serializer>(
    input: &AuxTraceRandElements<Felt>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let flattened: Vec<Vec<FeltWrapper>> = input
        .0
        .iter()
        .map(|v| v.iter().map(|e| e.into()).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut s = serializer.serialize_seq(Some(flattened.len()))?;
    for e in flattened {
        s.serialize_element(&e)?;
    }
    s.end()
}

fn deserialize_aux_rand_elements<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<AuxTraceRandElements<Felt>, D::Error> {
    struct AuxRandElementsVisitor;

    impl<'de> serde::de::Visitor<'de> for AuxRandElementsVisitor {
        type Value = AuxTraceRandElements<Felt>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a AuxTraceRandElements<Felt>")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut vec = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(v) = seq.next_element::<Vec<FeltWrapper>>()? {
                vec.push(
                    v.iter()
                        .map(|wrapper| wrapper.clone().into())
                        .collect::<Vec<_>>(),
                );
            }
            Ok(AuxTraceRandElements(vec))
        }
    }

    deserializer.deserialize_seq(AuxRandElementsVisitor)
}

// fn serialize_constraint_coeffs<S: Serializer>(
//     input: &ConstraintCompositionCoefficients<Felt>,
//     serializer: S,
// ) -> Result<S::Ok, S::Error> {
//     let transition = input
//         .transition
//         .iter()
//         .map(|v| VecWrapper(v.clone()))
//         .collect::<Vec<_>>();
//     let mut s = serializer.serialize_seq(Some(transition.len()))?;
//     for e in flattened {
//         s.serialize_element(&e)?;
//     }
//     s.end()
// }

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ConstraintComputeWorkItem {
    #[serde(
        serialize_with = "serialize_trace_info",
        deserialize_with = "deserialize_trace_info"
    )]
    pub trace_info: TraceInfo,
    #[serde(
        serialize_with = "serialize_public_inputs",
        deserialize_with = "deserialize_public_inputs"
    )]
    pub public_inputs: PublicInputs,
    #[serde(
        serialize_with = "serialize_proof_options",
        deserialize_with = "deserialize_proof_options"
    )]
    pub proof_options: ProofOptions,
    #[serde(
        serialize_with = "serialize_aux_rand_elements",
        deserialize_with = "deserialize_aux_rand_elements"
    )]
    pub aux_rand_elements: AuxTraceRandElements<Felt>,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProvingWorkItem {
    pub program: Vec<u8>,
    pub program_inputs: Vec<u8>,
    pub proof_options: Vec<u8>,
    pub chunk_size: usize,
    pub is_sequential: bool,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HashingWorkItem {
    pub data: Vec<Vec<FeltWrapper>>,
    pub batch_idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FeltWrapper(pub Felt);

impl serde::Serialize for FeltWrapper {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0.as_int())
    }
}

impl<'de> serde::Deserialize<'de> for FeltWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FeltWrapperVisitor;

        impl<'de> serde::de::Visitor<'de> for FeltWrapperVisitor {
            type Value = FeltWrapper;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a u64")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(FeltWrapper(Felt::new(value)))
            }
        }

        deserializer.deserialize_u64(FeltWrapperVisitor)
    }
}

impl From<&Felt> for FeltWrapper {
    fn from(f: &Felt) -> Self {
        Self(f.clone())
    }
}

impl Into<Felt> for FeltWrapper {
    fn into(self) -> Felt {
        self.0
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HashingResult {
    pub batch_idx: usize,
    pub hashes: Vec<[u8; 32]>,
}

#[wasm_bindgen(getter_with_clone)]
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ProverOutput {
    pub proof: Vec<u8>,
    pub program_outputs: Vec<u8>,
    pub public_inputs: Vec<u8>,
}

#[inline]
pub fn set_once_logger() {
    static SET_SINGLETONS: Once = Once::new();
    SET_SINGLETONS.call_once(|| {
        console_error_panic_hook::set_once();
        log::set_logger(&DEFAULT_LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Info);
    });
}

pub fn to_uint8array<T: serde::Serialize>(data: &T) -> Uint8Array {
    let serialized = bincode::serialize(data).unwrap();
    Uint8Array::from(serialized.as_slice())
}

pub fn from_uint8array<T: serde::de::DeserializeOwned>(data: &Uint8Array) -> T {
    let bytes = data.to_vec();
    bincode::deserialize(bytes.as_slice()).unwrap()
}

#[cfg(test)]
mod work_item_test {
    use miden::Assembler;
    use miden_core::{Program, ProgramOutputs};
    use miden_stdlib::StdLibrary;

    use super::*;

    #[test]
    fn test_work_item_serialization() {
        let data = vec![
            vec![Felt::from(1u64), Felt::from(2u64)],
            vec![Felt::from(3u64), Felt::from(4u64)],
        ];

        let converted = data
            .iter()
            .map(|v| v.iter().map(|e| e.into()).collect())
            .collect();

        let work_item = HashingWorkItem {
            data: converted,
            batch_idx: 0,
        };
        let serialized = bincode::serialize(&work_item).unwrap();
        let deserialized: HashingWorkItem = bincode::deserialize(&serialized).unwrap();
        assert_eq!(work_item.data, deserialized.data);
    }

    #[test]
    fn test_constraint_work_item_serialization() {
        let trace_info = TraceInfo::new_multi_segment(TraceLayout::new(2, [1], [1]), 8, vec![]);
        let program = generate_fibonacci_program(10);
        let stack_inputs = vec![Felt::from(0u64), Felt::from(1u64)];
        let program_outputs = ProgramOutputs::new(vec![2, 3], vec![]);
        let proof_options = ProofOptions::new(
            27,
            8,
            17,
            winter_air::HashFunction::Blake2s_256,
            winter_air::FieldExtension::None,
            16,
            128,
        );
        let aux_rand_elements = AuxTraceRandElements(vec![
            vec![Felt::from(1u64)],
            vec![Felt::from(2u64), Felt::from(3u64)],
            vec![Felt::from(4u64), Felt::from(5u64), Felt::from(6u64)],
        ]);

        let public_inputs = PublicInputs::new(program.hash(), stack_inputs, program_outputs);
        let work_item = ConstraintComputeWorkItem {
            trace_info,
            public_inputs,
            proof_options,
            aux_rand_elements,
        };
        println!("{}", serde_json::to_string(&work_item).unwrap());
        let serialized = bincode::serialize(&work_item).unwrap();
        let deserialized: ConstraintComputeWorkItem = bincode::deserialize(&serialized).unwrap();
        // let serialized = serde_json::to_string(&work_item).unwrap();
        // let deserialized: ConstraintComputeWorkItem = serde_json::from_str(&serialized).unwrap();
        assert_eq!(work_item.trace_info, deserialized.trace_info);
        assert_eq!(
            work_item.public_inputs.program_hash,
            deserialized.public_inputs.program_hash
        );
        assert_eq!(
            work_item.public_inputs.stack_inputs,
            deserialized.public_inputs.stack_inputs
        );
        assert_eq!(
            work_item.public_inputs.outputs.stack,
            deserialized.public_inputs.outputs.stack
        );
        assert_eq!(work_item.proof_options, deserialized.proof_options);
        assert_eq!(
            work_item.aux_rand_elements.0,
            deserialized.aux_rand_elements.0
        );
    }

    /// Generates a program to compute the `n`-th term of Fibonacci sequence
    fn generate_fibonacci_program(n: usize) -> Program {
        // the program is a simple repetition of 4 stack operations:
        // the first operation moves the 2nd stack item to the top,
        // the second operation duplicates the top 2 stack items,
        // the third operation removes the top item from the stack
        // the last operation pops top 2 stack items, adds them, and pushes
        // the result back onto the stack
        let program = format!(
            "begin 
            repeat.{}
                swap dup.1 add
            end
        end",
            n - 1
        );

        Assembler::new()
            .with_module_provider(StdLibrary::default())
            .compile(&program)
            .unwrap()
    }
}
