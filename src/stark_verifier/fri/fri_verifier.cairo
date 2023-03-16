from stark_verifier.air.air_instance import AirInstance
from stark_verifier.channel import Channel
from stark_verifier.air.stark_proof import ProofOptions
from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from stark_verifier.crypto.random import PublicCoin, reseed, draw, reseed_endian, hash_elements
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.memset import memset
from starkware.cairo.common.math import assert_le, assert_not_zero, unsigned_div_rem
from starkware.cairo.common.math_cmp import is_le
from starkware.cairo.common.pow import pow
from utils.pow2 import pow2
from stark_verifier.channel import verify_merkle_proof, QueriesProofs, QueriesProof, read_remainder
from stark_verifier.fri.polynomials import lagrange_eval, interpolate_poly_and_verify
from stark_verifier.crypto.random import contains
from utils.math_goldilocks import mul_g, sub_g, add_g, div_g, pow_g
from crypto.hash_utils import assert_hashes_equal
from starkware.cairo.common.registers import get_fp_and_pc
from stark_verifier.utils import Vec

// TODO HARDCODE - use param instead
const FOLDING_FACTOR = 8;
const g = 7;  // domain offset for goldilocks
const HASH_FELT_SIZE = 8;
const NUM_QUERIES = 27;

struct FriQueryProof {
    length: felt,
    path: felt*,
    values: felt*,
}

struct FriOptions {
    folding_factor: felt,
    max_remainder_size: felt,
    blowup_factor: felt,
}

func to_fri_options(proof_options: ProofOptions) -> FriOptions {
    let folding_factor = proof_options.fri_folding_factor;
    let max_remainder_size = proof_options.fri_max_remainder_size;  // stored as power of 2
    let fri_options = FriOptions(folding_factor, max_remainder_size, proof_options.blowup_factor);
    return fri_options;
}

struct FriVerifier {
    max_poly_degree: felt,
    domain_size: felt,
    domain_generator: felt,
    layer_commitments: felt*,
    layer_alphas: felt*,
    options: FriOptions,
    num_partitions: felt,
}

func _fri_verifier_new{
    range_check_ptr, blake2s_ptr: felt*, bitwise_ptr: BitwiseBuiltin*, public_coin: PublicCoin
}(
    options: FriOptions,
    max_degree_plus_1,
    layer_commitment_ptr: felt*,
    layer_alpha_ptr: felt*,
    count,
) {
    if (count == 0) {
        return ();
    }
    alloc_locals;

    reseed_endian(layer_commitment_ptr);
    let alpha = draw();
    assert [layer_alpha_ptr] = alpha;

    _fri_verifier_new(
        options,
        max_degree_plus_1 / options.folding_factor,
        layer_commitment_ptr + 8,
        layer_alpha_ptr + 1,
        count - 1,
    );
    return ();
}

func read_fri_proofs{
    range_check_ptr, blake2s_ptr: felt*, channel: Channel, bitwise_ptr: BitwiseBuiltin*
}(positions: felt*) -> FriQueryProof** {
    alloc_locals;

    let (local fri_queries_proof_ptr: FriQueryProof**) = alloc();
    %{
        from src.stark_verifier.utils import read_fri_queries_proofs
        read_fri_queries_proofs(ids.positions, ids.fri_queries_proof_ptr, ids.NUM_QUERIES, memory, segments)
    %}

    return fri_queries_proof_ptr;
}

func fri_verifier_new{
    range_check_ptr,
    blake2s_ptr: felt*,
    bitwise_ptr: BitwiseBuiltin*,
    public_coin: PublicCoin,
    channel: Channel,
}(options: FriOptions, max_poly_degree) -> FriVerifier {
    alloc_locals;

    let _next_power_of_two = next_power_of_two(max_poly_degree);
    // using normal mul since it should not overflow
    let domain_size = _next_power_of_two * options.blowup_factor;

    let domain_size_log2 = log2(domain_size);
    let domain_generator = get_root_of_unity(domain_size_log2);
    // air.trace_domain_generator ?
    // air.lde_domain_generator ?

    let num_partitions = 1;
    // channel.read_fri_num_partitions() ?

    // read layer commitments from the channel and use them to build a list of alphas
    let (layer_alphas) = alloc();
    let layer_commitments = channel.fri_roots;
    _fri_verifier_new(
        options, max_poly_degree + 1, layer_commitments, layer_alphas, channel.fri_roots_len
    );

    let res = FriVerifier(
        max_poly_degree,
        domain_size,
        domain_generator,
        layer_commitments,
        layer_alphas,
        options,
        num_partitions,
    );
    return res;
}

func next_power_of_two{range_check_ptr}(x) -> felt {
    // leaving regular cairo field math since it shouldn't overflow or underflow
    // is this secure?
    alloc_locals;
    local n_bits;
    %{ ids.n_bits = len( bin(ids.x - 1).replace('0b', '') ) %}
    let next_power_of_two = pow2(n_bits);
    local next_power_of_two = next_power_of_two;
    local x2_1 = x * 2 - 1;
    with_attr error_message("{x} <= {next_power_of_two} <= {x2_1}") {
        assert_le(x, next_power_of_two);
        assert_le(next_power_of_two, x * 2 - 1);
    }
    return next_power_of_two;
}

const TWO_ADICITY = 32;
const TWO_ADIC_ROOT_OF_UNITY = 1753635133440165772;

func get_root_of_unity{range_check_ptr}(n) -> felt {
    with_attr error_message("cannot get root of unity for n = 0") {
        assert_not_zero(n);
    }
    with_attr error_message("order cannot exceed 2^{TWO_ADICITY}") {
        assert_le(n, TWO_ADICITY);
    }
    let base = sub_g(TWO_ADICITY, n);
    let power = pow_g(2, base);
    let root_of_unity = pow_g(TWO_ADIC_ROOT_OF_UNITY, power);
    return root_of_unity;
}

func log2(n) -> felt {
    alloc_locals;
    local n_bits;
    %{ ids.n_bits = len( bin(ids.n - 1).replace('0b', '') ) %}
    let next_power_of_two = pow2(n_bits);
    with_attr error_message("n must be a power of two") {
        assert next_power_of_two = n;
    }
    return n_bits;
}

func verify_fri_merkle_proofs{range_check_ptr, blake2s_ptr: felt*, bitwise_ptr: BitwiseBuiltin*}(
    proofs: QueriesProof*,
    positions: felt*,
    trace_roots: felt*,
    loop_counter,
    evaluations: felt*,
    n_evaluations: felt,
) {
    if (loop_counter == 0) {
        return ();
    }

    // let digest = hash_elements(n_elements=n_evaluations, elements=evaluations);  // TODO: hash the evaluation correctly
    // assert_hashes_equal(digest, proofs[0].digests);

    verify_merkle_proof(proofs[0].length, proofs[0].digests, positions[0], trace_roots);
    verify_fri_merkle_proofs(
        &proofs[1],
        positions + 1,
        trace_roots,
        loop_counter - 1,
        evaluations + n_evaluations,
        n_evaluations,
    );
    return ();
}

func num_fri_layers{range_check_ptr}(fri_verifier: FriVerifier*, domain_size) -> felt {
    let is_leq = is_le(fri_verifier.options.max_remainder_size + 1, domain_size);
    if (is_leq == 0) {
        return 0;
    }
    let res = num_fri_layers(fri_verifier, domain_size / fri_verifier.options.folding_factor);
    return 1 + res;
}

// pre-compute roots of unity used in computing x coordinates in the folded domain
func compute_folding_roots{range_check_ptr}(omega_folded: felt*, omega, log_degree: felt, i: felt) {
    if (i == FOLDING_FACTOR) {
        return ();
    }
    let degree = pow_g(2, log_degree);
    let new_domain_size = degree / FOLDING_FACTOR * i;
    let res = pow_g(omega, new_domain_size);
    assert [omega_folded] = res;
    compute_folding_roots(omega_folded + 1, omega, log_degree, i + 1);
    return ();
}

func assign_folding_roots_loop{range_check_ptr}(
    idx: felt, folding_roots: felt*, xe: felt, x_values: felt*
) {
    if (idx == 0) {
        return ();
    }

    let r = mul_g(folding_roots[idx - 1], xe);
    assert x_values[idx - 1] = r;

    return assign_folding_roots_loop(idx - 1, folding_roots, xe, x_values);
}

func verify_layers{range_check_ptr, blake2s_ptr: felt*, bitwise_ptr: BitwiseBuiltin*}(
    omega: felt,
    alphas: felt*,
    position: felt,
    evaluations: felt*,
    num_layer_evaluations: felt,
    num_layers: felt,
    previous_eval: felt,
    fri_proofs: FriQueryProof**,
    modulus: felt,
    verified_positions: felt**,
    verified_positions_len: felt*,
    next_verified_positions_len: felt*,
    layer_commitments: felt*,
    folding_roots: felt*,
    remainders: Vec*,
) {
    alloc_locals;
    if (num_layers == 0) {
        // Check that the claimed remainder is equal to the final evaluation.
        // TODO reenable once remainders are ready
        // assert_contains(remainders.elements, remainders.n_elements, query_evaluations[0]);
        return ();
    }

    let (local query_position, folded_position) = unsigned_div_rem(position, modulus);

    // Check if we have already verified this folded_position
    local index: felt;
    let curr_len = verified_positions_len[0];
    let prev_positions = verified_positions[0];
    // This hint gives us the index of the position if included, or it returns -1
    %{
        from src.stark_verifier.utils import index_of
        ids.index = index_of(ids.prev_positions, ids.curr_len, ids.folded_position, memory)
    %}
    // If so, copy the previous verified_positions_len, and we're done
    if (index != -1) {
        // Verify the index given by the hint
        assert folded_position = verified_positions[0][index];
        // Copy previous lenghts
        memcpy(next_verified_positions_len, verified_positions_len, num_layers);
        return ();
    }
    let index = curr_len;

    // Otherwise, verify this folded_position
    assert verified_positions[0][index] = folded_position;
    // and add it to verified_positions
    assert next_verified_positions_len[0] = index + 1;

    // Verify that evaluations are consistent with the layer commitment
    let query_proof = fri_proofs[0][index];
    verify_merkle_proof(query_proof.length, query_proof.path, folded_position, layer_commitments);
    let leaf_hash = hash_elements(n_elements=FOLDING_FACTOR, elements=query_proof.values);
    assert_hashes_equal(leaf_hash, query_proof.path);
    let is_contained = contains(evaluations[0], query_proof.values, FOLDING_FACTOR);
    assert_not_zero(is_contained);

    // Compare poly evaluations to the query proof
    let query_value = query_proof.values[query_position];
    assert query_value = evaluations[0];

    // Interpolate the evaluations at the x-coordinates, and evaluate at alpha.
    let alpha = [alphas];
    let xe = pow_g(omega, folded_position);
    local xe = mul_g(xe, g);
    let (local x_values) = alloc();

    tempvar i = FOLDING_FACTOR;

    assign_folding_roots_loop(i, folding_roots, xe, x_values);

    let previous_eval = lagrange_eval(query_proof.values, x_values, FOLDING_FACTOR, alpha);

    // Update variables for the next layer
    let omega = pow_g(omega, FOLDING_FACTOR);
    let modulus = modulus / FOLDING_FACTOR;
    let (evaluations) = alloc();
    assert evaluations[0] = previous_eval;

    return verify_layers(
        omega,
        alphas + 1,
        folded_position,
        evaluations,
        num_layer_evaluations,
        num_layers - 1,
        previous_eval,
        &fri_proofs[1],
        modulus,
        &verified_positions[1],
        &verified_positions_len[1],
        &next_verified_positions_len[1],
        &layer_commitments[HASH_FELT_SIZE],
        folding_roots,
        remainders,
    );
}

func verify_queries{
    range_check_ptr, channel: Channel, blake2s_ptr: felt*, bitwise_ptr: BitwiseBuiltin*
}(
    fri_verifier: FriVerifier*,
    positions: felt*,
    evaluations: felt*,
    num_queries: felt,
    fri_proofs: FriQueryProof**,
    num_layers: felt,
    verified_positions: felt**,
    verified_positions_len: felt*,
    folding_roots: felt*,
    remainders: Vec*,
) {
    if (num_queries == 0) {
        return ();
    }
    alloc_locals;

    // Iterate over the layers within this query
    verify_layers(
        omega=fri_verifier.domain_generator,
        alphas=fri_verifier.layer_alphas,
        position=[positions],
        evaluations=evaluations,
        num_layer_evaluations=FOLDING_FACTOR * num_layers,
        num_layers=num_layers,
        previous_eval=0,
        fri_proofs=fri_proofs,
        modulus=fri_verifier.domain_size / FOLDING_FACTOR,
        verified_positions=verified_positions,
        verified_positions_len=verified_positions_len,
        next_verified_positions_len=&verified_positions_len[num_layers],
        layer_commitments=channel.fri_roots,
        folding_roots=folding_roots,
        remainders=remainders,
    );

    // Iterate over the remaining queries
    verify_queries(
        fri_verifier,
        &positions[1],
        &evaluations[1],
        num_queries - 1,
        fri_proofs,
        num_layers,
        verified_positions,
        &verified_positions_len[num_layers],
        folding_roots,
        remainders,
    );
    return ();
}

func fri_verify{
    range_check_ptr, blake2s_ptr: felt*, channel: Channel, bitwise_ptr: BitwiseBuiltin*
}(fri_verifier: FriVerifier, evaluations: felt*, positions: felt*) {
    alloc_locals;
    let (__fp__, _) = get_fp_and_pc();
    // Read FRI Merkle proofs from a hint
    let fri_proofs = read_fri_proofs(positions);

    // Read remainders from a hint
    // and check that a Merkle tree of the claimed remainders hash to the final layer commitment
    let remainder: Vec = read_remainder();
    let remainder_ptr = remainder;

    let num_layers = num_fri_layers(&fri_verifier, fri_verifier.domain_size);

    // Initialize an empty array of verified positions for each layer
    let (local verified_positions: felt**) = alloc();
    tempvar verified_positions_ptr = verified_positions;
    tempvar n = num_layers;

    init_loop:
    let (array) = alloc();
    assert [verified_positions_ptr] = array;
    tempvar verified_positions_ptr = verified_positions_ptr + 1;
    tempvar n = n - 1;
    jmp init_loop if n != 0;

    let (verified_positions_len: felt*) = alloc();
    memset(verified_positions_len, 0, num_layers);

    // Compute the remaining folded roots of unity
    let (folding_roots) = alloc();
    let log2_domain_size = log2(fri_verifier.domain_size);
    compute_folding_roots(
        omega_folded=folding_roots,
        omega=fri_verifier.domain_generator,
        log_degree=log2_domain_size,
        i=0,
    );

    // Verify a round for each query
    verify_queries(
        &fri_verifier,
        positions,
        evaluations,
        NUM_QUERIES,
        fri_proofs,
        num_layers,
        verified_positions,
        verified_positions_len,
        folding_roots,
        &remainder_ptr,
    );

    return ();
}

// Ensure that a given array contains a particular element
// func assert_contains(array: felt*, array_len, element) {
//     alloc_locals;
//     local index: felt;
//     %{ ids.index = index_of(ids.array, ids.array_len, ids.element, memory) %}
//     // TODO: Do we have to verify that `0 < index < array_len` here?
//     assert element = array[index];
//     return ();
// }
