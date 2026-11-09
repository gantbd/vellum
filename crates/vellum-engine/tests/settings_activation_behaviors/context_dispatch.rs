use super::scoped_basis::{context, facts};
use super::*;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
fn ready() -> (SourceCoordinator, Recorder) {
    ready_with_extra(0)
}
fn ready_with_extra(extra: usize) -> (SourceCoordinator, Recorder) {
    scoped_basis::ready_with_headroom(
        16,
        8,
        Some(1048576),
        discovered_context::limits()
            .declared_bytes()
            .unwrap()
            .checked_add(extra)
            .unwrap(),
    )
}
#[test]
fn created_layer_facade_preserves_original_admission_replay_and_reauthorized_receipt() {
    for capacity in [0, 1, 512] {
        let (mut o, _) = ready_with_extra(1048576);
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(
                vellum_engine::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
            );
        let c = context();
        let route = discovered_context::Route::install(&mut o, &c, &mut r);
        o.configure_settings_layers(native_lifecycle::limits(), &mut r)
            .unwrap();
        let grants = [native_lifecycle::created()];
        let common = facts(&c, &grants);
        let f = native_lifecycle::gate(&common);
        let job = o.issue_invocation(&mut r).unwrap();
        let profile = observation_support::Profile {
            epoch: 7,
            maximum: 65536,
            capabilities: InvocationReceiptCapabilitiesV1 {
                created_settings: true,
                ..Default::default()
            },
        };
        profile.bind(&mut o, &job, &c, &mut r);
        let reads = reads();
        let expected = o.settings_layer_basis(&mut r).unwrap();
        let q = native_lifecycle::decode(&job, &reads, &c.profile, expected, &mut r);
        let mut admission = TrustedCreatedLayerResultAdmissionV1 {
            context: &c,
            max_result_bytes: 0,
            references_allowed: Some(true),
            outcome_allowed: Some(true),
            context_current: Some(true),
        };
        assert_eq!(
            route.submit_lifecycle_with_results(&mut o, &c, &q, &f, &admission, &mut r),
            Err(DiscoveredNativeCommandErrorV1::Invocation(
                InvocationError::Limit
            ))
        );
        assert_eq!(
            o.invocation_state(&job, &mut r),
            Ok(InvocationState::Issued)
        );
        assert_eq!(o.settings_layer_basis(&mut r).unwrap(), expected);
        admission.max_result_bytes = 65536;
        let mut wrong_context = c.clone();
        wrong_context.profile = "different".into();
        assert_eq!(
            route.submit_lifecycle_with_results(&mut o, &wrong_context, &q, &f, &admission, &mut r),
            Err(DiscoveredNativeCommandErrorV1::Invocation(
                InvocationError::Unavailable
            ))
        );
        route
            .submit_lifecycle_with_results(&mut o, &c, &q, &f, &admission, &mut r)
            .unwrap();
        o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
            .unwrap();
        let mut observer = profile.open(&mut o, &job, &c, &mut r);
        let mut release = TrustedCreatedLayerResultReleaseV1 {
            context: &c,
            outcome_readable: Some(true),
            created_result_readable: None,
            created_result_context_current: Some(true),
            historical_layers_readable: Some(true),
        };
        assert!(
            profile
                .query(
                    &o,
                    &mut observer,
                    InvocationReceiptRequestV1::CreatedSettings(&release),
                    &c,
                    &mut r
                )
                .is_err()
        );
        release.created_result_readable = Some(true);
        let InvocationReceiptV1::CreatedSettings(reply) = profile
            .query(
                &o,
                &mut observer,
                InvocationReceiptRequestV1::CreatedSettings(&release),
                &c,
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(reply.created.len(), 1);
        let reference = reply.created[0].reference.clone();
        let generation = o.generation();
        assert_eq!(
            route.submit_lifecycle_with_results(&mut o, &c, &q, &f, &admission, &mut r),
            Err(DiscoveredNativeCommandErrorV1::Invocation(
                InvocationError::Unavailable
            ))
        );
        assert_eq!(
            route.submit(
                &mut o,
                &c,
                NativeDiscoveredContextCommandV1::NativeLifecycle {
                    arguments: &q,
                    facts: &f,
                },
                &mut r
            ),
            Ok(InvocationState::Finished)
        );
        let InvocationReceiptV1::CreatedSettings(reply) = profile
            .query(
                &o,
                &mut observer,
                InvocationReceiptRequestV1::CreatedSettings(&release),
                &c,
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(reply.created[0].reference, reference);
        assert_eq!(o.generation(), generation);
        assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 2);
        let control = SettingsBasisControlV1 {
            context: &c,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        assert_eq!(
            o.forget_created_layer_results(&job, &control, &mut r),
            Ok(true)
        );
        assert_eq!(
            o.forget_created_layer_results(&job, &control, &mut r),
            Ok(false)
        );
        assert!(
            profile
                .query(
                    &o,
                    &mut observer,
                    InvocationReceiptRequestV1::CreatedSettings(&release),
                    &c,
                    &mut r
                )
                .is_err()
        );
        if capacity == 512 {
            use vellum_engine::instrumentation::Boundary;
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::NativeLifecycleDecodedAdmission,
                Boundary::CreatedResultsAdmission,
                Boundary::CreatedResultsPublication,
                Boundary::CreatedResultsRelease,
            ] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == boundary && e.service_time.is_some())
                );
            }
        }
    }
}
#[test]
fn created_layer_result_opt_in_cannot_upgrade_an_already_admitted_command() {
    let (mut o, mut r) = ready_with_extra(1048576);
    let c = context();
    let route = discovered_context::Route::install(&mut o, &c, &mut r);
    o.configure_settings_layers(native_lifecycle::limits(), &mut r)
        .unwrap();
    let grants = [native_lifecycle::created()];
    let common = facts(&c, &grants);
    let f = native_lifecycle::gate(&common);
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let q = native_lifecycle::decode(&job, &reads, &c.profile, expected, &mut r);
    let mut admission = TrustedCreatedLayerResultAdmissionV1 {
        context: &c,
        max_result_bytes: 65536,
        references_allowed: None,
        outcome_allowed: Some(true),
        context_current: Some(true),
    };
    assert_eq!(
        route.submit_lifecycle_with_results(&mut o, &c, &q, &f, &admission, &mut r),
        Err(DiscoveredNativeCommandErrorV1::Invocation(
            InvocationError::Unavailable
        ))
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    route
        .submit(
            &mut o,
            &c,
            NativeDiscoveredContextCommandV1::NativeLifecycle {
                arguments: &q,
                facts: &f,
            },
            &mut r,
        )
        .unwrap();
    admission.references_allowed = Some(true);
    assert_eq!(
        route.submit_lifecycle_with_results(&mut o, &c, &q, &f, &admission, &mut r),
        Err(DiscoveredNativeCommandErrorV1::Invocation(
            InvocationError::Unavailable
        ))
    );
    o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
        .unwrap();
    let release = TrustedCreatedLayerResultReleaseV1 {
        context: &c,
        outcome_readable: Some(true),
        created_result_readable: Some(true),
        created_result_context_current: Some(true),
        historical_layers_readable: Some(true),
    };
    assert!(o.created_layer_results(&job, &release, &mut r).is_err());
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 2);
}
#[test]
fn installed_native_and_scoped_serialization_dispatch_preserve_text_and_original_intent() {
    use SerializationEnvelopeValueV1 as V;
    let limits = SourceSerializationLimitsV1 {
        max_dependencies: 8,
        max_label_bytes: 128,
        max_request_bytes: 4096,
        max_plan_bytes: 4096,
        max_work: 4096,
    };
    let pool = SerializationBasisLimitsV1 {
        serialization: limits,
        max_contexts: 4,
        max_issued_contexts: 16,
        max_context_bytes: 1024,
        max_held_bytes: 32768,
    };
    for scoped in [false, true] {
        let (mut o, mut r) = ready_with_extra(pool.required_pool_bytes().unwrap());
        let c = context();
        let route = discovered_context::Route::install(&mut o, &c, &mut r);
        o.configure_source_serialization("engine", "local", limits, &mut r)
            .unwrap();
        let d = o.create(b"English\r\nDeutsch\n", &mut r).unwrap();
        let job = o.issue_invocation(&mut r).unwrap();
        let reads = reads();
        let f = TrustedSerializationFactsV1 {
            context: &c,
            document: &d.document,
            profile: limits,
            authorized: Some(true),
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            interpretation_current: Some(true),
            companions_ready: Some(true),
            settings_writable: Some(true),
            resources_ready: Some(true),
            write_allowed: Some(true),
            required_reads: &[],
        };
        let serialization = [
            ("profile", D::Text("strict-utf8-one-bom/1")),
            ("bom", D::Boolean(true)),
        ];
        let mut rows = vec![
            (
                "contract",
                V::Data(D::Text(if scoped {
                    "scoped-source-serialization/1"
                } else {
                    "source-serialization/1"
                })),
            ),
            ("operation", V::Data(D::Text("semantic-settings"))),
            ("invocation", V::Invocation(&job)),
            ("profile", V::Data(D::Text(&c.profile))),
            ("serialization", V::Data(D::Object(&serialization))),
            ("dependencies", V::Dependencies(&reads)),
        ];
        if scoped {
            o.configure_serialization_bases(
                pool,
                &SerializationPoolControlV1 {
                    owner_control_allowed: Some(true),
                    resources_ready: Some(true),
                    retention_released: Some(true),
                    delivery_drained: Some(true),
                },
                &mut r,
            )
            .unwrap();
            let g = TrustedSerializationBasisFactsV1 {
                native: &f,
                owner_control_allowed: Some(true),
                basis_allowed: Some(true),
                host_basis_allowed: Some(true),
            };
            let basis = o
                .capture_serialization_basis(&d.document, &c, &g, &mut r)
                .unwrap();
            let q = normalize_scoped_serialization_envelope_v1(
                &V::Object(&rows),
                limits,
                CaptureLimitsV1 {
                    max_bytes: 16384,
                    max_nodes: 64,
                    max_depth: 8,
                },
                32768,
                &mut r,
            )
            .unwrap();
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::ScopedSerialization {
                        arguments: &q,
                        basis: &basis,
                        facts: &g
                    },
                    &mut r
                ),
                Ok(InvocationState::Pending)
            );
            o.run_scoped_source_serialization(&job, &c, &g, &mut r)
                .unwrap();
            let generation = o.generation();
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::ScopedSerialization {
                        arguments: &q,
                        basis: &basis,
                        facts: &g
                    },
                    &mut r
                ),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
        } else {
            rows.extend([
                ("document", V::Document(&d.document)),
                ("expectedSerialization", V::Data(D::Number(1.0))),
            ]);
            let q = normalize_source_serialization_envelope_v1(
                &V::Object(&rows),
                limits,
                CaptureLimitsV1 {
                    max_bytes: 16384,
                    max_nodes: 64,
                    max_depth: 8,
                },
                32768,
                &mut r,
            )
            .unwrap();
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::NativeSerialization {
                        arguments: &q,
                        facts: &f
                    },
                    &mut r
                ),
                Ok(InvocationState::Pending)
            );
            o.run_source_serialization(&job, &c, &f, &mut r).unwrap();
            let generation = o.generation();
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::NativeSerialization {
                        arguments: &q,
                        facts: &f
                    },
                    &mut r
                ),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
        }
        let serial = o.source_serialization(&d.document, &mut r).unwrap();
        assert_eq!(serial.generation, 2);
        assert!(serial.serialization.bom);
        assert_eq!(o.read(&d.document, &mut r).unwrap().version, d);
        assert_eq!(
            o.read(&d.document, &mut r).unwrap().source.text(),
            "English\r\nDeutsch\n"
        );
    }
}
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    }
}
#[test]
fn installed_scoped_values_dispatch_keeps_the_original_basis_and_permission_owner() {
    let (mut o, mut r) = ready();
    let c = context();
    let route = discovered_context::Route::install(&mut o, &c, &mut r);
    let layers = [id("left")];
    let common = facts(&c, &layers);
    let f = scoped::gateway(&common);
    let bases = [o
        .capture_settings_write_basis(&layers[0], &c, &common, &mut r)
        .unwrap()];
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let q = scoped::arguments::decode(&job, &reads, &c.profile, 0.0, &mut r);
    assert_eq!(
        route.submit(
            &mut o,
            &c,
            NativeDiscoveredContextCommandV1::ScopedValues {
                arguments: &q,
                bases: &bases,
                facts: &f
            },
            &mut r
        ),
        Ok(InvocationState::Pending)
    );
    o.run_scoped_settings_change(&job, &c, &f, &mut r).unwrap();
    assert_eq!(read(&o, "A", &mut r), 2.0);
    let generation = o.generation();
    assert_eq!(
        route.submit(
            &mut o,
            &c,
            NativeDiscoveredContextCommandV1::ScopedValues {
                arguments: &q,
                bases: &bases,
                facts: &f
            },
            &mut r
        ),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
}
#[test]
fn installed_native_lifecycle_and_definition_dispatch_reuse_their_guarded_owners() {
    for definition in [false, true] {
        let (mut o, mut r) = ready();
        let c = context();
        let route = discovered_context::Route::install(&mut o, &c, &mut r);
        let grants = [native_lifecycle::created()];
        let common = facts(&c, &grants);
        let job = o.issue_invocation(&mut r).unwrap();
        let reads = reads();
        if definition {
            o.configure_settings_definitions(native_definition::limits(), &mut r)
                .unwrap();
            let f = native_definition::gate(&common);
            let expected = o.settings_basis(&mut r).unwrap().interpretation();
            let q = native_definition::decode(&job, &reads, &c.profile, expected, &mut r);
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::NativeDefinition {
                        arguments: &q,
                        facts: &f
                    },
                    &mut r
                ),
                Ok(InvocationState::Pending)
            );
            o.run_native_settings_definition_v1(&job, &c, &f, &mut r)
                .unwrap();
            assert!(matches!(
                o.invocation_outcome(&job, &mut r),
                Ok(InvocationOutcome::SettingsDefinitionCommitted(_))
            ));
            assert_eq!(
                o.settings_basis(&mut r).unwrap().schema_generation,
                expected.schema + 1
            );
            let generation = o.generation();
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::NativeDefinition {
                        arguments: &q,
                        facts: &f
                    },
                    &mut r
                ),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
        } else {
            o.configure_settings_layers(native_lifecycle::limits(), &mut r)
                .unwrap();
            let f = native_lifecycle::gate(&common);
            let expected = o.settings_layer_basis(&mut r).unwrap();
            let q = native_lifecycle::decode(&job, &reads, &c.profile, expected, &mut r);
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::NativeLifecycle {
                        arguments: &q,
                        facts: &f
                    },
                    &mut r
                ),
                Ok(InvocationState::Pending)
            );
            o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
                .unwrap();
            assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 2);
            let generation = o.generation();
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::NativeLifecycle {
                        arguments: &q,
                        facts: &f
                    },
                    &mut r
                ),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
        }
    }
}
#[test]
fn installed_scoped_lifecycle_and_definition_dispatch_preserve_original_basis_roles() {
    for definition in [false, true] {
        let (mut o, mut r) = ready();
        let c = context();
        let route = discovered_context::Route::install(&mut o, &c, &mut r);
        let grants = [scoped_lifecycle::new_id(), id("left")];
        let common = facts(&c, &grants);
        let job = o.issue_invocation(&mut r).unwrap();
        let reads = reads();
        if definition {
            o.configure_settings_definitions(scoped_definition::limits(), &mut r)
                .unwrap();
            let basis_facts = scoped_definition::basis(&common);
            let basis = o
                .capture_settings_definition_basis(&c, &basis_facts, &mut r)
                .unwrap();
            let f = scoped_definition::gate(&basis_facts);
            let expected = o.settings_basis(&mut r).unwrap().schema_generation;
            let q = scoped_definition::arguments::decode(&job, &reads, &c.profile, 1.0, &mut r);
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::ScopedDefinition {
                        arguments: &q,
                        basis: &basis,
                        facts: &f
                    },
                    &mut r
                ),
                Ok(InvocationState::Pending)
            );
            o.run_scoped_settings_definition(&job, &c, &f, &mut r)
                .unwrap();
            assert!(matches!(
                o.invocation_outcome(&job, &mut r),
                Ok(InvocationOutcome::ScopedDefinitionCommitted(_))
            ));
            assert_eq!(
                o.settings_basis(&mut r).unwrap().schema_generation,
                expected + 1
            );
            let generation = o.generation();
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::ScopedDefinition {
                        arguments: &q,
                        basis: &basis,
                        facts: &f
                    },
                    &mut r
                ),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
        } else {
            o.configure_settings_layers(scoped_lifecycle::limits(), &mut r)
                .unwrap();
            let basis_facts = scoped_lifecycle::basis(&common);
            let basis = o
                .capture_settings_lifecycle_basis(&c, &basis_facts, &mut r)
                .unwrap();
            let f = scoped_lifecycle::gate(&basis_facts, &grants);
            let q = scoped_lifecycle::arguments::decode(&job, &reads, &c.profile, &mut r);
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::ScopedLifecycle {
                        arguments: &q,
                        basis: &basis,
                        facts: &f
                    },
                    &mut r
                ),
                Ok(InvocationState::Pending)
            );
            o.run_scoped_settings_lifecycle(&job, &c, &f, &mut r)
                .unwrap();
            assert_eq!(read(&o, "A", &mut r), 3.0);
            let generation = o.generation();
            assert_eq!(
                route.submit(
                    &mut o,
                    &c,
                    NativeDiscoveredContextCommandV1::ScopedLifecycle {
                        arguments: &q,
                        basis: &basis,
                        facts: &f
                    },
                    &mut r
                ),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
        }
    }
}
#[test]
fn installed_native_values_dispatch_reuses_guarded_owner_and_original_replay() {
    use NativeSettingsEnvelopeValueV1 as V;
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for capacity in [0, 1, 512] {
        let (mut o, mut r) = ready();
        let c = context();
        let route = discovered_context::Route::install(&mut o, &c, &mut r);
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
        r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let mut lost = discovered_context::facts();
        lost.authorized = Some(false);
        assert_eq!(
            route.submit_with(
                &mut o,
                &c,
                NativeDiscoveredContextCommandV1::NativeValues {
                    arguments: &q,
                    facts: &f
                },
                1,
                &lost,
                &mut r
            ),
            Err(DiscoveredNativeCommandErrorV1::Discovery(
                DiscoveryErrorV1::Unavailable
            ))
        );
        let mut denied_common = facts(&c, &layers);
        denied_common.settings_writable = Some(false);
        let denied = TrustedNativeSettingsInvocationV1 {
            common: &denied_common,
            required_reads: &[],
        };
        assert_eq!(
            route.submit(
                &mut o,
                &c,
                NativeDiscoveredContextCommandV1::NativeValues {
                    arguments: &q,
                    facts: &denied
                },
                &mut r
            ),
            Err(DiscoveredNativeCommandErrorV1::Invocation(
                InvocationError::Unavailable
            ))
        );
        assert_eq!(
            o.invocation_state(&job, &mut r),
            Ok(InvocationState::Issued)
        );
        assert_eq!(
            route.submit(
                &mut o,
                &c,
                NativeDiscoveredContextCommandV1::NativeValues {
                    arguments: &q,
                    facts: &f
                },
                &mut r
            ),
            Ok(InvocationState::Pending)
        );
        o.run_native_settings_change_v1(&job, &c, &f, &mut r)
            .unwrap();
        assert_eq!(read(&o, "A", &mut r), 2.0);
        let generation = o.generation();
        assert_eq!(
            route.submit(
                &mut o,
                &c,
                NativeDiscoveredContextCommandV1::NativeValues {
                    arguments: &q,
                    facts: &f
                },
                &mut r
            ),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.generation(), generation);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == 512 {
            assert!(
                records
                    .iter()
                    .any(|v| v.boundary == Boundary::DiscoveryQuery && v.service_time.is_some())
            );
            assert!(
                records
                    .iter()
                    .any(|v| v.boundary == Boundary::NativeSettingsDecodedAdmission
                        && v.service_time.is_some())
            );
        } else {
            assert!(records.len() <= capacity);
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
        }
    }
}
