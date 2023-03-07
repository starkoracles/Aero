%lang starknet

from utils.math_goldilocks import sub_g, PG, add_g, mul_g, inv_g

@external
func test_sub_goldilocks{range_check_ptr}() {
    let res = sub_g(2, 1);
    assert res = 1;

    let reverse = sub_g(1, 2);
    assert reverse = PG - 1;

    return ();
}

@external
func test_add_goldilocks{range_check_ptr}() {
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
