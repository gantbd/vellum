use super::*;

fn logical_request<'a>(
    c: &'a SourceReadContextId,
    s: &'a SourceReadScopeV1,
    reference: &LiteralSearchReferenceV1,
    generation: u64,
    profile: &str,
    r: &mut Recorder,
) -> NormalizedLiteralSearchArgumentsV1<'a> {
    let logical_ref = [
        ("authority", D::Text(&reference.authority)),
        ("document", D::Text(&reference.document)),
        ("incarnation", D::Number(reference.incarnation as f64)),
        ("revision", D::Number(reference.revision as f64)),
    ];
    let range = [("lo", D::Number(0.0)), ("hi", D::Number(6.0))];
    let scopes = [D::Object(&range)];
    let mut rows = fields(
        SearchReadContextV1::Source(c),
        "literal-matches",
        D::Object(&logical_ref),
        "a",
        profile,
        D::Array(&scopes),
        D::Null,
    );
    rows.iter_mut().find(|(k, _)| *k == "generation").unwrap().1 =
        V::Data(D::Number(generation as f64));
    normalize_literal_search_arguments_v1(V::Object(&rows), &s.context, limits(), r).unwrap()
}

#[test]
fn binding_checks_original_identity_generation_profile_loss_and_protected_cleanup() {
    let (mut o, c, mut r, d, s) = setup();
    let (mut other, foreign, _, _, _) = setup();
    let reference = LiteralSearchReferenceV1 {
        authority: "authority".into(),
        document: "logical".into(),
        incarnation: 7,
        revision: d.revision,
    };
    let premises = |max_binding_bytes| TrustedLogicalSearchBindingV1 {
        source: SearchReadContextV1::Source(&c),
        scope: &s,
        expected: &d,
        reference: &reference,
        generation: 9,
        binding_allowed: Some(true),
        correspondence_current: Some(true),
        max_label_bytes: 128,
        max_binding_bytes,
    };
    assert_eq!(
        o.bind_logical_search_source_v1(premises(1), &mut r)
            .unwrap_err(),
        SourceReadContextErrorV1::Limit
    );
    let binding = o
        .bind_logical_search_source_v1(premises(4096), &mut r)
        .unwrap();
    let native = SearchRequest {
        query: "a".into(),
        scope: SearchScope::Captured(vec![ByteRange { start: 0, end: 6 }]),
    };
    let sf = facts(&s);
    let correct = logical_request(
        &c,
        &s,
        &reference,
        9,
        LITERAL_EXACT_MATCHING_PROFILE_V1,
        &mut r,
    );
    let wrong_generation = logical_request(
        &c,
        &s,
        &reference,
        10,
        LITERAL_EXACT_MATCHING_PROFILE_V1,
        &mut r,
    );
    let unsupported = logical_request(&c, &s, &reference, 9, "not-supported/1", &mut r);
    let foreign_request = logical_request(
        &foreign,
        &s,
        &reference,
        9,
        LITERAL_EXACT_MATCHING_PROFILE_V1,
        &mut r,
    );
    let before = o.source_search_usage();
    for q in [&wrong_generation, &foreign_request] {
        // Invalid original target must not poison the legitimate binding even
        // when coupled with a negative current-correspondence assertion.
        assert!(
            o.query_bound_literal_search_v1(
                &binding,
                q,
                SearchReadFactsV1::Source(&sf),
                &qfacts(&native),
                Some(false),
                &mut r
            )
            .is_err()
        );
    }
    assert!(matches!(
        o.query_bound_literal_search_v1(
            &binding,
            &unsupported,
            SearchReadFactsV1::Source(&sf),
            &qfacts(&native),
            Some(true),
            &mut r
        ),
        Err(BoundLiteralSearchErrorV1::UnsupportedProfile)
    ));
    assert_eq!(o.source_search_usage(), before);
    assert!(
        other
            .query_bound_literal_search_v1(
                &binding,
                &correct,
                SearchReadFactsV1::Source(&sf),
                &qfacts(&native),
                Some(true),
                &mut r
            )
            .is_err()
    );
    let mut result = o
        .query_bound_literal_search_v1(
            &binding,
            &correct,
            SearchReadFactsV1::Source(&sf),
            &qfacts(&native),
            Some(true),
            &mut r,
        )
        .unwrap();
    let mut control = SourceReadContextControlV1 {
        scope: &s,
        owner_control_allowed: None,
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert!(!o.retire_bound_literal_search_v1(
        &mut result,
        SearchResultControlV1::Single(&control),
        &mut r
    ));
    assert_eq!(o.source_search_usage().results, 1);
    // Unknown readiness is transient, observed loss is absorbing.
    assert!(
        o.release_bound_literal_search_v1(
            &result,
            SearchReadFactsV1::Source(&sf),
            &qfacts(&native),
            None,
            &mut r
        )
        .is_err()
    );
    assert!(
        o.release_bound_literal_search_v1(
            &result,
            SearchReadFactsV1::Source(&sf),
            &qfacts(&native),
            Some(true),
            &mut r
        )
        .is_ok()
    );
    assert!(
        o.release_bound_literal_search_v1(
            &result,
            SearchReadFactsV1::Source(&sf),
            &qfacts(&native),
            Some(false),
            &mut r
        )
        .is_err()
    );
    assert!(
        o.release_bound_literal_search_v1(
            &result,
            SearchReadFactsV1::Source(&sf),
            &qfacts(&native),
            Some(true),
            &mut r
        )
        .is_err()
    );
    control.owner_control_allowed = Some(true);
    assert!(o.retire_bound_literal_search_v1(
        &mut result,
        SearchResultControlV1::Single(&control),
        &mut r
    ));
    assert!(!o.retire_bound_literal_search_v1(
        &mut result,
        SearchResultControlV1::Single(&control),
        &mut r
    ));
    assert_eq!(o.source_search_usage().results, 0);
    assert_eq!(o.source_read_context_usage().holds, 0);
}

pub(super) fn setup() -> (
    SourceCoordinator,
    SourceReadContextId,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
) {
    setup_with_reply_bytes(8192)
}
fn setup_with_reply_bytes(
    max_reply_bytes: usize,
) -> (
    SourceCoordinator,
    SourceReadContextId,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
) {
    let (mut o, c, mut r, d, s) = binding_fixture();
    o.configure_source_search(
        SourceSearchLimitsV1 {
            max_results: 4,
            max_issued: 8,
            max_request_bytes: 8192,
            max_capture_bytes: 16384,
            max_reply_bytes,
            max_pool_bytes: 524288,
            max_query_bytes: 128,
            max_scope_ranges: 8,
            max_comparisons: 4096,
            max_matches: 16,
            max_replacement_bytes: 1024,
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
    (o, c, r, d, s)
}
pub(super) fn facts(s: &SourceReadScopeV1) -> TrustedSourceReadFactsV1<'_> {
    TrustedSourceReadFactsV1 {
        scope: s,
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
    }
}
#[test]
fn logical_reply_header_pressure_refuses_before_native_issuance_or_hold() {
    let (mut o, c, mut r, d, s) =
        setup_with_reply_bytes(std::mem::size_of::<SourceSearchReplyV1>());
    let reference = LiteralSearchReferenceV1 {
        authority: "authority".into(),
        document: "logical".into(),
        incarnation: 7,
        revision: d.revision,
    };
    let binding = o
        .bind_logical_search_source_v1(
            TrustedLogicalSearchBindingV1 {
                source: SearchReadContextV1::Source(&c),
                scope: &s,
                expected: &d,
                reference: &reference,
                generation: 9,
                binding_allowed: Some(true),
                correspondence_current: Some(true),
                max_label_bytes: 128,
                max_binding_bytes: 4096,
            },
            &mut r,
        )
        .unwrap();
    let q = logical_request(
        &c,
        &s,
        &reference,
        9,
        LITERAL_EXACT_MATCHING_PROFILE_V1,
        &mut r,
    );
    let native = SearchRequest {
        query: "a".into(),
        scope: SearchScope::Captured(vec![ByteRange { start: 0, end: 6 }]),
    };
    let sf = facts(&s);
    let before = (
        o.source_search_usage(),
        o.source_read_context_usage(),
        o.retention_usage(),
    );
    assert!(matches!(
        o.query_bound_literal_search_v1(
            &binding,
            &q,
            SearchReadFactsV1::Source(&sf),
            &qfacts(&native),
            Some(true),
            &mut r
        ),
        Err(BoundLiteralSearchErrorV1::Native(
            SourceReadContextErrorV1::Limit
        ))
    ));
    assert_eq!(
        (
            o.source_search_usage(),
            o.source_read_context_usage(),
            o.retention_usage()
        ),
        before
    );
}
pub(super) fn qfacts(q: &SearchRequest) -> TrustedSourceSearchFactsV1<'_> {
    TrustedSourceSearchFactsV1 {
        request: q,
        grant: Some(true),
        scope: Some(true),
        continuity: Some(true),
        capacity: Some(true),
        retained: Some(true),
        not_stopped: Some(true),
    }
}
#[test]
fn bound_logical_search_preserves_provenance_and_retires_original_result_under_capture_pressure() {
    for capacity in [0, 1, 256] {
        for operation in ["literal-matches", "next-match", "previous-match"] {
            let (mut o, c, mut r, d, s) = setup();
            let reference = LiteralSearchReferenceV1 {
                authority: "authority".into(),
                document: "PRIVATE-logical".into(),
                incarnation: 7,
                revision: d.revision,
            };
            let binding = o
                .bind_logical_search_source_v1(
                    TrustedLogicalSearchBindingV1 {
                        source: SearchReadContextV1::Source(&c),
                        scope: &s,
                        expected: &d,
                        reference: &reference,
                        generation: 0,
                        binding_allowed: Some(true),
                        correspondence_current: Some(true),
                        max_label_bytes: 128,
                        max_binding_bytes: 4096,
                    },
                    &mut r,
                )
                .unwrap();
            r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            let logical_ref = [
                ("authority", D::Text("authority")),
                ("document", D::Text("PRIVATE-logical")),
                ("incarnation", D::Number(7.0)),
                ("revision", D::Number(d.revision as f64)),
            ];
            let range = [("lo", D::Number(0.0)), ("hi", D::Number(6.0))];
            let scopes = [D::Object(&range)];
            let nav = [
                (
                    "start",
                    D::Number(if operation == "previous-match" {
                        6.0
                    } else {
                        0.0
                    }),
                ),
                ("current", D::Null),
                ("wrap", D::Boolean(false)),
            ];
            let rows = fields(
                SearchReadContextV1::Source(&c),
                operation,
                D::Object(&logical_ref),
                "a",
                "literal-exact/1",
                D::Array(&scopes),
                if operation == "literal-matches" {
                    D::Null
                } else {
                    D::Object(&nav)
                },
            );
            let q = normalize_literal_search_arguments_v1(
                V::Object(&rows),
                &s.context,
                limits(),
                &mut r,
            )
            .unwrap();
            let native = SearchRequest {
                query: "a".into(),
                scope: SearchScope::Captured(vec![ByteRange { start: 0, end: 6 }]),
            };
            let sf = facts(&s);
            let mut result = o
                .query_bound_literal_search_v1(
                    &binding,
                    &q,
                    SearchReadFactsV1::Source(&sf),
                    &qfacts(&native),
                    Some(true),
                    &mut r,
                )
                .unwrap();
            let generation = o.generation();
            let mut denied = qfacts(&native);
            denied.grant = None;
            assert!(
                o.release_bound_literal_search_v1(
                    &result,
                    SearchReadFactsV1::Source(&sf),
                    &denied,
                    Some(true),
                    &mut r
                )
                .is_err()
            );
            let reply = o
                .release_bound_literal_search_v1(
                    &result,
                    SearchReadFactsV1::Source(&sf),
                    &qfacts(&native),
                    Some(true),
                    &mut r,
                )
                .unwrap();
            assert_eq!(reply.reference.document, "PRIVATE-logical");
            assert_eq!(reply.matching_profile, "literal-exact/1");
            assert_eq!(reply.generation, 0);
            match reply.product {
                BoundLiteralSearchProductV1::Matches(v) => {
                    assert_eq!(v.matches, vec![ByteRange { start: 0, end: 1 }])
                }
                BoundLiteralSearchProductV1::Navigation(v) => {
                    assert_eq!(v.found.unwrap().range, ByteRange { start: 0, end: 1 })
                }
            }
            assert_eq!(o.generation(), generation);
            let control = SourceReadContextControlV1 {
                scope: &s,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            assert!(o.retire_bound_literal_search_v1(
                &mut result,
                SearchResultControlV1::Single(&control),
                &mut r
            ));
            assert!(
                o.release_bound_literal_search_v1(
                    &result,
                    SearchReadFactsV1::Source(&sf),
                    &qfacts(&native),
                    Some(true),
                    &mut r
                )
                .is_err()
            );
            assert_eq!(o.source_search_usage().results, 0);
            assert_eq!(o.source_read_context_usage().holds, 0);
            o.retire_logical_search_binding_v1(&binding, &mut r);
            assert!(
                o.query_bound_literal_search_v1(
                    &binding,
                    &q,
                    SearchReadFactsV1::Source(&sf),
                    &qfacts(&native),
                    Some(true),
                    &mut r
                )
                .is_err()
            );
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(!format!("{events:?}").contains("PRIVATE"));
            if capacity == 256 {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::LogicalSearchRelease
                            && e.service_time.is_some())
                );
            }
        }
    }
}
