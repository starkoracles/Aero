# Miden verifier on Starknet
[MidenVM](https://github.com/0xPolygonMiden/miden-vm) is an MIT licensed StarkVM which can now be verified on Starknet.

[![Twitter URL](https://img.shields.io/twitter/follow/stark_oracle?style=social)](https://twitter.com/stark_oracle)

**The code in the project is incomplete, DO NOT USE IN PRODUCTION!!!**

## Why should you care?

* Composability - Different zkVMs take different design trade-offs. Making Miden proofs verifiable on Starknet increases the design space
application can be built. For example, certain VMs might make trade-offs that make it cheaper to compute keccak, by allowing composability
we can offload those computations to the appropriate VMs but leverage network effects to achieve cost-effectiveness.

* Privacy - Since Miden prover is fully open-source, you can generate proofs locally and verify them on Starknet. The proof itself
will divulge a lot less about the activity than the current approach of running your contracts directly on Starknet.

* Mobile - Miden is an extremely efficient prover which can be run with much smaller RAM configurations. Furthermore, Miden prover can 
be compiled down to WASM which allows easy integration with web applications, including Metamask.

## Requirements
- Python 3.9 (Activate environment: `source ~/cairo_venv/bin/activate`)
- Cairo. [Installation Guide](https://www.cairo-lang.org/docs/quickstart.html) (Programming language for provable programs)
- [Protostar](https://docs.swmansion.com/protostar/docs/tutorials/installation) (Automated testing)
- Rustup

## Commands

### Generate proof
```
make generate-proof
```

### Verify in Cairo
```
make integration_test
```

## Roadmap

* Add FRI verification
* Resolve code TODO - Trace queries 4 -> 54
* Add AIR verification
* Enable extension field for security 
* Deploy on Starknet

## Acknowledgements

This code is heavily reliant on the work done by [ZeroSync](https://github.com/ZeroSync/ZeroSync) and [Max Gillet](https://github.com/maxgillett), please give them a star for their great work!