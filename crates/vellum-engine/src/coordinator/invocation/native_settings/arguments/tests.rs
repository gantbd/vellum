use super::*;
use crate::coordinator::scoped_definition::tests as fixture;
fn cap() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 65536,
        max_nodes: 256,
        max_depth: 32,
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
    changes: D<'_>,
    profile: SettingsActivationLimitsV1,
    cap: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedNativeSettingsArgumentsV1<'h>> {
    use NativeSettingsEnvelopeValueV1 as V;
    let rows = [
        (
            "contract",
            V::Data(D::Text("semantic-settings-activation/1")),
        ),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(id)),
        ("profile", V::Data(D::Text("v1"))),
        ("changes", V::Data(changes)),
        ("dependencies", V::Dependencies(reads)),
    ];
    normalize_native_settings_envelope_v1(&V::Object(&rows), profile, cap, 128, maximum, r).inspect(
        |q| {
            assert!(std::ptr::eq(
                q.argument_declaration_v1(),
                &super::super::declaration::VALUES
            ));
        },
    )
}
#[test]
fn native_versions_operations_duplicate_slots_and_contradictory_layer_conditions_are_closed() {
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
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
        for incarnation in [false, true] {
            let layer = [
                ("authority", D::Text("engine")),
                ("id", D::Text("target")),
                ("incarnation", D::Number(if incarnation { n } else { 1.0 })),
            ];
            let ch = [
                ("layer", D::Object(&layer)),
                (
                    "expectedGeneration",
                    D::Number(if incarnation { 1.0 } else { n }),
                ),
                ("key", D::Text("flag")),
                ("operation", D::Text("set")),
                ("value", D::Boolean(true)),
            ];
            let changes = [D::Object(&ch)];
            assert_eq!(
                parse(
                    &id,
                    &reads,
                    D::Array(&changes),
                    fixture::activation(),
                    cap(),
                    65536,
                    &mut r
                )
                .is_ok(),
                n.is_finite() && n >= 1.0 && n.fract() == 0.0 && n <= 9_007_199_254_740_991.0
            );
        }
    }
    let layer = [
        ("authority", D::Text("engine")),
        ("id", D::Text("target")),
        ("incarnation", D::Number(1.0)),
    ];
    for op in ["set", "remove", "reset", "delete"] {
        for v in [D::Null, D::Boolean(false), D::Text("null")] {
            let ch = [
                ("layer", D::Object(&layer)),
                ("expectedGeneration", D::Number(1.0)),
                ("key", D::Text("flag")),
                ("operation", D::Text(op)),
                ("value", v),
            ];
            let changes = [D::Object(&ch)];
            let q = parse(
                &id,
                &reads,
                D::Array(&changes),
                fixture::activation(),
                cap(),
                65536,
                &mut r,
            );
            assert_eq!(
                q.is_ok(),
                op == "set" || (matches!(op, "remove" | "reset") && matches!(v, D::Null))
            );
            if let Ok(q) = q {
                assert_eq!(
                    matches!(q.changes()[0].operation, SettingOperationV1::Reset),
                    op == "reset"
                );
            }
        }
    }
    let ch = [
        ("layer", D::Object(&layer)),
        ("expectedGeneration", D::Number(1.0)),
        ("key", D::Text("a")),
        ("operation", D::Text("remove")),
        ("value", D::Null),
    ];
    for (key, generation, allowed) in [("a", 1.0, false), ("b", 2.0, false), ("b", 1.0, true)] {
        let mut second = ch;
        second[1].1 = D::Number(generation);
        second[2].1 = D::Text(key);
        let changes = [D::Object(&ch), D::Object(&second)];
        assert_eq!(
            parse(
                &id,
                &reads,
                D::Array(&changes),
                fixture::activation(),
                cap(),
                65536,
                &mut r
            )
            .is_ok(),
            allowed
        );
    }
}
#[test]
fn exact_envelope_and_change_fields_cannot_substitute_ui_permission_or_inert_roles() {
    use NativeSettingsEnvelopeValueV1 as V;
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    for case in 0..10 {
        let mut rows = vec![
            (
                "contract",
                V::Data(D::Text("semantic-settings-activation/1")),
            ),
            ("operation", V::Data(D::Text("semantic-settings"))),
            ("invocation", V::Invocation(&id)),
            ("profile", V::Data(D::Text("v1"))),
            ("changes", V::Data(D::Array(&[]))),
            ("dependencies", V::Dependencies(&reads)),
        ];
        match case {
            0 => {
                rows.pop();
            }
            1 => rows.push(("keymaps", V::Data(D::Array(&[])))),
            2 => rows[4] = rows[0],
            3 => rows[0].1 = V::Data(D::Text("scoped-semantic-settings/1")),
            4 => rows[1].1 = V::Data(D::Text("supported-registry")),
            5 => rows[2].1 = V::Data(D::Text("job")),
            6 => rows[5].1 = V::Data(D::Array(&[])),
            7 => rows[3].1 = V::Data(D::Text("")),
            8 => rows[4].1 = V::Data(D::Null),
            _ => rows[2].1 = V::Dependencies(&reads),
        }
        assert!(
            normalize_native_settings_envelope_v1(
                &V::Object(&rows),
                fixture::activation(),
                cap(),
                128,
                65536,
                &mut r
            )
            .is_err()
        );
    }
    let layer = [
        ("authority", D::Text("engine")),
        ("id", D::Text("target")),
        ("incarnation", D::Number(1.0)),
    ];
    for case in 0..6 {
        let mut ch = vec![
            ("layer", D::Object(&layer)),
            ("expectedGeneration", D::Number(1.0)),
            ("key", D::Text("flag")),
            ("operation", D::Text("set")),
            ("value", D::Boolean(true)),
        ];
        match case {
            0 => {
                ch.pop();
            }
            1 => ch.push(("authorized", D::Boolean(true))),
            2 => ch[0] = ("basis", D::Number(0.0)),
            3 => ch[2].1 = D::Text(""),
            4 => ch[1].1 = D::Text("1"),
            _ => ch[4] = ch[0],
        }
        let changes = [D::Object(&ch)];
        assert!(
            parse(
                &id,
                &reads,
                D::Array(&changes),
                fixture::activation(),
                cap(),
                65536,
                &mut r
            )
            .is_err()
        );
    }
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn joint_inert_and_native_value_limits_are_separate_and_nested_values_preserve_meaning() {
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let list = [D::Number(-0.0), D::Boolean(true), D::Text("文"), D::Null];
    let map = [("list", D::Array(&list)), ("empty", D::Object(&[]))];
    let layer = [
        ("authority", D::Text("engine")),
        ("id", D::Text("target")),
        ("incarnation", D::Number(1.0)),
    ];
    let ch = [
        ("layer", D::Object(&layer)),
        ("expectedGeneration", D::Number(1.0)),
        ("key", D::Text("flag")),
        ("operation", D::Text("set")),
        ("value", D::Object(&map)),
    ];
    let changes = [D::Object(&ch)];
    let profile = SettingsActivationLimitsV1 {
        max_request_nodes: 0,
        max_request_depth: 0,
        ..fixture::activation()
    };
    let q = parse(
        &id,
        &reads,
        D::Array(&changes),
        profile,
        cap(),
        65536,
        &mut r,
    )
    .unwrap();
    assert!(std::ptr::eq(q.invocation(), &id));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    assert_eq!(q.native_profile(), profile);
    let SettingOperationV1::Set(crate::settings::SettingValueV1::Map(m)) =
        &q.changes()[0].operation
    else {
        panic!("map")
    };
    let crate::settings::SettingValueV1::List(v) = &m[0].1 else {
        panic!("list")
    };
    assert!(
        matches!(v[0],crate::settings::SettingValueV1::Number(n) if n.to_bits() == (-0.0f64).to_bits())
    );
    assert!(matches!(
        v[1],
        crate::settings::SettingValueV1::Boolean(true)
    ));
    assert!(matches!(&v[2],crate::settings::SettingValueV1::Text(s) if s == "文"));
    assert!(matches!(v[3], crate::settings::SettingValueV1::Null));
    let peak = q.normalization_peak_bytes();
    assert_eq!(
        parse(
            &id,
            &reads,
            D::Array(&changes),
            profile,
            cap(),
            peak - 1,
            &mut r
        )
        .unwrap_err(),
        InvocationError::Limit
    );
    assert!(
        parse(
            &id,
            &reads,
            D::Array(&changes),
            profile,
            cap(),
            peak,
            &mut r
        )
        .is_ok()
    );
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
            parse(&id, &reads, D::Array(&changes), profile, cap, 65536, &mut r).unwrap_err(),
            InvocationError::Limit
        );
    }
}
