use std::io::Result;

fn main() -> Result<()> {
    let mut config = prost_build::Config::new();
    config.btree_map(&["."]);
    config.compile_protos(
        &[
            "service.proto",
            "stark_proof.proto",
            "miden_vm.proto",
            "commitments.proto",
            "common.proto",
            "context.proto",
            "fri_proof.proto",
            "miden_prover.proto",
            "ood_frame.proto",
            "queries.proto",
        ],
        &["proto"],
    )?;
    Ok(())
}
