use super::*;

#[test]
fn exact_reservation_target_and_expansion_limits_never_partially_apply_paste() {
    let f = Fixture::new();
    let bounded = PasteLimitsV1 {
        max_targets: 2,
        max_plain_bytes: 2,
        max_expanded_bytes: 4,
    };
    let mut probe = host(0, false, limits());
    probe.configure_paste_transfers(bounded).unwrap();
    let p = probe.register_copy_provider(registration()).unwrap();
    let (_, c) = context(&mut probe, &f, &[range(0, 2), range(4, 11)]);
    let id = probe.issue_invocation().unwrap();
    let task = probe.admit_paste(&id, &c, &p, admit()).unwrap();
    let exact = probe.copy_usage().unwrap().held_bytes;
    probe.stop_copy(&task).unwrap();
    probe.retire_copy(&task, retirement()).unwrap();
    probe.retire_copy_provider(&p, retirement()).unwrap();
    for (budget, succeeds) in [(exact - 1, false), (exact, true)] {
        let mut bounds = limits();
        bounds.max_held_bytes = budget;
        // Paste captures targets, not selected source fragments.
        bounds.capture.max_logical_bytes = 0;
        bounds.capture.max_plain_bytes = 0;
        bounds.capture.max_fragments = 0;
        let mut host = host(0, false, bounds);
        host.configure_paste_transfers(bounded).unwrap();
        let p = host.register_copy_provider(registration()).unwrap();
        let (_, c) = context(&mut host, &f, &[range(0, 2), range(4, 11)]);
        let id = host.issue_invocation().unwrap();
        let before = host.copy_usage().unwrap().held_bytes;
        let result = host.admit_paste(&id, &c, &p, admit());
        if succeeds {
            let task = result.unwrap();
            assert_eq!(host.copy_usage().unwrap().held_bytes, exact);
            host.stop_copy(&task).unwrap();
            host.retire_copy(&task, retirement()).unwrap();
        } else {
            error(result, CopyErrorV1::Limit);
            assert_eq!(host.invocation_claim_usage().retained, 0);
            assert_eq!(host.invocation_state(&id).unwrap(), InvocationState::Issued);
            host.retire_invocation(&id).unwrap();
        }
        assert_eq!(host.copy_usage().unwrap().held_bytes, before);
        host.retire_copy_provider(&p, retirement()).unwrap();
    }
    for expansion in [3, 4] {
        let mut host = host(0, false, limits());
        host.configure_paste_transfers(PasteLimitsV1 {
            max_expanded_bytes: expansion,
            ..bounded
        })
        .unwrap();
        let p = host.register_copy_provider(registration()).unwrap();
        let (doc, c) = context(&mut host, &f, &[range(0, 2), range(4, 11)]);
        let id = host.issue_invocation().unwrap();
        let task = host.admit_paste(&id, &c, &p, admit()).unwrap();
        let d = host.dispatch_paste(&task, dispatch_facts()).unwrap();
        let receipt =
            host.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("ß"), receipt_facts()); // 2 bytes * 2 targets
        if expansion == 3 {
            error(receipt, CopyErrorV1::Limit);
            assert_eq!(
                host.copy_status(&task).unwrap().effect,
                CopyEffectV1::Unknown
            );
            host.record_paste_evidence(&p, &d, PasteEvidenceV1::NotApplied, receipt_facts())
                .unwrap();
            host.stop_copy(&task).unwrap();
        } else {
            receipt.unwrap();
            assert_eq!(
                host.continue_paste(&task, publish()).unwrap(),
                TransferLocalDispositionV1::Applied
            );
            assert_eq!(host.read(&doc).unwrap().source.text(), "ß\r\nß\nend");
        }
        quiet(&mut host, &p, &d);
        host.retire_copy(&task, retirement()).unwrap();
        host.retire_copy_provider(&p, retirement()).unwrap();
    }
    for mode in ["targets", "descriptor", "native-expanded"] {
        let mut cfg = config(0, false);
        if mode == "descriptor" {
            cfg.engine.invocation.as_mut().unwrap().max_intent_bytes = 0;
        }
        if mode == "native-expanded" {
            cfg.engine.limits.source.max_edits = 1;
        }
        let mut host = LocalHost::new(cfg).unwrap();
        host.configure_copy_transfers(limits()).unwrap();
        host.configure_paste_transfers(PasteLimitsV1 {
            max_targets: if mode == "targets" { 1 } else { 2 },
            ..bounded
        })
        .unwrap();
        let p = host.register_copy_provider(registration()).unwrap();
        let (doc, c) = context(&mut host, &f, &[range(0, 2), range(4, 11)]);
        let id = host.issue_invocation().unwrap();
        let result = host.admit_paste(&id, &c, &p, admit());
        if mode == "native-expanded" {
            let task = result.unwrap();
            let d = receive(&mut host, &p, &task, "ß");
            assert_eq!(
                host.continue_paste(&task, publish()).unwrap(),
                TransferLocalDispositionV1::NotApplied
            );
            assert!(!host.copy_usage().unwrap().paste_fenced);
            quiet(&mut host, &p, &d);
            host.retire_copy(&task, retirement()).unwrap();
        } else {
            error(result, CopyErrorV1::Limit);
            assert_eq!(host.invocation_claim_usage().retained, 0);
            host.retire_invocation(&id).unwrap();
        }
        assert_eq!(host.read(&doc).unwrap().version, c.source);
        assert_eq!(host.history_usage().retained_groups, 0);
        host.retire_copy_provider(&p, retirement()).unwrap();
    }
}

#[test]
fn capabilities_foreign_handles_and_late_stopped_results_preserve_original_obligations() {
    let f = Fixture::new();
    let mut a = host(1, true, limits());
    let mut b = paste_host(0, false);
    let p = a.register_copy_provider(registration()).unwrap();
    let foreign = b.register_copy_provider(registration()).unwrap();
    let (doc, c) = context(&mut a, &f, &[range(0, 2)]);
    let id = a.issue_invocation().unwrap();
    error(
        a.admit_paste(&id, &c, &p, admit()),
        CopyErrorV1::Unavailable,
    );
    a.configure_paste_transfers(paste_limits()).unwrap();
    assert!(a.configure_paste_transfers(paste_limits()).is_err());
    let mut r = registration();
    r.readable = false;
    let write_only = a.register_copy_provider(r).unwrap();
    error(
        a.admit_paste(&id, &c, &write_only, admit()),
        CopyErrorV1::Unavailable,
    );
    error(
        a.admit_paste(&id, &c, &p, PasteAdmissionFactsV1::default()),
        CopyErrorV1::Denied,
    );
    assert!(a.admit_paste(&id, &c, &foreign, admit()).is_err());
    assert!(b.admit_paste(&id, &c, &foreign, admit()).is_err());
    let task = a.admit_paste(&id, &c, &p, admit()).unwrap();
    error(
        a.dispatch_paste(&task, PasteDispatchFactsV1::default()),
        CopyErrorV1::Denied,
    );
    let d = a.dispatch_paste(&task, dispatch_facts()).unwrap();
    assert!(
        b.record_paste_evidence(
            &foreign,
            &d,
            PasteEvidenceV1::Accepted("forged"),
            receipt_facts()
        )
        .is_err()
    );
    assert!(b.paste_report(&task).is_err());
    assert!(b.stop_copy(&task).is_err());
    a.stop_copy(&task).unwrap();
    a.revoke_copy_provider(&p).unwrap();
    quiet(&mut a, &p, &d);
    assert_eq!(
        a.paste_report(&task).unwrap().local,
        TransferLocalDispositionV1::Stopped
    );
    assert_eq!(a.copy_usage().unwrap().unknown_tasks, 1);
    assert_eq!(a.copy_usage().unwrap().provider_work, 0);
    assert!(a.copy_usage().unwrap().oldest_unknown.is_some());
    error(a.retire_copy(&task, retirement()), CopyErrorV1::Outstanding);
    let mut a = match a.into_parts() {
        Err(host) => host,
        Ok(_) => panic!("lost unknown paste"),
    };
    a.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("late"), receipt_facts())
        .unwrap();
    assert_eq!(
        a.continue_paste(&task, publish()).unwrap(),
        TransferLocalDispositionV1::Stopped
    );
    assert_eq!(a.read(&doc).unwrap().version, c.source);
    assert_eq!(a.invocation_state(&id).unwrap(), InvocationState::Issued);
    assert!(a.invocation_outcome(&id).is_err());
    a.retire_copy(&task, retirement()).unwrap();
    a.retire_copy_provider(&p, retirement()).unwrap();
    a.retire_copy_provider(&write_only, retirement()).unwrap();
    b.retire_copy_provider(&foreign, retirement()).unwrap();
    assert_eq!(
        format!("{:?}", PasteEvidenceV1::Accepted("private clipboard text")),
        "PasteEvidenceV1(<redacted>)"
    );
    assert!(a.into_parts().is_ok());
    assert!(b.into_parts().is_ok());
}
