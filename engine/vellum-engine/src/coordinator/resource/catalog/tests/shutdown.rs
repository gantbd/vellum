use super::*;
use crate::coordinator::shutdown::tests::install;

#[test]
fn shutdown_blocks_initial_catalog_installation_but_keeps_matching_configuration_and_reads() {
    for configured in [false, true] {
        let mut o = bare();
        let mut r = Recorder::disabled();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        if configured {
            o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
                .unwrap();
        }
        install(&mut o, &mut r);
        let before = (o.native_root_identity().unwrap(), o.retention_usage());
        assert_eq!(
            o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r),
            if configured {
                Ok(false)
            } else {
                Err(ResourceError::Unavailable)
            }
        );
        assert_eq!(
            (o.native_root_identity().unwrap(), o.retention_usage()),
            before
        );
        if configured {
            assert_eq!(
                o.resource_catalog_registration_v1(
                    &address(ResourceCatalogTableV1::Read, "absent"),
                    &mut r
                ),
                Ok(None)
            );
        }
        assert!(!o.is_faulted());
    }
}

#[test]
fn shutdown_blocks_catalog_preparation_and_final_admission_for_changed_and_unchanged_plans() {
    for changed in [false, true] {
        let mut o = bare();
        let mut r = Recorder::disabled();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
            .unwrap();
        let changes = if changed {
            vec![ResourceCatalogChangeV1 {
                address: address(ResourceCatalogTableV1::Read, "new"),
                expected: CatalogExpectationV1::Absent,
                action: ResourceCatalogActionV1::Install(config(
                    ResourceCatalogTableV1::Read,
                    "key",
                )),
            }]
        } else {
            vec![]
        };
        let plan = o
            .prepare_resource_catalog_v1(&changes, &facts(), &mut r)
            .unwrap();
        install(&mut o, &mut r);
        let before = o.native_root_identity().unwrap();
        assert!(matches!(
            o.prepare_resource_catalog_v1(&changes, &facts(), &mut r),
            Err(ResourceError::Unavailable)
        ));
        assert_eq!(
            o.commit_resource_catalog_v1(plan, &facts(), &mut r),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(o.native_root_identity().unwrap(), before);
        assert!(!o.is_faulted());
    }
}
