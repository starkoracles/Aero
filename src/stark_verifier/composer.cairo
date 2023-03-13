from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.pow import pow

from stark_verifier.air.air_instance import AirInstance, DeepCompositionCoefficients
from stark_verifier.air.transitions.frame import EvaluationFrame
from stark_verifier.channel import Table
from stark_verifier.utils import Vec
from utils.math_goldilocks import mul_g, sub_g, add_g, div_g

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
    n: felt,
    composer: DeepComposer,
    queried_trace_states: Table,
    ood_frame: EvaluationFrame,
    cc_offset: felt,
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
        row_ptr + (offset * n_cols), 0, ood_frame, composer, 0, 0, 72, cc_offset
    );

    tempvar x = [x_coord_ptr];
    let x_z_curr = sub_g(x, z_curr);
    let x_z_next = sub_g(x, z_next);
    let s_curr = div_g(sum_curr, x_z_curr);
    let s_next = div_g(sum_next, x_z_next);
    tempvar sum = add_g(s_curr, s_next);
    %{ print("sum:", ids.n, ids.sum) %}
    assert [result_ptr] = sum;

    return compose_loop(
        result_ptr + 1, n - 1, composer, queried_trace_states, ood_frame, cc_offset
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
    let (local result: felt*) = alloc();
    // TODO HARDCODE: Don't hardcode the number of query and columns
    tempvar n = 27;
    tempvar result_ptr = result;

    compose_loop(result_ptr, n, composer, queried_main_trace_states, ood_main_frame, 0);

    // // Aux trace coefficient rows
    // let n_cols = queried_aux_trace_states.n_cols;

    // // Compose columns of the aux segments
    // let row = queried_aux_trace_states.elements;
    // tempvar n = 27;  // TODO HARDCODE: double-check this value!
    // tempvar row_ptr = row;
    // tempvar x_coord_ptr = composer.x_coordinates;
    // tempvar result_ptr = result_ptr;

    // loop_aux:
    // tempvar sum_curr = 0;
    // tempvar sum_next = 0;

    // let cc_offset = queried_main_trace_states.n_cols;
    // tempvar i = 0;

    // inner_aux:
    // let row_cell = [row_ptr + i];
    // let aux_frame_curr = ood_aux_frame.current[i];
    // let curr = sub_g(row_cell, aux_frame_curr);
    // tempvar mul_curr = mul_g(curr, composer.cc.trace[i + cc_offset].values[0]);
    // tempvar sum_curr = add_g(sum_curr, mul_curr);

    // tempvar aux_frame_next = ood_aux_frame.next[i];
    // let next = sub_g(row_cell, aux_frame_next);
    // tempvar mul_next = mul_g(next, composer.cc.trace[i + cc_offset].values[1]);
    // tempvar sum_next = add_g(sum_next, mul_next);

    // tempvar i = i + 1;
    // tempvar iter_left = 27 - i;
    // jmp inner_main if iter_left != 0;

    // tempvar x = [x_coord_ptr];
    // let x_z_curr = mul_g(x, z_curr);
    // let x_z_next = mul_g(x, z_next);
    // let s_curr = div_g(sum_curr, x_z_curr);
    // let s_next = div_g(sum_next, x_z_next);
    // tempvar sum = add_g(s_curr, s_next);
    // assert [result_ptr] = sum;

    // tempvar n = n - 1;
    // tempvar row_ptr = row_ptr + n_cols;
    // tempvar x_coord_ptr = x_coord_ptr + 1;
    // tempvar result_ptr = result_ptr + 1;
    // jmp loop_aux if n != 0;

    return result;
}

func compose_constraint_evaluations{range_check_ptr}(
    composer: DeepComposer, queried_evaluations: Table, ood_evaluations: Vec
) -> felt* {
    alloc_locals;

    // Compute z^m
    let num_eval_columns = ood_evaluations.n_elements;
    let z = composer.z_curr;
    let (local z_m) = pow(z, num_eval_columns);
    local range_check_ptr = range_check_ptr;

    local n_cols = queried_evaluations.n_cols;
    local cc_constraint: felt* = composer.cc.constraints;

    local row: felt* = queried_evaluations.elements;
    let (local result: felt*) = alloc();
    // TODO: Don't hardcode number of queries
    tempvar n = 54;
    tempvar row_ptr = row;
    tempvar x_coord_ptr = composer.x_coordinates;
    tempvar result_ptr = result;

    loop:
    tempvar sum = 0;

    tempvar sum = sum + (row_ptr[0] - ood_evaluations.elements[0]) * cc_constraint[0];
    tempvar sum = sum + (row_ptr[1] - ood_evaluations.elements[1]) * cc_constraint[1];
    tempvar sum = sum + (row_ptr[2] - ood_evaluations.elements[2]) * cc_constraint[2];
    tempvar sum = sum + (row_ptr[3] - ood_evaluations.elements[3]) * cc_constraint[3];
    tempvar x = [x_coord_ptr];
    tempvar sum = sum / (x - z_m);
    assert [result_ptr] = sum;

    tempvar n = n - 1;
    tempvar row_ptr = row_ptr + n_cols;
    tempvar x_coord_ptr = x_coord_ptr + 1;
    tempvar result_ptr = result_ptr + 1;
    jmp loop if n != 0;

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
