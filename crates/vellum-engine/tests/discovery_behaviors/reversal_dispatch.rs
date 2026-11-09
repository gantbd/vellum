use super::observation_support as observation;
use super::*;
use vellum_engine::source::Replacement;
fn edits(d: DocumentVersion, before: &str, after: &str) -> DocumentEdits {
    DocumentEdits {
        expected: d,
        edits: vec![Replacement {
            start: 0,
            end: before.len(),
            expected: before.into(),
            inserted: after.into(),
        }],
    }
}
#[test]
fn installed_reversal_preserves_read_dependencies_through_final_publication_validation() {
    let (mut o, mut r) = lifecycle::owner();
    let a = o.create(b"A", &mut r).unwrap();
    let dependency = o.create(b"old", &mut r).unwrap();
    let p = o
        .prepare(&[edits(a.clone(), "A", "a")], &[], &mut r)
        .unwrap();
    let target =
        HistoryReversalTargetV1::Group(o.commit(p, &mut r).unwrap().history_group.unwrap());
    let reads = ReadDependencyRequestV3 {
        predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
            ReadDependencyV1::Source(dependency.clone()),
        ))],
        budget: ReadDependencyBudget {
            max_predicates: 16,
            max_work: 256,
        },
    };
    let mut c = lifecycle::scope4().context;
    c.profile = "native-reversal/1".into();
    let id = o.issue_invocation(&mut r).unwrap();
    let profile = observation::Profile {
        epoch: 7,
        maximum: 512,
        capabilities: InvocationReceiptCapabilitiesV1 {
            reversal: true,
            ..Default::default()
        },
    };
    profile.bind(&mut o, &id, &c, &mut r);
    let (dc, e, v) = lifecycle::selectors_for(&mut o, &mut r, 1);
    let p = lifecycle::prepare(&mut o, &mut r);
    let admission = TrustedReversalResultAdmissionV1 {
        context: &c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        context_current: Some(true),
        result_admission_allowed: Some(true),
        max_result_bytes: 512,
        target_grant: None,
    };
    o.submit_discovered_native_reversal_v1(
        NativeDiscoveredCommandRouteV1 {
            context: &dc,
            scope: &lifecycle::scope4(),
            cut: &p,
            entry: &e,
            variant: &v[0].reference,
            facts: &facts(),
        },
        NativeDiscoveredReversalRequestV1 {
            invocation: &id,
            target: &target,
            direction: HistoryDirection::Undo,
            reads: &reads,
            result: &admission,
        },
        &mut r,
    )
    .unwrap();
    let mut observer = profile.open(&mut o, &id, &c, &mut r);
    let change = o
        .prepare(&[edits(dependency, "old", "new")], &[], &mut r)
        .unwrap();
    o.commit(change, &mut r).unwrap();
    let generation = o.generation();
    o.run(&id, &mut r).unwrap();
    assert_eq!(o.generation(), generation);
    assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "a");
    let release = TrustedReversalResultReleaseV1 {
        context: &c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_context_current: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: 512,
        target_grant: None,
    };
    let InvocationReceiptV1::Reversal(out) = profile
        .query(
            &o,
            &mut observer,
            InvocationReceiptRequestV1::Reversal(&release),
            &c,
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(matches!(out.decision, ReversalDecisionV1::NotPublished));
}
#[test]
fn installed_undo_redo_keep_cross_document_atomicity_original_reads_and_historical_f06_results() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for frontier in [false, true] {
        for disclose in [false, true] {
            for capacity in [0, 1, 256] {
                let (mut o, mut r) = lifecycle::owner();
                let a = o.create(b"A", &mut r).unwrap();
                let b = o.create(b"B", &mut r).unwrap();
                let dependency = o.create(b"independent", &mut r).unwrap();
                let p = o
                    .prepare(
                        &[edits(a.clone(), "A", "a"), edits(b.clone(), "B", "b")],
                        &[],
                        &mut r,
                    )
                    .unwrap();
                let group = o.commit(p, &mut r).unwrap().history_group.unwrap();
                let target = if frontier {
                    HistoryReversalTargetV1::SourceFrontier {
                        document: a.document.clone(),
                        expected_group: group,
                    }
                } else {
                    HistoryReversalTargetV1::Group(group)
                };
                let reads = ReadDependencyRequestV3 {
                    predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
                        ReadDependencyV1::Source(dependency.clone()),
                    ))],
                    budget: ReadDependencyBudget {
                        max_predicates: 16,
                        max_work: 256,
                    },
                };
                let mut c = lifecycle::scope4().context;
                c.profile = "native-reversal/1".into();
                for (index, direction) in [HistoryDirection::Undo, HistoryDirection::Redo]
                    .into_iter()
                    .enumerate()
                {
                    let id = o.issue_invocation(&mut r).unwrap();
                    let profile = observation::Profile {
                        epoch: 7,
                        maximum: 512,
                        capabilities: InvocationReceiptCapabilitiesV1 {
                            reversal: true,
                            ..Default::default()
                        },
                    };
                    profile.bind(&mut o, &id, &c, &mut r);
                    let (dc, e, v) = lifecycle::selectors_for(&mut o, &mut r, 1);
                    let p = lifecycle::prepare(&mut o, &mut r);
                    let grant = TrustedReversalResultGrantV1 {
                        invocation: &id,
                        authority: &c.authority,
                        target: &target,
                        disclose_target: disclose,
                    };
                    let admission = TrustedReversalResultAdmissionV1 {
                        context: &c,
                        publication_epoch: 7,
                        publication_allowed: Some(true),
                        context_current: Some(true),
                        result_admission_allowed: Some(true),
                        max_result_bytes: 512,
                        target_grant: Some(grant),
                    };
                    let request = NativeDiscoveredReversalRequestV1 {
                        invocation: &id,
                        target: &target,
                        direction,
                        reads: &reads,
                        result: &admission,
                    };
                    r = Recorder::try_with_capacity(capacity)
                        .unwrap()
                        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                    let mut lost = facts();
                    lost.authorized = Some(false);
                    assert_eq!(
                        o.submit_discovered_native_reversal_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &dc,
                                scope: &lifecycle::scope4(),
                                cut: &p,
                                entry: &e,
                                variant: &v[1 - index].reference,
                                facts: &lost
                            },
                            request,
                            &mut r
                        ),
                        Err(DiscoveredNativeCommandErrorV1::Discovery(
                            DiscoveryErrorV1::Unavailable
                        ))
                    );
                    assert!(!o.discovery_context_usage_v1(&dc).unwrap().stopped);
                    assert_eq!(
                        o.submit_discovered_native_reversal_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &dc,
                                scope: &lifecycle::scope4(),
                                cut: &p,
                                entry: &e,
                                variant: &v[index].reference,
                                facts: &facts()
                            },
                            request,
                            &mut r
                        ),
                        Ok(InvocationState::Pending)
                    );
                    let mut observer = profile.open(&mut o, &id, &c, &mut r);
                    o.run(&id, &mut r).unwrap();
                    for (d, text) in [
                        (&a, if index == 0 { "A" } else { "a" }),
                        (&b, if index == 0 { "B" } else { "b" }),
                    ] {
                        assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), text);
                    }
                    assert_eq!(
                        o.read(&dependency.document, &mut r).unwrap().version,
                        dependency
                    );
                    let release = TrustedReversalResultReleaseV1 {
                        context: &c,
                        publication_epoch: 7,
                        publication_allowed: Some(true),
                        outcome_readable: Some(true),
                        result_context_current: Some(true),
                        reply_ready: Some(true),
                        resources_ready: Some(true),
                        max_result_bytes: 512,
                        target_grant: Some(grant),
                    };
                    for current_disclose in [false, true, true] {
                        let release = TrustedReversalResultReleaseV1 {
                            target_grant: Some(TrustedReversalResultGrantV1 {
                                disclose_target: current_disclose,
                                ..grant
                            }),
                            ..release
                        };
                        let InvocationReceiptV1::Reversal(out) = profile
                            .query(
                                &o,
                                &mut observer,
                                InvocationReceiptRequestV1::Reversal(&release),
                                &c,
                                &mut r,
                            )
                            .unwrap()
                        else {
                            panic!()
                        };
                        assert_eq!(out.invocation, id);
                        assert_eq!(out.operation, direction);
                        let ReversalDecisionV1::Applied {
                            publication,
                            target: actual,
                        } = out.decision
                        else {
                            panic!()
                        };
                        assert_eq!(publication, id);
                        assert_eq!(
                            actual,
                            if disclose && current_disclose {
                                Some(target.clone())
                            } else {
                                None
                            }
                        );
                    }
                    let generation = o.generation();
                    let p = lifecycle::prepare(&mut o, &mut r);
                    assert_eq!(
                        o.submit_discovered_native_reversal_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &dc,
                                scope: &lifecycle::scope4(),
                                cut: &p,
                                entry: &e,
                                variant: &v[index].reference,
                                facts: &facts()
                            },
                            request,
                            &mut r
                        ),
                        Ok(InvocationState::Finished)
                    );
                    assert_eq!(o.generation(), generation);
                    let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                    if capacity == 256 {
                        assert!(
                            records
                                .iter()
                                .any(|v| v.boundary == Boundary::DiscoveryQuery
                                    && v.service_time.is_some())
                        );
                        assert!(
                            records
                                .iter()
                                .any(|v| v.boundary == Boundary::InvocationReceiptRelease
                                    && v.service_time.is_some())
                        );
                    } else {
                        assert!(records.len() <= capacity);
                    }
                }
            }
        }
    }
}
