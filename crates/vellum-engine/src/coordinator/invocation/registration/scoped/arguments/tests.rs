use super::super::super::tests as native;
use super::super::tests as fixture;
use super::*;
use InertValueV1 as D;
use ScopedRegistrationEnvelopeValueV1 as V;
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 65536,
        max_nodes: 256,
        max_depth: 16,
    }
}
fn config() -> Vec<(&'static str, D<'static>)> {
    vec![
        ("identityDomain", D::Text("domain")),
        ("identityKey", D::Text("PRIVATE_KEY")),
        ("identityKnown", D::Boolean(false)),
        ("locator", D::Text("PRIVATE_LOCATOR")),
        ("profile", D::Text("strict-utf8-one-bom/1")),
        ("atomicRead", D::Boolean(false)),
        ("insertionLineBreak", D::Text("CRLF")),
    ]
}
fn normalize_request<'h>(
    id: &'h InvocationId,
    request: D<'_>,
    count: usize,
    l: CaptureLimitsV1,
    max: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedRegistrationArgumentsV1<'h>> {
    let outer = [
        (
            "contract",
            V::Data(D::Text("scoped-registration-invocation/1")),
        ),
        ("operation", V::Data(D::Text("activate-context"))),
        ("invocation", V::Invocation(id)),
        (
            "profile",
            V::Data(D::Text(SCOPED_RESOURCE_REGISTRATION_PROFILE_V1)),
        ),
        ("request", V::Data(request)),
    ];
    normalize_scoped_registration_envelope_v1(
        &V::Object(&outer),
        count,
        native::profile(),
        l,
        max,
        r,
    )
    .inspect(|q| {
        assert!(std::ptr::eq(
            q.argument_declaration_v1(),
            NormalizedScopedRegistrationArgumentsV1::ARGUMENT_DECLARATION
        ))
    })
}
fn normalize_changes<'h>(
    id: &'h InvocationId,
    changes: &[D<'_>],
    count: usize,
    l: CaptureLimitsV1,
    max: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedRegistrationArgumentsV1<'h>> {
    let request = [("changes", D::Array(changes))];
    normalize_request(id, D::Object(&request), count, l, max, r)
}
#[test]
fn exact_outer_fields_constants_and_original_invocation_role_are_mandatory() {
    let (mut o, mut r, _, _) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let changes = [];
    let request = [("changes", D::Array(&changes))];
    for case in 0..9 {
        let mut outer = vec![
            (
                "contract",
                V::Data(D::Text("scoped-registration-invocation/1")),
            ),
            ("operation", V::Data(D::Text("activate-context"))),
            ("invocation", V::Invocation(&id)),
            (
                "profile",
                V::Data(D::Text(SCOPED_RESOURCE_REGISTRATION_PROFILE_V1)),
            ),
            ("request", V::Data(D::Object(&request))),
        ];
        match case {
            0 => {
                outer.pop();
            }
            1 => {
                outer.push(("extra", V::Data(D::Null)));
            }
            2 => outer[4] = outer[0],
            3 => outer[0].1 = V::Data(D::Text("native-registration-invocation/1")),
            4 => outer[1].1 = V::Data(D::Text("run")),
            5 => outer[3].1 = V::Data(D::Text("scoped-resource-registration/2")),
            6 => outer[2].1 = V::Data(D::Null),
            7 => outer[2].1 = V::Data(D::Text("invocation-1")),
            _ => outer[4].1 = V::Invocation(&id),
        }
        assert!(
            matches!(
                normalize_scoped_registration_envelope_v1(
                    &V::Object(&outer),
                    0,
                    native::profile(),
                    limits(),
                    131072,
                    &mut r
                ),
                Err(InvocationError::MalformedIntent)
            ),
            "case {case}"
        );
    }
    assert_eq!(o.invocation_usage().pending, 0);
    assert!(!o.is_faulted());
}
#[test]
fn closed_request_and_action_schemas_reject_extra_null_and_native_identity_shadows() {
    let (mut o, mut r, _, _) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    for case in 0..10 {
        let mut change = vec![("basis", D::Number(0.0)), ("action", D::Text("remove"))];
        match case {
            0 => change.push(("configuration", D::Null)),
            1 => change.push(("basis", D::Number(0.0))),
            2 => change.push(("table", D::Text("read"))),
            3 => change.push(("expected", D::Text("absent"))),
            4 => change.push(("provider", D::Null)),
            5 => change.push(("resource", D::Null)),
            6 => change[1].1 = D::Text("patch"),
            7 => change[1].1 = D::Text("install"),
            8 => change[0].1 = D::Text("0"),
            _ => change[1].1 = D::Boolean(false),
        }
        assert!(matches!(
            normalize_changes(&id, &[D::Object(&change)], 1, limits(), 131072, &mut r),
            Err(InvocationError::MalformedIntent)
        ));
    }
    let request = [("changes", D::Array(&[])), ("scope", D::Null)];
    assert!(matches!(
        normalize_request(&id, D::Object(&request), 0, limits(), 131072, &mut r),
        Err(InvocationError::MalformedIntent)
    ));
    assert!(matches!(
        normalize_request(&id, D::Null, 0, limits(), 131072, &mut r),
        Err(InvocationError::MalformedIntent)
    ));
}
#[test]
fn basis_indices_are_noncoercing_safe_naturals_and_a_complete_unique_permutation() {
    let (mut o, mut r, _, _) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    for n in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        0.5,
        1.0,
        9_007_199_254_740_992.0,
    ] {
        let v = [("basis", D::Number(n)), ("action", D::Text("remove"))];
        assert!(matches!(
            normalize_changes(&id, &[D::Object(&v)], 1, limits(), 131072, &mut r),
            Err(InvocationError::MalformedIntent)
        ));
    }
    let zero = [("basis", D::Number(-0.0)), ("action", D::Text("remove"))];
    assert_eq!(
        normalize_changes(&id, &[D::Object(&zero)], 1, limits(), 131072, &mut r)
            .unwrap()
            .changes[0]
            .basis,
        0
    );
    let one = [("basis", D::Number(1.0)), ("action", D::Text("remove"))];
    let ordered = normalize_changes(
        &id,
        &[D::Object(&one), D::Object(&zero)],
        2,
        limits(),
        131072,
        &mut r,
    )
    .unwrap();
    assert_eq!(
        ordered
            .changes()
            .iter()
            .map(|c| c.basis)
            .collect::<Vec<_>>(),
        [1, 0]
    );
    assert!(matches!(
        normalize_changes(
            &id,
            &[D::Object(&zero), D::Object(&zero)],
            2,
            limits(),
            131072,
            &mut r
        ),
        Err(InvocationError::MalformedIntent)
    ));
    assert!(matches!(
        normalize_changes(&id, &[D::Object(&zero)], 2, limits(), 131072, &mut r),
        Err(InvocationError::MalformedIntent)
    ));
    assert!(matches!(
        normalize_changes(
            &id,
            &[],
            native::profile().max_relations + 1,
            limits(),
            131072,
            &mut r
        ),
        Err(InvocationError::MalformedIntent)
    ));
}
#[test]
fn configurations_are_closed_profiles_and_booleans_never_default_or_coerce() {
    let (mut o, mut r, _, _) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    for name in ["identityKnown", "atomicRead"] {
        for value in [D::Null, D::Text("false"), D::Number(0.0)] {
            let mut cfg = config();
            cfg.iter_mut().find(|(k, _)| *k == name).unwrap().1 = value;
            let change = [
                ("basis", D::Number(0.0)),
                ("action", D::Text("install")),
                ("configuration", D::Object(&cfg)),
            ];
            assert!(matches!(
                normalize_changes(&id, &[D::Object(&change)], 1, limits(), 131072, &mut r),
                Err(InvocationError::MalformedIntent)
            ));
        }
    }
    for case in 0..10 {
        let mut cfg = config();
        match case {
            0 => {
                cfg.pop();
            }
            1 => cfg.push(("generation", D::Number(0.0))),
            2 => cfg.push(("provider", D::Null)),
            3 => cfg.push(("resource", D::Null)),
            4 => cfg.push(("unknown", D::Null)),
            5 => cfg.push(("procedure", D::Text("write"))),
            6 => cfg[4].1 = D::Text("strict-utf8-one-bom/2"),
            7 => cfg[6].1 = D::Text("crlf"),
            8 => cfg[0].1 = D::Text(""),
            _ => cfg.push(cfg[0]),
        }
        let change = [
            ("basis", D::Number(0.0)),
            ("action", D::Text("install")),
            ("configuration", D::Object(&cfg)),
        ];
        assert!(
            matches!(
                normalize_changes(&id, &[D::Object(&change)], 1, limits(), 131072, &mut r),
                Err(InvocationError::MalformedIntent)
            ),
            "case {case}"
        );
    }
}
#[test]
fn joint_temporary_output_and_independent_capture_bounds_are_checked_at_exact_native_limits() {
    let (mut o, mut r, _, _) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let cfg = config();
    let change = [
        ("basis", D::Number(0.0)),
        ("action", D::Text("install")),
        ("configuration", D::Object(&cfg)),
    ];
    let v = [D::Object(&change)];
    let q = normalize_changes(&id, &v, 1, limits(), 131072, &mut r).unwrap();
    let exact = CaptureLimitsV1 {
        max_bytes: q.usage.peak_bytes,
        max_nodes: q.usage.nodes,
        max_depth: q.usage.depth,
    };
    assert!(normalize_changes(&id, &v, 1, exact, q.peak_bytes, &mut r).is_ok());
    assert!(matches!(
        normalize_changes(&id, &v, 1, exact, q.peak_bytes - 1, &mut r),
        Err(InvocationError::Limit)
    ));
    for i in 0..3 {
        let mut l = exact;
        *[&mut l.max_bytes, &mut l.max_nodes, &mut l.max_depth][i] -= 1;
        assert!(
            matches!(
                normalize_changes(&id, &v, 1, l, 131072, &mut r),
                Err(InvocationError::Limit)
            ),
            "bound {i}"
        );
    }
    assert!(q.owned_bytes() < q.normalization_peak_bytes());
    assert_eq!(format!("{q:?}"), "NormalizedScopedRegistration(<redacted>)");
    assert!(!o.is_faulted());
}
#[test]
fn normalized_data_cannot_grant_authority_or_replace_current_profile_reference_count_or_original_ticket()
 {
    let (mut o, mut r, c, s) = fixture::setup();
    let refs = fixture::refs(&mut o, &s, &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    let change = [("basis", D::Number(0.0)), ("action", D::Text("remove"))];
    let q = normalize_changes(&id, &[D::Object(&change)], 1, limits(), 131072, &mut r).unwrap();
    let mut b = fixture::basis(&s);
    b.presence_observable = None;
    assert_eq!(
        o.submit_normalized_scoped_registration_v1(
            &q,
            &refs,
            &c,
            &fixture::host(&c, &s, &b, &native::native()),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    b.presence_observable = Some(true);
    assert_eq!(
        o.submit_normalized_scoped_registration_v1(
            &q,
            &[],
            &c,
            &fixture::host(&c, &s, &b, &native::native()),
            &mut r
        ),
        Err(InvocationError::MalformedIntent)
    );
    b.native_profile.max_relations += 1;
    assert_eq!(
        o.submit_normalized_scoped_registration_v1(
            &q,
            &refs,
            &c,
            &fixture::host(&c, &s, &b, &native::native()),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    let (mut foreign, _, _, _) = fixture::setup();
    assert!(
        foreign
            .submit_normalized_scoped_registration_v1(
                &q,
                &refs,
                &c,
                &fixture::host(&c, &s, &fixture::basis(&s), &native::native()),
                &mut r
            )
            .is_err()
    );
    o.retire_invocation(&id, &mut r).unwrap();
    assert!(
        o.submit_normalized_scoped_registration_v1(
            &q,
            &refs,
            &c,
            &fixture::host(&c, &s, &fixture::basis(&s), &native::native()),
            &mut r
        )
        .is_err()
    );
    assert!(!o.is_faulted());
    assert!(!foreign.is_faulted());
}
