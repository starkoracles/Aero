use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct ProofData {
    pub input_bytes: Vec<u8>,
    pub proof_bytes: Vec<u8>,
}
