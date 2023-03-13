%lang starknet

from utils.math_goldilocks import sub_g, PG, add_g, mul_g, inv_g, pow_g

@external
func test_sub_g{range_check_ptr}() {
    let res = sub_g(2, 1);
    assert res = 1;

    let reverse = sub_g(1, 2);
    assert reverse = PG - 1;

    return ();
}

@external
func test_add_g{range_check_ptr}() {
    let res = add_g(2, 1);
    assert res = 3;

    let reverse = add_g(PG - 1, 2);
    assert reverse = 1;

    return ();
}

@external
func test_mul_g{range_check_ptr}() {
    let res = mul_g(2, 5);
    assert res = 10;

    let t = PG - 1;
    let overflow = mul_g(t, 2);
    assert overflow = PG - 2;

    let overflow2 = mul_g(t, 4);
    assert overflow2 = PG - 4;

    return ();
}

@external
func test_inv_g{range_check_ptr}() {
    let l1 = 25;
    let l2 = inv_g(l1);

    assert mul_g(l1, l2) = 1;

    let l3 = 55;
    let l4 = inv_g(l3);

    assert mul_g(l3, l4) = 1;

    return ();
}

@external
func test_pow_g{range_check_ptr}() {
    let b1 = 5;
    let e1 = 3;

    let r1 = pow_g(b1, e1);

    assert r1 = 125;

    let b2 = PG - 5;
    let e2 = 2;

    let r2 = pow_g(b2, e2);
    let expected = mul_g(b2, b2);

    assert r2 = expected;

    return ();
}
