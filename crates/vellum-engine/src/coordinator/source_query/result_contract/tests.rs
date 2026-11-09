use super::*;
pub(in crate::coordinator) struct Fixture {
    pub(in crate::coordinator) o: SourceCoordinator,
    pub(in crate::coordinator) r: Recorder,
    pub(in crate::coordinator) s: SourceReadScopeV1,
    pub(in crate::coordinator) version: DocumentVersion,
    pub(in crate::coordinator) source: SourceReadContextId,
    pub(in crate::coordinator) combined: AnnotationReadContextId,
    pub(in crate::coordinator) set: VersionSetAcquisitionV1,
}
fn control() -> SourceReadPoolControlV1 {
    SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(in crate::coordinator) fn cleanup(s: &SourceReadScopeV1) -> SourceReadContextControlV1<'_> {
    SourceReadContextControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(in crate::coordinator) fn source_facts(s: &SourceReadScopeV1) -> TrustedSourceReadFactsV1<'_> {
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
pub(in crate::coordinator) fn member_facts(
    s: &SourceReadScopeV1,
) -> TrustedVersionSetMemberFactsV1<'_> {
    TrustedVersionSetMemberFactsV1 {
        scope: s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        annotation_readable: Some(true),
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    }
}
pub(in crate::coordinator) fn owner() -> Fixture {
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 4096,
            max_edits: 16,
            max_edit_payload_bytes: 4096,
        },
    });
    let mut r = Recorder::disabled();
    let version = o.create(b"body\r\nlast", &mut r).unwrap();
    let s = SourceReadScopeV1 {
        document: version.document.clone(),
        context: crate::coordinator::scoped_definition::tests::context(),
        policy: "p1".into(),
    };
    o.configure_retention_budget(usize::MAX, &mut r).unwrap();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_source_read_contexts(
        "engine",
        "local",
        SourceReadContextLimitsV1 {
            max_contexts: 4,
            max_holds: 8,
            max_issued: 16,
            max_root_bytes: 1048576,
            max_capture_bytes: 8192,
            max_context_bytes: 4096,
            max_request_bytes: 4096,
            max_reply_bytes: 4096,
            max_label_bytes: 128,
            max_scan_bytes: 4096,
            max_pool_bytes: 1048576,
        },
        &control(),
        &mut r,
    )
    .unwrap();
    let source = o
        .capture_source_read_context(&s, &source_facts(&s), &mut r)
        .unwrap();
    let profile = SourceProjectionLimitsV1 {
        max_source_bytes: 4096,
        max_request_bytes: 4096,
        max_reply_bytes: 4096,
        max_scan_bytes: 4096,
    };
    o.configure_annotation_preparation(
        "engine",
        "local",
        AnnotationPreparationLimitsV1 {
            max_prepared: 4,
            max_issued: 8,
            max_rows: 16,
            max_root_bytes: 1048576,
            max_capture_bytes: 16384,
            max_context_bytes: 8192,
            max_request_bytes: 16384,
            max_selector_bytes: 128,
            max_label_bytes: 128,
            max_pool_bytes: 1048576,
            source: profile,
        },
        &control(),
        &mut r,
    )
    .unwrap();
    let capture = TrustedAnnotationCaptureFactsV1 {
        scope: &s,
        grants: &[],
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        annotation_readable: Some(true),
        visibility_complete: Some(true),
        projection_order_attested: Some(true),
        resources_ready: Some(true),
    };
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &capture,
            &mut r,
        )
        .unwrap();
    o.configure_annotation_contexts(
        AnnotationContextLimitsV1 {
            max_contexts: 4,
            max_selectors: 32,
            max_holds: 8,
            max_page_rows: 16,
            max_row_bytes: 4096,
            max_selector_bytes: 128,
            max_issued: 64,
            max_rows: 16,
            max_capture_bytes: 16384,
            max_context_bytes: 8192,
            max_request_bytes: 16384,
            max_reply_bytes: 8192,
            max_label_bytes: 128,
            max_pool_bytes: 1048576,
        },
        &control(),
        &mut r,
    )
    .unwrap();
    let f = TrustedPreparedAnnotationFactsV1 {
        projection: &cut,
        scope: &s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        annotation_readable: Some(true),
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    let combined = o
        .install_annotation_context(&cut, &s, &f, &mut r)
        .unwrap()
        .context;
    o.configure_version_set_preparation(
        "engine",
        "local",
        VersionSetPreparationLimitsV1 {
            max_prepared: 4,
            max_issued: 8,
            max_members: 8,
            max_rows: 16,
            max_root_bytes: 1048576,
            max_capture_bytes: 16384,
            max_context_bytes: 8192,
            max_request_bytes: 16384,
            max_selector_bytes: 128,
            max_label_bytes: 128,
            max_pool_bytes: 8388608,
            source: profile,
        },
        &control(),
        &mut r,
    )
    .unwrap();
    o.configure_version_set_contexts(
        VersionSetContextLimitsV1 {
            max_holds: 8,
            max_page_rows: 16,
            max_row_bytes: 4096,
            max_contexts: 4,
            max_members: 8,
            max_selectors: 32,
            max_issued: 64,
            max_rows: 16,
            max_capture_bytes: 16384,
            max_context_bytes: 8192,
            max_request_bytes: 16384,
            max_reply_bytes: 8192,
            max_selector_bytes: 128,
            max_label_bytes: 128,
            max_pool_bytes: 1048576,
        },
        &control(),
        &mut r,
    )
    .unwrap();
    let cut = o
        .prepare_version_set(
            &[VersionSetParticipantV1 {
                scope: &s,
                data: AnnotationProjectionClassesV1::Combined,
            }],
            &[capture],
            &mut r,
        )
        .unwrap();
    let set = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &[member_facts(&s)],
            },
            &mut r,
        )
        .unwrap();
    Fixture {
        o,
        r,
        s,
        version,
        source,
        combined,
        set,
    }
}
fn query(f: &mut Fixture, kind: usize, line: bool) -> Option<SourceContextContentV1> {
    let q = if line {
        SourceContextQueryV1::Line { index: 0 }
    } else {
        SourceContextQueryV1::Range {
            start: BytePosition(0),
            end: BytePosition(4),
        }
    };
    match kind {
        0 => {
            f.o.query_source_read_context(&f.source, &f.s, q, &source_facts(&f.s), &mut f.r)
                .map(|r| r.content)
        }
        1 => {
            f.o.query_annotation_context_source(
                &f.combined,
                &f.s,
                q,
                &TrustedAnnotationContextFactsV1 {
                    context: &f.combined,
                    scope: &f.s,
                    authorized: Some(true),
                    scope_allowed: Some(true),
                    continuous: Some(true),
                    live: Some(true),
                    not_stopped: Some(true),
                    source_readable: Some(true),
                    annotation_readable: Some(true),
                    projection_current: Some(true),
                    retained: Some(true),
                    resources_ready: Some(true),
                    reply_ready: Some(true),
                    transfer_ready: Some(true),
                },
                &mut f.r,
            )
            .map(|r| r.content)
        }
        _ => {
            f.o.query_version_set_source(
                &f.set.participants[0].snapshot,
                q,
                &TrustedVersionSetContextFactsV1 {
                    context: &f.set.context,
                    members: &[member_facts(&f.s)],
                },
                &mut f.r,
            )
            .map(|r| r.content)
        }
    }
}
#[test]
fn all_producers_validate_against_original_roots_not_later_publication() {
    for kind in 0..3 {
        let mut f = owner();
        let p =
            f.o.prepare(
                &[DocumentEdits {
                    expected: f.version.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 4,
                        expected: "body".into(),
                        inserted: "new".into(),
                    }],
                }],
                &[],
                &mut f.r,
            )
            .unwrap();
        f.o.commit(p, &mut f.r).unwrap();
        for line in [false, true] {
            match query(&mut f, kind, line).unwrap() {
                SourceContextContentV1::Range(v) => {
                    assert_eq!(v.source, f.version);
                    assert_eq!(v.bytes(), b"body");
                }
                SourceContextContentV1::Line(v) => {
                    assert_eq!(v.source, f.version);
                    assert_eq!(v.body(), b"body");
                    assert_eq!(v.delimiter(), b"\r\n");
                }
            }
        }
        assert!(!f.o.is_faulted());
        assert_eq!(f.o.read_result_invariant(), None);
    }
}
#[test]
fn malformed_native_products_never_escape_or_change_registry_issuance_root_or_charges() {
    for kind in 0..3 {
        for line in [false, true] {
            let faults = [
                SourceResultFault::Alias,
                SourceResultFault::Reference,
                SourceResultFault::Extent,
                SourceResultFault::Length,
                SourceResultFault::Variant,
                SourceResultFault::Delimiter,
                SourceResultFault::LineIndex,
            ];
            for fault in faults {
                if !line
                    && matches!(
                        fault,
                        SourceResultFault::Delimiter | SourceResultFault::LineIndex
                    )
                {
                    continue;
                }
                let mut f = owner();
                let reserved = f.o.retention_usage().reserved;
                let generation = f.o.generation();
                let issued = (
                    f.o.source_read_context_usage().issued_lifetime,
                    f.o.annotation_context_usage().issued_lifetime,
                    f.o.version_set_context_usage().issued_lifetime,
                );
                f.o.source_read_result_fault = Some(fault);
                assert!(query(&mut f, kind, line).is_none());
                assert!(f.o.is_faulted());
                assert_eq!(
                    f.o.read_result_invariant(),
                    Some(ReadResultInvariantV1::NativeSourceResultInvariant)
                );
                assert_eq!(f.o.retention_usage().reserved, reserved);
                assert_eq!(f.o.generation(), generation);
                assert_eq!(
                    issued,
                    (
                        f.o.source_read_context_usage().issued_lifetime,
                        f.o.annotation_context_usage().issued_lifetime,
                        f.o.version_set_context_usage().issued_lifetime
                    )
                );
                f.o.source_read_result_fault = None;
                assert!(query(&mut f, kind, line).is_none());
                match kind {
                    0 => {
                        assert!(f.o.stop_source_read_context(&f.source, &cleanup(&f.s), &mut f.r));
                        assert!(f.o.retire_source_read_context(
                            &f.source,
                            &cleanup(&f.s),
                            &mut f.r
                        ));
                    }
                    1 => {
                        assert!(f.o.stop_annotation_context(&f.combined, &cleanup(&f.s), &mut f.r));
                        assert!(f.o.retire_annotation_context(
                            &f.combined,
                            &cleanup(&f.s),
                            &mut f.r
                        ));
                    }
                    _ => {
                        assert!(f.o.stop_version_set_context(
                            VersionSetControlTargetV1::Context(&f.set.context),
                            &[cleanup(&f.s)],
                            &mut f.r
                        ));
                        assert!(f.o.retire_version_set_context(
                            VersionSetControlTargetV1::Context(&f.set.context),
                            &[cleanup(&f.s)],
                            &mut f.r
                        ));
                    }
                }
                assert!(f.o.is_faulted());
                assert_eq!(
                    f.o.read_result_invariant().unwrap().code(),
                    "NATIVE_SOURCE_RESULT_INVARIANT"
                );
            }
        }
    }
}
#[test]
fn invariant_capture_is_optional_bounded_redacted_and_never_the_fault_authority() {
    for capacity in [None, Some(64), Some(1)] {
        for kind in 0..3 {
            let mut f = owner();
            f.r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            f.o.source_read_result_fault = Some(SourceResultFault::Reference);
            assert!(query(&mut f, kind, true).is_none());
            assert!(f.o.is_faulted());
            assert!(f.o.read_result_invariant().is_some());
            if capacity == Some(64) {
                let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
                assert_eq!(
                    events
                        .iter()
                        .filter(|e| e.boundary == Boundary::SourceReadResultValidation
                            && e.disposition == Disposition::Unknown)
                        .count(),
                    1
                );
                assert!(
                    events
                        .iter()
                        .all(|e| e.service_time.is_none() && e.boundary != Boundary::Publication)
                );
                assert!(!format!("{events:?}").contains("body"));
            } else if capacity == Some(1) {
                assert!(f.r.dropped() > 0);
            } else {
                assert_eq!(f.r.queued(), 0);
            }
        }
    }
}
#[test]
fn shape_guard_is_not_an_independent_payload_oracle_and_malformed_expected_geometry_does_not_panic()
{
    let mut f = owner();
    let SourceContextContentV1::Range(mut v) = query(&mut f, 0, false).unwrap() else {
        panic!("wrong product")
    };
    v.bytes[0] = b'X';
    let content = SourceContextContentV1::Range(v);
    assert!(valid(
        Resolved::Range(BytePosition(0), BytePosition(4)),
        &f.version,
        &content,
        true
    ));
    assert!(!valid(
        Resolved::Range(BytePosition(4), BytePosition(0)),
        &f.version,
        &content,
        true
    ));
    assert!(!valid(
        Resolved::Range(BytePosition(0), BytePosition(4)),
        &f.version,
        &content,
        false
    ));
}
