use super::*;
use crate::coordinator::shutdown::tests::install;
fn empty() -> DiscoveryCutInputV1<'static> {
    DiscoveryCutInputV1 {
        entries: &[],
        target: None,
        advice_present: false,
    }
}

#[test]
fn shutdown_blocks_new_discovery_owner_without_renewing_matching_setup() {
    for configured in [false, true] {
        let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
        let s = scope();
        let f = facts();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        if configured {
            o.configure_discovery_v1(&s, limits(), &f, &mut r).unwrap();
        }
        install(&mut o, &mut r);
        let before = (o.native_root_identity().unwrap(), o.retention_usage());
        assert_eq!(
            o.configure_discovery_v1(&s, limits(), &f, &mut r),
            if configured {
                Ok(())
            } else {
                Err(Error::Unavailable)
            }
        );
        assert_eq!(
            (o.native_root_identity().unwrap(), o.retention_usage()),
            before
        );
    }
}

#[test]
fn shutdown_blocks_discovery_establishment_without_consuming_visibility_needed_by_existing_reads() {
    let (mut o, mut r, _) = crate::coordinator::serialization::tests::setup();
    let s = scope();
    let f = facts();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_discovery_v1(&s, limits(), &f, &mut r).unwrap();
    let cut = o.prepare_discovery_cut_v1(&s, empty(), &f, &mut r).unwrap();
    let original = o
        .acquire_discovery_v1(&s, &cut, &f, &mut r)
        .unwrap()
        .context;
    install(&mut o, &mut r);
    // Existing queries require independently current visibility evaluation.
    // A refreshed evaluation is not permission to establish a new context.
    let cut = o.prepare_discovery_cut_v1(&s, empty(), &f, &mut r).unwrap();
    let before = (o.discovery_usage_v1().unwrap(), o.retention_usage());
    assert!(matches!(
        o.acquire_discovery_v1(&s, &cut, &f, &mut r),
        Err(Error::Unavailable)
    ));
    assert_eq!(
        (o.discovery_usage_v1().unwrap(), o.retention_usage()),
        before
    );
    assert!(matches!(
        o.query_discovery_v1(
            &original,
            &s,
            &cut,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None
            },
            &f,
            &mut r
        ),
        Ok(DiscoveryReplyV1::Page { .. })
    ));
    o.stop_discovery_context_v1(&original, &s, &f, &mut r)
        .unwrap();
    o.retire_discovery_context_v1(&original, &f, &mut r)
        .unwrap();
    o.stop_discovery_admission_v1(&f, &mut r).unwrap();
    o.retire_discovery_registry_v1(&f, &mut r).unwrap();
    assert!(o.retention_usage().reserved > 0);
    assert!(!o.is_faulted());
}
