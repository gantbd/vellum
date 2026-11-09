use vellum_engine::coordinator::*;
use vellum_engine::resource::*;
#[path = "support/resource_catalog_fixture.rs"]
mod catalog;
#[path = "catalog_writer_behaviors/exports.rs"]
mod exports;
#[path = "catalog_writer_behaviors/owned.rs"]
mod owned;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;
fn facts() -> TrustedCatalogWriterBindingV1 {
    TrustedCatalogWriterBindingV1 {
        profile: catalog::limits(),
        expected_generation: 0,
        max_binding_bytes: 16384,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        identity_allowed: Some(true),
        write_allowed: Some(true),
        provider_available: Some(true),
        correspondence_current: Some(true),
        resources_ready: Some(true),
    }
}
fn setup() -> support::Fixture {
    let mut cfg = catalog::configuration(ResourceCatalogTableV1::Write);
    cfg.identity_known = true;
    setup_configuration(cfg)
}
fn setup_configuration(cfg: ResourceCatalogConfigurationV1) -> support::Fixture {
    let mut f = support::Fixture::new();
    f.owner
        .configure_resource_catalog_v1(
            &[ResourceCatalogRowV1 {
                address: catalog::address(ResourceCatalogTableV1::Write, "writer"),
                generation: 0,
                configuration: cfg,
            }],
            catalog::limits(),
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    f
}
fn binding(f: &mut support::Fixture, key: &[u8]) -> CatalogWriterBindingV1 {
    let target = f.port.source_binding_target(key).unwrap();
    f.owner
        .bind_catalog_writer_v1(
            target,
            &catalog::address(ResourceCatalogTableV1::Write, "writer"),
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Ordinary,
    }
}
fn remove(f: &mut support::Fixture) {
    let p = f
        .owner
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address: catalog::address(ResourceCatalogTableV1::Write, "writer"),
                expected: CatalogExpectationV1::Present { generation: 0 },
                action: ResourceCatalogActionV1::Remove,
            }],
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .commit_resource_catalog_v1(p, &catalog::facts(), &mut f.recorder)
        .unwrap();
}
#[test]
fn catalog_writer_withdrawal_refuses_new_source_work_and_queued_dispatch_without_refunding_it() {
    let mut f = setup();
    let binding = binding(&mut f, b"source");
    let pair = f.pair();
    let id = f
        .owner
        .queue_catalog_pinned_source_save_v1(
            &f.doc,
            &binding,
            requirements(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let usage = f.owner.resource_usage();
    remove(&mut f);
    let pair = f.pair();
    assert_eq!(
        f.owner.queue_catalog_pinned_source_save_v1(
            &f.doc,
            &binding,
            requirements(),
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    );
    let pair = f.pair().unwrap();
    assert!(matches!(
        f.owner.dispatch_save_with_pair(
            &id,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    ));
    assert_eq!(f.owner.resource_usage(), usage);
}

fn queue_source(f: &mut support::Fixture, binding: &CatalogWriterBindingV1) -> SaveId {
    let pair = f.pair();
    f.owner
        .queue_catalog_pinned_source_save_v1(&f.doc, binding, requirements(), pair, &mut f.recorder)
        .unwrap()
}
fn dispatch_source(f: &mut support::Fixture, id: &SaveId) -> SaveDispatch {
    let pair = f.pair().unwrap();
    f.owner
        .dispatch_save_with_pair(
            id,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap()
}
fn finish(f: &mut support::Fixture, dispatch: &SaveDispatch, bytes: usize) {
    for e in [
        f.port
            .result(
                dispatch,
                Application::Written,
                bytes,
                Synchronization::NotRequested,
                Some(b"next"),
            )
            .unwrap(),
        f.port.no_future_effects(dispatch).unwrap(),
        f.port.quiescence(dispatch).unwrap(),
    ] {
        f.owner.record_save_evidence(e, &mut f.recorder).unwrap();
    }
}
#[test]
fn original_write_receipts_survive_withdrawal_but_cannot_activate_a_new_baseline() {
    let mut f = setup();
    let binding = binding(&mut f, b"source");
    let id = queue_source(&mut f, &binding);
    let dispatch = dispatch_source(&mut f, &id);
    remove(&mut f);
    finish(&mut f, &dispatch, 4);
    let outcome = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
    assert!(outcome.quiescent && outcome.no_future_effects);
    let pair = f.pair().unwrap();
    let generation = f.owner.generation();
    assert_eq!(
        f.owner
            .publish_save_baseline_with_pair(&id, pair, &mut f.recorder),
        Err(ResourceError::Stale)
    );
    assert_eq!(f.owner.generation(), generation);
}
#[test]
fn unrelated_registration_does_not_stale_the_writer_and_installed_baselines_remain_historical() {
    let mut f = setup();
    let binding = binding(&mut f, b"source");
    let id = queue_source(&mut f, &binding);
    let p = f
        .owner
        .prepare_resource_catalog_v1(
            &[catalog::install(ResourceCatalogTableV1::Read, "other")],
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .commit_resource_catalog_v1(p, &catalog::facts(), &mut f.recorder)
        .unwrap();
    let dispatch = dispatch_source(&mut f, &id);
    finish(&mut f, &dispatch, 4);
    let pair = f.pair().unwrap();
    f.owner
        .publish_save_baseline_with_pair(&id, pair, &mut f.recorder)
        .unwrap();
    let before = f
        .owner
        .saved_state(&f.doc.document, &mut f.recorder)
        .unwrap();
    remove(&mut f);
    assert_eq!(
        f.owner
            .saved_state(&f.doc.document, &mut f.recorder)
            .unwrap(),
        before
    );
}
fn queue_annotations(
    f: &mut support::Fixture,
    binding: &CatalogWriterBindingV1,
) -> Result<SaveId, ResourceError> {
    let encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let snapshot = f
        .owner
        .capture_annotations(&f.doc, 0, &mut f.recorder)
        .unwrap();
    let encoded = encoder.seal(snapshot, b"[]".to_vec()).unwrap();
    let pair = f.pair();
    f.owner.queue_catalog_annotation_save_v1(
        encoded,
        f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
        binding,
        requirements(),
        pair,
        &mut f.recorder,
    )
}
#[test]
fn annotation_writer_currentness_gates_admission_dispatch_and_baseline_without_losing_receipts() {
    let mut f = setup();
    let binding = binding(&mut f, b"comments");
    let id = queue_annotations(&mut f, &binding).unwrap();
    let artifact = f.owner.save_outcome(&id, &mut f.recorder).unwrap().artifact;
    assert_eq!(
        f.owner.queue_annotation_export(
            &artifact,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            requirements(),
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    ); // Cannot drop the original catalog dependency.
    let queued = queue_annotations(&mut f, &binding).unwrap();
    let pair = f.pair().unwrap();
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &id,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    remove(&mut f);
    finish(&mut f, &dispatch, 2);
    let pair = f.pair().unwrap();
    assert!(matches!(
        f.owner.dispatch_save_with_pair(
            &queued,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    ));
    let pair = f.pair();
    assert_eq!(
        f.owner
            .install_annotation_save_baseline(&id, pair, &mut f.recorder),
        Err(ResourceError::Stale)
    );
    assert_eq!(
        queue_annotations(&mut f, &binding),
        Err(ResourceError::Stale)
    );
    assert!(
        f.owner
            .save_outcome(&id, &mut f.recorder)
            .unwrap()
            .quiescent
    );
}
#[test]
fn a_catalog_binding_for_another_native_destination_is_an_expected_refusal_not_an_owner_fault() {
    let mut f = setup();
    let wrong = binding(&mut f, b"different");
    let pair = f.pair();
    assert_eq!(
        f.owner.queue_catalog_pinned_source_save_v1(
            &f.doc,
            &wrong,
            requirements(),
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    let correct = binding(&mut f, b"source");
    queue_source(&mut f, &correct);
}

#[test]
fn strict_binding_facts_and_optional_instrumentation_do_not_change_writer_currentness() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
    for capacity in [0, 1, 128] {
        let mut f = setup();
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        for index in 0..9 {
            for value in [None, Some(false)] {
                let mut facts = facts();
                let flags = [
                    &mut facts.authorized,
                    &mut facts.scope,
                    &mut facts.continuous,
                    &mut facts.live,
                    &mut facts.identity_allowed,
                    &mut facts.write_allowed,
                    &mut facts.provider_available,
                    &mut facts.correspondence_current,
                    &mut facts.resources_ready,
                ];
                *flags.into_iter().nth(index).unwrap() = value;
                assert!(matches!(
                    f.owner.bind_catalog_writer_v1(
                        f.port.source_binding_target(b"source").unwrap(),
                        &catalog::address(ResourceCatalogTableV1::Write, "writer"),
                        &facts,
                        &mut f.recorder
                    ),
                    Err(ResourceError::Unavailable)
                ));
            }
        }
        let binding = binding(&mut f, b"source");
        let id = queue_source(&mut f, &binding);
        remove(&mut f);
        let pair = f.pair().unwrap();
        assert!(matches!(
            f.owner.dispatch_save_with_pair(
                &id,
                f.port.read_result(b"source", b"s0", b"text").unwrap(),
                pair,
                &mut f.recorder
            ),
            Err(ResourceError::Stale)
        ));
        let mut bindings = 0;
        let mut checks = 0;
        while let Some(o) = f.recorder.pop_front() {
            match o.boundary {
                Boundary::CatalogWriterBinding => {
                    bindings += 1;
                    assert!(o.service_time.is_some());
                }
                Boundary::CatalogWriterCurrentness => {
                    checks += 1;
                    assert!(o.service_time.is_some());
                }
                _ => {}
            }
        }
        if capacity == 128 {
            assert_eq!(bindings, 19);
            assert_eq!(checks, 2);
        }
        if capacity == 1 {
            assert!(f.recorder.dropped() > 0);
        }
        assert_eq!(f.owner.resource_usage().issued_lifetime, 1);
    }
}

#[test]
fn advertised_synchronization_cannot_create_missing_native_support() {
    let mut cfg = catalog::configuration(ResourceCatalogTableV1::Write);
    cfg.identity_known = true;
    if let ResourceCatalogCapabilitiesV1::Write {
        synchronization_supported,
        ..
    } = &mut cfg.capabilities
    {
        *synchronization_supported = true;
    }
    let mut f = setup_configuration(cfg);
    let binding = binding(&mut f, b"source");
    let pair = f.pair();
    let before = f.owner.resource_usage();
    assert_eq!(
        f.owner.queue_catalog_pinned_source_save_v1(
            &f.doc,
            &binding,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized
            },
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Unsupported)
    );
    assert_eq!(f.owner.resource_usage(), before);
}
#[test]
fn reinstalling_a_writer_does_not_revive_old_bindings_or_require_equal_document_binding_generations()
 {
    let mut f = setup();
    let old = binding(&mut f, b"source");
    remove(&mut f);
    let mut cfg = catalog::configuration(ResourceCatalogTableV1::Write);
    cfg.identity_known = true;
    let p = f
        .owner
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address: catalog::address(ResourceCatalogTableV1::Write, "writer"),
                expected: CatalogExpectationV1::Absent,
                action: ResourceCatalogActionV1::Install(cfg),
            }],
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .commit_resource_catalog_v1(p, &catalog::facts(), &mut f.recorder)
        .unwrap();
    let pair = f.pair();
    assert_eq!(
        f.owner.queue_catalog_pinned_source_save_v1(
            &f.doc,
            &old,
            requirements(),
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    );
    let mut facts = facts();
    facts.expected_generation = 1;
    let fresh = f
        .owner
        .bind_catalog_writer_v1(
            f.port.source_binding_target(b"source").unwrap(),
            &catalog::address(ResourceCatalogTableV1::Write, "writer"),
            &facts,
            &mut f.recorder,
        )
        .unwrap();
    let id = queue_source(&mut f, &fresh);
    assert_eq!(
        f.owner
            .save_outcome(&id, &mut f.recorder)
            .unwrap()
            .binding_generation,
        Some(0)
    );
}
