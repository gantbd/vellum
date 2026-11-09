use super::*;
use crate::capture::InertValueV1 as D;
use NativeSettingsEnvelopeValueV1 as V;

#[test]
fn native_argument_headers_are_exact_order_independent_and_never_cross_contracts() {
    for declaration in [&VALUES, &DEFINITION, &LIFECYCLE] {
        let mut rows: Vec<_> = declaration
            .variant
            .fields
            .iter()
            .map(|(key, _)| (*key, V::Data(D::Null)))
            .collect();
        rows[0].1 = V::Data(D::Text(declaration.contract));
        rows[1].1 = V::Data(D::Text(declaration.variant.operation));
        assert_eq!(header(&rows, declaration), Ok(()));
        for i in 0..rows.len() {
            let mut missing = rows.clone();
            missing.remove(i);
            assert_eq!(
                header(&missing, declaration),
                Err(InvocationError::MalformedIntent)
            );
        }
        let mut extra = rows.clone();
        extra.push(("unexpected", V::Data(D::Null)));
        assert_eq!(
            header(&extra, declaration),
            Err(InvocationError::MalformedIntent)
        );
        // This guard makes no capability claim for the null role placeholders.
        rows.reverse();
        assert_eq!(header(&rows, declaration), Ok(()));
        rows.reverse();
        for other in [&VALUES, &DEFINITION, &LIFECYCLE] {
            rows[0].1 = V::Data(D::Text(other.contract));
            assert_eq!(header(&rows, declaration).is_ok(), other == declaration);
        }
        rows[0].1 = V::Data(D::Text(declaration.contract));
        for i in 0..rows.len() {
            let old = rows[i].0;
            rows[i].0 = "unexpected";
            assert_eq!(
                header(&rows, declaration),
                Err(InvocationError::MalformedIntent)
            );
            rows[i].0 = old;
        }
        rows[1].1 = V::Data(D::Text("unknown"));
        assert_eq!(
            header(&rows, declaration),
            Err(InvocationError::MalformedIntent)
        );
        rows[1].1 = V::Data(D::Text(declaration.variant.operation));
        rows[2].0 = "profile";
        assert_eq!(
            header(&rows, declaration),
            Err(InvocationError::MalformedIntent)
        );
    }
}
