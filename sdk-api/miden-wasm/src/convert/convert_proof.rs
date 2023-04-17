use crate::convert::convert_inputs::*;
use crate::convert::sdk::sdk;
use miden::{FieldExtension, HashFunction, StarkProof};
use miden_air::{Felt, FieldElement, ProcessorAir};
use miden_core::utils::Serializable;
use winter_air::{
    proof::{Commitments, Context, OodFrame, Queries, Table},
    Air, EvaluationFrame, ProofOptions, TraceLayout,
};
use winter_crypto::{hash::ByteDigest, hashers::Blake2s_256, Digest};
use winter_fri::FriProof;
use winter_verifier::TraceQueries;

impl IntoSdk<StarkProof, &ProcessorAir> for sdk::StarkProof {
    fn into_sdk(input: StarkProof, params: &ProcessorAir) -> Self {
        Self {
            ood_frame: Some(sdk::OodFrame::into_sdk(input.ood_frame, params)),
            context: Some(input.context.into()),
            commitments: Some(sdk::Commitments::into_sdk(input.commitments, params)),
            trace_queries: Some(sdk::TraceQueries::into_sdk(input.trace_queries, params)),
            constraint_queries: todo!(),
            fri_proof: todo!(),
            pow_nonce: todo!(),
        }
    }
}

trait IntoSdk<Input, Parameters> {
    fn into_sdk(input: Input, params: Parameters) -> Self;
}

impl IntoSdk<OodFrame, &ProcessorAir> for sdk::OodFrame {
    fn into_sdk(input: OodFrame, params: &ProcessorAir) -> Self {
        let main_trace_width = params.trace_layout().main_trace_width();
        let aux_trace_width = params.trace_layout().aux_trace_width();
        let num_evaluations = params.ce_blowup_factor();
        let (ood_main_trace_frame, ood_aux_trace_frame, ood_constraint_evaluations) = input
            .clone()
            .parse::<Felt>(main_trace_width, aux_trace_width, num_evaluations)
            .unwrap();

        Self {
            main_frame: Some(ood_main_trace_frame.into()),
            aux_frame: ood_aux_trace_frame.map(|f| f.into()),
            evaluations: ood_constraint_evaluations
                .iter()
                .map(|e| e.into())
                .collect(),
        }
    }
}

impl From<&Felt> for sdk::FieldElement {
    fn from(element: &Felt) -> Self {
        Self {
            element: element.to_bytes(),
            // we know that Felt is represented by u64
            size: 8,
        }
    }
}

impl From<EvaluationFrame<Felt>> for sdk::EvaluationFrame {
    fn from(frame: EvaluationFrame<Felt>) -> Self {
        let current = frame.current().iter().map(|e| e.into()).collect::<Vec<_>>();
        let next = frame.next().iter().map(|e| e.into()).collect::<Vec<_>>();

        Self { current, next }
    }
}

impl From<Context> for sdk::Context {
    fn from(value: Context) -> Self {
        let binding = value.get_trace_info();
        let trace_meta = binding.meta();
        let field_modulus = sdk::FieldElement {
            element: value.field_modulus_bytes().to_vec(),
            size: 8,
        };

        Self {
            trace_layout: Some(value.trace_layout().into()),
            trace_length: value.trace_length() as u64,
            trace_meta: trace_meta.to_vec(),
            field_modulus: Some(field_modulus),
            options: Some(value.options().into()),
        }
    }
}

impl From<&TraceLayout> for sdk::TraceLayout {
    fn from(layout: &TraceLayout) -> Self {
        let mut aux_segment_widths = Vec::new();
        let mut aux_segment_rands = Vec::new();

        for i in 0..layout.num_aux_segments() {
            aux_segment_widths.push(layout.get_aux_segment_width(i) as u64);
            aux_segment_rands.push(layout.get_aux_segment_rand_elements(i) as u64);
        }

        Self {
            main_segment_width: layout.main_trace_width() as u64,
            aux_segment_widths,
            aux_segment_rands,
            num_aux_segments: layout.num_aux_segments() as u64,
        }
    }
}

impl From<&ProofOptions> for sdk::ProofOptions {
    fn from(value: &ProofOptions) -> Self {
        let hash_fn: sdk::HashFunction = value.hash_fn().into();
        let field_extension: sdk::FieldExtension = value.field_extension().into();
        let fri_options = value.to_fri_options();

        Self {
            num_queries: value.num_queries() as u32,
            blowup_factor: value.blowup_factor() as u32,
            grinding_factor: value.grinding_factor(),
            hash_fn: hash_fn.into(),
            field_extension: field_extension.into(),
            fri_folding_factor: fri_options.folding_factor() as u32,
            fri_max_remainder_size: fri_options.max_remainder_size() as u32,
            // this should be configurable
            prime_field: sdk::PrimeField::Goldilocks.into(),
        }
    }
}

impl From<HashFunction> for sdk::HashFunction {
    fn from(value: HashFunction) -> Self {
        match value {
            HashFunction::Blake2s_256 => Self::Blake2s,
            HashFunction::Blake3_192 => todo!(),
            HashFunction::Blake3_256 => todo!(),
            HashFunction::Sha3_256 => todo!(),
        }
    }
}

impl From<FieldExtension> for sdk::FieldExtension {
    fn from(value: FieldExtension) -> Self {
        match value {
            FieldExtension::None => Self::None,
            FieldExtension::Quadratic => todo!(),
            FieldExtension::Cubic => todo!(),
        }
    }
}

impl<const N: usize> From<&ByteDigest<N>> for sdk::Digest {
    fn from(value: &ByteDigest<N>) -> Self {
        Self {
            size: N as u32,
            data: value.to_bytes().to_vec(),
        }
    }
}

impl IntoSdk<Commitments, &ProcessorAir> for sdk::Commitments {
    fn into_sdk(input: Commitments, params: &ProcessorAir) -> Self {
        let num_trace_segments = params.trace_layout().num_segments();
        let lde_domain_size = params.lde_domain_size();
        let fri_options = params.options().to_fri_options();
        let num_fri_layers = fri_options.num_fri_layers(lde_domain_size);

        let (trace_commitments, constraint_commitment, fri_commitments) = input
            .clone()
            .parse::<Blake2s_256<Felt>>(num_trace_segments, num_fri_layers)
            .unwrap();

        let constraint_root: sdk::Digest = (&constraint_commitment).into();

        Self {
            trace_roots: trace_commitments.iter().map(|d| d.into()).collect(),
            constraint_root: Some(constraint_root),
            fri_roots: fri_commitments.iter().map(|d| d.into()).collect(),
        }
    }
}

impl From<Table<Felt>> for sdk::Table {
    fn from(table: Table<Felt>) -> Self {
        // table saved as a single dim array
        let data = table.data().iter().map(|e| e.into()).collect::<Vec<_>>();

        Self {
            n_rows: table.num_rows() as u32,
            n_cols: table.num_columns() as u32,
            elements: data,
        }
    }
}

impl IntoSdk<Vec<Queries>, &ProcessorAir> for sdk::TraceQueries {
    fn into_sdk(input: Vec<Queries>, params: &ProcessorAir) -> Self {
        let trace_queries =
            TraceQueries::<Felt, Blake2s_256<Felt>>::new(input.clone(), params).unwrap();

        Self {
            main_states: Some(trace_queries.main_states.into()),
            aux_states: trace_queries.aux_states.map(|t| t.into()),
            query_proofs: trace_queries
                .query_proofs
                .iter()
                .map(|p| sdk::BatchMerkleProof {
                    nodes: p.serialize_nodes(),
                })
                .collect::<Vec<_>>(),
        }
    }
}
