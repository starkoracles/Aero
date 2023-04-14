use std::convert::TryInto;

use crate::sdk;
use crate::sdk::{MidenProgram, MidenProgramInputs};
use miden::{Assembler, FieldExtension, HashFunction, Program, ProgramInputs, ProofOptions};
use miden_stdlib::StdLibrary;

impl Into<ProgramInputs> for MidenProgramInputs {
    fn into(self) -> ProgramInputs {
        ProgramInputs::new(&self.stack_init, &self.advice_tape, vec![])
            .expect("cannot parse miden program inputs")
    }
}

impl From<u64> for sdk::FieldElement {
    fn from(value: u64) -> Self {
        Self {
            size: 8,
            element: value.to_le_bytes().to_vec(),
        }
    }
}

impl Into<u64> for &sdk::FieldElement {
    fn into(self) -> u64 {
        u64::from_le_bytes(self.element.clone().try_into().unwrap())
    }
}

impl Into<Program> for MidenProgram {
    fn into(self) -> Program {
        Assembler::new()
            .with_module_provider(StdLibrary::default())
            .compile(&self.program)
            .expect("cannot assemble miden program")
    }
}

impl Into<HashFunction> for sdk::HashFunction {
    fn into(self) -> HashFunction {
        match self {
            sdk::HashFunction::Blake2s => HashFunction::Blake2s_256,
        }
    }
}

impl Into<FieldExtension> for sdk::FieldExtension {
    fn into(self) -> FieldExtension {
        match self {
            sdk::FieldExtension::None => FieldExtension::None,
        }
    }
}

impl Into<ProofOptions> for sdk::ProofOptions {
    fn into(self) -> ProofOptions {
        ProofOptions::new(
            self.num_queries as usize,
            self.blowup_factor as usize,
            self.grinding_factor,
            self.hash_fn().into(),
            self.field_extension().into(),
            self.fri_folding_factor as usize,
            self.fri_max_remainder_size as usize,
        )
    }
}
