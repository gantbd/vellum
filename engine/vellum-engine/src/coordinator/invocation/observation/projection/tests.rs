use super::*;
fn policy(mask: usize) -> InvocationObservationPolicyV1 {
    InvocationObservationPolicyV1 {
        recipient: "r".into(),
        epoch: 7,
        grant: true,
        publication: mask & 1 != 0,
        control: mask & 2 != 0,
        effect: mask & 4 != 0,
        reasons: mask & 8 != 0,
        details: false,
        recovery: mask & 16 != 0,
    }
}
#[test]
fn all_original_captured_current_disclosure_meets_preserve_the_five_local_terminal_products() {
    use InvocationControlDispositionV1 as C;
    use InvocationPublicationV1 as P;
    let policies: Vec<_> = (0..32).map(policy).collect();
    let cases = [
        (
            InvocationOutcome::ScopedSettingsCommitted(ScopedSettingsUpdateV1 { changed: true }),
            P::Applied,
            C::Done,
            InvocationRecoveryV1::Inspect,
        ),
        (
            InvocationOutcome::ScopedSettingsCommitted(ScopedSettingsUpdateV1 { changed: false }),
            P::Unchanged,
            C::Done,
            InvocationRecoveryV1::Inspect,
        ),
        (
            InvocationOutcome::Rejected(CoordinatorError::Unavailable),
            P::NotApplied,
            C::Rejected,
            InvocationRecoveryV1::Inspect,
        ),
        (
            InvocationOutcome::StoppedBeforePublication,
            P::NotApplied,
            C::Stopped,
            InvocationRecoveryV1::Inspect,
        ),
        (
            InvocationOutcome::Rejected(CoordinatorError::Faulted),
            P::Unknown,
            C::Faulted,
            InvocationRecoveryV1::Reconcile,
        ),
    ];
    for (outcome, publication, control, recovery) in cases {
        for original in 0..32 {
            for captured in 0..32 {
                for current in 0..32 {
                    let meet = original & captured & current;
                    let actual = project(
                        &outcome,
                        &policies[original],
                        &policies[captured],
                        &policies[current],
                    );
                    assert_eq!(
                        actual,
                        InvocationOutcomeProjectionV1 {
                            publication: (meet & 1 != 0).then_some(publication),
                            control: (meet & 2 != 0).then_some(control),
                            effect: (meet & 4 != 0).then_some(InvocationLocalEffectV1::None),
                            reason: InvocationOutcomeReasonV1::Unavailable,
                            recovery: (meet & 16 != 0).then_some(recovery),
                        }
                    );
                }
            }
        }
    }
}
#[test]
fn private_rejection_kind_does_not_create_public_reason_or_recovery_authority() {
    for mask in 0..32 {
        let p = policy(mask);
        let baseline = project(
            &InvocationOutcome::Rejected(CoordinatorError::Unavailable),
            &p,
            &p,
            &p,
        );
        for error in [
            CoordinatorError::Stale,
            CoordinatorError::Capacity,
            CoordinatorError::Limit,
            CoordinatorError::DuplicateParticipant,
            CoordinatorError::VersionExhausted,
        ] {
            assert_eq!(
                project(&InvocationOutcome::Rejected(error), &p, &p, &p),
                baseline
            );
        }
    }
}
