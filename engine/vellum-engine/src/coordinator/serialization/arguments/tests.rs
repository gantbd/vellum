use super::super::tests as fixture;
use super::*;
fn cap() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 32768,
        max_nodes: 128,
        max_depth: 8,
    }
}
fn parse<'h>(
    id: &'h InvocationId,
    d: &'h DocumentId,
    reads: &'h ReadDependencyRequestV3,
    expected: D<'_>,
    serialization: D<'_>,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedSourceSerializationArgumentsV1<'h>> {
    use SerializationEnvelopeValueV1 as V;
    let rows = [
        ("contract", V::Data(D::Text("source-serialization/1"))),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(id)),
        ("profile", V::Data(D::Text("v1"))),
        ("document", V::Document(d)),
        ("expectedSerialization", V::Data(expected)),
        ("serialization", V::Data(serialization)),
        ("dependencies", V::Dependencies(reads)),
    ];
    normalize_source_serialization_envelope_v1(
        &V::Object(&rows),
        fixture::limits(),
        cap(),
        maximum,
        r,
    )
    .inspect(|q| {
        assert!(std::ptr::eq(
            q.argument_declaration_v1(),
            NormalizedSourceSerializationArgumentsV1::ARGUMENT_DECLARATION
        ))
    })
}
#[test]
fn positive_safe_version_and_closed_profile_boolean_are_required_without_coercion_or_shadows() {
    let (mut o, mut r, d) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = fixture::reads();
    let valid = [
        ("profile", D::Text("strict-utf8-one-bom/1")),
        ("bom", D::Boolean(false)),
    ];
    for n in [
        0.0,
        -0.0,
        -1.0,
        0.5,
        1.0,
        9_007_199_254_740_991.0,
        9_007_199_254_740_992.0,
        f64::NAN,
        f64::INFINITY,
    ] {
        let q = parse(
            &id,
            &d.document,
            &reads,
            D::Number(n),
            D::Object(&valid),
            65536,
            &mut r,
        );
        assert_eq!(
            q.is_ok(),
            n.is_finite() && n >= 1.0 && n.fract() == 0.0 && n <= 9_007_199_254_740_991.0
        );
    }
    for bad in [D::Null, D::Array(&[]), D::Text("true"), D::Number(1.0)] {
        let v = [("profile", D::Text("strict-utf8-one-bom/1")), ("bom", bad)];
        assert!(
            parse(
                &id,
                &d.document,
                &reads,
                D::Number(1.0),
                D::Object(&v),
                65536,
                &mut r
            )
            .is_err()
        );
    }
    for v in [
        vec![("profile", D::Text("utf8/1")), ("bom", D::Boolean(true))],
        vec![("profile", D::Text("strict-utf8-one-bom/1"))],
        vec![
            ("profile", D::Text("strict-utf8-one-bom/1")),
            ("bom", D::Boolean(true)),
            ("extra", D::Null),
        ],
        vec![
            ("profile", D::Text("strict-utf8-one-bom/1")),
            ("bom", D::Boolean(true)),
            ("bom", D::Null),
        ],
    ] {
        assert!(
            parse(
                &id,
                &d.document,
                &reads,
                D::Number(1.0),
                D::Object(&v),
                65536,
                &mut r
            )
            .is_err()
        );
    }
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn outer_roles_native_and_scoped_shapes_are_distinct_and_inert_values_cannot_mint_roles() {
    use SerializationEnvelopeValueV1 as V;
    let (mut o, mut r, d) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = fixture::reads();
    let value = [
        ("profile", D::Text("strict-utf8-one-bom/1")),
        ("bom", D::Boolean(true)),
    ];
    for case in 0..12 {
        let mut rows = vec![
            ("contract", V::Data(D::Text("source-serialization/1"))),
            ("operation", V::Data(D::Text("semantic-settings"))),
            ("invocation", V::Invocation(&id)),
            ("profile", V::Data(D::Text("v1"))),
            ("document", V::Document(&d.document)),
            ("expectedSerialization", V::Data(D::Number(1.0))),
            ("serialization", V::Data(D::Object(&value))),
            ("dependencies", V::Dependencies(&reads)),
        ];
        match case {
            0 => {
                rows.pop();
            }
            1 => rows.push(("extra", V::Data(D::Null))),
            2 => rows[7] = rows[0],
            3 => rows[0].1 = V::Data(D::Text("scoped-source-serialization/1")),
            4 => rows[1].1 = V::Data(D::Text("run")),
            5 => rows[2].1 = V::Data(D::Text("invocation")),
            6 => rows[4].1 = V::Data(D::Object(&[])),
            7 => rows[7].1 = V::Data(D::Array(&[])),
            8 => rows[6].1 = V::Document(&d.document),
            9 => rows[3].1 = V::Data(D::Text("")),
            10 => rows[5].1 = V::Data(D::Text("1")),
            _ => rows[4].1 = V::Invocation(&id),
        }
        assert!(
            normalize_source_serialization_envelope_v1(
                &V::Object(&rows),
                fixture::limits(),
                cap(),
                65536,
                &mut r
            )
            .is_err()
        );
        assert!(
            normalize_scoped_serialization_envelope_v1(
                &V::Object(&rows),
                fixture::limits(),
                cap(),
                65536,
                &mut r
            )
            .is_err()
        );
    }
}
#[test]
fn exact_joint_and_inert_bounds_precede_output_and_dependency_roles_remain_original_borrows() {
    use SerializationEnvelopeValueV1 as V;
    let (mut o, mut r, d) = fixture::setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let reads = fixture::reads();
    let value = [
        ("profile", D::Text("strict-utf8-one-bom/1")),
        ("bom", D::Boolean(true)),
    ];
    let q = parse(
        &id,
        &d.document,
        &reads,
        D::Number(1.0),
        D::Object(&value),
        65536,
        &mut r,
    )
    .unwrap();
    assert!(std::ptr::eq(q.invocation(), &id));
    assert!(std::ptr::eq(q.request().document, &d.document));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    let peak = q.normalization_peak_bytes();
    assert!(peak >= q.owned_bytes());
    assert!(matches!(
        parse(
            &id,
            &d.document,
            &reads,
            D::Number(1.0),
            D::Object(&value),
            peak - 1,
            &mut r
        ),
        Err(InvocationError::Limit)
    ));
    assert_eq!(
        parse(
            &id,
            &d.document,
            &reads,
            D::Number(1.0),
            D::Object(&value),
            peak,
            &mut r
        )
        .unwrap()
        .serialization(),
        q.serialization()
    );
    let predicate =
        ReadDependencyV3::Base(ReadDependencyV2::Base(ReadDependencyV1::Source(d.clone())));
    for declared in [
        ReadDependencyRequestV3 {
            predicates: vec![predicate.clone(); 9],
            budget: reads.budget,
        },
        ReadDependencyRequestV3 {
            predicates: vec![predicate],
            budget: ReadDependencyBudget {
                max_predicates: 0,
                ..reads.budget
            },
        },
    ] {
        assert!(matches!(
            parse(
                &id,
                &d.document,
                &declared,
                D::Number(1.0),
                D::Object(&value),
                65536,
                &mut r
            ),
            Err(InvocationError::Limit)
        ));
    }
    let rows = [
        ("contract", V::Data(D::Text("source-serialization/1"))),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(&id)),
        ("profile", V::Data(D::Text("v1"))),
        ("document", V::Document(&d.document)),
        ("expectedSerialization", V::Data(D::Number(1.0))),
        ("serialization", V::Data(D::Object(&value))),
        ("dependencies", V::Dependencies(&reads)),
    ];
    for l in [
        CaptureLimitsV1 {
            max_bytes: 1,
            ..cap()
        },
        CaptureLimitsV1 {
            max_nodes: 1,
            ..cap()
        },
        CaptureLimitsV1 {
            max_depth: 0,
            ..cap()
        },
    ] {
        assert!(matches!(
            normalize_source_serialization_envelope_v1(
                &V::Object(&rows),
                fixture::limits(),
                l,
                65536,
                &mut r
            ),
            Err(InvocationError::Limit)
        ));
    }
    let scoped = [
        (
            "contract",
            V::Data(D::Text("scoped-source-serialization/1")),
        ),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(&id)),
        ("profile", V::Data(D::Text("v1"))),
        ("serialization", V::Data(D::Object(&value))),
        ("dependencies", V::Dependencies(&reads)),
    ];
    let scoped_q = normalize_scoped_serialization_envelope_v1(
        &V::Object(&scoped),
        fixture::limits(),
        cap(),
        65536,
        &mut r,
    )
    .unwrap();
    assert!(std::ptr::eq(scoped_q.dependencies(), &reads));
    assert!(std::ptr::eq(
        scoped_q.argument_declaration_v1(),
        NormalizedScopedSerializationArgumentsV1::ARGUMENT_DECLARATION
    ));
    let peak = scoped_q.normalization_peak_bytes();
    assert!(matches!(
        normalize_scoped_serialization_envelope_v1(
            &V::Object(&scoped),
            fixture::limits(),
            cap(),
            peak - 1,
            &mut r
        ),
        Err(InvocationError::Limit)
    ));
    assert_eq!(
        normalize_scoped_serialization_envelope_v1(
            &V::Object(&scoped),
            fixture::limits(),
            cap(),
            peak,
            &mut r
        )
        .unwrap()
        .serialization(),
        q.serialization()
    );
}
