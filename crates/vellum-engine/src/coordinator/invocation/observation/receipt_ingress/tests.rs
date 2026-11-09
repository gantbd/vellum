use super::*;
#[test]
fn receipt_index_must_be_exact_safe_and_in_the_original_native_role() {
    for n in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        0.5,
        9_007_199_254_740_992.0,
    ] {
        assert!(!index_matches(V::Data(D::Number(n)), 0));
    }
    assert!(index_matches(V::Data(D::Number(0.0)), 0));
    assert!(index_matches(V::Data(D::Number(7.0)), 7));
    assert!(!index_matches(V::Data(D::Number(7.0)), 8));
    assert!(!index_matches(V::Data(D::Text("0")), 0));
    assert!(!index_matches(V::Data(D::Boolean(false)), 0));
    if let Ok(maximum) = usize::try_from(9_007_199_254_740_991u64) {
        assert!(index_matches(
            V::Data(D::Number(9_007_199_254_740_991.0)),
            maximum
        ));
        assert!(!index_matches(
            V::Data(D::Number(9_007_199_254_740_992.0)),
            maximum + 1
        ));
    }
}
