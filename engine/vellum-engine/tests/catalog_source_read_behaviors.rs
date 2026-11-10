//! Host-established correspondence; engine-owned original catalog dependencies.
use vellum_engine::coordinator::*;
use vellum_engine::resource::*;
#[path = "support/resource_catalog_fixture.rs"]
mod catalog;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;
fn facts(generation: u64) -> TrustedCatalogSourceReadBindingV1 {
    TrustedCatalogSourceReadBindingV1 {
        profile: catalog::limits(),
        expected_generation: generation,
        max_binding_bytes: 16384,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        identity_allowed: Some(true),
        read_allowed: Some(true),
        provider_available: Some(true),
        correspondence_current: Some(true),
        resources_ready: Some(true),
    }
}
fn config() -> ResourceCatalogConfigurationV1 {
    let mut cfg = catalog::configuration(ResourceCatalogTableV1::Read);
    cfg.identity_known = true;
    cfg.capabilities = ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
        atomic_read: false,
        insertion_line_break: CatalogInsertionLineBreakV1::CrLf,
    };
    cfg
}
fn setup() -> support::Fixture {
    setup_policy(CatalogInsertionLineBreakV1::CrLf)
}
fn setup_policy(policy: CatalogInsertionLineBreakV1) -> support::Fixture {
    let mut f = support::Fixture::new();
    f.owner
        .configure_retention_budget(1048576, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_source_reader_v1(
            SourceReaderLimitsV1 {
                max_live: 4,
                max_issued: 8,
                max_input_bytes: 128,
                max_chunks: 8,
                max_held_bytes: 65536,
                max_store_bytes: 131072,
            },
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .configure_resource_catalog_v1(
            &[ResourceCatalogRowV1 {
                address: catalog::address(ResourceCatalogTableV1::Read, "source"),
                generation: 0,
                configuration: ResourceCatalogConfigurationV1 {
                    capabilities: ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
                        atomic_read: false,
                        insertion_line_break: policy,
                    },
                    ..config()
                },
            }],
            catalog::limits(),
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    f
}
fn target(f: &mut support::Fixture) -> SourceReadTargetV1 {
    let raw = f.port.source_read_target_v1(b"\xffnative-key").unwrap();
    f.owner
        .bind_catalog_source_read_target_v1(
            raw,
            &catalog::address(ResourceCatalogTableV1::Read, "source"),
            &facts(0),
            &mut f.recorder,
        )
        .unwrap()
}
fn remove(f: &mut support::Fixture) {
    let p = f
        .owner
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address: catalog::address(ResourceCatalogTableV1::Read, "source"),
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
fn given_a_catalog_bound_target_when_the_row_is_removed_then_admission_and_ready_dispatch_refuse_without_refunding_work()
 {
    let mut f = setup();
    let admitted = target(&mut f);
    let not_admitted = target(&mut f);
    let id = f
        .owner
        .admit_source_read_v1(admitted, ReadStrength::CapturedRead, &mut f.recorder)
        .unwrap();
    let held = f.owner.source_reader_usage_v1().unwrap();
    remove(&mut f);
    assert_eq!(
        f.owner
            .admit_source_read_v1(not_admitted, ReadStrength::CapturedRead, &mut f.recorder),
        Err(ResourceError::Stale)
    );
    assert!(matches!(
        f.owner.submit_source_read_v1(&id, &mut f.recorder),
        Err(ResourceError::Stale)
    ));
    assert_eq!(f.owner.source_reader_usage_v1().unwrap(), held);
    f.owner.stop_source_read_v1(&id, &mut f.recorder).unwrap();
    f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
    assert_eq!(f.owner.source_reader_usage_v1().unwrap().held_bytes, 0);
}
#[test]
fn given_dispatched_work_when_registration_is_withdrawn_then_original_late_transcript_survives_but_cannot_activate()
 {
    let mut f = setup();
    let target = target(&mut f);
    let id = f
        .owner
        .admit_source_read_v1(target, ReadStrength::CapturedRead, &mut f.recorder)
        .unwrap();
    let dispatch = f.owner.submit_source_read_v1(&id, &mut f.recorder).unwrap();
    assert_eq!(dispatch.resource_key(), b"\xffnative-key"); // Neither locator nor catalog key.
    remove(&mut f);
    let before = f.owner.generation();
    let v = SourceReadVersionV1::Opaque(b"version");
    for evidence in [
        f.port.source_read_start_v1(&dispatch, v).unwrap(),
        f.port
            .source_read_chunk_v1(&dispatch, 0, 0, b"prose\r\ncode")
            .unwrap(),
        f.port.source_read_end_v1(&dispatch, v, 1, 11).unwrap(),
        f.port.source_read_quiescence_v1(&dispatch).unwrap(),
    ] {
        f.owner
            .record_source_read_v1(evidence, &mut f.recorder)
            .unwrap();
    }
    let status = f.owner.source_read_status_v1(&id, &mut f.recorder).unwrap();
    assert!(status.candidate_ready && status.quiescent && !status.contested);
    assert_eq!(
        f.owner.activate_source_read_v1(&id, &mut f.recorder),
        Err(ResourceError::Stale)
    );
    assert_eq!(f.owner.generation(), before);
    assert_eq!(
        f.owner.retire_source_read_v1(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    drop(dispatch); // The outstanding native capability must be released, even after withdrawal.
    f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
}

#[test]
fn unrelated_catalog_publication_does_not_invalidate_the_original_read_row() {
    let mut f = setup();
    let target = target(&mut f);
    let p = f
        .owner
        .prepare_resource_catalog_v1(
            &[catalog::install(ResourceCatalogTableV1::Write, "other")],
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .commit_resource_catalog_v1(p, &catalog::facts(), &mut f.recorder)
        .unwrap();
    let id = f
        .owner
        .admit_source_read_v1(target, ReadStrength::CapturedRead, &mut f.recorder)
        .unwrap();
    let dispatch = f.owner.submit_source_read_v1(&id, &mut f.recorder).unwrap();
    assert_eq!(dispatch.resource_key(), b"\xffnative-key");
}

#[test]
fn replacement_cannot_revive_an_old_target_or_upgrade_native_read_strength() {
    let mut f = setup();
    let old = target(&mut f);
    remove(&mut f);
    let p = f
        .owner
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address: catalog::address(ResourceCatalogTableV1::Read, "source"),
                expected: CatalogExpectationV1::Absent,
                action: ResourceCatalogActionV1::Install(config()),
            }],
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .commit_resource_catalog_v1(p, &catalog::facts(), &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .admit_source_read_v1(old, ReadStrength::CapturedRead, &mut f.recorder),
        Err(ResourceError::Stale)
    );
    let raw = f.port.source_read_target_v1(b"native").unwrap();
    let bound = f
        .owner
        .bind_catalog_source_read_target_v1(
            raw,
            &catalog::address(ResourceCatalogTableV1::Read, "source"),
            &facts(1),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner
            .admit_source_read_v1(bound, ReadStrength::AtomicResourceSnapshot, &mut f.recorder),
        Err(ResourceError::Unsupported)
    );
    assert_eq!(f.owner.source_reader_usage_v1().unwrap().issued, 0);
}

#[test]
fn optional_saturated_instrumentation_does_not_change_binding_or_staleness() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
    for capacity in [0, 1, 128] {
        let mut f = setup();
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let bound = target(&mut f);
        remove(&mut f);
        assert_eq!(
            f.owner
                .admit_source_read_v1(bound, ReadStrength::CapturedRead, &mut f.recorder),
            Err(ResourceError::Stale)
        );
        assert_eq!(f.owner.source_reader_usage_v1().unwrap().issued, 0);
        let mut seen = Vec::new();
        while let Some(record) = f.recorder.pop_front() {
            if matches!(
                record.boundary,
                Boundary::CatalogSourceReadBinding | Boundary::CatalogSourceReadCurrentness
            ) {
                assert!(record.service_time.is_some());
            }
            seen.push(record.boundary);
        }
        if capacity != 0 {
            assert!(seen.contains(&Boundary::CatalogSourceReadBinding));
        }
        if capacity == 128 {
            assert!(seen.contains(&Boundary::CatalogSourceReadCurrentness));
        }
        if capacity == 1 {
            assert!(f.recorder.dropped() > 0);
        }
    }
}

#[test]
fn a_current_binding_activates_losslessly_and_later_withdrawal_does_not_rewrite_the_document() {
    for (policy, expected) in [
        (CatalogInsertionLineBreakV1::Lf, GeneratedNewlineV1::Lf),
        (CatalogInsertionLineBreakV1::CrLf, GeneratedNewlineV1::CrLf),
        (CatalogInsertionLineBreakV1::Cr, GeneratedNewlineV1::Cr),
    ] {
        let mut f = setup_policy(policy);
        let target = target(&mut f);
        let id = f
            .owner
            .admit_source_read_v1(target, ReadStrength::CapturedRead, &mut f.recorder)
            .unwrap();
        let dispatch = f.owner.submit_source_read_v1(&id, &mut f.recorder).unwrap();
        let raw = "\u{feff}English\r\nDeutsch: Grüße\n".as_bytes();
        let v = SourceReadVersionV1::Opaque(b"version");
        for e in [
            f.port.source_read_start_v1(&dispatch, v).unwrap(),
            f.port.source_read_chunk_v1(&dispatch, 0, 0, raw).unwrap(),
            f.port
                .source_read_end_v1(&dispatch, v, 1, raw.len())
                .unwrap(),
        ] {
            f.owner.record_source_read_v1(e, &mut f.recorder).unwrap();
        }
        let activated = f
            .owner
            .activate_source_read_v1(&id, &mut f.recorder)
            .unwrap();
        remove(&mut f);
        assert_eq!(
            f.owner.activate_source_read_v1(&id, &mut f.recorder),
            Err(ResourceError::Stale)
        );
        let read = f
            .owner
            .read(&activated.document.document, &mut f.recorder)
            .unwrap();
        assert_eq!(read.source.encode().unwrap(), raw);
        assert_eq!(
            f.owner
                .source_insertion_policy_v1(&activated.document, &mut f.recorder)
                .unwrap(),
            expected
        );
    }
}

#[test]
fn joining_an_existing_owner_does_not_replace_its_policy_with_the_incoming_catalog_default() {
    let mut f = setup_policy(CatalogInsertionLineBreakV1::Cr);
    let raw = f.port.source_read_target_v1(b"source").unwrap();
    let target = f
        .owner
        .bind_catalog_source_read_target_v1(
            raw,
            &catalog::address(ResourceCatalogTableV1::Read, "source"),
            &facts(0),
            &mut f.recorder,
        )
        .unwrap();
    let id = f
        .owner
        .admit_source_read_v1(target, ReadStrength::CapturedRead, &mut f.recorder)
        .unwrap();
    let dispatch = f.owner.submit_source_read_v1(&id, &mut f.recorder).unwrap();
    let v = SourceReadVersionV1::Opaque(b"next");
    for e in [
        f.port.source_read_start_v1(&dispatch, v).unwrap(),
        f.port
            .source_read_chunk_v1(&dispatch, 0, 0, b"different\r")
            .unwrap(),
        f.port.source_read_end_v1(&dispatch, v, 1, 10).unwrap(),
    ] {
        f.owner.record_source_read_v1(e, &mut f.recorder).unwrap();
    }
    let generation = f.owner.generation();
    let activated = f
        .owner
        .activate_source_read_v1(&id, &mut f.recorder)
        .unwrap();
    assert_eq!(
        activated.disposition,
        SourceReadActivationDispositionV1::Joined
    );
    assert_eq!(activated.document, f.doc);
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        f.owner
            .source_insertion_policy_v1(&f.doc, &mut f.recorder)
            .unwrap(),
        GeneratedNewlineV1::Lf
    );
    assert_eq!(
        f.owner
            .read(&f.doc.document, &mut f.recorder)
            .unwrap()
            .source
            .text(),
        "text"
    );
}
