%lang starknet

from utils.math_goldilocks import sub_g, PG

// / Test deserialization of StarkProof from file
@external
func test_sub_goldilocks{range_check_ptr}() {
    let res = sub_g(2, 1);
    assert res = 1;

    let reverse = sub_g(1, 2);
    assert reverse = PG - 1;

    return ();
}
