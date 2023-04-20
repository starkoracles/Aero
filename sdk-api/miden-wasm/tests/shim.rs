use miden_air::{Felt, FieldElement};
use wasm_bindgen_test::wasm_bindgen_test;
use winter_crypto::{ByteDigest, Digest};

#[wasm_bindgen_test]
fn byte_digest_into_js_value() {
    let d = ByteDigest::new([255_u8; 32]);
    let js_value = d.into_js_value();
    assert_eq!(
        js_value.as_string().unwrap(),
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    );

    let deser = ByteDigest::from_js_value(js_value);
    assert_eq!(d, deser);
}

#[wasm_bindgen_test]
fn element_into_js_value() {
    let e = Felt::from(42u64);
    let js_value = e.into_js_value();
    assert_eq!(js_value.as_string().unwrap(), "2a00000000000000"); // 42 in hex;

    let deser = Felt::from_js_value(js_value);
    assert_eq!(e, deser);
}
