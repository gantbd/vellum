use super::*;
#[test]
fn document_read_pins_keep_the_whole_coherent_parent_for_one_member_search() {
    let mut f = owner(8192, 8192, 16384).unwrap();
    let result = query(&mut f).unwrap();
    let a = f.o.document_read_pins_v1(&f.a.document, &mut f.r).unwrap();
    let b = f.o.document_read_pins_v1(&f.b.document, &mut f.r).unwrap();
    assert_eq!(
        (
            a.version_set_contexts,
            a.version_set_holds,
            a.managed_search_results
        ),
        (1, 1, 1)
    );
    assert_eq!(
        (
            b.version_set_contexts,
            b.version_set_holds,
            b.managed_search_results
        ),
        (1, 1, 0)
    );
    let controls = [cleanup(&f.a), cleanup(&f.b)];
    assert!(f.o.stop_version_set_context(
        VersionSetControlTargetV1::Context(&f.acq.context),
        &controls,
        &mut f.r
    ));
    assert!(!f.o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&f.acq.context),
        &controls,
        &mut f.r
    ));
    assert!(f.o.retire_managed_search_result(
        &result,
        SearchResultControlV1::VersionSet(&controls),
        &mut f.r
    ));
    for document in [&f.a.document, &f.b.document] {
        let pins = f.o.document_read_pins_v1(document, &mut f.r).unwrap();
        assert_eq!(
            (
                pins.version_set_contexts,
                pins.version_set_holds,
                pins.managed_search_results
            ),
            (1, 0, 0)
        );
    }
    assert!(f.o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&f.acq.context),
        &controls,
        &mut f.r
    ));
    assert_eq!(
        f.o.document_read_pins_v1(&f.b.document, &mut f.r)
            .unwrap()
            .version_set_contexts,
        0
    );
}
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
fn q() -> SearchRequest {
    SearchRequest {
        query: "a".into(),
        scope: SearchScope::Document,
    }
}
fn qf(q: &SearchRequest) -> TrustedSourceSearchFactsV1<'_> {
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
struct Fixture {
    o: SourceCoordinator,
    r: Recorder,
    a: SourceReadScopeV1,
    b: SourceReadScopeV1,
    acq: VersionSetAcquisitionV1,
}
fn owner(search_request: usize, source_request: usize, group_request: usize) -> Option<Fixture> {
    let (mut o, mut r, _, a, g) = base();
    o.configure_retention_budget(usize::MAX, &mut r).ok()?;
    o.configure_semantic_root_budget(1048576, &mut r).ok()?;
    let doc = o.create(b"peer", &mut r).ok()?;
    let mut b = a.clone();
    b.document = doc.document;
    let mut prep = super::super::super::tests::profile();
    prep.source.max_request_bytes = source_request;
    o.configure_version_set_preparation("engine", "local", prep, &control(), &mut r)
        .ok()?;
    let mut p = super::super::tests::profile();
    p.max_request_bytes = group_request;
    o.configure_version_set_contexts(p, &control(), &mut r)
        .ok()?;
    let cut = o.prepare_version_set(
        &[
            VersionSetParticipantV1 {
                scope: &a,
                data: AnnotationProjectionClassesV1::Combined,
            },
            VersionSetParticipantV1 {
                scope: &b,
                data: AnnotationProjectionClassesV1::Source,
            },
        ],
        &[facts(&a, &g), facts(&b, &[])],
        &mut r,
    )?;
    let acq = o.install_version_set(
        &cut,
        &TrustedPreparedVersionSetFactsV1 {
            projection: &cut,
            members: &[current(&a), current(&b)],
        },
        &mut r,
    )?;
    o.configure_source_search(
        SourceSearchLimitsV1 {
            max_results: 4,
            max_issued: 8,
            max_request_bytes: search_request,
            max_capture_bytes: 8192,
            max_reply_bytes: 4096,
            max_pool_bytes: 1048576,
            max_query_bytes: 128,
            max_scope_ranges: 8,
            max_comparisons: 4096,
            max_matches: 32,
            max_replacement_bytes: 4096,
        },
        &control(),
        &mut r,
    )
    .ok()?;
    Some(Fixture { o, r, a, b, acq })
}
fn query(f: &mut Fixture) -> Option<SourceSearchResultId> {
    let parent = TrustedVersionSetContextFactsV1 {
        context: &f.acq.context,
        members: &[current(&f.a), current(&f.b)],
    };
    let sf = TrustedVersionSetSearchFactsV1 {
        member: &f.acq.participants[0].snapshot,
        scope: &f.a,
        parent: &parent,
    };
    let q = q();
    f.o.query_managed_search(
        SearchReadContextV1::VersionSet(sf.member),
        &f.a,
        &q,
        SearchReadFactsV1::VersionSet(&sf),
        &qf(&q),
        &mut f.r,
    )
}
#[test]
fn full_peer_frames_and_selected_source_requests_have_independent_exact_bounds() {
    for coordinate in 0..3 {
        let admitted = |n| {
            let mut p = [16384; 3];
            p[coordinate] = n;
            owner(p[0], p[1], p[2]).is_some_and(|mut f| query(&mut f).is_some())
        };
        assert!(admitted(16384));
        let mut lo = 0usize;
        let mut hi = 16384usize;
        while lo < hi {
            let n = lo + (hi - lo) / 2;
            if admitted(n) { hi = n } else { lo = n + 1 }
        }
        for n in [lo - 1, lo, lo + 1] {
            let mut p = [16384; 3];
            p[coordinate] = n;
            let mut f = owner(p[0], p[1], p[2]).unwrap();
            assert_eq!(query(&mut f).is_some(), n >= lo);
            assert_eq!(f.o.version_set_context_usage().holds, usize::from(n >= lo));
            assert_eq!(
                f.o.source_search_usage().issued_lifetime,
                usize::from(n >= lo)
            );
        }
    }
}
#[test]
fn unpublished_hold_cleanup_spends_issuance_but_never_creates_a_public_result_or_refunds_funding() {
    let mut f = owner(16384, 16384, 16384).unwrap();
    let reserved = f.o.retention_usage().reserved;
    let generation = f.o.generation();
    let parent = TrustedVersionSetContextFactsV1 {
        context: &f.acq.context,
        members: &[current(&f.a), current(&f.b)],
    };
    let hold =
        f.o.hold_version_search(&f.acq.participants[0].snapshot, &f.a, &parent, &mut f.r)
            .unwrap();
    assert_eq!(f.o.version_set_context_usage().holds, 1);
    f.o.discard_unpublished_version_search_hold(&hold).unwrap();
    assert!(f.o.discard_unpublished_version_search_hold(&hold).is_err());
    assert_eq!(f.o.version_set_context_usage().holds, 0);
    assert_eq!(f.o.version_set_context_usage().issued_lifetime, 4);
    assert_eq!(f.o.source_search_usage().results, 0);
    assert_eq!(f.o.retention_usage().reserved, reserved);
    assert_eq!(f.o.generation(), generation);
}
#[test]
fn actual_hold_count_drift_faults_without_discharging_any_result_or_charge() {
    let mut f = owner(16384, 16384, 16384).unwrap();
    let result = query(&mut f).unwrap();
    let reserved = f.o.retention_usage().reserved;
    let VersionSetContextState::Active(p) = &mut f.o.version_set_contexts else {
        unreachable!()
    };
    p.rows[f.acq.context.slot].as_mut().unwrap().holds = 0;
    assert!(!f.o.retire_managed_search_result(
        &result,
        SearchResultControlV1::VersionSet(&[cleanup(&f.a), cleanup(&f.b)]),
        &mut f.r
    ));
    assert!(f.o.is_faulted());
    assert_eq!(f.o.version_set_context_usage().holds, 1);
    assert_eq!(f.o.source_search_usage().results, 1);
    assert_eq!(f.o.retention_usage().reserved, reserved);
}
#[test]
fn search_first_observes_actual_peer_disposal_or_fault_and_protected_retirement_survives() {
    for fault in [false, true] {
        let mut f = owner(16384, 16384, 16384).unwrap();
        let result = query(&mut f).unwrap();
        if fault {
            f.o.faulted = true;
        } else {
            f.o.dispose(&f.b.document, &mut f.r).unwrap();
        }
        let parent = TrustedVersionSetContextFactsV1 {
            context: &f.acq.context,
            members: &[current(&f.a), current(&f.b)],
        };
        let sf = TrustedVersionSetSearchFactsV1 {
            member: &f.acq.participants[0].snapshot,
            scope: &f.a,
            parent: &parent,
        };
        let q = q();
        assert!(
            f.o.release_managed_search(
                &result,
                &f.a,
                SearchReadFactsV1::VersionSet(&sf),
                &qf(&q),
                &mut f.r
            )
            .is_none()
        );
        assert_eq!(f.o.version_set_context_usage().stopped, 1);
        assert!(!f.o.retire_version_set_context(
            VersionSetControlTargetV1::Context(&f.acq.context),
            &[cleanup(&f.a), cleanup(&f.b)],
            &mut f.r
        ));
        assert!(f.o.retire_managed_search_result(
            &result,
            SearchResultControlV1::VersionSet(&[cleanup(&f.a), cleanup(&f.b)]),
            &mut f.r
        ));
        assert!(f.o.retire_version_set_context(
            VersionSetControlTargetV1::Context(&f.acq.context),
            &[cleanup(&f.a), cleanup(&f.b)],
            &mut f.r
        ));
        assert_eq!(f.o.is_faulted(), fault);
    }
}
