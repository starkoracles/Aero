from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.pow import pow

from stark_verifier.air.air_instance import AirInstance, DeepCompositionCoefficients
from stark_verifier.air.transitions.frame import EvaluationFrame
from stark_verifier.channel import Table
from stark_verifier.utils import Vec
from utils.math_goldilocks import mul_g, sub_g, add_g, div_g, pow_g

struct DeepComposer {
    cc: DeepCompositionCoefficients,
    x_coordinates: felt*,
    z_curr: felt,
    z_next: felt,
}

func deep_composer_new{range_check_ptr}(
    air: AirInstance, query_positions: felt*, z: felt, cc: DeepCompositionCoefficients
) -> DeepComposer {
    alloc_locals;

    let g = air.trace_domain_generator;
    let g_lde = air.lde_domain_generator;
    let domain_offset = 7;  // TODO why is this hardcoded?

    // TODO: Don't hardcode the number of query positions here
    let (x_coordinates: felt*) = alloc();

    let z_next = [range_check_ptr];
    let range_check_ptr = range_check_ptr + 1;

    // TODO this is insecure - need to properly run goldilocks mul within cairo
    %{
        PG = 18446744069414584321 # 2^64 - 2^32 - 1
        for i in range(27):
            x = pow(ids.g_lde, memory[ids.query_positions + i], PG)
            x = (x * ids.domain_offset) % PG
            memory[ids.x_coordinates + i] = x
        ids.z_next = (ids.z * ids.g) % PG
    %}

    let z_curr = z;

    let res = DeepComposer(cc, x_coordinates, z_curr, z_next);
    return res;
}

func compose_row{range_check_ptr}(
    row_ptr: felt*,
    i: felt,
    ood_frame: EvaluationFrame,
    composer: DeepComposer,
    sum_curr: felt,
    sum_next: felt,
    n_cols: felt,
    cc_offset: felt,
) -> (felt, felt) {
    alloc_locals;

    if (i == n_cols) {
        return (sum_curr, sum_next);
    }

    let row_cell = [row_ptr + i];

    let frame_curr = ood_frame.current[i];
    let curr = sub_g(row_cell, frame_curr);
    local mul_curr = mul_g(curr, composer.cc.trace[i + cc_offset].values[0]);
    local sum_curr_new = add_g(sum_curr, mul_curr);

    tempvar frame_next = ood_frame.next[i];
    let next = sub_g(row_cell, frame_next);
    local mul_next = mul_g(next, composer.cc.trace[i + cc_offset].values[1]);
    local sum_next_new = add_g(sum_next, mul_next);

    return compose_row(
        row_ptr, i + 1, ood_frame, composer, sum_curr_new, sum_next_new, n_cols, cc_offset
    );
}

func compose_loop{range_check_ptr}(
    result_ptr: felt*,
    prev_result_ptr: felt*,
    n: felt,
    composer: DeepComposer,
    queried_trace_states: Table,
    ood_frame: EvaluationFrame,
    cc_offset: felt,
    inner_loop_len: felt,
    add_to_previous_result: felt,
) -> () {
    alloc_locals;

    if (n == 0) {
        return ();
    }

    let row_ptr = queried_trace_states.elements;
    let z_curr = composer.z_curr;
    let z_next = composer.z_next;
    let n_cols = queried_trace_states.n_cols;
    let offset = queried_trace_states.n_rows - n;
    tempvar x_coord_ptr = composer.x_coordinates + offset;

    let (sum_curr, sum_next) = compose_row(
        row_ptr + (offset * n_cols), 0, ood_frame, composer, 0, 0, inner_loop_len, cc_offset
    );

    tempvar x = [x_coord_ptr];
    let x_z_curr = sub_g(x, z_curr);
    let x_z_next = sub_g(x, z_next);
    let s_curr = div_g(sum_curr, x_z_curr);
    let s_next = div_g(sum_next, x_z_next);
    let sum = add_g(s_curr, s_next);

    local to_add;
    if (add_to_previous_result == 1) {
        let prev_sum = [prev_result_ptr];
        to_add = prev_sum;
    } else {
        to_add = 0;
    }

    let sum_new = add_g(sum, to_add);
    assert [result_ptr] = sum_new;

    return compose_loop(
        result_ptr + 1,
        prev_result_ptr + 1,
        n - 1,
        composer,
        queried_trace_states,
        ood_frame,
        cc_offset,
        inner_loop_len,
        add_to_previous_result,
    );
}

func compose_trace_columns{range_check_ptr}(
    composer: DeepComposer,
    queried_main_trace_states: Table,
    queried_aux_trace_states: Table,
    ood_main_frame: EvaluationFrame,
    ood_aux_frame: EvaluationFrame,
) -> felt* {
    alloc_locals;

    // Main trace coefficient rows
    let n_cols = queried_main_trace_states.n_cols;

    let z_curr = composer.z_curr;
    let z_next = composer.z_next;

    // Compose columns of the main segment
    let (local mock_prev_result: felt*) = alloc();
    let (local result: felt*) = alloc();
    // TODO HARDCODE: Don't hardcode the number of query and columns
    tempvar n = 27;
    tempvar result_ptr = result;
    tempvar mock_prev_results_ptr = mock_prev_result;

    // TODO HARDCODE do not hardcode inner loop len
    compose_loop(
        result_ptr,
        mock_prev_results_ptr,
        n,
        composer,
        queried_main_trace_states,
        ood_main_frame,
        0,
        72,
        0,
    );

    // // Aux trace coefficient rows
    let n_cols = queried_aux_trace_states.n_cols;

    let z_curr = composer.z_curr;
    let z_next = composer.z_next;

    // Compose columns of the main segment
    let (local with_aux_result: felt*) = alloc();
    // TODO HARDCODE: Don't hardcode the number of query and columns
    tempvar n = 27;
    tempvar result_ptr = with_aux_result;
    tempvar prev_result_ptr = result;

    // TODO HARDCODE do not hardcode inner loop len
    compose_loop(
        result_ptr, prev_result_ptr, n, composer, queried_aux_trace_states, ood_aux_frame, 72, 9, 1
    );
    return with_aux_result;
}

func compose_constraint_evaluations_add_terms{range_check_ptr}(
    composer: DeepComposer,
    queried_evaluations: Table,
    ood_evaluations: Vec,
    row_ptr: felt*,
    sum: felt,
    idx: felt,
    iter: felt,
) -> felt {
    if (idx == iter) {
        return sum;
    }

    let r = row_ptr[idx];
    let e = ood_evaluations.elements[idx];
    let r1 = sub_g(r, e);
    let c = composer.cc.constraints[idx];
    let r2 = mul_g(r1, c);
    let sum_new = add_g(sum, r2);

    return compose_constraint_evaluations_add_terms(
        composer, queried_evaluations, ood_evaluations, row_ptr, sum_new, idx + 1, iter
    );
}

func compose_constraint_evaluations_loop{range_check_ptr}(
    composer: DeepComposer,
    queried_evaluations: Table,
    ood_evaluations: Vec,
    idx: felt,
    result_ptr: felt*,
    iterations: felt,
    z_m: felt,
) -> () {
    alloc_locals;
    if (idx == iterations) {
        return ();
    }

    let row: felt* = queried_evaluations.elements;
    let n_cols = queried_evaluations.n_cols;
    let row_ptr = row + (idx * n_cols);
    let cc_constraint: felt* = composer.cc.constraints;
    let x_coord_ptr = composer.x_coordinates + idx;

    let sum = compose_constraint_evaluations_add_terms(
        composer, queried_evaluations, ood_evaluations, row_ptr, 0, 0, 8
    );

    tempvar x = [x_coord_ptr];
    let div = sub_g(x, z_m);
    tempvar sum_final = div_g(sum, div);
    assert [result_ptr] = sum_final;

    return compose_constraint_evaluations_loop(
        composer, queried_evaluations, ood_evaluations, idx + 1, result_ptr + 1, iterations, z_m
    );
}

func compose_constraint_evaluations{range_check_ptr}(
    composer: DeepComposer, queried_evaluations: Table, ood_evaluations: Vec
) -> felt* {
    alloc_locals;

    // Compute z^m
    let num_eval_columns = ood_evaluations.n_elements;
    let z = composer.z_curr;
    let z_m = pow_g(z, num_eval_columns);
    local range_check_ptr = range_check_ptr;
    let (local result: felt*) = alloc();

    tempvar result_ptr = result;

    // TODO HARDCODE: don't hardcode number of queries
    compose_constraint_evaluations_loop(
        composer, queried_evaluations, ood_evaluations, 0, result_ptr, 27, z_m
    );

    return result;
}

func combine_compositions(
    composer: DeepComposer, t_composition: felt*, c_composition: felt*
) -> felt* {
    alloc_locals;

    let cc_degree_0 = composer.cc.degree[0];
    let cc_degree_1 = composer.cc.degree[1];

    let (local result: felt*) = alloc();
    // TODO: Don't hardcode number of queries
    tempvar n = 54;
    tempvar t_ptr = t_composition;
    tempvar c_ptr = c_composition;
    tempvar x_coord_ptr = composer.x_coordinates;
    tempvar result_ptr = result;

    loop:
    tempvar x = [x_coord_ptr];
    tempvar t = [t_ptr];
    tempvar c = [c_ptr];
    tempvar composition = t + c;
    tempvar composition = composition * (cc_degree_0 + x * cc_degree_1);
    assert [result_ptr] = composition;

    tempvar n = n - 1;
    tempvar t_ptr = t_ptr + 1;
    tempvar c_ptr = c_ptr + 1;
    tempvar x_coord_ptr = x_coord_ptr + 1;
    tempvar result_ptr = result_ptr + 1;
    jmp loop if n != 0;

    return result;
}
