use super::*;
use crate::coordinator::scoped_definition::tests as fixture;
fn cap() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 65536,
        max_nodes: 256,
        max_depth: 32,
    }
}
fn limits() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 8,
        max_created_value_bytes: 2048,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn id(name: &str, n: f64) -> [(&str, D<'_>); 3] {
    [
        ("authority", D::Text("engine")),
        ("id", D::Text(name)),
        ("incarnation", D::Number(n)),
    ]
}
fn parse<'h>(
    job: &'h InvocationId,
    reads: &'h ReadDependencyRequestV3,
    parts: [D<'_>; 4],
    cap: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedNativeLifecycleArgumentsV1<'h>> {
    use NativeLifecycleEnvelopeValueV1 as V;
    let rows = [
        ("contract", V::Data(D::Text("engine-settings-layers/1"))),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(job)),
        ("profile", V::Data(D::Text("v1"))),
        ("expected", V::Data(parts[0])),
        ("create", V::Data(parts[1])),
        ("remove", V::Data(parts[2])),
        ("targets", V::Data(parts[3])),
        ("dependencies", V::Dependencies(reads)),
    ];
    normalize_native_lifecycle_envelope_v1(
        &V::Object(&rows),
        fixture::activation(),
        limits(),
        cap,
        128,
        maximum,
        r,
    )
    .inspect(|q| {
        assert!(std::ptr::eq(
            q.argument_declaration_v1(),
            &super::super::declaration::LIFECYCLE
        ));
    })
}
#[test]
fn generation_is_positive_highwater_is_natural_and_outer_roles_are_exact() {
    use NativeLifecycleEnvelopeValueV1 as V;
    let (mut o, mut r) = fixture::setup();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = fixture::reads();
    for high in [false, true] {
        for n in [
            -1.0,
            -0.0,
            0.0,
            0.5,
            1.0,
            9_007_199_254_740_991.0,
            9_007_199_254_740_992.0,
            f64::NAN,
        ] {
            let e = [
                ("generation", D::Number(if high { 1.0 } else { n })),
                ("layerIncarnation", D::Number(if high { n } else { 0.0 })),
            ];
            let q = parse(
                &job,
                &reads,
                [D::Object(&e), D::Array(&[]), D::Array(&[]), D::Array(&[])],
                cap(),
                65536,
                &mut r,
            );
            assert_eq!(
                q.is_ok(),
                n.is_finite()
                    && n >= if high { 0.0 } else { 1.0 }
                    && n.fract() == 0.0
                    && n <= 9_007_199_254_740_991.0
            );
        }
    }
    let e = [
        ("generation", D::Number(1.0)),
        ("layerIncarnation", D::Number(0.0)),
    ];
    for case in 0..10 {
        let mut rows = vec![
            ("contract", V::Data(D::Text("engine-settings-layers/1"))),
            ("operation", V::Data(D::Text("semantic-settings"))),
            ("invocation", V::Invocation(&job)),
            ("profile", V::Data(D::Text("v1"))),
            ("expected", V::Data(D::Object(&e))),
            ("create", V::Data(D::Array(&[]))),
            ("remove", V::Data(D::Array(&[]))),
            ("targets", V::Data(D::Array(&[]))),
            ("dependencies", V::Dependencies(&reads)),
        ];
        match case {
            0 => {
                rows.pop();
            }
            1 => rows.push(("authorized", V::Data(D::Boolean(true)))),
            2 => rows[4] = rows[0],
            3 => rows[0].1 = V::Data(D::Text("scoped-engine-settings-layers/1")),
            4 => rows[1].1 = V::Data(D::Text("supported-registry")),
            5 => rows[2].1 = V::Data(D::Text("job")),
            6 => rows[8].1 = V::Data(D::Array(&[])),
            7 => rows[3].1 = V::Data(D::Text("")),
            8 => rows[5].1 = V::Data(D::Null),
            _ => rows[6].1 = V::Data(D::Object(&[])),
        }
        assert!(
            normalize_native_lifecycle_envelope_v1(
                &V::Object(&rows),
                fixture::activation(),
                limits(),
                cap(),
                128,
                65536,
                &mut r
            )
            .is_err()
        );
    }
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
}
#[test]
fn native_creation_removal_and_target_products_reject_duplicate_identities_and_scoped_shadows() {
    let (mut o, mut r) = fixture::setup();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = fixture::reads();
    let e = [
        ("generation", D::Number(1.0)),
        ("layerIncarnation", D::Number(1.0)),
    ];
    let new = id("same-label", 2.0);
    let next = id("same-label", 3.0);
    let create = [
        ("layer", D::Object(&new)),
        ("kind", D::Text("host")),
        ("values", D::Object(&[])),
    ];
    let mut second = create;
    second[0].1 = D::Object(&next);
    let distinct = [D::Object(&create), D::Object(&second)];
    assert!(
        parse(
            &job,
            &reads,
            [
                D::Object(&e),
                D::Array(&distinct),
                D::Array(&[]),
                D::Array(&[])
            ],
            cap(),
            65536,
            &mut r
        )
        .is_ok()
    );
    let duplicate = [D::Object(&create), D::Object(&create)];
    assert!(
        parse(
            &job,
            &reads,
            [
                D::Object(&e),
                D::Array(&duplicate),
                D::Array(&[]),
                D::Array(&[])
            ],
            cap(),
            65536,
            &mut r
        )
        .is_err()
    );
    let creates = [D::Object(&create)];
    let remove = [
        ("layer", D::Object(&new)),
        ("expectedGeneration", D::Number(1.0)),
    ];
    let removes = [D::Object(&remove)];
    assert!(
        parse(
            &job,
            &reads,
            [
                D::Object(&e),
                D::Array(&creates),
                D::Array(&removes),
                D::Array(&[])
            ],
            cap(),
            65536,
            &mut r
        )
        .is_err()
    );
    for shadow in ["id", "basis", "expected"] {
        let mut row = create.to_vec();
        row[0] = (shadow, D::Text("same-label"));
        let rows = [D::Object(&row)];
        assert!(
            parse(
                &job,
                &reads,
                [D::Object(&e), D::Array(&rows), D::Array(&[]), D::Array(&[])],
                cap(),
                65536,
                &mut r
            )
            .is_err()
        );
    }
    let target_id = id("target", 1.0);
    let selector = [("create", D::Number(0.0))];
    let refs = [D::Object(&selector)];
    let target = [
        ("target", D::Object(&target_id)),
        ("layers", D::Array(&refs)),
    ];
    let targets = [D::Object(&target)];
    assert!(
        parse(
            &job,
            &reads,
            [
                D::Object(&e),
                D::Array(&creates),
                D::Array(&[]),
                D::Array(&targets)
            ],
            cap(),
            65536,
            &mut r
        )
        .is_err()
    );
}
#[test]
fn joint_capture_bounds_precede_output_and_shared_contributions_keep_signed_zero_and_reset() {
    let (mut o, mut r) = fixture::setup();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = fixture::reads();
    let e = [
        ("generation", D::Number(1.0)),
        ("layerIncarnation", D::Number(1.0)),
    ];
    let new = id("new", 2.0);
    let list = [D::Number(-0.0), D::Text("文"), D::Null];
    let set = [("kind", D::Text("set")), ("value", D::Array(&list))];
    let reset = [("kind", D::Text("reset"))];
    let values = [("a", D::Object(&set)), ("b", D::Object(&reset))];
    let create = [
        ("layer", D::Object(&new)),
        ("kind", D::Text("host")),
        ("values", D::Object(&values)),
    ];
    let creates = [D::Object(&create)];
    let parts = [
        D::Object(&e),
        D::Array(&creates),
        D::Array(&[]),
        D::Array(&[]),
    ];
    let q = parse(&job, &reads, parts, cap(), 65536, &mut r).unwrap();
    assert!(std::ptr::eq(q.invocation(), &job));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    let crate::settings::SettingContributionV1::Set(SettingValueV1::List(v)) =
        &q.changes().create[0].values[0].1
    else {
        panic!("list")
    };
    assert!(matches!(v[0],SettingValueV1::Number(n) if n.to_bits() == (-0.0f64).to_bits()));
    assert!(matches!(&v[1],SettingValueV1::Text(s) if s == "文"));
    assert!(matches!(
        q.changes().create[0].values[1].1,
        crate::settings::SettingContributionV1::Reset
    ));
    let peak = q.normalization_peak_bytes();
    assert_eq!(
        parse(&job, &reads, parts, cap(), peak - 1, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    assert!(parse(&job, &reads, parts, cap(), peak, &mut r).is_ok());
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
            parse(&job, &reads, parts, cap, 65536, &mut r).unwrap_err(),
            InvocationError::Limit
        );
    }
}
