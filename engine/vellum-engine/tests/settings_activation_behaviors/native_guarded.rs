use super::scoped_basis::{context, facts, ready};
use super::*;
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    }
}
fn request() -> [SettingChangeV1; 1] {
    [change(
        "left",
        1,
        SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    )]
}
#[test]
fn native_fact_bearing_job_binds_full_context_and_requires_fresh_facts_at_service() {
    let (mut o, mut r) = ready(16, 8);
    let c = context();
    let layers = [id("left")];
    let mut common = facts(&c, &layers);
    // Native identities and generations are supplied explicitly; basis-observation
    // permission is neither needed nor inferred from a layer write grant.
    common.basis_allowed = None;
    let f = TrustedNativeSettingsInvocationV1 {
        common: &common,
        required_reads: &[],
    };
    let job = o.issue_invocation(&mut r).unwrap();
    let request = request();
    let reads = reads();
    o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r)
        .unwrap();
    let before = o.generation();
    assert_eq!(o.run(&job, &mut r), Err(InvocationError::Unavailable));
    let mut foreign = c.clone();
    foreign.recipient = "other".into();
    assert_eq!(
        o.run_native_settings_change_v1(&job, &foreign, &f, &mut r),
        Err(InvocationError::Unavailable)
    );
    common.settings_writable = Some(false);
    let f = TrustedNativeSettingsInvocationV1 {
        common: &common,
        required_reads: &[],
    };
    assert_eq!(
        o.run_native_settings_change_v1(&job, &c, &f, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), before);
    assert_eq!(read(&o, "A", &mut r), 1.0);
}

#[test]
fn native_intent_is_exclusive_and_full_context_replay_preserves_disjoint_progress() {
    let (mut o, mut r) = ready(16, 8);
    let c = context();
    let layers = [id("left")];
    let common = facts(&c, &layers);
    let f = TrustedNativeSettingsInvocationV1 {
        common: &common,
        required_reads: &[],
    };
    let job = o.issue_invocation(&mut r).unwrap();
    let mut request = request();
    let reads = reads();
    let before_retained = o.settings_retention_usage().retained;
    o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.settings_retention_usage().retained, before_retained);
    assert_eq!(
        o.submit_settings_change_with_reads(&job, &request, &reads, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    for field in 0..7 {
        let mut wrong = c.clone();
        match field {
            0 => wrong.authority.push('!'),
            1 => wrong.engine.push('!'),
            2 => wrong.recipient.push('!'),
            3 => wrong.purpose.push('!'),
            4 => wrong.destination.push('!'),
            5 => wrong.profile.push('!'),
            _ => wrong.continuity.push('!'),
        }
        assert_eq!(
            o.submit_native_settings_change_v1(&job, &request, &reads, &wrong, &f, &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.run_native_settings_change_v1(&job, &wrong, &f, &mut r),
            Err(InvocationError::Unavailable)
        );
    }
    request[0].operation = SettingOperationV1::Set(SettingValueV1::Number(3.0));
    assert_eq!(
        o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    request[0].operation = SettingOperationV1::Set(SettingValueV1::Number(2.0));
    set(&mut o, "right", 4.0, &mut r);
    let source = o.create(b"source progress", &mut r).unwrap();
    o.run_native_settings_change_v1(&job, &c, &f, &mut r)
        .unwrap();
    assert_eq!(read(&o, "A", &mut r), 2.0);
    assert_eq!(read(&o, "B", &mut r), 4.0);
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "source progress"
    );
    let generation = o.generation();
    assert_eq!(
        o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.run_native_settings_change_v1(&job, &c, &f, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
    let raw = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_change_with_reads(&raw, &[], &reads, &mut r)
        .unwrap();
    assert_eq!(
        o.submit_native_settings_change_v1(&raw, &[], &reads, &c, &f, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run(&raw, &mut r).unwrap();
}

#[test]
fn native_required_reads_cannot_be_dropped_or_replaced_and_capture_is_behavior_independent() {
    use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};
    for capacity in [0, 1, 128] {
        for withdraw in [false, true] {
            let (mut o, _) = ready(16, 8);
            let mut r = if capacity == 0 {
                Recorder::disabled()
            } else {
                Recorder::try_with_capacity(capacity).unwrap()
            }
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            let c = context();
            let layers = [id("left")];
            let common = facts(&c, &layers);
            let source = o.create(b"A", &mut r).unwrap();
            let required = [ReadDependencyV3::Base(ReadDependencyV2::Base(
                ReadDependencyV1::Source(source),
            ))];
            let f = TrustedNativeSettingsInvocationV1 {
                common: &common,
                required_reads: &required,
            };
            let job = o.issue_invocation(&mut r).unwrap();
            let request = request();
            let reads = reads();
            o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r)
                .unwrap();
            let f = TrustedNativeSettingsInvocationV1 {
                common: &common,
                required_reads: if withdraw { &[] } else { &required },
            };
            let before = o.generation();
            o.run_native_settings_change_v1(&job, &c, &f, &mut r)
                .unwrap();
            assert_eq!(read(&o, "A", &mut r), if withdraw { 1.0 } else { 2.0 });
            if withdraw {
                assert_eq!(o.generation(), before);
            }
            let generation = o.generation();
            assert_eq!(
                o.run_native_settings_change_v1(&job, &c, &f, &mut r),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
            let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            if capacity == 0 {
                assert!(records.is_empty());
            }
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
            if capacity == 128 {
                assert!(
                    records
                        .iter()
                        .any(|v| v.boundary == Boundary::NativeSettingsAdmission
                            && v.service_time.is_some()
                            && v.disposition == Disposition::Completed)
                );
                let disposition = if withdraw {
                    Disposition::Refused
                } else {
                    Disposition::Completed
                };
                for boundary in [
                    Boundary::NativeSettingsValidation,
                    Boundary::NativeSettingsPublication,
                ] {
                    assert!(
                        records
                            .iter()
                            .any(|v| v.boundary == boundary && v.disposition == disposition)
                    );
                }
            }
        }
    }
}

#[test]
fn native_context_stop_uses_protected_control_without_edit_rights_and_never_rearms() {
    let (mut o, mut r) = ready(16, 8);
    let c = context();
    let layers = [id("left")];
    let mut common = facts(&c, &layers);
    let f = TrustedNativeSettingsInvocationV1 {
        common: &common,
        required_reads: &[],
    };
    let job = o.issue_invocation(&mut r).unwrap();
    let request = request();
    let reads = reads();
    o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r)
        .unwrap();
    let control = TrustedInvocationControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        control_context_current: Some(true),
        control_scope_valid: Some(true),
        protected_control_ready: Some(true),
    };
    o.request_invocation_stop(&job, &control, &mut r).unwrap();
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Pending)
    );
    common.authorized = Some(false);
    common.settings_writable = Some(false);
    let f = TrustedNativeSettingsInvocationV1 {
        common: &common,
        required_reads: &[],
    };
    o.run_native_settings_change_v1(&job, &c, &f, &mut r)
        .unwrap();
    assert!(matches!(
        o.invocation_outcome(&job, &mut r),
        Ok(InvocationOutcome::StoppedBeforePublication)
    ));
    assert_eq!(read(&o, "A", &mut r), 1.0);
    assert_eq!(
        o.submit_native_settings_change_v1(&job, &request, &reads, &c, &f, &mut r),
        Ok(InvocationState::Finished)
    );
    o.forget_invocation_outcome(&job, &mut r).unwrap();
    o.retire_invocation(&job, &mut r).unwrap();
    assert!(
        o.run_native_settings_change_v1(&job, &c, &f, &mut r)
            .is_err()
    );
}

#[test]
fn normalized_native_envelope_enters_the_guarded_job_not_the_legacy_trusted_runner() {
    use NativeSettingsEnvelopeValueV1 as V;
    use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
    use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};
    for capacity in [0, 1, 128] {
        let (mut o, _) = ready(16, 8);
        let mut r = if capacity == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(capacity).unwrap()
        }
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let c = context();
        let layers = [id("left")];
        let common = facts(&c, &layers);
        let f = TrustedNativeSettingsInvocationV1 {
            common: &common,
            required_reads: &[],
        };
        let job = o.issue_invocation(&mut r).unwrap();
        let reads = reads();
        let layer = [
            ("authority", D::Text("engine")),
            ("id", D::Text("left")),
            ("incarnation", D::Number(1.0)),
        ];
        let ch = [
            ("layer", D::Object(&layer)),
            ("expectedGeneration", D::Number(1.0)),
            ("key", D::Text("unit")),
            ("operation", D::Text("set")),
            ("value", D::Number(2.0)),
        ];
        let changes = [D::Object(&ch)];
        let rows = [
            (
                "contract",
                V::Data(D::Text("semantic-settings-activation/1")),
            ),
            ("operation", V::Data(D::Text("semantic-settings"))),
            ("invocation", V::Invocation(&job)),
            ("profile", V::Data(D::Text(&c.profile))),
            ("changes", V::Data(D::Array(&changes))),
            ("dependencies", V::Dependencies(&reads)),
        ];
        assert_eq!(
            normalize_native_settings_envelope_v1(
                &V::Data(D::Null),
                profile(),
                CaptureLimitsV1 {
                    max_bytes: 65536,
                    max_nodes: 128,
                    max_depth: 16
                },
                128,
                65536,
                &mut r
            )
            .unwrap_err(),
            InvocationError::MalformedIntent
        );
        let q = normalize_native_settings_envelope_v1(
            &V::Object(&rows),
            profile(),
            CaptureLimitsV1 {
                max_bytes: 65536,
                max_nodes: 128,
                max_depth: 16,
            },
            128,
            65536,
            &mut r,
        )
        .unwrap();
        assert!(std::ptr::eq(q.invocation(), &job));
        assert!(std::ptr::eq(q.dependencies(), &reads));
        let mut wrong = c.clone();
        wrong.profile.push('!');
        assert_eq!(
            o.submit_normalized_native_settings_v1(&q, &wrong, &f, &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.invocation_state(&job, &mut r),
            Ok(InvocationState::Issued)
        );
        o.submit_normalized_native_settings_v1(&q, &c, &f, &mut r)
            .unwrap();
        assert_eq!(o.run(&job, &mut r), Err(InvocationError::Unavailable));
        o.run_native_settings_change_v1(&job, &c, &f, &mut r)
            .unwrap();
        assert_eq!(read(&o, "A", &mut r), 2.0);
        let generation = o.generation();
        assert_eq!(
            o.submit_normalized_native_settings_v1(&q, &c, &f, &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.generation(), generation);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == 0 {
            assert!(records.is_empty());
        }
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
        if capacity == 128 {
            for (boundary, expected) in [
                (
                    Boundary::NativeSettingsArgumentNormalization,
                    vec![Disposition::Refused, Disposition::Completed],
                ),
                (
                    Boundary::NativeSettingsDecodedAdmission,
                    vec![
                        Disposition::Refused,
                        Disposition::Completed,
                        Disposition::Completed,
                    ],
                ),
            ] {
                let selected: Vec<_> = records.iter().filter(|v| v.boundary == boundary).collect();
                assert_eq!(
                    selected.iter().map(|v| v.disposition).collect::<Vec<_>>(),
                    expected
                );
                assert!(selected.iter().all(|v| v.service_time.is_some()));
            }
        }
    }
}
