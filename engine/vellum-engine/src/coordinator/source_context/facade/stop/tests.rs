use super::*;
use crate::coordinator::source_query::result_contract::tests::{
    Fixture, cleanup, member_facts, owner, source_facts,
};
fn invoke(
    f: &mut Fixture,
    kind: usize,
    allowed: Option<bool>,
    scope: Option<&SourceReadScopeV1>,
) -> Result<(), Error> {
    let control = SourceReadContextControlV1 {
        scope: scope.unwrap_or(&f.s),
        owner_control_allowed: allowed,
        retention_released: None,
        delivery_drained: Some(false),
    };
    let controls = [control];
    let port = match kind {
        0 => ReadObservationStopPortV1::Source {
            context: &f.source,
            control: &controls[0],
        },
        1 => ReadObservationStopPortV1::Annotation {
            context: &f.combined,
            control: &controls[0],
        },
        2 => ReadObservationStopPortV1::VersionSet {
            target: VersionSetControlTargetV1::Context(&f.set.context),
            controls: &controls,
        },
        _ => ReadObservationStopPortV1::VersionSet {
            target: VersionSetControlTargetV1::Member(&f.set.participants[0].snapshot),
            controls: &controls,
        },
    };
    f.o.stop_read_observation_facade_v1(port, &mut f.r)
}
fn available(f: &mut Fixture) -> [bool; 3] {
    let sf = source_facts(&f.s);
    let source =
        f.o.query_source_read_context(
            &f.source,
            &f.s,
            SourceContextQueryV1::Line { index: 0 },
            &sf,
            &mut f.r,
        )
        .is_some();
    let cf = TrustedAnnotationContextFactsV1 {
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
    let combined =
        f.o.query_annotation_context_source(
            &f.combined,
            &f.s,
            SourceContextQueryV1::Line { index: 0 },
            &cf,
            &mut f.r,
        )
        .is_some();
    let vf = TrustedVersionSetContextFactsV1 {
        context: &f.set.context,
        members: &[member_facts(&f.s)],
    };
    let set =
        f.o.query_version_set_source(
            &f.set.participants[0].snapshot,
            SourceContextQueryV1::Line { index: 0 },
            &vf,
            &mut f.r,
        )
        .is_some();
    [source, combined, set]
}
fn retire(f: &mut Fixture, kind: usize) -> bool {
    match kind {
        0 => {
            f.o.retire_source_read_context(&f.source, &cleanup(&f.s), &mut f.r)
        }
        1 => {
            f.o.retire_annotation_context(&f.combined, &cleanup(&f.s), &mut f.r)
        }
        _ => f.o.retire_version_set_context(
            VersionSetControlTargetV1::Context(&f.set.context),
            &[cleanup(&f.s)],
            &mut f.r,
        ),
    }
}
#[test]
fn native_stop_facade_is_original_target_local_idempotent_and_does_not_retire() {
    for kind in 0..4 {
        let mut f = owner();
        let generation = f.o.generation();
        let base = f.o.retention_usage().reserved;
        let issued = [
            f.o.source_read_context_usage().issued_lifetime,
            f.o.annotation_context_usage().issued_lifetime,
            f.o.version_set_context_usage().issued_lifetime,
        ];
        assert_eq!(available(&mut f), [true; 3]);
        for _ in 0..2 {
            assert_eq!(invoke(&mut f, kind, Some(true), None), Ok(()));
        }
        let mut expected = [true; 3];
        expected[kind.min(2)] = false;
        assert_eq!(available(&mut f), expected);
        assert_eq!(f.o.retention_usage().reserved, base);
        assert_eq!(f.o.generation(), generation);
        assert_eq!(
            [
                f.o.source_read_context_usage().issued_lifetime,
                f.o.annotation_context_usage().issued_lifetime,
                f.o.version_set_context_usage().issued_lifetime
            ],
            issued
        );
        assert!(retire(&mut f, kind));
        assert_eq!(
            invoke(&mut f, kind, Some(true), None),
            Err(Error::Unavailable)
        );
        assert!(!f.o.is_faulted());
    }
}
#[test]
fn native_stop_facade_rejects_unknown_owner_control_or_wrong_scope_without_closing_other_contexts()
{
    for kind in 0..4 {
        let mut f = owner();
        for allowed in [None, Some(false)] {
            assert_eq!(invoke(&mut f, kind, allowed, None), Err(Error::Unavailable));
        }
        let mut wrong = f.s.clone();
        wrong.context.recipient.push('x');
        assert_eq!(
            invoke(&mut f, kind, Some(true), Some(&wrong)),
            Err(Error::Unavailable)
        );
        assert_eq!(available(&mut f), [true; 3]);
        assert_eq!(invoke(&mut f, kind, Some(true), None), Ok(()));
        assert!(!f.o.is_faulted());
    }
}
#[test]
fn native_stop_facade_preserves_existing_faults_and_unrelated_workspace_debt_without_ordinary_headroom()
 {
    for kind in 0..4 {
        for faulted in [false, true] {
            let mut f = owner();
            f.o.faulted = faulted;
            f.o.source_facade_workspace = 7;
            let remaining = f.o.retention_usage().maximum.unwrap() - f.o.retention_usage().reserved;
            let mut pressure = f.o.retention.reserve(remaining).unwrap();
            let base = f.o.retention_usage().reserved;
            assert_eq!(invoke(&mut f, kind, Some(true), None), Ok(()));
            assert_eq!(f.o.is_faulted(), faulted);
            assert_eq!(f.o.read_facade_usage().workspace_bytes, 7);
            assert_eq!(f.o.retention_usage().reserved, base);
            // Invalid control under a preexisting fault is still unavailable,
            // not a fault classification guessed from the global health bit.
            assert_eq!(invoke(&mut f, kind, None, None), Err(Error::Unavailable));
            f.o.retention.release(&mut pressure).unwrap();
            assert!(retire(&mut f, kind));
        }
    }
}
#[test]
fn native_stop_facade_foreign_original_targets_never_close_any_local_context() {
    for kind in 0..4 {
        let mut f = owner();
        let other = owner();
        let controls = [cleanup(&f.s)];
        let port = match kind {
            0 => ReadObservationStopPortV1::Source {
                context: &other.source,
                control: &controls[0],
            },
            1 => ReadObservationStopPortV1::Annotation {
                context: &other.combined,
                control: &controls[0],
            },
            2 => ReadObservationStopPortV1::VersionSet {
                target: VersionSetControlTargetV1::Context(&other.set.context),
                controls: &controls,
            },
            _ => ReadObservationStopPortV1::VersionSet {
                target: VersionSetControlTargetV1::Member(&other.set.participants[0].snapshot),
                controls: &controls,
            },
        };
        assert_eq!(format!("{port:?}"), "ReadObservationStopPortV1(<redacted>)");
        assert_eq!(
            f.o.stop_read_observation_facade_v1(port, &mut f.r),
            Err(Error::Unavailable)
        );
        assert_eq!(available(&mut f), [true; 3]);
        assert!(!f.o.is_faulted());
    }
}
#[test]
fn native_stop_facade_unwind_fences_without_rolling_back_a_late_stop_or_refunding_anything() {
    for kind in 0..4 {
        for fault in [Fault::BeforeNative, Fault::AfterNative] {
            let mut f = owner();
            let base = f.o.retention_usage().reserved;
            let generation = f.o.generation();
            f.o.source_facade_fault = Some(fault);
            assert_eq!(invoke(&mut f, kind, Some(true), None), Err(Error::Faulted));
            assert!(f.o.is_faulted());
            let stopped = match kind {
                0 => f.o.source_context_row(&f.source, &f.s).unwrap().closed,
                1 => f.o.annotation_context_usage().stopped == 1,
                _ => f.o.version_set_context_usage().stopped == 1,
            };
            assert_eq!(stopped, fault == Fault::AfterNative);
            assert_eq!(f.o.generation(), generation);
            assert_eq!(f.o.retention_usage().reserved, base);
            f.o.source_facade_fault = None;
            assert_eq!(invoke(&mut f, kind, Some(true), None), Ok(()));
            assert!(retire(&mut f, kind));
            assert!(f.o.is_faulted());
        }
    }
}
#[test]
fn native_stop_facade_observations_are_optional_and_control_is_not_counted_as_a_read_release() {
    use crate::coordinator::release_measurements::tests as metrics;
    for kind in 0..4 {
        for cap in [None, Some(0), Some(1), Some(64)] {
            let mut f = owner();
            metrics::enable(&mut f.o, &mut f.r);
            f.r = cap.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            assert_eq!(invoke(&mut f, kind, Some(true), None), Ok(()));
            let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
            if cap == Some(64) {
                let last = events.last().unwrap();
                assert_eq!(last.boundary, Boundary::ReadObservationStopFacade);
                assert_eq!(last.disposition, Disposition::Completed);
                assert!(events.iter().all(|e| e.service_time.is_none()));
            }
            assert_eq!(metrics::totals(&mut f.o, &mut f.r), [0; 6]);
            assert!(retire(&mut f, kind));
            assert!(!f.o.is_faulted());
        }
    }
}
