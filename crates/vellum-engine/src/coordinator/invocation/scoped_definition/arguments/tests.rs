use super::*;
use crate::coordinator::scoped_definition::tests as fixture;
fn cap() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 65536,
        max_nodes: 512,
        max_depth: 64,
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
    d: D<'_>,
    cap: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedDefinitionArgumentsV1<'h>> {
    use ScopedDefinitionEnvelopeValueV1 as V;
    let rows = [
        (
            "contract",
            V::Data(D::Text("scoped-engine-settings-definition/1")),
        ),
        ("operation", V::Data(D::Text("supported-registry"))),
        ("invocation", V::Invocation(id)),
        ("profile", V::Data(D::Text("v1"))),
        ("definition", V::Data(d)),
        ("dependencies", V::Dependencies(reads)),
    ];
    normalize_scoped_definition_envelope_v1(
        &V::Object(&rows),
        fixture::activation(),
        fixture::limits(),
        cap,
        128,
        maximum,
        r,
    )
    .inspect(|q| {
        assert!(std::ptr::eq(
            q.argument_declaration_v1(),
            NormalizedScopedDefinitionArgumentsV1::ARGUMENT_DECLARATION
        ))
    })
}
fn type_capture(t: D<'_>, r: &mut Recorder) -> InertCaptureV1 {
    InertCaptureV1::copy(&t, cap(), r).unwrap()
}
#[test]
fn all_six_closed_types_convert_iteratively_and_keep_nested_enum_and_map_meaning() {
    let mut r = Recorder::disabled();
    for (kind, is_bool) in [("boolean", true), ("text", false)] {
        let t = [("kind", D::Text(kind))];
        let c = type_capture(D::Object(&t), &mut r);
        types::cost(&c, 0, 128).unwrap();
        assert_eq!(
            matches!(types::build(&c, 0).unwrap(), SettingTypeV1::Boolean),
            is_bool
        );
    }
    let integer = [
        ("kind", D::Text("integer")),
        ("min", D::Number(-9_007_199_254_740_991.0)),
        ("max", D::Number(9_007_199_254_740_991.0)),
    ];
    let c = type_capture(D::Object(&integer), &mut r);
    types::cost(&c, 0, 128).unwrap();
    assert!(matches!(
        types::build(&c, 0).unwrap(),
        SettingTypeV1::Integer {
            min: -9_007_199_254_740_991,
            max: 9_007_199_254_740_991
        }
    ));
    let options = [D::Number(-0.0), D::Text("文"), D::Null, D::Boolean(true)];
    let e = [("kind", D::Text("enum")), ("values", D::Array(&options))];
    let list = [("kind", D::Text("list")), ("element", D::Object(&e))];
    let fields = [
        ("ordered", D::Object(&list)),
        ("integer", D::Object(&integer)),
    ];
    let map = [("kind", D::Text("map")), ("fields", D::Object(&fields))];
    let c = type_capture(D::Object(&map), &mut r);
    types::cost(&c, 0, 128).unwrap();
    let SettingTypeV1::Map(m) = types::build(&c, 0).unwrap() else {
        panic!("map")
    };
    assert_eq!(m[0].0, "ordered");
    assert_eq!(m[1].0, "integer");
    let SettingTypeV1::List(element) = &m[0].1 else {
        panic!("list")
    };
    let SettingTypeV1::Enum(v) = element.as_ref() else {
        panic!("enum")
    };
    assert!(matches!(v[0],SettingValueV1::Number(n) if n.to_bits() == (-0.0f64).to_bits()));
    assert!(matches!(&v[1],SettingValueV1::Text(s) if s == "文"));
    assert!(matches!(v[2], SettingValueV1::Null));
    assert!(matches!(v[3], SettingValueV1::Boolean(true)));
}
#[test]
fn unknown_type_fields_coercing_integer_bounds_and_bad_schema_metadata_refuse() {
    let mut r = Recorder::disabled();
    for ty in [
        vec![("kind", D::Text("callback"))],
        vec![("kind", D::Text("text")), ("extra", D::Null)],
        vec![("kind", D::Text("list"))],
        vec![("kind", D::Text("map")), ("fields", D::Array(&[]))],
        vec![("kind", D::Text("enum")), ("values", D::Array(&[]))],
    ] {
        let c = type_capture(D::Object(&ty), &mut r);
        assert!(types::cost(&c, 0, 128).is_err());
    }
    for (min, max) in [
        (D::Text("0"), D::Number(1.0)),
        (D::Number(0.5), D::Number(1.0)),
        (D::Number(2.0), D::Number(1.0)),
        (D::Number(0.0), D::Number(9_007_199_254_740_992.0)),
    ] {
        let t = [("kind", D::Text("integer")), ("min", min), ("max", max)];
        let c = type_capture(D::Object(&t), &mut r);
        assert!(types::cost(&c, 0, 128).is_err());
    }
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let ty = [("kind", D::Text("boolean"))];
    let scopes = [D::Text("host")];
    for case in 0..9 {
        let mut s = vec![
            ("key", D::Text("flag")),
            ("type", D::Object(&ty)),
            ("default", D::Boolean(false)),
            ("domain", D::Text("source")),
            ("scopes", D::Array(&scopes)),
            ("composition", D::Text("replace")),
            ("sensitivity", D::Boolean(false)),
        ];
        match case {
            0 => {
                s.pop();
            }
            1 => s.push(("callback", D::Null)),
            2 => s[0].1 = D::Text(""),
            3 => s[3].1 = D::Text("ui"),
            4 => s[4].1 = D::Array(&[]),
            5 => s[4].1 = D::Text("host"),
            6 => s[5].1 = D::Text("merge"),
            7 => s[6].1 = D::Text("false"),
            _ => s[6] = s[0],
        }
        let schemas = [D::Object(&s)];
        let d = [
            ("schemas", D::Array(&schemas)),
            ("restrictions", D::Object(&[])),
            ("targets", D::Array(&[])),
        ];
        assert!(parse(&id, &reads, D::Object(&d), cap(), 65536, &mut r).is_err());
    }
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn exact_outer_roles_and_definition_fields_reject_expectations_layers_and_inert_handles() {
    use ScopedDefinitionEnvelopeValueV1 as V;
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let d = [
        ("schemas", D::Array(&[])),
        ("restrictions", D::Object(&[])),
        ("targets", D::Array(&[])),
    ];
    for case in 0..10 {
        let mut rows = vec![
            (
                "contract",
                V::Data(D::Text("scoped-engine-settings-definition/1")),
            ),
            ("operation", V::Data(D::Text("supported-registry"))),
            ("invocation", V::Invocation(&id)),
            ("profile", V::Data(D::Text("v1"))),
            ("definition", V::Data(D::Object(&d))),
            ("dependencies", V::Dependencies(&reads)),
        ];
        match case {
            0 => {
                rows.pop();
            }
            1 => rows.push(("expected", V::Data(D::Object(&[])))),
            2 => rows[4] = rows[0],
            3 => rows[0].1 = V::Data(D::Text("engine-settings-definition/1")),
            4 => rows[1].1 = V::Data(D::Text("semantic-settings")),
            5 => rows[2].1 = V::Data(D::Text("job")),
            6 => rows[5].1 = V::Data(D::Array(&[])),
            7 => rows[3].1 = V::Data(D::Text("")),
            8 => rows[4].1 = V::Data(D::Null),
            _ => rows[2].1 = V::Dependencies(&reads),
        }
        assert!(
            normalize_scoped_definition_envelope_v1(
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
    for field in ["layers", "expected", "schemaGeneration"] {
        let mut d = d.to_vec();
        d.push((field, D::Array(&[])));
        assert!(parse(&id, &reads, D::Object(&d), cap(), 65536, &mut r).is_err());
    }
}
#[test]
fn joint_and_inert_bounds_are_independent_of_native_value_limits_and_preserve_restriction_order() {
    let (mut o, mut r) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let choices = [D::Number(-0.0), D::Text("文"), D::Array(&[])];
    let restrictions = [("flag", D::Array(&choices))];
    let d = [
        ("schemas", D::Array(&[])),
        ("restrictions", D::Object(&restrictions)),
        ("targets", D::Array(&[])),
    ];
    let q = parse(&id, &reads, D::Object(&d), cap(), 65536, &mut r).unwrap();
    assert!(std::ptr::eq(q.invocation(), &id));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    let v = &q.declaration().restrictions[0].1;
    assert!(matches!(v[0],SettingValueV1::Number(n) if n.to_bits() == (-0.0f64).to_bits()));
    assert!(matches!(&v[1],SettingValueV1::Text(s) if s == "文"));
    assert!(matches!(&v[2],SettingValueV1::List(v) if v.is_empty()));
    let peak = q.normalization_peak_bytes();
    assert_eq!(
        parse(&id, &reads, D::Object(&d), cap(), peak - 1, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    assert!(parse(&id, &reads, D::Object(&d), cap(), peak, &mut r).is_ok());
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
            parse(&id, &reads, D::Object(&d), cap, 65536, &mut r).unwrap_err(),
            InvocationError::Limit
        );
    }
    // Native value limits belong to typed candidate/request admission, not this
    // independently bounded representation of an inert declaration proposal.
    use ScopedDefinitionEnvelopeValueV1 as V;
    let rows = [
        (
            "contract",
            V::Data(D::Text("scoped-engine-settings-definition/1")),
        ),
        ("operation", V::Data(D::Text("supported-registry"))),
        ("invocation", V::Invocation(&id)),
        ("profile", V::Data(D::Text("v1"))),
        ("definition", V::Data(D::Object(&d))),
        ("dependencies", V::Dependencies(&reads)),
    ];
    let activation = SettingsActivationLimitsV1 {
        max_request_nodes: 0,
        max_request_depth: 0,
        ..fixture::activation()
    };
    let zero = normalize_scoped_definition_envelope_v1(
        &V::Object(&rows),
        activation,
        fixture::limits(),
        cap(),
        128,
        65536,
        &mut r,
    )
    .unwrap();
    assert_eq!(zero.activation_profile(), activation);
    assert_eq!(zero.capture_usage(), q.capture_usage());
}
