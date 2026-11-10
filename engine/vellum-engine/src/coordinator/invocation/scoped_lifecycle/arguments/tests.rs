use super::*;
use crate::coordinator::scoped_definition::tests as fixture;
fn cap() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 65536,
        max_nodes: 256,
        max_depth: 32,
    }
}
fn profile() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 8,
        max_created_value_bytes: 2048,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 256,
        },
    }
}
fn parse<'h>(
    id: &'h InvocationId,
    reads: &'h ReadDependencyRequestV3,
    parts: [D<'_>; 3],
    cap: CaptureLimitsV1,
    max: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedLifecycleArgumentsV1<'h>> {
    use ScopedLifecycleEnvelopeValueV1 as V;
    let rows = [
        (
            "contract",
            V::Data(D::Text("scoped-engine-settings-layers/1")),
        ),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(id)),
        ("profile", V::Data(D::Text("v1"))),
        ("create", V::Data(parts[0])),
        ("remove", V::Data(parts[1])),
        ("targets", V::Data(parts[2])),
        ("dependencies", V::Dependencies(reads)),
    ];
    normalize_scoped_lifecycle_envelope_v1(
        &V::Object(&rows),
        fixture::activation(),
        profile(),
        cap,
        128,
        max,
        r,
    )
    .inspect(|q| {
        assert!(std::ptr::eq(
            q.argument_declaration_v1(),
            NormalizedScopedLifecycleArgumentsV1::ARGUMENT_DECLARATION
        ))
    })
}
fn identity(name: &str, n: f64) -> [(&str, D<'_>); 3] {
    [
        ("authority", D::Text("engine")),
        ("id", D::Text(name)),
        ("incarnation", D::Number(n)),
    ]
}
#[test]
fn outer_envelope_is_exact_and_inert_data_never_mints_original_roles() {
    use ScopedLifecycleEnvelopeValueV1 as V;
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    for case in 0..13 {
        let mut rows = vec![
            (
                "contract",
                V::Data(D::Text("scoped-engine-settings-layers/1")),
            ),
            ("operation", V::Data(D::Text("semantic-settings"))),
            ("invocation", V::Invocation(&id)),
            ("profile", V::Data(D::Text("v1"))),
            ("create", V::Data(D::Array(&[]))),
            ("remove", V::Data(D::Array(&[]))),
            ("targets", V::Data(D::Array(&[]))),
            ("dependencies", V::Dependencies(&reads)),
        ];
        match case {
            0 => {
                rows.pop();
            }
            1 => rows.push(("expected", V::Data(D::Object(&[])))),
            2 => rows[4] = rows[0],
            3 => rows[0].1 = V::Data(D::Text("engine-settings-layers/1")),
            4 => rows[1].1 = V::Data(D::Text("supported-registry")),
            5 => rows[2].1 = V::Data(D::Text("job")),
            6 => rows[7].1 = V::Data(D::Array(&[])),
            7 => rows[3].1 = V::Data(D::Text("")),
            8 => rows[3].1 = V::Invocation(&id),
            9 => rows[4].1 = V::Data(D::Null),
            10 => rows[5].1 = V::Data(D::Object(&[])),
            11 => rows[6].1 = V::Data(D::Boolean(false)),
            _ => rows[2].1 = V::Dependencies(&reads),
        }
        assert!(
            normalize_scoped_lifecycle_envelope_v1(
                &V::Object(&rows),
                fixture::activation(),
                profile(),
                cap(),
                128,
                65536,
                &mut r
            )
            .is_err()
        );
    }
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn creation_scope_contribution_and_positive_identity_products_are_closed_without_coercion() {
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    for kind in [
        "host",
        "language",
        "workspace",
        "document",
        "view",
        "window",
        "Workspace",
    ] {
        for reset in [false, true] {
            let contribution = if reset {
                vec![("kind", D::Text("reset"))]
            } else {
                vec![("kind", D::Text("set")), ("value", D::Null)]
            };
            let values = [("flag", D::Object(&contribution))];
            let row = [
                ("id", D::Text("same-label")),
                ("kind", D::Text(kind)),
                ("values", D::Object(&values)),
            ];
            let rows = [D::Object(&row)];
            let q = parse(
                &id,
                &reads,
                [D::Array(&rows), D::Array(&[]), D::Array(&[])],
                cap(),
                65536,
                &mut r,
            );
            assert_eq!(q.is_ok(), !matches!(kind, "window" | "Workspace"));
            if let Ok(q) = q {
                assert_eq!(
                    matches!(q.create()[0].values[0].1, SettingContributionV1::Reset),
                    reset
                );
            }
        }
    }
    for e in [
        vec![("kind", D::Text("reset")), ("value", D::Null)],
        vec![("kind", D::Text("set"))],
        vec![("kind", D::Text("remove"))],
        vec![("kind", D::Boolean(true))],
    ] {
        let values = [("flag", D::Object(&e))];
        let row = [
            ("id", D::Text("new")),
            ("kind", D::Text("host")),
            ("values", D::Object(&values)),
        ];
        let rows = [D::Object(&row)];
        assert!(
            parse(
                &id,
                &reads,
                [D::Array(&rows), D::Array(&[]), D::Array(&[])],
                cap(),
                65536,
                &mut r
            )
            .is_err()
        );
    }
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
        let identity = identity("old", n);
        let remove = [D::Object(&identity)];
        let q = parse(
            &id,
            &reads,
            [D::Array(&[]), D::Array(&remove), D::Array(&[])],
            cap(),
            65536,
            &mut r,
        );
        assert_eq!(
            q.is_ok(),
            n.is_finite()
                && n >= 1.0
                && n.fract() == 0.0
                && n <= 9_007_199_254_740_991.0
                && n < usize::MAX as f64
        );
    }
    let old = identity("old", 1.0);
    let remove = [D::Object(&old), D::Object(&old)];
    assert!(
        parse(
            &id,
            &reads,
            [D::Array(&[]), D::Array(&remove), D::Array(&[])],
            cap(),
            65536,
            &mut r
        )
        .is_err()
    );
}
#[test]
fn target_references_reject_shadows_out_of_range_and_duplicate_members_or_targets() {
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let create = [
        ("id", D::Text("new")),
        ("kind", D::Text("host")),
        ("values", D::Object(&[])),
    ];
    let creates = [D::Object(&create)];
    let target_id = identity("target", 1.0);
    let existing = identity("old", 1.0);
    for case in 0..9 {
        let mut reference = vec![("create", D::Number(0.0))];
        match case {
            0 => reference[0].1 = D::Number(1.0),
            1 => reference[0].1 = D::Number(-1.0),
            2 => reference[0].1 = D::Number(0.5),
            3 => reference[0].1 = D::Text("0"),
            4 => reference.push(("basis", D::Number(0.0))),
            5 => reference.push(("id", D::Text("new"))),
            _ => {}
        }
        let layers = if case == 7 {
            vec![D::Object(&existing), D::Object(&existing)]
        } else if case == 6 {
            vec![D::Object(&reference), D::Object(&reference)]
        } else {
            vec![D::Object(&reference)]
        };
        let t = [
            ("target", D::Object(&target_id)),
            ("layers", D::Array(&layers)),
        ];
        let targets = [D::Object(&t), D::Object(&t)];
        let targets = if case == 8 {
            &targets[..]
        } else {
            &targets[..1]
        };
        assert!(
            parse(
                &id,
                &reads,
                [D::Array(&creates), D::Array(&[]), D::Array(targets)],
                cap(),
                65536,
                &mut r
            )
            .is_err()
        );
    }
}
#[test]
fn joint_and_inert_limits_precede_output_and_nested_values_keep_order_signed_zero_and_text() {
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let list = [D::Number(-0.0), D::Text("文"), D::Boolean(true), D::Null];
    let fields = [("list", D::Array(&list)), ("empty", D::Object(&[]))];
    let set = [("kind", D::Text("set")), ("value", D::Object(&fields))];
    let values = [("key", D::Object(&set))];
    let create = [
        ("id", D::Text("same-label")),
        ("kind", D::Text("host")),
        ("values", D::Object(&values)),
    ];
    // Equal labels are intentionally legal: native allocation supplies distinct incarnations.
    let creates = [D::Object(&create), D::Object(&create)];
    let parts = [D::Array(&creates), D::Array(&[]), D::Array(&[])];
    let q = parse(&id, &reads, parts, cap(), 65536, &mut r).unwrap();
    assert_eq!(q.create().len(), 2);
    let SettingContributionV1::Set(SettingValueV1::Map(m)) = &q.create()[0].values[0].1 else {
        panic!("map")
    };
    let SettingValueV1::List(v) = &m[0].1 else {
        panic!("list")
    };
    assert!(matches!(v[0], SettingValueV1::Number(n) if n.to_bits() == (-0.0f64).to_bits()));
    assert!(matches!(&v[1], SettingValueV1::Text(s) if s == "文"));
    assert!(matches!(v[2], SettingValueV1::Boolean(true)));
    assert!(matches!(v[3], SettingValueV1::Null));
    assert!(std::ptr::eq(q.invocation(), &id));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    let peak = q.normalization_peak_bytes();
    assert_eq!(
        parse(&id, &reads, parts, cap(), peak - 1, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    assert!(parse(&id, &reads, parts, cap(), peak, &mut r).is_ok());
    for cap in [
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
            parse(&id, &reads, parts, cap, 65536, &mut r).unwrap_err(),
            InvocationError::Limit
        );
    }
}
