from starkware.cairo.common.registers import get_ap, get_fp_and_pc

// 2^64 - 2^32 - 1;
const PG = 18446744069414584321;

// multiply to felts modulo PG, these numbers must be smaller than PG
func mul_goldilocks{range_check_ptr}(a: felt, b: felt) -> felt {
    // add range checks for a, b
    let res = a * b;

    let r = [range_check_ptr];
    let q = [range_check_ptr + 1];
    let range_check_ptr = range_check_ptr + 2;

    %{
        ids.r = ids.res % ids.PG
        ids.q = ids.res // ids.PG
    %}
    assert q * PG + r = res;
    return r;
}

// // Returns base ** exp % PG, for 0 <= exp < 2**251.
// func pow_goldilocks{range_check_ptr}(base, exp) -> (res: felt) {
//     struct LoopLocals {
//         bit: felt,
//         temp0: felt,

// res: felt,
//         base: felt,
//         exp: felt,
//     }

// if (exp == 0) {
//         return (res=1);
//     }

// let initial_locs: LoopLocals* = cast(fp - 2, LoopLocals*);
//     initial_locs.res = 1, ap++;
//     initial_locs.base = base, ap++;
//     initial_locs.exp = exp, ap++;

// loop:
//     let prev_locs: LoopLocals* = cast(ap - LoopLocals.SIZE, LoopLocals*);
//     let locs: LoopLocals* = cast(ap, LoopLocals*);
//     locs.base = mul_goldilocks(prev_locs.base, prev_locs.base), ap++;
//     %{ ids.locs.bit = (ids.prev_locs.exp % PG) & 1 %}
//     jmp odd if locs.bit != 0, ap++;

// even:
//     locs.exp = prev_locs.exp / 2, ap++;
//     locs.res = prev_locs.res, ap++;
//     // exp cannot be 0 here.
//     static_assert ap + 1 == locs + LoopLocals.SIZE;
//     jmp loop, ap++;

// odd:
//     locs.temp0 = prev_locs.exp - 1;
//     locs.exp = locs.temp0 / 2, ap++;
//     locs.res = mul_goldilocks(prev_locs.base, prev_locs.base), ap++;
//     static_assert ap + 1 == locs + LoopLocals.SIZE;
//     jmp loop if locs.exp != 0, ap++;

// // Cap the number of steps.
//     let (__ap__) = get_ap();
//     let (__fp__, _) = get_fp_and_pc();
//     let n_steps = (__ap__ - cast(initial_locs, felt*)) / LoopLocals.SIZE - 1;
//     return (res=locs.res);
// }
