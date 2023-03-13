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

func pow_g_loop{range_check_ptr}(base, exp, res) -> felt {
    if (exp == 0) {
        return res;
    }

    let base_square = mul_g(base, base);

    let bit = [range_check_ptr];
    let range_check_ptr = range_check_ptr + 1;

    %{ ids.bit = (ids.exp % ids.PG) & 1 %}
    if (bit == 1) {
        // odd case
        let tmp = exp - 1;
        let new_exp = tmp / 2;
        let r = mul_g(base, res);
        return pow_g_loop(base_square, new_exp, r);
    } else {
        // even case
        let new_exp = exp / 2;
        return pow_g_loop(base_square, new_exp, res);
    }
}

// Returns base ** exp % PG, for 0 <= exp < 2**63.
func pow_g{range_check_ptr}(base, exp) -> felt {
    if (exp == 0) {
        return 1;
    }

    if (base == 0) {
        return 0;
    }

    return pow_g_loop(base, exp, 1);
}
