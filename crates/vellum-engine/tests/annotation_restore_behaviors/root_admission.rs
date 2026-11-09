use super::*;
fn ready() -> (Fixture, AnnotationRestoration) {
    let mut f = Fixture::new();
    let (_, _, capture) = captured(&mut f);
    let port = f
        .owner
        .register_annotation_restorer(&mut f.recorder)
        .unwrap();
    let witness = source(&mut f);
    let package = port
        .seal(capture, vec![witness], vec![row(&port, &f.doc)])
        .unwrap();
    (f, package)
}
#[test]
fn restoration_checks_slots_issuer_links_bodies_and_imported_identity_keys_before_any_installation()
{
    let (mut probe, package) = ready();
    let pair = probe.pair();
    probe
        .owner
        .restore_annotations(package, pair, &mut probe.recorder)
        .unwrap();
    let exact = probe
        .owner
        .semantic_root_usage(&mut probe.recorder)
        .unwrap()
        .total_bytes;
    for delta in [-1isize, 0, 1] {
        let (mut f, package) = ready();
        f.owner
            .configure_semantic_root_budget(
                exact.checked_add_signed(delta).unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let before = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
        let usage = f.owner.annotation_usage();
        let generation = f.owner.generation();
        let pair = f.pair();
        let result = f.owner.restore_annotations(package, pair, &mut f.recorder);
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(f.owner.generation(), generation);
            let after = f.owner.annotation_usage();
            assert_eq!(after.issued_lifetime, usage.issued_lifetime);
            assert_eq!(after.retained_records, usage.retained_records);
            assert_eq!(after.present_records, usage.present_records);
            assert_eq!(after.body_bytes, usage.body_bytes);
            // Consuming/dropping the refused transient package can release its
            // source witness; that is not a partial authored publication.
            assert_eq!(
                f.owner.semantic_root_usage(&mut f.recorder).unwrap(),
                before
            );
        } else {
            assert!(result.unwrap().changed);
            assert_eq!(
                f.owner
                    .semantic_root_usage(&mut f.recorder)
                    .unwrap()
                    .total_bytes,
                exact
            );
        }
    }
}
