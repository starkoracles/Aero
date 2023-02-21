%lang starknet

from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.hash import HashBuiltin

from stark_verifier.air.stark_proof import read_stark_proof, StarkProof
from stark_verifier.air.pub_inputs import read_public_inputs, PublicInputs, read_seed_data, SeedData
from stark_verifier.stark_verifier import verify

// / Test deserialization of StarkProof from file
@external
func test_read_stark_proof{}() {
    %{
        from tests.integration.utils import parse_proof
        json_data = parse_proof('fib')
    %}
    let proof: StarkProof* = read_stark_proof();

    %{
        # TODO: Assert that all proof fields were deserialized correctly using utils.py
        print('main_segment_width:', ids.proof.context.trace_layout.main_segment_width)
        print('num_queries:', ids.proof.context.options.num_queries)
        print('blowup_factor:', ids.proof.context.options.blowup_factor)
        print('pow_nonce:', ids.proof.pow_nonce)
    %}
    return ();
}

// / Test deserialization of PublicInputs from file
@external
func test_read_pub_inputs{}() {
    %{
        from tests.integration.utils import parse_public_inputs
        json_data = parse_public_inputs('fib')
    %}
    let pub_inputs: PublicInputs* = read_public_inputs();

    %{
        # TODO: Assert that all proof fields were deserialized correctly using utils.py
        print('program_hash:', ids.pub_inputs.program_hash)
        print('stack_inputs:', ids.pub_inputs.stack_inputs)
        print('ouputs.stack:', ids.pub_inputs.outputs.stack)
        print('ouputs.overflow_addrs:', ids.pub_inputs.outputs.overflow_addrs)
    %}
    return ();
}

@external
func test_verify{range_check_ptr, pedersen_ptr: HashBuiltin*, bitwise_ptr: BitwiseBuiltin*}() {
    %{
        from tests.integration.utils import parse_public_inputs
        json_data = parse_public_inputs('fib')
    %}
    let pub_inputs: PublicInputs* = read_public_inputs();

    %{
        from tests.integration.utils import parse_proof
        json_data = parse_proof('fib')
    %}
    let proof: StarkProof* = read_stark_proof();

    %{
        from tests.integration.utils import parse_seed_data
        json_data = parse_seed_data('fib')
    %}
    let seed_data: SeedData* = read_seed_data();

    verify(proof, pub_inputs, seed_data);
    return ();
}
