use super::*;
use crate::coordinator::resource::catalog::tests::{address, config, facts, limits};
fn binding() -> TrustedCatalogAnnotationReadBindingV1 {
    TrustedCatalogAnnotationReadBindingV1 {
        profile: limits(),
        expected_generation: 0,
        max_binding_bytes: 16384,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        identity_allowed: Some(true),
        read_allowed: Some(true),
        annotation_readable: Some(true),
        provider_available: Some(true),
        correspondence_current: Some(true),
        resources_ready: Some(true),
    }
}
fn setup() -> (
    SourceCoordinator,
    ProviderPort,
    DocumentVersion,
    Recorder,
    ResourceCatalogAddressV1,
) {
    let (mut o, p, old, mut r) = crate::coordinator::resource::read::tests::fixture();
    let d = crate::coordinator::resource::read::tests::original_source(&o, &old);
    o.stop_annotation_read(&old, &mut r).unwrap();
    o.retire_annotation_read(&old, &mut r).unwrap();
    let a = address(ResourceCatalogTableV1::Read, "annotations");
    let mut cfg = config(ResourceCatalogTableV1::Read, "not-the-native-key");
    cfg.capabilities = ResourceCatalogCapabilitiesV1::AnnotationsV1 { atomic_read: true };
    o.configure_resource_catalog_v1(
        &[ResourceCatalogRowV1 {
            address: a.copy().unwrap(),
            generation: 0,
            configuration: cfg,
        }],
        limits(),
        &facts(),
        &mut r,
    )
    .unwrap();
    (o, p, d, r, a)
}
#[test]
fn annotation_capture_has_an_exact_bound_and_neither_profile_nor_guarantee_can_be_substituted() {
    let (mut o, p, d, mut r, a) = setup();
    let target = o
        .bind_catalog_annotation_read_target_v1(
            p.annotation_read_target(b"comments").unwrap(),
            &a,
            &binding(),
            &mut r,
        )
        .unwrap();
    let bytes = target.catalog.as_ref().unwrap().native_bytes().unwrap();
    let mut f = binding();
    f.max_binding_bytes = bytes - 1;
    assert!(matches!(
        o.bind_catalog_annotation_read_target_v1(
            p.annotation_read_target(b"comments").unwrap(),
            &a,
            &f,
            &mut r
        ),
        Err(ResourceError::Limit)
    ));
    f.max_binding_bytes = bytes;
    o.bind_catalog_annotation_read_target_v1(
        p.annotation_read_target(b"comments").unwrap(),
        &a,
        &f,
        &mut r,
    )
    .unwrap();
    assert!(matches!(
        o.bind_catalog_source_read_target_v1(
            p.source_read_target_v1(b"comments").unwrap(),
            &a,
            &super::super::tests::binding(),
            &mut r
        ),
        Err(ResourceError::Unsupported)
    ));
    assert_eq!(
        o.admit_annotation_read(
            &d,
            0,
            target,
            ReadStrength::AtomicResourceSnapshot,
            None,
            &mut r
        ),
        Err(ResourceError::Unsupported)
    );
    assert_eq!(o.resource_usage().issued_reads_lifetime, 1); // Original fixture only.
}
#[test]
fn full_catalog_capture_is_charged_and_an_undersized_reader_cannot_issue_another_origin() {
    let (mut o, p, d, mut r, a) = setup();
    let target = o
        .bind_catalog_annotation_read_target_v1(
            p.annotation_read_target(b"comments").unwrap(),
            &a,
            &binding(),
            &mut r,
        )
        .unwrap();
    let catalog_bytes = target.catalog.as_ref().unwrap().native_bytes().unwrap();
    let id = o
        .admit_annotation_read(&d, 0, target, ReadStrength::CapturedRead, None, &mut r)
        .unwrap();
    let total = o.resource_usage().held_read_bytes;
    assert!(total > catalog_bytes);
    o.stop_annotation_read(&id, &mut r).unwrap();
    o.retire_annotation_read(&id, &mut r).unwrap();
    o.resources.limits.as_mut().unwrap().max_held_read_bytes = total - 1;
    let target = o
        .bind_catalog_annotation_read_target_v1(
            p.annotation_read_target(b"comments").unwrap(),
            &a,
            &binding(),
            &mut r,
        )
        .unwrap();
    let usage = o.resource_usage();
    assert_eq!(
        o.admit_annotation_read(&d, 0, target, ReadStrength::CapturedRead, None, &mut r),
        Err(ResourceError::Limit)
    );
    assert_eq!(o.resource_usage(), usage);
}
