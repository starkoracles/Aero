# SDK api
Users can leverage the npm api library to proofs in the browser. The library leverages protobuf/grpc to create
a consistent user interaction across the sdk, prover, and grpc backend service.

* [Prover interface](https://github.com/starkoracles/starknet-miden-verifier/blob/proto-api/sdk-api/src/sdk.ts)
* [GRPC service](https://github.com/starkoracles/starknet-miden-verifier/blob/proto-api/sdk-api/proto/service.proto)
* [Usage example](https://github.com/starkoracles/starknet-miden-verifier/blob/proto-api/sdk-api/src/demo/index.ts#L21)

# Demo
```
npm run build
npm run serve:demo
```