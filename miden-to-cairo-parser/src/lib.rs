#![feature(array_chunks)]
use miden_air::StarkField;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read};
use winter_crypto::RandomCoin;
use winter_fri::FriProof;
use winter_math::log2;

use winter_air::proof::{Commitments, Context, OodFrame, Queries, Table};
pub use winter_air::{
    ConstraintCompositionCoefficients, DeepCompositionCoefficients, EvaluationFrame, ProofOptions,
    TraceLayout,
};
use winter_crypto::{hash::ByteDigest, hashers::Blake2s_256, Digest};
pub use winterfell::{Air, AirContext, FieldExtension, HashFunction, StarkProof};
use winterfell::{AuxTraceRandElements, ConstraintQueries, DeepComposer, TraceQueries};

pub use miden_air::{Felt, ProcessorAir, PublicInputs};
use miden_core::ProgramOutputs;

pub mod memory;
use memory::{DynamicMemory, Writeable, WriteableWith};

#[derive(Serialize, Deserialize)]
pub struct BinaryProofData {
    pub input_bytes: Vec<u8>,
    pub proof_bytes: Vec<u8>,
}

impl BinaryProofData {
    pub fn from_file(file_path: &String) -> BinaryProofData {
        let file = File::open(file_path).unwrap();
        let mut data = Vec::new();
        BufReader::new(file)
            .read_to_end(&mut data)
            .expect("Unable to read data");
        bincode::deserialize(&data).unwrap()
    }
}

impl Writeable for PublicInputs {
    fn write_into(&self, target: &mut DynamicMemory) {
        let program_hash_elements = self.program_hash.as_elements();
        target.write_sized_array(program_hash_elements.to_vec());
        target.write_sized_array(self.stack_inputs.clone());
        self.outputs.write_into(target);
    }
}

impl Writeable for ProgramOutputs {
    fn write_into(&self, target: &mut DynamicMemory) {
        target.write_sized_array(self.stack.clone());
        target.write_sized_array(self.overflow_addrs.clone());
    }
}

impl Writeable for (&u64, Felt) {
    fn write_into(&self, target: &mut DynamicMemory) {
        self.0.write_into(target);
        Writeable::write_into(&self.1, target);
    }
}

impl WriteableWith<&ProcessorAir> for StarkProof {
    fn write_into(&self, target: &mut DynamicMemory, air: &ProcessorAir) {
        self.context.write_into(target);
        self.commitments.write_into(target, air);
        self.ood_frame.write_into(target, air);
        self.pow_nonce.write_into(target);
        self.trace_queries.write_into(target, air);
        self.constraint_queries.write_into(target, air);
        target.write_sized_array(self.fri_proof.parse_remainder::<Felt>().unwrap());
    }
}

impl Writeable for Context {
    fn write_into(&self, target: &mut DynamicMemory) {
        self.trace_layout().write_into(target);
        self.trace_length().write_into(target);
        log2(self.get_trace_info().length()).write_into(target);

        self.get_trace_info().meta().len().write_into(target);
        target.write_array(self.get_trace_info().meta().to_vec());

        self.field_modulus_bytes().len().write_into(target);
        target.write_array(self.field_modulus_bytes().to_vec());

        self.options().write_into(target);

        self.lde_domain_size().write_into(target);
    }
}

impl WriteableWith<&ProcessorAir> for Commitments {
    fn write_into(&self, target: &mut DynamicMemory, air: &ProcessorAir) {
        let num_trace_segments = air.trace_layout().num_segments();
        let lde_domain_size = air.lde_domain_size();
        let fri_options = air.options().to_fri_options();
        let num_fri_layers = fri_options.num_fri_layers(lde_domain_size);

        let (trace_commitments, constraint_commitment, fri_commitments) = self
            .clone()
            .parse::<Blake2s_256<Felt>>(num_trace_segments, num_fri_layers)
            .unwrap();

        target.write_array(
            trace_commitments
                .iter()
                .map(|x| ByteDigest::new(x.as_bytes()))
                .collect::<Vec<_>>(),
        );

        let mut temp_memory = target.alloc();
        ByteDigest::new(constraint_commitment.as_bytes()).write_into(&mut temp_memory);

        fri_commitments.len().write_into(target);
        target.write_array(
            fri_commitments
                .iter()
                .map(|x| ByteDigest::new(x.as_bytes()))
                .collect::<Vec<_>>(),
        );
    }
}

impl WriteableWith<&ProcessorAir> for OodFrame {
    fn write_into(&self, target: &mut DynamicMemory, air: &ProcessorAir) {
        let main_trace_width = air.trace_layout().main_trace_width();
        let aux_trace_width = air.trace_layout().aux_trace_width();
        let num_evaluations = air.ce_blowup_factor();
        let (ood_main_trace_frame, ood_aux_trace_frame, ood_constraint_evaluations) = self
            .clone()
            .parse::<Felt>(main_trace_width, aux_trace_width, num_evaluations)
            .unwrap();

        ood_main_trace_frame.write_into(target);
        ood_aux_trace_frame.unwrap().write_into(target);
        target.write_sized_array(ood_constraint_evaluations);
    }
}

impl WriteableWith<&ProcessorAir> for Vec<Queries> {
    fn write_into(&self, target: &mut DynamicMemory, air: &ProcessorAir) {
        let trace_queries =
            TraceQueries::<Felt, Blake2s_256<Felt>>::new(self.clone(), air).unwrap();
        trace_queries.main_states.write_into(target);
        trace_queries.aux_states.unwrap().write_into(target);
    }
}

impl WriteableWith<&ProcessorAir> for Queries {
    fn write_into(&self, target: &mut DynamicMemory, air: &ProcessorAir) {
        let constraint_queries =
            ConstraintQueries::<Felt, Blake2s_256<Felt>>::new(self.clone(), air).unwrap();
        constraint_queries.evaluations.write_into(target);
    }
}

impl Writeable for Table<Felt> {
    fn write_into(&self, target: &mut DynamicMemory) {
        self.num_rows().write_into(target);
        self.num_columns().write_into(target);
        target.write_array(self.data().to_vec());
    }
}

impl Writeable for ByteDigest<32> {
    fn write_into(&self, target: &mut DynamicMemory) {
        for chunk in self.0.to_vec().array_chunks::<4>() {
            let int = u32::from_le_bytes(*chunk);
            int.write_into(target);
        }
    }
}

impl Writeable for TraceLayout {
    fn write_into(&self, target: &mut DynamicMemory) {
        let mut aux_segment_widths = Vec::new();
        let mut aux_segment_rands = Vec::new();

        for i in 0..self.num_aux_segments() {
            aux_segment_widths.push(self.get_aux_segment_width(i));
            aux_segment_rands.push(self.get_aux_segment_rand_elements(i));
        }

        self.main_trace_width().write_into(target);
        self.num_aux_segments().write_into(target);
        target.write_array(aux_segment_widths);
        target.write_array(aux_segment_rands);
    }
}

impl Writeable for ProofOptions {
    fn write_into(&self, target: &mut DynamicMemory) {
        self.num_queries().write_into(target);
        self.blowup_factor().write_into(target);
        log2(self.blowup_factor()).write_into(target);
        self.grinding_factor().write_into(target);

        self.hash_fn().write_into(target);
        self.field_extension().write_into(target);

        let fri_options = self.to_fri_options();
        fri_options.folding_factor().write_into(target);
        fri_options.max_remainder_size().write_into(target);
    }
}

impl Writeable for HashFunction {
    fn write_into(&self, target: &mut DynamicMemory) {
        (*self as u8).write_into(target);
    }
}

impl Writeable for FieldExtension {
    fn write_into(&self, target: &mut DynamicMemory) {
        (*self as u8).write_into(target);
    }
}

impl Writeable for EvaluationFrame<Felt> {
    fn write_into(&self, target: &mut DynamicMemory) {
        target.write_sized_array(self.current().to_vec());
        target.write_sized_array(self.next().to_vec());
    }
}

impl Writeable for Felt {
    fn write_into(&self, target: &mut DynamicMemory) {
        let raw = self.as_int();
        let mut hex_string = "0x".to_owned();
        for byte in raw.to_be_bytes().iter() {
            hex_string.push_str(&format!("{:02x}", byte));
        }
        target.write_hex_value(hex_string);
    }
}

impl Writeable for [u8; 32] {
    // Convert 32 x u8 to 8 x u32
    fn write_into(&self, target: &mut DynamicMemory) {
        let mut uint32_array = Vec::new();
        for i in 0..8 {
            let mut uint32 = 0;
            // Store as big endian
            let mut base = 1 << 24;
            for j in 0..4 {
                let index = (i * 4 + j) as usize;
                uint32 += base * self[index] as usize;
                base >>= 8;
            }
            uint32_array.push(uint32);
        }
        target.write_array(uint32_array);
    }
}

pub struct ProcessorAirParams<'a> {
    pub proof: &'a StarkProof,
    pub public_inputs: &'a PublicInputs,
}

impl WriteableWith<ProcessorAirParams<'_>> for ProcessorAir {
    fn write_into(&self, target: &mut DynamicMemory, params: ProcessorAirParams) {
        // Layout
        self.trace_layout().main_trace_width().write_into(target);
        self.trace_layout().aux_trace_width().write_into(target);

        let mut aux_segment_widths = vec![];
        let mut aux_segment_rands = vec![];
        for segment_idx in 0..self.trace_layout().num_aux_segments() {
            aux_segment_widths.push(self.trace_layout().get_aux_segment_width(segment_idx));
            aux_segment_rands.push(
                self.trace_layout()
                    .get_aux_segment_rand_elements(segment_idx),
            );
        }
        target.write_array(aux_segment_widths);
        target.write_array(aux_segment_rands);
        self.trace_layout().num_aux_segments().write_into(target);

        // Context
        self.options().write_into(target);
        params.proof.context.write_into(target);

        self.context()
            .num_transition_constraints()
            .write_into(target);
        self.context().num_assertions().write_into(target);

        self.ce_blowup_factor().write_into(target);
        // self.eval_frame_size::<Felt>().write_into(target);

        self.trace_domain_generator().write_into(target);
        self.lde_domain_generator().write_into(target);

        // pub_inputs is a pointer to a PublicInput
        let mut child_target = target.alloc();
        params.public_inputs.write_into(&mut child_target);
    }
}

impl Writeable for ConstraintCompositionCoefficients<Felt> {
    fn write_into(&self, target: &mut DynamicMemory) {
        let mut transition_a = Vec::new();
        let mut transition_b = Vec::new();
        for elem in self.transition.iter().cloned() {
            transition_a.push(elem.0);
            transition_b.push(elem.1);
        }
        target.write_array(transition_a);
        target.write_array(transition_b);

        let mut boundary_a = Vec::new();
        let mut boundary_b = Vec::new();
        for elem in self.boundary.iter().cloned() {
            boundary_a.push(elem.0);
            boundary_b.push(elem.1);
        }
        target.write_array(boundary_a);
        target.write_array(boundary_b);
    }
}

impl Writeable for AuxTraceRandElements<Felt> {
    fn write_into(&self, target: &mut DynamicMemory) {
        // let mut child_target = target.alloc();
        for elems in self.0.iter().cloned() {
            target.write_array(elems);
        }
    }
}

impl Writeable for RandomCoin<Felt, Blake2s_256<Felt>> {
    fn write_into(&self, target: &mut DynamicMemory) {
        self.seed.as_bytes().write_into(target);
        self.counter.write_into(target);
    }
}

impl Writeable for DeepCompositionCoefficients<Felt> {
    fn write_into(&self, target: &mut DynamicMemory) {
        let mut child_target = target.alloc();
        for elem in &self.trace {
            child_target.write_sized_array(vec![elem.0, elem.1, elem.2]);
        }
        target.write_array(self.constraints.clone());
        self.degree.0.write_into(target);
        self.degree.1.write_into(target);
    }
}

impl Writeable for DeepComposer<Felt> {
    fn write_into(&self, target: &mut DynamicMemory) {
        self.cc.write_into(target);
        target.write_array(self.x_coordinates.to_vec());
        self.z[0].write_into(target);
        self.z[1].write_into(target);
    }
}

impl WriteableWith<&[usize]> for TraceQueries<Felt, Blake2s_256<Felt>> {
    fn write_into(&self, target: &mut DynamicMemory, indexes: &[usize]) {
        for query_proof in &self.query_proofs {
            let paths = query_proof.into_paths(indexes).unwrap();
            // for p in &paths[0] {
            //     for word in p.to_words() {
            //         println!("{:2x}", word);
            //     }
            // }
            let mut child_target = target.alloc();
            for path in paths {
                child_target.write_sized_array(path);
            }
        }
    }
}

impl WriteableWith<&[usize]> for ConstraintQueries<Felt, Blake2s_256<Felt>> {
    fn write_into(&self, target: &mut DynamicMemory, indexes: &[usize]) {
        let paths = self.query_proofs.into_paths(indexes).unwrap();
        let mut child_target = target.alloc();
        for path in paths {
            child_target.write_sized_array(path);
        }
    }
}

pub struct FriProofParams<'a> {
    pub air: &'a ProcessorAir,
    pub indexes: &'a Vec<usize>,
}

impl WriteableWith<FriProofParams<'_>> for FriProof {
    fn write_into(&self, target: &mut DynamicMemory, params: FriProofParams) {
        let air = &params.air;
        let folding_factor = air.options().to_fri_options().folding_factor();
        let (queries_values, proofs) = self
            .clone()
            .parse_layers::<Blake2s_256<Felt>, Felt>(air.lde_domain_size(), folding_factor)
            .unwrap();
        let mut indices = params.indexes.clone();
        let mut source_domain_size = air.lde_domain_size();

        for (proof, query_values) in proofs.into_iter().zip(queries_values) {
            indices = fold_positions(&indices, source_domain_size, folding_factor);
            let mut child_target = target.alloc();
            source_domain_size /= folding_factor;
            let paths = proof.into_paths(&indices).unwrap();
            for (index, path) in paths.iter().enumerate() {
                child_target.write_sized_array(path.to_vec());
                let query_values =
                    &query_values[index * folding_factor..(index + 1) * folding_factor];
                child_target.write_array(query_values.to_vec());
            }
        }
    }
}

pub fn fold_positions(
    positions: &[usize],
    source_domain_size: usize,
    folding_factor: usize,
) -> Vec<usize> {
    let target_domain_size = source_domain_size / folding_factor;
    let mut result = Vec::new();
    for position in positions {
        let position = position % target_domain_size;
        // make sure we don't record duplicated values
        if !result.contains(&position) {
            result.push(position);
        }
    }
    result
}
