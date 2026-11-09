use super::*;
fn ready() -> Loaded {
    let mut f = Loaded::new(true);
    f.attach();
    f
}
#[test]
fn loaded_baseline_adoption_admits_actual_payload_before_marking_the_read_adopted() {
    let mut probe = ready();
    probe.adopt().unwrap();
    let exact = probe
        .f
        .owner
        .semantic_root_usage(&mut probe.f.recorder)
        .unwrap()
        .total_bytes;
    for delta in [-1isize, 0, 1] {
        let mut f = ready();
        f.f.owner
            .configure_semantic_root_budget(
                exact.checked_add_signed(delta).unwrap(),
                &mut f.f.recorder,
            )
            .unwrap();
        let before = f.f.owner.semantic_root_usage(&mut f.f.recorder).unwrap();
        let generation = f.f.owner.generation();
        let result = f.adopt();
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(f.f.owner.generation(), generation);
            assert_eq!(
                f.f.owner.semantic_root_usage(&mut f.f.recorder).unwrap(),
                before
            );
            assert!(
                !f.f.owner
                    .annotation_restoration_outcome(&f.read, &mut f.f.recorder)
                    .unwrap()
                    .loaded_baseline_adopted
            );
        } else {
            assert_eq!(result, Ok(true));
            assert_eq!(
                f.f.owner
                    .semantic_root_usage(&mut f.f.recorder)
                    .unwrap()
                    .total_bytes,
                exact
            );
            assert_eq!(f.adopt(), Ok(false));
        }
    }
}
