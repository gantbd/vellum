//! Passive argument preservation, not native source-ref binding or authority.
use LiteralSearchEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
use vellum_engine::source::Limits;
fn owner() -> SourceCoordinator {
    SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 4,
        max_participants: 4,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 1024,
            max_edits: 4,
            max_edit_payload_bytes: 4096,
        },
    })
}
fn actor() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "authority".into(),
        engine: "engine".into(),
        recipient: "reader".into(),
        purpose: "search".into(),
        destination: "client".into(),
        profile: "source-read/1".into(),
        continuity: "continuity".into(),
    }
}
fn limits() -> LiteralSearchArgumentLimitsV1 {
    LiteralSearchArgumentLimitsV1 {
        max_query_bytes: 1024,
        max_label_bytes: 64,
        max_scope_ranges: 8,
        max_owned_bytes: 4096,
    }
}
fn fields<'a, 'h>(
    context: SearchReadContextV1<'h>,
    operation: &'a str,
    reference: D<'a>,
    query: &'a str,
    profile: &'a str,
    scopes: D<'a>,
    navigation: D<'a>,
) -> Vec<(&'a str, V<'a, 'h>)> {
    vec![
        ("contract", V::Data(D::Text("literal-search/1"))),
        ("operation", V::Data(D::Text(operation))),
        ("context", V::Context(context)),
        ("ref", V::Data(reference)),
        ("query", V::Data(D::Text(query))),
        ("profile", V::Data(D::Text(profile))),
        ("scopes", V::Data(scopes)),
        ("generation", V::Data(D::Number(0.0))),
        ("navigation", V::Data(navigation)),
    ]
}
// A real issued read context from the existing native owner; normalization must
// not inspect or duplicate its retained snapshot.
fn fixture() -> (SourceCoordinator, SourceReadContextId, Recorder) {
    let (o, c, r, _, _) = binding_fixture();
    (o, c, r)
}
fn binding_fixture() -> (
    SourceCoordinator,
    SourceReadContextId,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.configure_retention_budget(2_097_152, &mut r).unwrap();
    o.configure_semantic_root_budget(1_048_576, &mut r).unwrap();
    o.configure_source_read_contexts(
        "authority",
        "engine",
        SourceReadContextLimitsV1 {
            max_contexts: 4,
            max_holds: 4,
            max_issued: 8,
            max_root_bytes: 1_048_576,
            max_capture_bytes: 4096,
            max_context_bytes: 2048,
            max_request_bytes: 4096,
            max_reply_bytes: 4096,
            max_label_bytes: 128,
            max_scan_bytes: 4096,
            max_pool_bytes: 1_048_576,
        },
        &SourceReadPoolControlV1 {
            owner_control_allowed: Some(true),
            resources_ready: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
        &mut r,
    )
    .unwrap();
    let d = o.create(b"abcdef", &mut r).unwrap();
    let s = SourceReadScopeV1 {
        document: d.document.clone(),
        context: actor(),
        policy: "policy".into(),
    };
    let f = TrustedSourceReadFactsV1 {
        scope: &s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        retained: Some(true),
        source_readable: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    let c = o.capture_source_read_context(&s, &f, &mut r).unwrap();
    (o, c, r, d, s)
}
#[path = "search_argument_behaviors/binding.rs"]
mod binding;
#[path = "support/discovered_context_fixture.rs"]
mod discovery_support;
#[path = "search_argument_behaviors/installed.rs"]
mod installed;
#[test]
fn all_three_search_forms_preserve_profile_generation_scope_order_and_navigation_without_lookup() {
    let (o, c, mut r) = fixture();
    let a = actor();
    let before = o.source_read_context_usage();
    let reference = [
        ("authority", D::Text("authority")),
        ("document", D::Text("document")),
        ("incarnation", D::Number(0.0)),
        ("revision", D::Number(0.0)),
    ];
    let scope_a = [("lo", D::Number(4.0)), ("hi", D::Number(6.0))];
    let scope_b = [("lo", D::Number(0.0)), ("hi", D::Number(4.0))];
    let scopes = [D::Object(&scope_a), D::Object(&scope_b)];
    let nav = [
        ("start", D::Number(3.0)),
        ("current", D::Number(1.0)),
        ("wrap", D::Boolean(true)),
    ];
    for (operation, expected) in [
        ("literal-matches", LiteralSearchOperationV1::LiteralMatches),
        ("next-match", LiteralSearchOperationV1::NextMatch),
        ("previous-match", LiteralSearchOperationV1::PreviousMatch),
    ] {
        let rows = fields(
            SearchReadContextV1::Source(&c),
            operation,
            D::Object(&reference),
            "",
            "unsupported-but-nonempty/1",
            D::Array(&scopes),
            if operation == "literal-matches" {
                D::Null
            } else {
                D::Object(&nav)
            },
        );
        let q =
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).unwrap();
        assert_eq!(q.operation(), expected);
        assert_eq!(q.argument_declaration_v1().variant.fields.len(), 9);
        assert_eq!(q.query(), "");
        assert_eq!(q.matching_profile(), "unsupported-but-nonempty/1");
        assert_eq!(q.generation(), 0);
        assert_eq!(q.reference().document, "document");
        assert_eq!(q.reference().incarnation, 0);
        assert_eq!(
            q.scopes(),
            &[
                ByteRange { start: 4, end: 6 },
                ByteRange { start: 0, end: 4 }
            ]
        );
        if operation == "literal-matches" {
            assert!(q.navigation().is_none());
        } else {
            assert_eq!(q.navigation().unwrap().current, Some(1));
        }
        assert!(!format!("{q:?}").contains("unsupported-but"));
    }
    assert_eq!(o.source_read_context_usage(), before);
}

const REFERENCE: &[(&str, D<'_>)] = &[
    ("authority", D::Text("authority")),
    ("document", D::Text("document")),
    ("incarnation", D::Number(0.0)),
    ("revision", D::Number(0.0)),
];
fn base<'a, 'h: 'a>(c: &'h SourceReadContextId) -> Vec<(&'a str, V<'a, 'h>)> {
    fields(
        SearchReadContextV1::Source(c),
        "literal-matches",
        D::Object(REFERENCE),
        "秘密 e\u{301}🙂",
        "literal-exact/1",
        D::Array(&[]),
        D::Null,
    )
}

#[test]
fn exact_header_rejects_missing_extra_duplicate_and_wrong_roles_before_any_source_use() {
    let (o, c, mut r) = fixture();
    let a = actor();
    let before = o.source_read_context_usage();
    for index in 0..9 {
        let mut rows = base(&c);
        rows.remove(index);
        assert!(
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err()
        );
        let mut rows = base(&c);
        rows.push(rows[index]);
        assert!(
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err()
        );
        let mut rows = base(&c);
        rows[index].1 = V::Data(D::Boolean(true));
        assert!(
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err()
        );
    }
    let mut rows = base(&c);
    rows[1].1 = V::Data(D::Text("evaluate-anything"));
    assert!(normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err());
    let mut rows = base(&c);
    rows[8].0 = "generation";
    assert!(normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err());
    assert_eq!(o.source_read_context_usage(), before);
}

#[test]
fn nested_objects_are_closed_and_safe_naturals_do_not_coerce_or_truncate() {
    let (_, c, mut r) = fixture();
    let a = actor();
    for value in [
        D::Number(-1.0),
        D::Number(0.5),
        D::Number(f64::NAN),
        D::Number(f64::INFINITY),
        D::Number(9_007_199_254_740_992.0),
        D::Text("0"),
        D::Null,
    ] {
        for index in [2, 3] {
            let mut reference = REFERENCE.to_vec();
            reference[index].1 = value;
            let mut rows = base(&c);
            rows[3].1 = V::Data(D::Object(&reference));
            assert!(
                normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r)
                    .is_err()
            );
        }
        let mut rows = base(&c);
        rows[7].1 = V::Data(value);
        assert!(
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err()
        );
        let scope = [("lo", value), ("hi", D::Number(6.0))];
        let scopes = [D::Object(&scope)];
        let mut rows = base(&c);
        rows[6].1 = V::Data(D::Array(&scopes));
        assert!(
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err()
        );
    }
    for reference in [
        vec![
            ("authority", D::Text("foreign")),
            REFERENCE[1],
            REFERENCE[2],
            REFERENCE[3],
        ],
        vec![REFERENCE[0], REFERENCE[0], REFERENCE[2], REFERENCE[3]],
        REFERENCE[..3].to_vec(),
        [REFERENCE, &[("extra", D::Null)]].concat(),
    ] {
        let mut rows = base(&c);
        rows[3].1 = V::Data(D::Object(&reference));
        assert!(
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err()
        );
    }
    for scope in [
        vec![("lo", D::Number(3.0)), ("hi", D::Number(2.0))],
        vec![("lo", D::Number(0.0)), ("lo", D::Number(0.0))],
        vec![("lo", D::Number(0.0))],
        vec![
            ("lo", D::Number(0.0)),
            ("hi", D::Number(1.0)),
            ("extra", D::Null),
        ],
    ] {
        let scopes = [D::Object(&scope)];
        let mut rows = base(&c);
        rows[6].1 = V::Data(D::Array(&scopes));
        assert!(
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err()
        );
    }
    let declared = literal_search_value_fields_v1();
    assert_eq!(
        declared.reference,
        &["authority", "document", "incarnation", "revision"]
    );
    assert_eq!(declared.scope, &["lo", "hi"]);
    assert_eq!(declared.navigation, &["start", "current", "wrap"]);
}

#[test]
fn navigation_is_operation_specific_and_never_defaults_or_coerces() {
    let (_, c, mut r) = fixture();
    let a = actor();
    let nav = [
        ("start", D::Number(0.0)),
        ("current", D::Null),
        ("wrap", D::Boolean(false)),
    ];
    let mut rows = base(&c);
    rows[8].1 = V::Data(D::Object(&nav));
    assert!(normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err());
    for op in ["next-match", "previous-match"] {
        let mut rows = base(&c);
        rows[1].1 = V::Data(D::Text(op));
        assert!(
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err()
        );
        rows[8].1 = V::Data(D::Object(&nav));
        assert_eq!(
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r)
                .unwrap()
                .navigation(),
            Some(LiteralSearchNavigationArgumentsV1 {
                start: 0,
                current: None,
                wrap: false
            })
        );
        for index in 0..3 {
            let mut bad = nav;
            bad[index].1 = D::Text("0");
            let mut rows = base(&c);
            rows[1].1 = V::Data(D::Text(op));
            rows[8].1 = V::Data(D::Object(&bad));
            assert!(
                normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r)
                    .is_err()
            );
        }
        let bad = [
            ("start", D::Number(0.0)),
            ("current", D::Null),
            ("current", D::Null),
        ];
        rows[8].1 = V::Data(D::Object(&bad));
        assert!(
            normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).is_err()
        );
    }
}

#[test]
fn exact_owned_budget_preserves_utf8_and_empty_scopes_and_refuses_one_byte_less() {
    let (_, c, mut r) = fixture();
    let a = actor();
    let rows = base(&c);
    let q = normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r).unwrap();
    assert_eq!(q.query(), "秘密 e\u{301}🙂");
    assert!(q.scopes().is_empty());
    assert!(std::ptr::eq(q.actor(), &a));
    assert!(matches!(q.source(), SearchReadContextV1::Source(id) if std::ptr::eq(id, &c)));
    let bytes =
        NormalizedLiteralSearchArgumentsV1::required_owned_bytes(q.query().len(), 9, 8, 15, 0)
            .unwrap();
    assert_eq!(bytes, q.owned_bytes());
    let mut exact = limits();
    exact.max_owned_bytes = bytes;
    assert!(normalize_literal_search_arguments_v1(V::Object(&rows), &a, exact, &mut r).is_ok());
    exact.max_owned_bytes -= 1;
    assert!(normalize_literal_search_arguments_v1(V::Object(&rows), &a, exact, &mut r).is_err());
    let mut small = limits();
    small.max_query_bytes = q.query().len() - 1;
    assert!(normalize_literal_search_arguments_v1(V::Object(&rows), &a, small, &mut r).is_err());
    small = limits();
    small.max_label_bytes = 14;
    assert!(normalize_literal_search_arguments_v1(V::Object(&rows), &a, small, &mut r).is_err());
    let scope = [("lo", D::Number(0.0)), ("hi", D::Number(0.0))];
    let scopes = [D::Object(&scope)];
    let mut rows = base(&c);
    rows[6].1 = V::Data(D::Array(&scopes));
    small = limits();
    small.max_scope_ranges = 0;
    assert!(normalize_literal_search_arguments_v1(V::Object(&rows), &a, small, &mut r).is_err());
    assert!(
        NormalizedLiteralSearchArgumentsV1::required_owned_bytes(usize::MAX, 0, 0, 0, 0).is_err()
    );
    assert!(
        NormalizedLiteralSearchArgumentsV1::required_owned_bytes(0, 0, 0, 0, usize::MAX).is_err()
    );
}

#[test]
fn optional_actual_timing_and_saturation_do_not_select_search_arguments() {
    use vellum_engine::instrumentation::Disposition;
    let (o, c, _) = fixture();
    let a = actor();
    let rows = base(&c);
    let before = o.source_read_context_usage();
    for capacity in [0, 1, 4] {
        for timed in [false, true] {
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(if timed {
                    NativeTimingProfileV1::ProcessMonotonicV1
                } else {
                    NativeTimingProfileV1::Off
                });
            for _ in 0..3 {
                let q =
                    normalize_literal_search_arguments_v1(V::Object(&rows), &a, limits(), &mut r)
                        .unwrap();
                assert_eq!(q.query(), "秘密 e\u{301}🙂");
                assert!(q.scopes().is_empty());
            }
            assert!(
                normalize_literal_search_arguments_v1(V::Data(D::Null), &a, limits(), &mut r)
                    .is_err()
            );
            assert_eq!(r.queued(), capacity.min(4));
            assert_eq!(
                r.dropped(),
                if capacity == 0 {
                    0
                } else {
                    (4 - capacity) as u64
                }
            );
            let mut index = 0;
            while let Some(observation) = r.pop_front() {
                assert_eq!(
                    observation.boundary,
                    Boundary::LiteralSearchArgumentNormalization
                );
                assert_eq!(
                    observation.disposition,
                    if index == 3 {
                        Disposition::Refused
                    } else {
                        Disposition::Completed
                    }
                );
                assert_eq!(observation.service_time.is_some(), timed);
                assert!(!format!("{observation:?}").contains("秘密"));
                index += 1;
            }
        }
    }
    assert_eq!(o.source_read_context_usage(), before);
}
