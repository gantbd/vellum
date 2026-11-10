use super::*;
use crate::coordinator::source_query::result_contract::tests::{owner, source_facts};
#[test]
fn shutdown_inventory_rejects_two_results_claiming_one_original_hold() {
    use crate::coordinator::shutdown::tests::{inspect, inspection_current};
    for family in 0..3 {
        let mut f = owner();
        configure_search(&mut f);
        for _ in 0..2 {
            query_family(&mut f, family);
        }
        let cut = inspect(&f.o, &mut f.r);
        let before = (
            f.o.source_search_usage(),
            f.o.source_read_context_usage(),
            f.o.retention_usage(),
        );
        let State::Active(p) = &mut f.o.managed_search else {
            panic!()
        };
        let first = p.rows[0].as_ref().unwrap().hold.clone();
        let original = std::mem::replace(&mut p.rows[1].as_mut().unwrap().hold, first);
        assert_eq!(
            inspection_current(&f.o, &cut, &mut f.r),
            Err(CoordinatorError::Faulted)
        );
        assert_eq!(
            (
                f.o.source_search_usage(),
                f.o.source_read_context_usage(),
                f.o.retention_usage()
            ),
            before
        );
        let State::Active(p) = &mut f.o.managed_search else {
            panic!()
        };
        p.rows[1].as_mut().unwrap().hold = original;
        assert_eq!(inspection_current(&f.o, &cut, &mut f.r), Ok(()));
    }
}
#[test]
fn shutdown_inventory_keeps_independent_issuer_slot_spaces_and_standalone_holds() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    let mut f = owner();
    configure_search(&mut f);
    let standalone =
        f.o.hold_source_read_context(
            &f.source,
            &f.s,
            &SourceReadHoldFactsV1 {
                owner_control_allowed: Some(true),
                retained: Some(true),
                resources_ready: Some(true),
            },
            &mut f.r,
        )
        .unwrap();
    for family in 0..3 {
        query_family(&mut f, family);
    }
    let before = f.o.retention_usage();
    let cut = inspect(&f.o, &mut f.r);
    let view = inspect_read(&f.o, &cut, &mut f.r);
    assert_eq!(
        view.owners
            .iter()
            .find(|row| row.owner.role() == ShutdownOwnerRoleV1::ManagedSearch)
            .unwrap()
            .retained_records,
        Some(3)
    );
    assert_eq!(f.o.retention_usage(), before);
    assert!(f.o.release_source_read_hold(
        &standalone,
        &crate::coordinator::source_query::result_contract::tests::cleanup(&f.s),
        &mut f.r
    ));
}
#[test]
fn shutdown_inventory_rejects_a_foreign_coherent_parent_on_an_original_member_hold() {
    use crate::coordinator::shutdown::tests::{inspect, inspection_current};
    let mut f = owner();
    configure_search(&mut f);
    query_family(&mut f, 2);
    let cut = inspect(&f.o, &mut f.r);
    let before = f.o.retention_usage();
    let foreign = owner().set.context;
    let State::Active(p) = &mut f.o.managed_search else {
        panic!()
    };
    let Issuer::VersionSet { parent, .. } = &mut p.rows[0].as_mut().unwrap().source else {
        panic!()
    };
    let original = std::mem::replace(parent, foreign);
    assert_eq!(
        inspection_current(&f.o, &cut, &mut f.r),
        Err(CoordinatorError::Faulted)
    );
    assert_eq!(f.o.retention_usage(), before);
    let State::Active(p) = &mut f.o.managed_search else {
        panic!()
    };
    let Issuer::VersionSet { parent, .. } = &mut p.rows[0].as_mut().unwrap().source else {
        panic!()
    };
    *parent = original;
    assert_eq!(inspection_current(&f.o, &cut, &mut f.r), Ok(()));
}
fn query_family(
    f: &mut crate::coordinator::source_query::result_contract::tests::Fixture,
    family: usize,
) {
    let q = SearchRequest {
        query: "body".into(),
        scope: SearchScope::Document,
    };
    let grant = TrustedSourceSearchFactsV1 {
        request: &q,
        grant: Some(true),
        scope: Some(true),
        continuity: Some(true),
        capacity: Some(true),
        retained: Some(true),
        not_stopped: Some(true),
    };
    let source = source_facts(&f.s);
    let combined = TrustedAnnotationContextFactsV1 {
        context: &f.combined,
        scope: &f.s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        annotation_readable: Some(true),
        source_readable: Some(true),
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    let members = [crate::coordinator::source_query::result_contract::tests::member_facts(&f.s)];
    let parent = TrustedVersionSetContextFactsV1 {
        context: &f.set.context,
        members: &members,
    };
    let version = TrustedVersionSetSearchFactsV1 {
        member: &f.set.participants[0].snapshot,
        scope: &f.s,
        parent: &parent,
    };
    let (issuer, facts) = match family {
        0 => (
            SearchReadContextV1::Source(&f.source),
            SearchReadFactsV1::Source(&source),
        ),
        1 => (
            SearchReadContextV1::Combined(&f.combined),
            SearchReadFactsV1::Combined(&combined),
        ),
        2 => (
            SearchReadContextV1::VersionSet(version.member),
            SearchReadFactsV1::VersionSet(&version),
        ),
        _ => unreachable!(),
    };
    f.o.query_managed_search(issuer, &f.s, &q, facts, &grant, &mut f.r)
        .unwrap();
}
fn configure_search(f: &mut crate::coordinator::source_query::result_contract::tests::Fixture) {
    f.o.configure_source_search(
        SourceSearchLimitsV1 {
            max_results: 4,
            max_issued: 8,
            max_request_bytes: 8192,
            max_capture_bytes: 8192,
            max_reply_bytes: 8192,
            max_pool_bytes: 1048576,
            max_query_bytes: 128,
            max_scope_ranges: 8,
            max_comparisons: 4096,
            max_matches: 32,
            max_replacement_bytes: 4096,
        },
        &SourceReadPoolControlV1 {
            owner_control_allowed: Some(true),
            resources_ready: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
        &mut f.r,
    )
    .unwrap();
}
#[test]
fn document_read_pins_reject_a_plausible_but_incorrect_live_result_gauge() {
    let mut f = owner();
    f.o.configure_source_search(
        SourceSearchLimitsV1 {
            max_results: 4,
            max_issued: 8,
            max_request_bytes: 8192,
            max_capture_bytes: 8192,
            max_reply_bytes: 8192,
            max_pool_bytes: 1048576,
            max_query_bytes: 128,
            max_scope_ranges: 8,
            max_comparisons: 4096,
            max_matches: 32,
            max_replacement_bytes: 4096,
        },
        &SourceReadPoolControlV1 {
            owner_control_allowed: Some(true),
            resources_ready: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
        &mut f.r,
    )
    .unwrap();
    let q = SearchRequest {
        query: "body".into(),
        scope: SearchScope::Document,
    };
    let grant = TrustedSourceSearchFactsV1 {
        request: &q,
        grant: Some(true),
        scope: Some(true),
        continuity: Some(true),
        capacity: Some(true),
        retained: Some(true),
        not_stopped: Some(true),
    };
    f.o.query_source_search(&f.source, &f.s, &q, &source_facts(&f.s), &grant, &mut f.r)
        .unwrap();
    assert_eq!(
        f.o.document_read_pins_v1(&f.s.document, &mut f.r)
            .unwrap()
            .managed_search_results,
        1
    );
    let State::Active(p) = &mut f.o.managed_search else {
        panic!()
    };
    p.live = 0;
    assert_eq!(
        f.o.document_read_pins_v1(&f.s.document, &mut f.r),
        Err(CoordinatorError::Faulted)
    );
}
