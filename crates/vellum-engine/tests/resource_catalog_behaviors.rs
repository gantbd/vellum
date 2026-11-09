//! Engine metadata activation only: no filesystem/provider execution or grants.
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1, Recorder};
use vellum_engine::source::{Limits, Replacement};
#[path = "support/resource_catalog_fixture.rs"]
mod support;
use support::*;
#[path = "resource_catalog_behaviors/recipient.rs"]
mod recipient;
fn owner() -> SourceCoordinator {
    SourceCoordinator::with_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 8,
        },
    )
}

#[test]
fn given_authored_history_when_resource_metadata_activates_then_text_history_and_provider_work_are_framed()
 {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"English / Deutsch", &mut r).unwrap();
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 7,
                    expected: "English".into(),
                    inserted: "Prose".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    let history = o.commit(p, &mut r).unwrap().history_group.unwrap();
    let version = o.read(&source.document, &mut r).unwrap().version;
    let providers = o.resource_usage();
    o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
        .unwrap();
    let before = o.generation();
    let p = o
        .prepare_resource_catalog_v1(
            &[
                install(ResourceCatalogTableV1::Read, "annotations"),
                install(ResourceCatalogTableV1::Write, "text"),
            ],
            &facts(),
            &mut r,
        )
        .unwrap();
    assert_eq!(o.generation(), before);
    let out = o.commit_resource_catalog_v1(p, &facts(), &mut r).unwrap();
    assert_eq!(out.status, ResourceCatalogStatusV1::Applied);
    assert_eq!(o.generation(), before + 1);
    assert_eq!(o.read(&source.document, &mut r).unwrap().version, version);
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "Prose / Deutsch"
    );
    assert_eq!(
        o.history_status(&source.document, HistoryDirection::Undo, &mut r)
            .unwrap(),
        HistoryAvailability::Eligible(history)
    );
    assert_eq!(o.resource_usage(), providers);
    assert!(
        !o.resource_catalog_registration_v1(
            &address(ResourceCatalogTableV1::Read, "annotations"),
            &mut r
        )
        .unwrap()
        .unwrap()
        .configuration
        .identity_known
    );
}

#[test]
fn given_explicit_initial_catalogs_when_another_table_installs_then_the_shared_floor_is_seeded_globally()
 {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let initial = [ResourceCatalogRowV1 {
        address: address(ResourceCatalogTableV1::Write, "existing"),
        generation: 97,
        configuration: configuration(ResourceCatalogTableV1::Write),
    }];
    o.configure_resource_catalog_v1(&initial, limits(), &facts(), &mut r)
        .unwrap();
    let p = o
        .prepare_resource_catalog_v1(
            &[install(ResourceCatalogTableV1::Read, "new")],
            &facts(),
            &mut r,
        )
        .unwrap();
    let out = o.commit_resource_catalog_v1(p, &facts(), &mut r).unwrap();
    assert_eq!(out.registrations[0].generation, Some(98));
    assert_eq!(
        o.resource_catalog_registration_v1(
            &address(ResourceCatalogTableV1::Write, "existing"),
            &mut r
        )
        .unwrap()
        .unwrap()
        .generation,
        97
    );
    assert_eq!(
        o.resource_catalog_usage_v1(&mut r)
            .unwrap()
            .registration_floor,
        98
    );
}

#[test]
fn given_a_native_catalog_certificate_when_another_batch_commits_then_the_old_candidate_is_not_installed()
 {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
        .unwrap();
    let stale = o
        .prepare_resource_catalog_v1(
            &[install(ResourceCatalogTableV1::Read, "old")],
            &facts(),
            &mut r,
        )
        .unwrap();
    let current = o
        .prepare_resource_catalog_v1(
            &[install(ResourceCatalogTableV1::Write, "current")],
            &facts(),
            &mut r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(current, &facts(), &mut r)
        .unwrap();
    let generation = o.generation();
    assert_eq!(
        o.commit_resource_catalog_v1(stale, &facts(), &mut r),
        Err(vellum_engine::resource::ResourceError::Stale)
    );
    assert_eq!(o.generation(), generation);
    assert!(
        o.resource_catalog_registration_v1(&address(ResourceCatalogTableV1::Read, "old"), &mut r)
            .unwrap()
            .is_none()
    );
    assert!(
        o.resource_catalog_registration_v1(
            &address(ResourceCatalogTableV1::Write, "current"),
            &mut r
        )
        .unwrap()
        .is_some()
    );
}

#[test]
fn given_optional_or_saturated_capture_when_catalog_batches_execute_then_results_are_identical_and_actual_stages_are_observed()
 {
    for capacity in [0, 1, 128] {
        let mut o = owner();
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
            .unwrap();
        let p = o
            .prepare_resource_catalog_v1(
                &[install(ResourceCatalogTableV1::Read, "r")],
                &facts(),
                &mut r,
            )
            .unwrap();
        let out = o.commit_resource_catalog_v1(p, &facts(), &mut r).unwrap();
        assert_eq!(out.status, ResourceCatalogStatusV1::Applied);
        assert_eq!(out.registrations[0].generation, Some(1));
        o.resource_catalog_usage_v1(&mut r).unwrap();
        assert!(r.queued() <= capacity);
        if capacity == 128 {
            let mut observed = Vec::new();
            let mut timed = false;
            while let Some(e) = r.pop_front() {
                assert_eq!(e.disposition, Disposition::Completed);
                observed.push(e.boundary);
                timed |= e.service_time.is_some();
            }
            for b in [
                Boundary::ResourceCatalogConfiguration,
                Boundary::ResourceCatalogPreparation,
                Boundary::ResourceCatalogValidation,
                Boundary::ResourceCatalogPublication,
                Boundary::ResourceCatalogQuery,
            ] {
                assert!(observed.contains(&b));
            }
            assert!(timed);
        }
    }
}
