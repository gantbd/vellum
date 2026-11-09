use super::*;
use crate::coordinator::shutdown::tests::install;

#[test]
fn shutdown_preserves_matching_registration_basis_but_forbids_new_or_renewed_issuance() {
    for capacity in [None, Some(1), Some(64)] {
        let (mut o, mut r, c) = setup(&[], profile());
        let original = capture(&mut o, &c, &mut r);
        install(&mut o, &mut r);
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let before = (
            o.registration_basis_usage_v1().unwrap(),
            o.retention_usage(),
        );
        let same = capture(&mut o, &c, &mut r);
        assert_eq!(same.basis, original.basis);
        assert_eq!(same.condition, original.condition);
        assert_eq!(
            o.inspect_registration_basis_v1(&original.basis, &c, &current(&c), &mut r),
            Ok(original.condition)
        );
        assert!(matches!(
            o.capture_registration_basis_v1(
                &c,
                &address(ResourceCatalogTableV1::Read, "other"),
                &current(&c),
                &mut r
            ),
            Err(ResourceError::Unavailable)
        ));
        assert_eq!(
            (
                o.registration_basis_usage_v1().unwrap(),
                o.retention_usage()
            ),
            before
        );
        o.retire_registration_basis_v1(&original.basis, &control(), &mut r)
            .unwrap();
        let retired = o.registration_basis_usage_v1().unwrap();
        assert!(matches!(
            o.capture_registration_basis_v1(
                &c,
                &address(ResourceCatalogTableV1::Read, "target"),
                &current(&c),
                &mut r
            ),
            Err(ResourceError::Unavailable)
        ));
        assert_eq!(o.registration_basis_usage_v1().unwrap(), retired);
        o.retire_registration_basis_issuer_v1(&control(), &mut r)
            .unwrap();
        assert!(o.retention_usage().reserved > 0);
        assert!(!o.is_faulted());
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for disposition in [Disposition::Completed, Disposition::Refused] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::RegistrationBasisCapture
                            && e.disposition == disposition)
                );
            }
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn shutdown_blocks_initial_registration_basis_issuer_but_not_matching_setup() {
    for configured in [false, true] {
        let mut o = bare();
        let mut r = Recorder::disabled();
        let c = scope();
        o.configure_resource_catalog_v1(&[], limits(), &facts(), &mut r)
            .unwrap();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        if configured {
            o.configure_registration_bases_v1(&c, limits(), profile(), &setup_facts(), &mut r)
                .unwrap();
        }
        install(&mut o, &mut r);
        let before = (o.native_root_identity().unwrap(), o.retention_usage());
        assert_eq!(
            o.configure_registration_bases_v1(&c, limits(), profile(), &setup_facts(), &mut r),
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
    }
}
