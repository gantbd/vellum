use super::*;
use crate::coordinator::scoped_definition::tests as fixture;
fn cap() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 65536,
        max_nodes: 128,
        max_depth: 16,
    }
}
fn parse<'h>(
    id: &'h InvocationId,
    reads: &'h ReadDependencyRequestV3,
    expected: D<'_>,
    maximum: usize,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedNativeDefinitionArgumentsV1<'h>> {
    use NativeDefinitionEnvelopeValueV1 as V;
    let d = [
        ("schemas", D::Array(&[])),
        ("restrictions", D::Object(&[])),
        ("targets", D::Array(&[])),
    ];
    let rows = [
        ("contract", V::Data(D::Text("engine-settings-definition/1"))),
        ("operation", V::Data(D::Text("supported-registry"))),
        ("invocation", V::Invocation(id)),
        ("profile", V::Data(D::Text("v1"))),
        ("expected", V::Data(expected)),
        ("definition", V::Data(D::Object(&d))),
        ("dependencies", V::Dependencies(reads)),
    ];
    normalize_native_definition_envelope_v1(
        &V::Object(&rows),
        fixture::activation(),
        fixture::limits(),
        limits,
        128,
        maximum,
        r,
    )
    .inspect(|q| {
        assert!(std::ptr::eq(
            q.argument_declaration_v1(),
            &super::super::declaration::DEFINITION
        ));
    })
}
#[test]
fn all_expected_epochs_are_positive_safe_noncoercing_and_exact() {
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = fixture::reads();
    for field in 0..3 {
        for n in [
            0.0,
            -0.0,
            -1.0,
            0.5,
            1.0,
            9_007_199_254_740_991.0,
            9_007_199_254_740_992.0,
            f64::NAN,
        ] {
            let mut expected = [
                ("schemaGeneration", D::Number(1.0)),
                ("restrictionGeneration", D::Number(1.0)),
                ("associationGeneration", D::Number(1.0)),
            ];
            expected[field].1 = D::Number(n);
            assert_eq!(
                parse(&id, &reads, D::Object(&expected), 65536, cap(), &mut r).is_ok(),
                n.is_finite() && n >= 1.0 && n.fract() == 0.0 && n <= 9_007_199_254_740_991.0
            );
        }
    }
    for case in 0..5 {
        let mut e = vec![
            ("schemaGeneration", D::Number(1.0)),
            ("restrictionGeneration", D::Number(1.0)),
            ("associationGeneration", D::Number(1.0)),
        ];
        match case {
            0 => {
                e.pop();
            }
            1 => e.push(("basis", D::Null)),
            2 => e[2] = e[0],
            3 => e[0].1 = D::Text("1"),
            _ => e[0].1 = D::Boolean(true),
        }
        assert!(parse(&id, &reads, D::Object(&e), 65536, cap(), &mut r).is_err());
    }
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn native_outer_roles_and_joint_workspace_bounds_include_the_expected_product() {
    use NativeDefinitionEnvelopeValueV1 as V;
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = fixture::reads();
    let e = [
        ("schemaGeneration", D::Number(1.0)),
        ("restrictionGeneration", D::Number(1.0)),
        ("associationGeneration", D::Number(1.0)),
    ];
    let q = parse(&id, &reads, D::Object(&e), 65536, cap(), &mut r).unwrap();
    let peak = q.normalization_peak_bytes();
    assert!(std::ptr::eq(q.invocation(), &id));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    assert_eq!(
        parse(&id, &reads, D::Object(&e), peak - 1, cap(), &mut r).unwrap_err(),
        InvocationError::Limit
    );
    assert!(parse(&id, &reads, D::Object(&e), peak, cap(), &mut r).is_ok());
    for limits in [
        CaptureLimitsV1 {
            max_bytes: q.capture_usage().peak_bytes - 1,
            ..cap()
        },
        CaptureLimitsV1 {
            max_nodes: q.capture_usage().nodes - 1,
            ..cap()
        },
        CaptureLimitsV1 {
            max_depth: q.capture_usage().depth - 1,
            ..cap()
        },
    ] {
        assert_eq!(
            parse(&id, &reads, D::Object(&e), 65536, limits, &mut r).unwrap_err(),
            InvocationError::Limit
        );
    }
    let d = [
        ("schemas", D::Array(&[])),
        ("restrictions", D::Object(&[])),
        ("targets", D::Array(&[])),
    ];
    for case in 0..8 {
        let mut rows = vec![
            ("contract", V::Data(D::Text("engine-settings-definition/1"))),
            ("operation", V::Data(D::Text("supported-registry"))),
            ("invocation", V::Invocation(&id)),
            ("profile", V::Data(D::Text("v1"))),
            ("expected", V::Data(D::Object(&e))),
            ("definition", V::Data(D::Object(&d))),
            ("dependencies", V::Dependencies(&reads)),
        ];
        match case {
            0 => {
                rows.pop();
            }
            1 => rows.push(("keymaps", V::Data(D::Array(&[])))),
            2 => rows[0].1 = V::Data(D::Text("scoped-engine-settings-definition/1")),
            3 => rows[1].1 = V::Data(D::Text("semantic-settings")),
            4 => rows[2].1 = V::Data(D::Text("job")),
            5 => rows[6].1 = V::Data(D::Array(&[])),
            6 => rows[3].1 = V::Data(D::Text("")),
            _ => rows[5] = rows[4],
        }
        assert!(
            normalize_native_definition_envelope_v1(
                &V::Object(&rows),
                fixture::activation(),
                fixture::limits(),
                cap(),
                128,
                65536,
                &mut r
            )
            .is_err()
        );
    }
}
