use super::*;
#[test]
fn native_lifecycle_output_floor_uses_checked_logical_accounting_without_a_sponsorship_claim() {
    let mut l = NativeDocumentLifecycleArgumentLimitsV1 {
        max_owned_bytes: usize::MAX,
        max_source_bytes: usize::MAX,
        max_selections: usize::MAX,
    };
    assert_eq!(owned_bytes(usize::MAX, l), Err(InvocationError::Limit));
    let header = NormalizedDocumentLifecycleArgumentsV1::required_header_bytes();
    l.max_owned_bytes = header;
    assert_eq!(owned_bytes(0, l), Ok(header));
    assert_eq!(owned_bytes(1, l), Err(InvocationError::Limit));
    assert_eq!(
        FORMS
            .iter()
            .map(|f| f.variant.fields.len())
            .collect::<Vec<_>>(),
        vec![5, 9, 6, 7, 10]
    );
}
