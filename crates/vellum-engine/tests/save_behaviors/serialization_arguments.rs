use super::*;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
#[test]
fn scoped_envelope_derives_its_target_from_the_original_basis_and_commutes_with_unrelated_text_edits()
 {
    let (mut o, mut r) = scoped::ready_scoped();
    let c = context();
    let d = o.create(b"A", &mut r).unwrap();
    let f = facts(&c, &d.document);
    let g = scoped::basis_facts(&f);
    let basis = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let usage = o.serialization_basis_usage();
    let id = o.issue_invocation(&mut r).unwrap();
    let deps = reads();
    use SerializationEnvelopeValueV1 as V;
    let s = [
        ("profile", D::Text("strict-utf8-one-bom/1")),
        ("bom", D::Boolean(true)),
    ];
    let fields = [
        (
            "contract",
            V::Data(D::Text("scoped-source-serialization/1")),
        ),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(&id)),
        ("profile", V::Data(D::Text("v1"))),
        ("serialization", V::Data(D::Object(&s))),
        ("dependencies", V::Dependencies(&deps)),
    ];
    let q = normalize_scoped_serialization_envelope_v1(
        &V::Object(&fields),
        limits(),
        CaptureLimitsV1 {
            max_bytes: 16384,
            max_nodes: 64,
            max_depth: 8,
        },
        32768,
        &mut r,
    )
    .unwrap();
    let mut bad = facts(&c, &d.document);
    bad.profile.max_work += 1;
    assert_eq!(
        o.submit_normalized_scoped_serialization_v1(
            &q,
            &basis,
            &c,
            &scoped::basis_facts(&bad),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.serialization_basis_usage(), usage);
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    o.submit_normalized_scoped_serialization_v1(&q, &basis, &c, &g, &mut r)
        .unwrap();
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 1,
                    end: 1,
                    expected: "".into(),
                    inserted: "!".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    o.run_scoped_source_serialization(&id, &c, &g, &mut r)
        .unwrap();
    let generation = o.generation();
    o.submit_normalized_scoped_serialization_v1(&q, &basis, &c, &g, &mut r)
        .unwrap();
    assert_eq!(o.generation(), generation);
    assert!(
        o.source_serialization(&d.document, &mut r)
            .unwrap()
            .serialization
            .bom
    );
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "A!");
    assert_eq!(o.serialization_basis_usage(), usage);
    assert_eq!(
        o.submit_source_serialization(
            &id,
            &SourceSerializationRequestV1 {
                document: &d.document,
                expected: 1,
                serialization: intent(true),
                dependencies: &deps
            },
            &c,
            &f,
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
}

#[test]
fn decoded_serialization_diagnostics_are_optional_timed_and_do_not_change_admission_replay_or_metadata()
 {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for capacity in [0, 1, 128] {
        let (mut o, mut r) = ready();
        let d = o.create(b"A", &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let deps = reads();
        r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        use SerializationEnvelopeValueV1 as V;
        let s = [
            ("profile", D::Text("strict-utf8-one-bom/1")),
            ("bom", D::Boolean(true)),
        ];
        let outer = [
            ("contract", V::Data(D::Text("source-serialization/1"))),
            ("operation", V::Data(D::Text("semantic-settings"))),
            ("invocation", V::Invocation(&id)),
            ("profile", V::Data(D::Text("v1"))),
            ("document", V::Document(&d.document)),
            ("expectedSerialization", V::Data(D::Number(1.0))),
            ("serialization", V::Data(D::Object(&s))),
            ("dependencies", V::Dependencies(&deps)),
        ];
        let q = normalize_source_serialization_envelope_v1(
            &V::Object(&outer),
            limits(),
            CaptureLimitsV1 {
                max_bytes: 16384,
                max_nodes: 64,
                max_depth: 8,
            },
            32768,
            &mut r,
        )
        .unwrap();
        let mut bad = facts(&c, &d.document);
        bad.profile.max_work += 1;
        assert_eq!(
            o.submit_normalized_source_serialization_v1(&q, &c, &bad, &mut r),
            Err(InvocationError::Unavailable)
        );
        let f = facts(&c, &d.document);
        o.submit_normalized_source_serialization_v1(&q, &c, &f, &mut r)
            .unwrap();
        o.run_source_serialization(&id, &c, &f, &mut r).unwrap();
        let generation = o.generation();
        o.submit_normalized_source_serialization_v1(&q, &c, &f, &mut r)
            .unwrap();
        assert_eq!(o.generation(), generation);
        assert!(
            o.source_serialization(&d.document, &mut r)
                .unwrap()
                .serialization
                .bom
        );
        let mut normalized = 0;
        let mut submitted = 0;
        while let Some(record) = r.pop_front() {
            match record.boundary {
                Boundary::SerializationArgumentNormalization => {
                    normalized += 1;
                    assert!(record.service_time.is_some());
                }
                Boundary::SerializationDecodedAdmission => {
                    submitted += 1;
                    assert!(record.service_time.is_some());
                }
                _ => {}
            }
        }
        if capacity == 128 {
            assert_eq!(normalized, 1);
            assert_eq!(submitted, 3);
        }
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
    }
}
#[test]
fn decoded_serialization_metadata_uses_original_roles_and_the_same_once_only_native_job() {
    let (mut o, mut r) = ready();
    let d = o.create(b"A\r\n", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let deps = reads();
    let c = context();
    let serialization = [
        ("bom", D::Boolean(true)),
        ("profile", D::Text("strict-utf8-one-bom/1")),
    ];
    use SerializationEnvelopeValueV1 as V;
    let outer = [
        ("contract", V::Data(D::Text("source-serialization/1"))),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(&id)),
        ("profile", V::Data(D::Text("v1"))),
        ("document", V::Document(&d.document)),
        ("expectedSerialization", V::Data(D::Number(1.0))),
        ("serialization", V::Data(D::Object(&serialization))),
        ("dependencies", V::Dependencies(&deps)),
    ];
    let q = normalize_source_serialization_envelope_v1(
        &V::Object(&outer),
        limits(),
        CaptureLimitsV1 {
            max_bytes: 16384,
            max_nodes: 64,
            max_depth: 8,
        },
        32768,
        &mut r,
    )
    .unwrap();
    let mut denied = facts(&c, &d.document);
    let mut different_context = context();
    different_context.profile = "another-profile".into();
    assert_eq!(
        o.submit_normalized_source_serialization_v1(
            &q,
            &different_context,
            &facts(&different_context, &d.document),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    denied.write_allowed = Some(false);
    assert!(
        o.submit_normalized_source_serialization_v1(&q, &c, &denied, &mut r)
            .is_err()
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    let f = facts(&c, &d.document);
    o.submit_normalized_source_serialization_v1(&q, &c, &f, &mut r)
        .unwrap();
    o.submit_source_serialization(
        &id,
        &SourceSerializationRequestV1 {
            document: &d.document,
            expected: 1,
            serialization: intent(true),
            dependencies: &deps,
        },
        &c,
        &f,
        &mut r,
    )
    .unwrap();
    o.run_source_serialization(&id, &c, &f, &mut r).unwrap();
    let generation = o.generation();
    o.submit_normalized_source_serialization_v1(&q, &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.generation(), generation);
    assert!(
        o.source_serialization(&d.document, &mut r)
            .unwrap()
            .serialization
            .bom
    );
    assert_eq!(o.read(&d.document, &mut r).unwrap().version, d);
}
