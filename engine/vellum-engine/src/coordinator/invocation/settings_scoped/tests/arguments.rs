use super::*;
use crate::capture::{CaptureLimitsV1, InertValueV1 as D};
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 65536,
        max_nodes: 128,
        max_depth: 32,
    }
}
fn parse<'h>(
    id: &'h InvocationId,
    reads: &'h ReadDependencyRequestV3,
    changes: D<'_>,
    p: SettingsActivationLimitsV1,
    cap: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedSettingsArgumentsV1<'h>, InvocationError> {
    use ScopedSettingsEnvelopeValueV1 as V;
    let rows = [
        ("contract", V::Data(D::Text("scoped-semantic-settings/1"))),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(id)),
        ("profile", V::Data(D::Text("v1"))),
        ("changes", V::Data(changes)),
        ("dependencies", V::Dependencies(reads)),
    ];
    normalize_scoped_settings_envelope_v1(&V::Object(&rows), p, cap, 128, maximum, r).inspect(|q| {
        assert!(std::ptr::eq(
            q.argument_declaration_v1(),
            NormalizedScopedSettingsArgumentsV1::ARGUMENT_DECLARATION
        ))
    })
}
#[test]
fn closed_change_schema_noncoercing_indices_null_operations_and_duplicate_keys() {
    let (mut o, mut r) = setup();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    for number in [
        -1.0,
        -0.0,
        0.0,
        0.5,
        1.0,
        9_007_199_254_740_991.0,
        9_007_199_254_740_992.0,
        f64::NAN,
        f64::INFINITY,
    ] {
        let ch = [
            ("basis", D::Number(number)),
            ("key", D::Text("flag")),
            ("operation", D::Text("set")),
            ("value", D::Boolean(true)),
        ];
        let changes = [D::Object(&ch)];
        let q = parse(
            &job,
            &reads,
            D::Array(&changes),
            activation(),
            limits(),
            65536,
            &mut r,
        );
        assert_eq!(
            q.is_ok(),
            number.is_finite()
                && number >= 0.0
                && number.fract() == 0.0
                && number <= 9_007_199_254_740_991.0
                && number < usize::MAX as f64
        );
    }
    for op in ["set", "remove", "reset", "delete", "Set"] {
        for val in [D::Null, D::Boolean(false), D::Text("null")] {
            let ch = [
                ("basis", D::Number(0.0)),
                ("key", D::Text("flag")),
                ("operation", D::Text(op)),
                ("value", val),
            ];
            let changes = [D::Object(&ch)];
            assert_eq!(
                parse(
                    &job,
                    &reads,
                    D::Array(&changes),
                    activation(),
                    limits(),
                    65536,
                    &mut r
                )
                .is_ok(),
                op == "set" || (matches!(op, "remove" | "reset") && matches!(val, D::Null))
            );
        }
    }
    for case in 0..8 {
        let mut ch = vec![
            ("basis", D::Number(0.0)),
            ("key", D::Text("flag")),
            ("operation", D::Text("set")),
            ("value", D::Null),
        ];
        match case {
            0 => {
                ch.pop();
            }
            1 => ch.push(("expectedGeneration", D::Number(1.0))),
            2 => ch[0] = ("layer", D::Text("target")),
            3 => ch[1].1 = D::Text(""),
            4 => ch[1].1 = D::Boolean(true),
            5 => ch[0].1 = D::Text("0"),
            6 => ch[3] = ch[0],
            _ => {}
        }
        let changes = [D::Object(&ch), D::Object(&ch)];
        let changes = if case == 7 {
            &changes[..]
        } else {
            &changes[..1]
        };
        assert!(
            parse(
                &job,
                &reads,
                D::Array(changes),
                activation(),
                limits(),
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
fn envelope_fields_and_opaque_roles_are_exact_and_never_copied_from_data() {
    use ScopedSettingsEnvelopeValueV1 as V;
    let (mut o, mut r) = setup();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    for case in 0..11 {
        let mut rows = vec![
            ("contract", V::Data(D::Text("scoped-semantic-settings/1"))),
            ("operation", V::Data(D::Text("semantic-settings"))),
            ("invocation", V::Invocation(&job)),
            ("profile", V::Data(D::Text("v1"))),
            ("changes", V::Data(D::Array(&[]))),
            ("dependencies", V::Dependencies(&reads)),
        ];
        match case {
            0 => {
                rows.pop();
            }
            1 => rows.push(("authority", V::Data(D::Text("engine")))),
            2 => rows[4] = rows[0],
            3 => rows[0].1 = V::Data(D::Text("semantic-settings-activation/1")),
            4 => rows[1].1 = V::Data(D::Text("set")),
            5 => rows[2].1 = V::Data(D::Object(&[])),
            6 => rows[5].1 = V::Data(D::Array(&[])),
            7 => rows[3].1 = V::Data(D::Text("")),
            8 => rows[3].1 = V::Invocation(&job),
            9 => rows[4].1 = V::Data(D::Null),
            _ => rows[2].1 = V::Dependencies(&reads),
        }
        assert!(
            normalize_scoped_settings_envelope_v1(
                &V::Object(&rows),
                activation(),
                limits(),
                128,
                65536,
                &mut r
            )
            .is_err()
        );
    }
}
#[test]
fn nested_values_are_preserved_and_joint_inert_count_and_dependency_bounds_are_independent() {
    let (mut o, mut r) = setup();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let list = [D::Null, D::Boolean(true), D::Number(-0.0), D::Text("文")];
    let map = [("list", D::Array(&list)), ("empty", D::Object(&[]))];
    let ch = [
        ("basis", D::Number(0.0)),
        ("key", D::Text("flag")),
        ("operation", D::Text("set")),
        ("value", D::Object(&map)),
    ];
    let changes = [D::Object(&ch)];
    let input = D::Array(&changes);
    let q = parse(&job, &reads, input, activation(), limits(), 65536, &mut r).unwrap();
    let SettingOperationV1::Set(SettingValueV1::Map(map)) = &q.changes()[0].operation else {
        panic!("map")
    };
    let SettingValueV1::List(v) = &map[0].1 else {
        panic!("list")
    };
    assert!(matches!(v[0], SettingValueV1::Null));
    assert!(matches!(v[1], SettingValueV1::Boolean(true)));
    assert!(matches!(v[2], SettingValueV1::Number(n) if n.to_bits() == (-0.0f64).to_bits()));
    assert!(matches!(&v[3], SettingValueV1::Text(s) if s == "文"));
    assert!(matches!(&map[1].1, SettingValueV1::Map(v) if v.is_empty()));
    assert!(std::ptr::eq(q.invocation(), &job));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    let peak = q.normalization_peak_bytes();
    assert_eq!(
        parse(
            &job,
            &reads,
            input,
            activation(),
            limits(),
            peak - 1,
            &mut r
        )
        .unwrap_err(),
        InvocationError::Limit
    );
    assert!(parse(&job, &reads, input, activation(), limits(), peak, &mut r).is_ok());
    for cap in [
        CaptureLimitsV1 {
            max_bytes: q.capture_usage().peak_bytes - 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_nodes: q.capture_usage().nodes - 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_depth: q.capture_usage().depth - 1,
            ..limits()
        },
    ] {
        assert_eq!(
            parse(&job, &reads, input, activation(), cap, 65536, &mut r).unwrap_err(),
            InvocationError::Limit
        );
    }
    let profile = SettingsActivationLimitsV1 {
        max_changes: 0,
        ..activation()
    };
    assert_eq!(
        parse(&job, &reads, input, profile, limits(), 65536, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    let mut bad_reads = reads;
    bad_reads
        .predicates
        .push(ReadDependencyV3::ConfigurationState {
            registry: o.settings_registry(&mut r).unwrap(),
            expected: 1,
            interpretation: o.settings_basis(&mut r).unwrap().interpretation(),
        });
    bad_reads.budget.max_predicates = 0;
    assert_eq!(
        parse(
            &job,
            &bad_reads,
            input,
            activation(),
            limits(),
            65536,
            &mut r
        )
        .unwrap_err(),
        InvocationError::Limit
    );
}

#[test]
fn empty_and_non_set_envelopes_do_not_spend_native_set_value_nodes_or_depth() {
    let (mut o, mut r) = setup();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let p = SettingsActivationLimitsV1 {
        max_request_nodes: 0,
        max_request_depth: 0,
        ..activation()
    };
    assert!(parse(&job, &reads, D::Array(&[]), p, limits(), 65536, &mut r).is_ok());
    let ch = [
        ("basis", D::Number(0.0)),
        ("key", D::Text("flag")),
        ("operation", D::Text("remove")),
        ("value", D::Null),
    ];
    let rows = [D::Object(&ch)];
    assert!(parse(&job, &reads, D::Array(&rows), p, limits(), 65536, &mut r).is_ok());
}
