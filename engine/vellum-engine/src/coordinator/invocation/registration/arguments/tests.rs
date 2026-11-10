use super::*;
use crate::capture::InertValueV1 as D;
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 65536,
        max_nodes: 256,
        max_depth: 16,
    }
}
fn normalize_request<'h>(
    id: &'h InvocationId,
    request: D<'_>,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedResourceRegistrationArgumentsV1<'h>> {
    use ResourceRegistrationEnvelopeValueV1 as V;
    let rows = [
        ("contract", V::Data(D::Text("registration-invocation/1"))),
        ("operation", V::Data(D::Text("activate-context"))),
        ("invocation", V::Invocation(id)),
        (
            "profile",
            V::Data(D::Text(NATIVE_RESOURCE_REGISTRATION_PROFILE_V1)),
        ),
        ("request", V::Data(request)),
    ];
    normalize_resource_registration_envelope_v1(
        &V::Object(&rows),
        super::super::tests::profile(),
        limits(),
        maximum,
        r,
    )
    .inspect(|q| {
        assert!(std::ptr::eq(
            q.argument_declaration_v1(),
            NormalizedResourceRegistrationArgumentsV1::ARGUMENT_DECLARATION
        ))
    })
}
fn normalize_changes<'h>(
    id: &'h InvocationId,
    changes: &[D<'_>],
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedResourceRegistrationArgumentsV1<'h>> {
    normalize_request(id, D::Object(&[("changes", D::Array(changes))]), maximum, r)
}
#[test]
fn closed_outer_and_request_shapes_cannot_be_coerced_or_shadowed() {
    use ResourceRegistrationEnvelopeValueV1 as V;
    let (mut o, mut r) = super::super::tests::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let req = [("changes", D::Array(&[]))];
    let before = o.invocation_usage();
    for case in 0..9 {
        let mut rows = vec![
            ("contract", V::Data(D::Text("registration-invocation/1"))),
            ("operation", V::Data(D::Text("activate-context"))),
            ("invocation", V::Invocation(&id)),
            (
                "profile",
                V::Data(D::Text(NATIVE_RESOURCE_REGISTRATION_PROFILE_V1)),
            ),
            ("request", V::Data(D::Object(&req))),
        ];
        match case {
            0 => {
                rows.pop();
            }
            1 => rows.push(("extra", V::Data(D::Null))),
            2 => rows[4] = rows[0],
            3 => rows[0].1 = V::Data(D::Text("scoped-registration-invocation/1")),
            4 => rows[1].1 = V::Data(D::Text("run")),
            5 => rows[2].1 = V::Data(D::Text("id")),
            6 => rows[2].1 = V::Data(D::Null),
            7 => rows[3].1 = V::Data(D::Text("native-resource-registration/2")),
            _ => rows[4].1 = V::Invocation(&id),
        }
        assert!(matches!(
            normalize_resource_registration_envelope_v1(
                &V::Object(&rows),
                super::super::tests::profile(),
                limits(),
                131072,
                &mut r
            ),
            Err(InvocationError::MalformedIntent)
        ));
    }
    for request in [
        D::Array(&[]),
        D::Object(&[("changes", D::Null)]),
        D::Object(&[("changes", D::Array(&[])), ("extra", D::Null)]),
    ] {
        assert!(matches!(
            normalize_request(&id, request, 131072, &mut r),
            Err(InvocationError::MalformedIntent)
        ));
    }
    assert_eq!(o.invocation_usage(), before);
}
#[test]
fn identity_expected_zero_table_and_duplicate_rules_are_exact_and_naturals_are_safe() {
    let (mut o, mut r) = super::super::tests::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    for number in [
        -1.0,
        0.5,
        9_007_199_254_740_992.0,
        f64::NAN,
        f64::INFINITY,
        -0.0,
        0.0,
        9_007_199_254_740_991.0,
    ] {
        let identity = [
            ("authority", D::Text("a")),
            ("id", D::Text("id")),
            ("incarnation", D::Number(number)),
        ];
        let e = [
            ("state", D::Text("present")),
            ("generation", D::Number(number)),
        ];
        let fields = [
            ("table", D::Text("read")),
            ("action", D::Text("remove")),
            ("provider", D::Object(&identity)),
            ("resource", D::Object(&identity)),
            ("expected", D::Object(&e)),
        ];
        let result = normalize_changes(&id, &[D::Object(&fields)], 131072, &mut r);
        if number.is_finite()
            && number >= 0.0
            && number.fract() == 0.0
            && number <= 9_007_199_254_740_991.0
        {
            let q = result.unwrap();
            assert_eq!(
                q.changes()[0].expected,
                CatalogExpectationV1::Present {
                    generation: number as u64
                }
            );
            assert!(
                normalize_changes(
                    &id,
                    &[D::Object(&fields), D::Object(&fields)],
                    131072,
                    &mut r
                )
                .is_err()
            );
        } else {
            assert!(result.is_err());
        }
    }
    let identity = [
        ("authority", D::Text("a")),
        ("id", D::Text("id")),
        ("incarnation", D::Number(0.0)),
    ];
    for table in ["read", "write", "Read", "unknown"] {
        for expectation in [
            D::Object(&[("state", D::Text("absent"))]),
            D::Object(&[("state", D::Text("absent")), ("generation", D::Null)]),
            D::Object(&[("state", D::Text("present"))]),
            D::Object(&[("state", D::Text("present")), ("generation", D::Text("0"))]),
        ] {
            let fields = [
                ("table", D::Text(table)),
                ("action", D::Text("remove")),
                ("provider", D::Object(&identity)),
                ("resource", D::Object(&identity)),
                ("expected", expectation),
            ];
            let result = normalize_changes(&id, &[D::Object(&fields)], 131072, &mut r);
            let valid_expectation = matches!(expectation, D::Object(v) if v.len()==1 && matches!(v[0].1,D::Text("absent")));
            assert_eq!(
                result.is_ok(),
                ["read", "write"].contains(&table) && valid_expectation
            );
        }
    }
}
#[test]
fn joint_and_inert_bounds_are_real_and_normalized_values_retain_no_admission_authority() {
    let (mut o, mut r) = super::super::tests::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let identity = [
        ("authority", D::Text("a")),
        ("id", D::Text("id")),
        ("incarnation", D::Number(0.0)),
    ];
    let expected = [("state", D::Text("absent"))];
    let fields = [
        ("table", D::Text("read")),
        ("action", D::Text("remove")),
        ("provider", D::Object(&identity)),
        ("resource", D::Object(&identity)),
        ("expected", D::Object(&expected)),
    ];
    let entries = [D::Object(&fields)];
    let q = normalize_changes(&id, &entries, 131072, &mut r).unwrap();
    assert!(q.owned_bytes() > size_of::<NormalizedResourceRegistrationArgumentsV1<'_>>());
    let peak = q.normalization_peak_bytes();
    assert!(peak >= q.owned_bytes());
    assert!(matches!(
        normalize_changes(&id, &entries, peak - 1, &mut r),
        Err(InvocationError::Limit)
    ));
    assert_eq!(
        normalize_changes(&id, &entries, peak, &mut r)
            .unwrap()
            .changes(),
        q.changes()
    );
    assert!(std::ptr::eq(q.invocation(), &id));
    let mut p = super::super::tests::profile();
    p.max_relations = 0;
    use ResourceRegistrationEnvelopeValueV1 as V;
    let request = [("changes", D::Array(&entries))];
    let outer = [
        ("contract", V::Data(D::Text("registration-invocation/1"))),
        ("operation", V::Data(D::Text("activate-context"))),
        ("invocation", V::Invocation(&id)),
        (
            "profile",
            V::Data(D::Text(NATIVE_RESOURCE_REGISTRATION_PROFILE_V1)),
        ),
        ("request", V::Data(D::Object(&request))),
    ];
    assert!(matches!(
        normalize_resource_registration_envelope_v1(
            &V::Object(&outer),
            p,
            limits(),
            131072,
            &mut r
        ),
        Err(InvocationError::Limit)
    ));
    for l in [
        CaptureLimitsV1 {
            max_bytes: 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_nodes: 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_depth: 0,
            ..limits()
        },
    ] {
        assert!(matches!(
            normalize_resource_registration_envelope_v1(
                &V::Object(&outer),
                super::super::tests::profile(),
                l,
                131072,
                &mut r
            ),
            Err(InvocationError::Limit)
        ));
    }
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
