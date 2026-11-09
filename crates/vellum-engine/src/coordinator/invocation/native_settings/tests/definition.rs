use super::*;
fn facts<'a>(
    common: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedNativeSettingsDefinitionInvocationV1<'a> {
    TrustedNativeSettingsDefinitionInvocationV1 {
        common,
        profile: fixture::limits(),
        schema_writable: Some(true),
        restrictions_writable: None,
        associations_writable: None,
        required_reads: &[],
    }
}
#[test]
fn definition_whole_parent_bound_and_original_preparation_identity_are_independent() {
    for substitute in [false, true] {
        let (mut o, mut r) = fixture::setup();
        let c = fixture::context();
        let common = fixture::common(&c);
        let f = facts(&common);
        let declaration = fixture::declaration(true);
        let reads = reads();
        let expected = o.settings_basis(&mut r).unwrap().interpretation();
        let input = Input::Definition {
            expected,
            declaration: &declaration,
        };
        let capture = o
            .capture_native_settings(input, &reads, &c, Facts::Definition(&f))
            .unwrap();
        let other = o.prepare_native_settings(capture, input, &mut r).unwrap();
        let bytes = other.bytes().unwrap();
        let q = NativeSettingsDefinitionRequestV1 {
            expected,
            declaration: &declaration,
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        let before = o.generation();
        if !substitute {
            drop(other);
            o.invocations.limits.max_intent_bytes = bytes - 1;
            assert_eq!(
                o.submit_native_settings_definition_v1(&job, &q, &c, &f, &mut r),
                Err(InvocationError::Limit)
            );
            assert_eq!(
                o.invocation_state(&job, &mut r),
                Ok(InvocationState::Issued)
            );
            assert_eq!(o.settings_retention_usage().open_preparations, 0);
            o.invocations.limits.max_intent_bytes = bytes;
            o.submit_native_settings_definition_v1(&job, &q, &c, &f, &mut r)
                .unwrap();
            o.run_native_settings_definition_v1(&job, &c, &f, &mut r)
                .unwrap();
        } else {
            o.submit_native_settings_definition_v1(&job, &q, &c, &f, &mut r)
                .unwrap();
            let Phase::NativeSettingsPending(p) =
                &mut o.invocations.slots[job.slot].as_mut().unwrap().phase
            else {
                panic!("pending")
            };
            p.native = other.native;
            assert_eq!(
                o.run_native_settings_definition_v1(&job, &c, &f, &mut r),
                Err(InvocationError::Faulted)
            );
            assert!(o.is_faulted());
            assert_eq!(o.generation(), before);
        }
    }
}
#[test]
fn native_definition_identity_needs_no_unused_counter_or_unrelated_write_grants() {
    let (mut o, mut r) = fixture::setup();
    let c = fixture::context();
    let common = fixture::common(&c);
    let mut f = facts(&common);
    f.schema_writable = None;
    let declaration = fixture::declaration(false);
    let reads = reads();
    let expected = o.settings_basis(&mut r).unwrap().interpretation();
    o.generation = u64::MAX;
    let q = NativeSettingsDefinitionRequestV1 {
        expected,
        declaration: &declaration,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_native_settings_definition_v1(&job, &q, &c, &f, &mut r)
        .unwrap();
    o.run_native_settings_definition_v1(&job, &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.generation(), u64::MAX);
    assert!(
        matches!(o.invocation_outcome(&job,&mut r),Ok(InvocationOutcome::SettingsDefinitionCommitted(outcome)) if !outcome.changed)
    );
}
#[test]
fn native_definition_post_install_interruption_is_unknown_not_a_clean_rejection() {
    let (mut o, mut r) = fixture::setup();
    let c = fixture::context();
    let common = fixture::common(&c);
    let f = facts(&common);
    let declaration = fixture::declaration(true);
    let reads = reads();
    let expected = o.settings_basis(&mut r).unwrap().interpretation();
    let q = NativeSettingsDefinitionRequestV1 {
        expected,
        declaration: &declaration,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_native_settings_definition_v1(&job, &q, &c, &f, &mut r)
        .unwrap();
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.run_native_settings_definition_v1(&job, &c, &f, &mut r)
        ))
        .is_err()
    );
    assert!(o.is_faulted());
    assert_eq!(
        o.settings.as_ref().unwrap().basis().schema_generation,
        expected.schema + 1
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.stop_invocation(&job, &mut r),
        Err(InvocationError::Faulted)
    );
}
