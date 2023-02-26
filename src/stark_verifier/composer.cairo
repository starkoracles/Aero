from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.pow import pow

from stark_verifier.air.air_instance import AirInstance, DeepCompositionCoefficients
from stark_verifier.air.transitions.frame import EvaluationFrame
from stark_verifier.channel import Table
from stark_verifier.utils import Vec

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
    let row = queried_main_trace_states.elements;
    let (local result: felt*) = alloc();
    // TODO: Don't hardcode the number of query and columns
    tempvar n = 54;
    tempvar row_ptr = row;
    tempvar x_coord_ptr = composer.x_coordinates;
    tempvar result_ptr = result;

    loop_main:
    tempvar sum_curr = 0;
    tempvar sum_next = 0;

    tempvar sum_curr = sum_curr + ([row_ptr + 0] - ood_main_frame.current[0]) * composer.cc.trace[0
        ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 0] - ood_main_frame.next[0]) * composer.cc.trace[0
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 1] - ood_main_frame.current[1]) * composer.cc.trace[1
        ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 1] - ood_main_frame.next[1]) * composer.cc.trace[1
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 2] - ood_main_frame.current[2]) * composer.cc.trace[2
        ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 2] - ood_main_frame.next[2]) * composer.cc.trace[2
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 3] - ood_main_frame.current[3]) * composer.cc.trace[3
        ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 3] - ood_main_frame.next[3]) * composer.cc.trace[3
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 4] - ood_main_frame.current[4]) * composer.cc.trace[4
        ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 4] - ood_main_frame.next[4]) * composer.cc.trace[4
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 5] - ood_main_frame.current[5]) * composer.cc.trace[5
        ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 5] - ood_main_frame.next[5]) * composer.cc.trace[5
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 6] - ood_main_frame.current[6]) * composer.cc.trace[6
        ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 6] - ood_main_frame.next[6]) * composer.cc.trace[6
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 7] - ood_main_frame.current[7]) * composer.cc.trace[7
        ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 7] - ood_main_frame.next[7]) * composer.cc.trace[7
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 8] - ood_main_frame.current[8]) * composer.cc.trace[8
        ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 8] - ood_main_frame.next[8]) * composer.cc.trace[8
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 9] - ood_main_frame.current[9]) * composer.cc.trace[9
        ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 9] - ood_main_frame.next[9]) * composer.cc.trace[9
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 10] - ood_main_frame.current[10]) * composer.cc.trace[
        10
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 10] - ood_main_frame.next[10]) * composer.cc.trace[10
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 11] - ood_main_frame.current[11]) * composer.cc.trace[
        11
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 11] - ood_main_frame.next[11]) * composer.cc.trace[11
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 12] - ood_main_frame.current[12]) * composer.cc.trace[
        12
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 12] - ood_main_frame.next[12]) * composer.cc.trace[12
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 13] - ood_main_frame.current[13]) * composer.cc.trace[
        13
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 13] - ood_main_frame.next[13]) * composer.cc.trace[13
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 14] - ood_main_frame.current[14]) * composer.cc.trace[
        14
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 14] - ood_main_frame.next[14]) * composer.cc.trace[14
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 15] - ood_main_frame.current[15]) * composer.cc.trace[
        15
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 15] - ood_main_frame.next[15]) * composer.cc.trace[15
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 16] - ood_main_frame.current[16]) * composer.cc.trace[
        16
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 16] - ood_main_frame.next[16]) * composer.cc.trace[16
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 17] - ood_main_frame.current[17]) * composer.cc.trace[
        17
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 17] - ood_main_frame.next[17]) * composer.cc.trace[17
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 18] - ood_main_frame.current[18]) * composer.cc.trace[
        18
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 18] - ood_main_frame.next[18]) * composer.cc.trace[18
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 19] - ood_main_frame.current[19]) * composer.cc.trace[
        19
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 19] - ood_main_frame.next[19]) * composer.cc.trace[19
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 20] - ood_main_frame.current[20]) * composer.cc.trace[
        20
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 20] - ood_main_frame.next[20]) * composer.cc.trace[20
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 21] - ood_main_frame.current[21]) * composer.cc.trace[
        21
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 21] - ood_main_frame.next[21]) * composer.cc.trace[21
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 22] - ood_main_frame.current[22]) * composer.cc.trace[
        22
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 22] - ood_main_frame.next[22]) * composer.cc.trace[22
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 23] - ood_main_frame.current[23]) * composer.cc.trace[
        23
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 23] - ood_main_frame.next[23]) * composer.cc.trace[23
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 24] - ood_main_frame.current[24]) * composer.cc.trace[
        24
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 24] - ood_main_frame.next[24]) * composer.cc.trace[24
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 25] - ood_main_frame.current[25]) * composer.cc.trace[
        25
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 25] - ood_main_frame.next[25]) * composer.cc.trace[25
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 26] - ood_main_frame.current[26]) * composer.cc.trace[
        26
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 26] - ood_main_frame.next[26]) * composer.cc.trace[26
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 27] - ood_main_frame.current[27]) * composer.cc.trace[
        27
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 27] - ood_main_frame.next[27]) * composer.cc.trace[27
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 28] - ood_main_frame.current[28]) * composer.cc.trace[
        28
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 28] - ood_main_frame.next[28]) * composer.cc.trace[28
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 29] - ood_main_frame.current[29]) * composer.cc.trace[
        29
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 29] - ood_main_frame.next[29]) * composer.cc.trace[29
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 30] - ood_main_frame.current[30]) * composer.cc.trace[
        30
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 30] - ood_main_frame.next[30]) * composer.cc.trace[30
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 31] - ood_main_frame.current[31]) * composer.cc.trace[
        31
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 31] - ood_main_frame.next[31]) * composer.cc.trace[31
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 32] - ood_main_frame.current[32]) * composer.cc.trace[
        32
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 32] - ood_main_frame.next[32]) * composer.cc.trace[32
        ].values[1];

    tempvar x = [x_coord_ptr];
    tempvar sum = sum_curr / (x - z_curr) + sum_next / (x - z_next);
    assert [result_ptr] = sum;

    tempvar n = n - 1;
    tempvar row_ptr = row_ptr + n_cols;
    tempvar x_coord_ptr = x_coord_ptr + 1;
    tempvar result_ptr = result_ptr + 1;
    jmp loop_main if n != 0;

    // Aux trace coefficient rows
    let n_cols = queried_aux_trace_states.n_cols;

    // Compose columns of the aux segments
    let row = queried_aux_trace_states.elements;
    tempvar n = 54;  // TODO: double-check this value!
    tempvar row_ptr = row;
    tempvar x_coord_ptr = composer.x_coordinates;
    tempvar result_ptr = result_ptr;

    loop_aux:
    tempvar sum_curr = 0;
    tempvar sum_next = 0;

    tempvar sum_curr = sum_curr + ([row_ptr + 33] - ood_aux_frame.current[33]) * composer.cc.trace[
        33
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 33] - ood_aux_frame.next[33]) * composer.cc.trace[33
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 34] - ood_aux_frame.current[34]) * composer.cc.trace[
        34
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 34] - ood_aux_frame.next[34]) * composer.cc.trace[34
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 35] - ood_aux_frame.current[35]) * composer.cc.trace[
        35
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 35] - ood_aux_frame.next[35]) * composer.cc.trace[35
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 36] - ood_aux_frame.current[36]) * composer.cc.trace[
        36
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 36] - ood_aux_frame.next[36]) * composer.cc.trace[36
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 37] - ood_aux_frame.current[37]) * composer.cc.trace[
        37
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 37] - ood_aux_frame.next[37]) * composer.cc.trace[37
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 38] - ood_aux_frame.current[38]) * composer.cc.trace[
        38
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 38] - ood_aux_frame.next[38]) * composer.cc.trace[38
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 39] - ood_aux_frame.current[39]) * composer.cc.trace[
        39
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 39] - ood_aux_frame.next[39]) * composer.cc.trace[39
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 40] - ood_aux_frame.current[40]) * composer.cc.trace[
        40
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 40] - ood_aux_frame.next[40]) * composer.cc.trace[40
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 41] - ood_aux_frame.current[41]) * composer.cc.trace[
        41
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 41] - ood_aux_frame.next[41]) * composer.cc.trace[41
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 42] - ood_aux_frame.current[42]) * composer.cc.trace[
        42
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 42] - ood_aux_frame.next[42]) * composer.cc.trace[42
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 43] - ood_aux_frame.current[43]) * composer.cc.trace[
        43
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 43] - ood_aux_frame.next[43]) * composer.cc.trace[43
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 44] - ood_aux_frame.current[44]) * composer.cc.trace[
        44
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 44] - ood_aux_frame.next[44]) * composer.cc.trace[44
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 45] - ood_aux_frame.current[45]) * composer.cc.trace[
        45
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 45] - ood_aux_frame.next[45]) * composer.cc.trace[45
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 46] - ood_aux_frame.current[46]) * composer.cc.trace[
        46
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 46] - ood_aux_frame.next[46]) * composer.cc.trace[46
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 47] - ood_aux_frame.current[47]) * composer.cc.trace[
        47
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 47] - ood_aux_frame.next[47]) * composer.cc.trace[47
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 48] - ood_aux_frame.current[48]) * composer.cc.trace[
        48
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 48] - ood_aux_frame.next[48]) * composer.cc.trace[48
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 49] - ood_aux_frame.current[49]) * composer.cc.trace[
        49
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 49] - ood_aux_frame.next[49]) * composer.cc.trace[49
        ].values[1];

    tempvar sum_curr = sum_curr + ([row_ptr + 50] - ood_aux_frame.current[50]) * composer.cc.trace[
        50
    ].values[0];
    tempvar sum_next = sum_next + ([row_ptr + 50] - ood_aux_frame.next[50]) * composer.cc.trace[50
        ].values[1];

    tempvar x = [x_coord_ptr];
    tempvar sum = sum_curr / (x - z_curr) + sum_next / (x - z_next);
    assert [result_ptr] = sum;

    tempvar n = n - 1;
    tempvar row_ptr = row_ptr + n_cols;
    tempvar x_coord_ptr = x_coord_ptr + 1;
    tempvar result_ptr = result_ptr + 1;
    jmp loop_aux if n != 0;

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
