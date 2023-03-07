from starkware.cairo.common.registers import get_ap, get_fp_and_pc

// 2^64 - 2^32 - 1;
const PG = 18446744069414584321;

// multiply to felts modulo PG, these numbers must be smaller than PG
func mul_g{range_check_ptr}(a: felt, b: felt) -> felt {
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

func add_g{range_check_ptr}(a: felt, b: felt) -> felt {
    let res = a + b;

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

func inv_g{range_check_ptr}(a: felt) -> felt {
    let inv = [range_check_ptr];
    let range_check_ptr = range_check_ptr + 1;

    %{
        def mul_g(a, b):
            return (a * b) % ids.PG

        def square_g(a):
            return (a ** 2) % ids.PG
            
        def exp_acc(base, tail, exp_bits):
            result = base
            for i in range(exp_bits):
                result = square_g(result)
            return mul_g(result, tail)
        # compute base^(M - 2) using 72 multiplications
        # M - 2 = 0b1111111111111111111111111111111011111111111111111111111111111111
        a = ids.a
        # compute base^11
        t2 = mul_g(square_g(a), a)

        # compute base^111
        t3 = mul_g(square_g(t2), a)

        # compute base^111111 (6 ones)
        t6 = exp_acc(t3, t3, 3)

        # compute base^111111111111 (12 ones)
        t12 = exp_acc(t6, t6, 6)

        # compute base^111111111111111111111111 (24 ones)
        t24 = exp_acc(t12, t12, 12)

        # compute base^1111111111111111111111111111111 (31 ones)
        t30 = exp_acc(t24, t6, 6)
        t31 = mul_g(square_g(t30), a)

        # compute base^111111111111111111111111111111101111111111111111111111111111111
        t63 = exp_acc(t31, t31, 32)

        # compute base^1111111111111111111111111111111011111111111111111111111111111111
        ids.inv = mul_g(square_g(t63), a)
    %}
    assert mul_g(inv, a) = 1;
    return inv;
}

func div_g{range_check_ptr}(a: felt, b: felt) -> felt {
    let inv = inv_g(b);
    return mul_g(a, inv);
}

func sub_g{range_check_ptr}(a: felt, b: felt) -> felt {
    let r = [range_check_ptr];
    let a_greater_than_b = [range_check_ptr + 1];
    let range_check_ptr = range_check_ptr + 2;

    %{
        if ids.a < ids.b:
            ids.r = ids.a + ids.PG - ids.b
            ids.a_greater_than_b = 0
        else:
            ids.r = ids.a - ids.b
            ids.a_greater_than_b = 1
    %}

    if (a_greater_than_b == 1) {
        assert r = a - b;
    } else {
        assert r + b = a + PG;
    }
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
