//! The delivered executable must exercise real public owners, not a mock facade.
#[path = "../examples/headless_edit.rs"]
#[allow(dead_code)]
mod example;

#[test]
fn runnable_edit_discovery_result_and_cleanup_are_capture_independent() {
    for (capacity, timed) in [(0, false), (128, false), (128, true), (1, true)] {
        let report = example::run(capacity, timed);
        assert_eq!(report.text, "Hello, agents!\n");
        assert_eq!(report.issued, 1, "matching replay must not execute again");
        assert_eq!(report.retained_invocations, 0);
        assert_eq!(report.retained_history, 0);
        assert_eq!(report.reserved_bytes, 0);
        if capacity == 1 {
            assert!(report.dropped > 0);
        } else if capacity == 0 {
            assert_eq!(report.dropped, 0, "disabled capture is not overflow");
        }
    }
}
