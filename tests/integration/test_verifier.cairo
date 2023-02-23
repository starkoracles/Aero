%lang starknet

from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.hash import HashBuiltin
from starkware.cairo.common.alloc import alloc

from stark_verifier.air.stark_proof import read_stark_proof, StarkProof
from stark_verifier.air.pub_inputs import read_public_inputs, PublicInputs
from stark_verifier.stark_verifier import verify
from stark_verifier.crypto.random import random_coin_new, seed_with_pub_inputs, draw_integers
from starkware.cairo.common.cairo_blake2s.blake2s import finalize_blake2s, blake2s_as_words

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
        expected_program_hash_elements = [2541413064022245539, 7129587402699328827, 5589074863266416554, 8033675306619022710]
        for i in range(ids.pub_inputs.program_hash_len):
            assert memory[ids.pub_inputs.program_hash + i] == expected_program_hash_elements[i]
        print('program_hash_len:', ids.pub_inputs.program_hash_len)
        print('stack_inputs:', ids.pub_inputs.stack_inputs)
        print('stack_inputs_len:', ids.pub_inputs.stack_inputs_len)
        print('ouputs.stack:', ids.pub_inputs.outputs.stack)
        print('outputs.stack_len:', ids.pub_inputs.outputs.stack_len)
        print('ouputs.overflow_addrs:', ids.pub_inputs.outputs.overflow_addrs)
        print('outputs.overflow_addrs_len:', ids.pub_inputs.outputs.overflow_addrs_len)
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

    verify(proof, pub_inputs);
    return ();
}

@external
func test_draw{range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pedersen_ptr: HashBuiltin*}() {
    alloc_locals;
    let (blake2s_ptr: felt*) = alloc();
    local blake2s_ptr_start: felt* = blake2s_ptr;

    %{
        from tests.integration.utils import parse_public_inputs
        json_data = parse_public_inputs('fib')
    %}
    let pub_inputs: PublicInputs* = read_public_inputs();
    let public_coin_seed: felt* = seed_with_pub_inputs{blake2s_ptr=blake2s_ptr}(pub_inputs);

    %{
        seed = [hex(memory[ids.public_coin_seed+ptr]) for ptr in range(8)] 
        print('seed:', seed)
    %}

    with blake2s_ptr {
        let public_coin = random_coin_new(public_coin_seed, 32);
    }

    let (local elements: felt*) = alloc();
    let n_elements = 20;
    let domain_size = 64;

    with public_coin, blake2s_ptr {
        draw_integers(n_elements=n_elements, elements=elements, domain_size=64);
    }
    %{
        expected = [56, 55, 46, 17, 44, 61, 8, 43, 39, 19, 3, 26, 31, 30, 4, 37, 40, 49, 7, 29]
        for i in range(ids.n_elements):
            assert memory[ids.elements + i] == expected[i]
    %}
    finalize_blake2s(blake2s_ptr_start, blake2s_ptr);
    return ();
}
