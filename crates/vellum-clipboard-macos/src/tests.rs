use super::*;

struct Fake {
    writes: usize,
    reads: usize,
    write_result: Result<bool, ()>,
    read_result: Result<Option<String>, Error>,
}
fn provider(timed: bool) -> Provider<Fake> {
    Provider::new(
        Fake {
            writes: 0,
            reads: 0,
            write_result: Ok(true),
            read_result: Ok(None),
        },
        Limits {
            max_plain_bytes: 32,
        },
        timed,
    )
    .unwrap()
}
impl Backend for Fake {
    fn write(&mut self, _: &str) -> Result<bool, ()> {
        self.writes += 1;
        self.write_result
    }
    fn read(&mut self, _: usize) -> Result<Option<String>, Error> {
        self.reads += 1;
        self.read_result.clone()
    }
}

#[test]
fn export_limit_refuses_before_any_platform_effect() {
    let mut p = provider(false);
    assert_eq!(
        p.export(&"x".repeat(33)),
        ExportOutcome::Refused(Error::Limit)
    );
    assert_eq!(p.backend.writes, 0);
    assert_eq!(p.usage.export_refused, 1);
    assert_eq!(p.usage.export_service_total, None);
    assert!(Provider::new(p.backend, Limits { max_plain_bytes: 0 }, false).is_err());
    assert!(
        Provider::new(
            provider(false).backend,
            Limits {
                max_plain_bytes: usize::MAX
            },
            false
        )
        .is_err()
    );
}

#[test]
fn setter_rejection_and_platform_exception_never_become_acceptance() {
    let mut p = provider(false);
    assert_eq!(p.export("plain"), ExportOutcome::Accepted);
    p.backend.write_result = Ok(false);
    assert_eq!(p.export("plain"), ExportOutcome::RejectedAfterClear);
    p.backend.write_result = Err(());
    assert_eq!(p.export("plain"), ExportOutcome::Unknown);
    assert_eq!(p.backend.writes, 3); // no hidden retries
    assert_eq!(
        (
            p.usage.export_accepted,
            p.usage.export_rejected,
            p.usage.export_unknown
        ),
        (1, 1, 1)
    );
}

#[test]
fn missing_empty_multilingual_and_literal_control_data_remain_distinct() {
    let mut p = provider(false);
    assert_eq!(p.read(), Ok(None));
    p.backend.read_result = Ok(Some(String::new()));
    assert_eq!(p.read(), Ok(Some(String::new())));
    let raw = "\u{feff}Grüße\r\n\0\x1b$(literal)";
    p.backend.read_result = Ok(Some(raw.into()));
    assert_eq!(p.read(), Ok(Some(raw.into())));
    assert_eq!((p.usage.read_no_data, p.usage.read_accepted), (1, 2));
}

#[test]
fn changed_unavailable_invalid_or_oversized_reads_are_never_insertion_payloads() {
    let mut p = provider(false);
    for error in [
        Error::ChangedDuringRead,
        Error::Unavailable,
        Error::InvalidText,
    ] {
        p.backend.read_result = Err(error);
        assert_eq!(p.read(), Err(error));
    }
    p.backend.read_result = Ok(Some("x".repeat(33)));
    assert_eq!(p.read(), Err(Error::Limit));
    assert_eq!(p.usage.read_refused, 4);
}

#[test]
fn optional_timing_and_saturated_counts_do_not_change_outcomes_or_expose_content() {
    let mut off = provider(false);
    let mut on = provider(true);
    on.usage.export_offers = u64::MAX;
    assert_eq!(off.export("private"), on.export("private"));
    assert_eq!(off.read(), on.read());
    assert_eq!(on.usage.export_offers, u64::MAX);
    assert_eq!(off.usage.export_service_total, None);
    assert_eq!(off.usage.export_untimed_samples, 1);
    assert_eq!(off.usage.read_untimed_samples, 1);
    assert_eq!(on.usage.export_timed_samples, 1);
    assert_eq!(on.usage.read_timed_samples, 1);
    assert!(on.usage.export_service_total.is_some());
    assert!(on.usage.read_service_total.is_some());
    assert!(!format!("{:?}", on.usage).contains("private"));
}

#[test]
fn public_plain_text_debug_requires_explicit_payload_disclosure() {
    let raw = "private clipboard\r\n\0";
    let text = PlainText(raw.into());
    assert_eq!(format!("{text:?}"), "PlainText(<redacted>)");
    assert_eq!(text.as_str(), raw);
    assert_eq!(text.into_string(), raw);
}
