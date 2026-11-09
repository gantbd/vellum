use super::*;
use crate::coordinates::BytePosition;
use crate::coordinator::annotation::projection::tests::{base, cleanup, control, facts};
fn current(s: &SourceReadScopeV1) -> TrustedVersionSetMemberFactsV1<'_> {
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
fn limits() -> VersionSetContextLimitsV1 {
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
    }
}
fn owner(
    outer: VersionSetContextLimitsV1,
    source: SourceProjectionLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    SourceReadScopeV1,
    VersionSetAcquisitionV1,
) {
    owner_text(outer, source, &"x".repeat(512))
}
fn owner_text(
    outer: VersionSetContextLimitsV1,
    source: SourceProjectionLimitsV1,
    text: &str,
) -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    SourceReadScopeV1,
    VersionSetAcquisitionV1,
) {
    let (mut o, mut r, d, a, g) = base();
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d,
                edits: vec![crate::source::Replacement {
                    start: 0,
                    end: 2,
                    expected: "ab".into(),
                    inserted: text.into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    let doc = o.create(b"peer", &mut r).unwrap();
    let mut b = a.clone();
    b.document = doc.document;
    o.configure_retention_budget(usize::MAX, &mut r).unwrap();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    let mut prep = super::super::super::tests::profile();
    prep.source = source;
    o.configure_version_set_preparation("engine", "local", prep, &control(), &mut r)
        .unwrap();
    o.configure_version_set_contexts(outer, &control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Combined,
                },
                VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Annotations,
                },
            ],
            &[facts(&a, &g), facts(&b, &[])],
            &mut r,
        )
        .unwrap();
    let acq = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &[current(&a), current(&b)],
            },
            &mut r,
        )
        .unwrap();
    (o, r, a, b, acq)
}
fn source_limits() -> SourceProjectionLimitsV1 {
    SourceProjectionLimitsV1 {
        max_source_bytes: 4096,
        max_request_bytes: 4096,
        max_reply_bytes: 4096,
        max_scan_bytes: 4096,
    }
}
#[test]
fn independent_complete_inner_and_outer_request_reply_frames_have_exact_native_boundaries() {
    let (o, _, a, b, acq) = owner(limits(), source_limits());
    let f = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &[current(&a), current(&b)],
    };
    let (row, _) = o
        .version_read_member(&acq.participants[0].snapshot, &f)
        .unwrap();
    let extra =
        std::mem::size_of::<VersionSetMemberId>() + std::mem::size_of::<SourceContextQueryV1>();
    let inner = scope_bytes(&a).unwrap() + extra;
    let outer = row.value.context_facts_request_bytes().unwrap() + extra;
    let inner_reply = std::mem::size_of::<SourceContextContentV1>() + 512;
    let outer_reply = std::mem::size_of::<VersionSetSourceReplyV1>() + 512;
    for (coordinate, bound) in [inner, outer, inner_reply, outer_reply]
        .into_iter()
        .enumerate()
    {
        for delta in [-1isize, 0, 1] {
            let mut p = limits();
            let mut s = source_limits();
            let n = bound.checked_add_signed(delta).unwrap();
            match coordinate {
                0 => s.max_request_bytes = n,
                1 => p.max_request_bytes = n,
                2 => s.max_reply_bytes = n,
                _ => p.max_reply_bytes = n,
            }
            let (mut o, mut r, a, b, acq) = owner(p, s);
            let issued = o.version_set_context_usage().issued_lifetime;
            let reserved = o.retention_usage().reserved;
            assert_eq!(
                o.query_version_set_source(
                    &acq.participants[0].snapshot,
                    SourceContextQueryV1::Range {
                        start: BytePosition(0),
                        end: BytePosition(512)
                    },
                    &TrustedVersionSetContextFactsV1 {
                        context: &acq.context,
                        members: &[current(&a), current(&b)]
                    },
                    &mut r
                )
                .is_some(),
                delta >= 0,
                "coordinate {coordinate} delta {delta}"
            );
            assert_eq!(o.version_set_context_usage().stopped, 0);
            assert_eq!(o.version_set_context_usage().issued_lifetime, issued);
            assert_eq!(o.retention_usage().reserved, reserved);
        }
    }
}
#[test]
fn tighter_delivery_scan_or_reply_refusal_cannot_hide_peer_loss() {
    for scan in [false, true] {
        let mut profile = source_limits();
        if scan {
            profile.max_scan_bytes = 0;
        } else {
            profile.max_reply_bytes = 0;
        }
        let (mut o, mut r, a, b, acq) = owner(limits(), profile);
        let q = SourceContextQueryV1::Line { index: 0 };
        assert!(
            o.query_version_set_source(
                &acq.participants[0].snapshot,
                q,
                &TrustedVersionSetContextFactsV1 {
                    context: &acq.context,
                    members: &[current(&a), current(&b)]
                },
                &mut r
            )
            .is_none()
        );
        assert_eq!(o.version_set_context_usage().stopped, 0);
        let mut peer = current(&b);
        peer.annotation_readable = Some(false);
        assert!(
            o.query_version_set_source(
                &acq.participants[0].snapshot,
                q,
                &TrustedVersionSetContextFactsV1 {
                    context: &acq.context,
                    members: &[current(&a), peer]
                },
                &mut r
            )
            .is_none()
        );
        assert_eq!(o.version_set_context_usage().stopped, 1);
    }
}
#[test]
fn source_handoff_first_observes_actual_peer_disposal_or_owner_fault_and_keeps_protected_cleanup() {
    for fault in [false, true] {
        let (mut o, mut r, a, b, acq) = owner(limits(), source_limits());
        if fault {
            o.faulted = true;
        } else {
            o.dispose(&b.document, &mut r).unwrap();
        }
        assert!(
            o.query_version_set_source(
                &acq.participants[0].snapshot,
                SourceContextQueryV1::Line { index: 0 },
                &TrustedVersionSetContextFactsV1 {
                    context: &acq.context,
                    members: &[current(&a), current(&b)]
                },
                &mut r
            )
            .is_none()
        );
        assert_eq!(o.version_set_context_usage().stopped, 1);
        assert!(o.retire_version_set_context(
            VersionSetControlTargetV1::Context(&acq.context),
            &[cleanup(&a), cleanup(&b)],
            &mut r
        ));
        assert_eq!(o.is_faulted(), fault);
    }
}
#[test]
fn malformed_scalar_and_foreign_member_packets_do_not_observe_loss_or_rebind() {
    let (mut o, mut r, a, b, acq) = owner_text(limits(), source_limits(), "é");
    let (_, _, _, _, foreign) = owner(limits(), source_limits());
    let mut peer = current(&b);
    peer.authorized = Some(false);
    let members = [current(&a), peer];
    let f = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &members,
    };
    assert!(
        o.query_version_set_source(
            &foreign.participants[0].snapshot,
            SourceContextQueryV1::Line { index: 0 },
            &f,
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.version_set_context_usage().stopped, 0);
    assert!(
        o.query_version_set_source(
            &acq.participants[0].snapshot,
            SourceContextQueryV1::Range {
                start: BytePosition(1),
                end: BytePosition(2)
            },
            &f,
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.version_set_context_usage().stopped, 0);
}
