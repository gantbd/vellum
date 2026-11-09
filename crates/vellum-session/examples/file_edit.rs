//! Explicit mutation: file_edit [--logical|--invocation] PATH START_BYTE END_BYTE EXPECTED INSERTED
//! Existing file only; arguments are literal UTF-8, not a command language.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(std::env::args_os().skip(1).collect())
}

#[cfg(target_os = "macos")]
fn run(mut args: Vec<std::ffi::OsString>) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;
    use support::LocalHost;
    use vellum_engine::coordinator::{HistoryDirection, TrustedDocumentReclamationV1};
    use vellum_engine::resource::{
        Acknowledgment, Application, Protection, SaveRequirements, Synchronization,
    };
    use vellum_engine::source::Replacement;

    if args.len() == 1 && args[0] == "--help" {
        println!(
            "file_edit [--logical|--invocation] PATH START_BYTE END_BYTE EXPECTED INSERTED\nExplicitly edits and saves an existing local file (64 KiB example-host limit).\n--logical uses native view/selection semantics.\n--invocation verifies original native-ID replay without re-execution in this process.\nThese modes are exclusive; the default exact-byte route is unchanged. No cross-process idempotency is claimed."
        );
        return Ok(());
    }
    let logical = args.len() == 6 && args[0] == "--logical";
    let invoked = args.len() == 6 && args[0] == "--invocation";
    if logical || invoked {
        args.remove(0);
    }
    if args.len() != 5 {
        return Err(
            "usage: file_edit [--logical|--invocation] PATH START_BYTE END_BYTE EXPECTED INSERTED"
                .into(),
        );
    }
    let text_arg = |index: usize| args[index].to_str().ok_or("edit arguments must be UTF-8");
    let start = text_arg(1)?.parse::<usize>()?;
    let end = text_arg(2)?.parse::<usize>()?;
    let path = Path::new(&args[0]);
    let mut config = support::config(256, true);
    if invoked {
        config.engine.invocation = Some(vellum_engine::coordinator::InvocationLimits {
            max_lifetime_issued: 1,
            max_retained: 1,
            max_intent_bytes: 262144,
        });
    }
    let mut session = LocalHost::new(config)?;
    let document = session.open(path)?;
    let version = session.read(&document)?.version;
    let mut native_view = None;
    let mut native_invocation = None;
    let committed_group = if logical {
        use vellum_engine::{
            attachment::Affinity,
            coordinates::BytePosition,
            coordinator::{LogicalViewContext, LogicalViewLimits, SelectionAfterV1},
            selection::{Selection, SelectionBudget},
        };
        if session.read(&document)?.source.text().get(start..end) != Some(text_arg(3)?) {
            return Err(
                "original selection/expected fragment refused; no edit or save attempted".into(),
            );
        }
        // Explicit example-host profile, not a selected product capacity.
        session.configure_logical_views(LogicalViewLimits {
            max_live_views: 1,
            max_issued_views: 1,
            max_total_selections: 1,
            normalization: SelectionBudget {
                max_input_selections: 1,
                max_output_selections: 1,
                max_work: 65536,
            },
            max_mapping_steps: 65536,
        })?;
        let view = session.create_logical_view()?;
        let detached = session.logical_view(&view)?.stamp;
        let attached = session.attach_logical_view(
            &view,
            detached,
            &version,
            &[Selection {
                anchor: BytePosition(start),
                head: BytePosition(end),
                affinity: Affinity::After,
            }],
            0,
        )?;
        let context = LogicalViewContext {
            view: view.clone(),
            source: version.clone(),
            stamp: attached.stamp,
        };
        native_view = Some(view);
        session
            .replace_logical_selections(&context, text_arg(4)?, SelectionAfterV1::CaretsAfter)?
            .history_group
    } else if invoked {
        use vellum_engine::coordinator::{DocumentEdits, InvocationOutcome, SourceIntent};
        let intent = SourceIntent {
            writes: vec![DocumentEdits {
                expected: version.clone(),
                edits: vec![Replacement {
                    start,
                    end,
                    expected: text_arg(3)?.into(),
                    inserted: text_arg(4)?.into(),
                }],
            }],
            reads: vec![],
        };
        let id = session.issue_invocation()?;
        session.submit_source_invocation(&id, &intent)?;
        session.run_invocation(&id)?;
        let group = match session.invocation_outcome(&id)? {
            InvocationOutcome::Committed(outcome) => outcome.history_group.clone(),
            _ => return Err("native invocation did not commit; no save attempted".into()),
        };
        let after = session.read(&document)?.version;
        session.submit_source_invocation(&id, &intent)?;
        session.run_invocation(&id)?;
        if session.read(&document)?.version != after {
            return Err("matching replay changed source; no save attempted".into());
        }
        native_invocation = Some(id);
        group
    } else {
        session
            .edit(
                &version,
                vec![Replacement {
                    start,
                    end,
                    expected: text_arg(3)?.into(),
                    inserted: text_arg(4)?.into(),
                }],
            )?
            .history_group
    };
    if committed_group.is_some()
        && (!session.reverse(&document, HistoryDirection::Undo)?
            || !session.reverse(&document, HistoryDirection::Redo)?)
    {
        return Err("history round trip did not complete; no save attempted".into());
    }
    let expected_bytes = session
        .read(&document)?
        .source
        .encode()
        .map_err(|_| "source encoding refused")?;
    let version = session.read(&document)?.version;
    let save = session.queue_save(
        &version,
        SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Synchronized,
        },
    )?;
    let report = session.service_save(&document, &save)?;
    println!(
        "application: {:?}; synchronization: {:?}; quiescent: {}; cleanup failed: {}",
        report.outcome.application,
        report.outcome.synchronization,
        report.outcome.quiescent,
        report.cleanup_failed
    );
    if report.failure.is_some()
        || report.cleanup_failed
        || report.outcome.application != Application::Written
        || report.outcome.synchronization != Synchronization::Succeeded
        || !report.outcome.quiescent
        || !report.outcome.no_future_effects
    {
        return Err("save not fully qualified; application facts above must not be interpreted as a rollback".into());
    }
    session.publish_save_baseline(&save)?;
    let mut reopened = LocalHost::new(support::config(0, false))?;
    let reopened_document = reopened.open(path)?;
    if reopened
        .read(&reopened_document)?
        .source
        .encode()
        .map_err(|_| "reopened source encoding refused")?
        != expected_bytes
    {
        return Err("reopened bytes differ from captured source".into());
    }
    // This synchronous trusted example owns no reader/view loans or workers.
    // Save and reopen are verified before disposition; actual native pins still
    // independently gate reclamation. No file deletion or global shutdown claim.
    let facts = TrustedDocumentReclamationV1 {
        authorized: Some(true),
        authored_disposition_ready: Some(true),
        session_references_released: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    let reopened_version = reopened.read(&reopened_document)?.version;
    let disposed = reopened.dispose(&reopened_version)?;
    let unbound = reopened.release_source_binding(&reopened_document, disposed)?;
    let released = reopened.release_provenance(&reopened_document, unbound)?;
    reopened.reclaim(&reopened_document, released, &facts)?;

    session.stop_save(&save)?;
    let settled = session.save_outcome(&save)?;
    if !settled.no_future_effects || !settled.quiescent {
        return Err("original save is not settled; cleanup incomplete".into());
    }
    if let Some(view) = native_view {
        let current = session.logical_view(&view)?.stamp;
        let detached = session.detach_logical_view(&view, current)?;
        session.retire_logical_view(&view, detached.stamp)?;
    }
    if let Some(invocation) = native_invocation {
        session.retire_invocation(&invocation)?;
    }
    let disposed = session.dispose(&version)?;
    let unbound = session.release_source_binding(&document, disposed)?;
    if let Some(group) = committed_group {
        session.retire_history(&group)?;
    }
    let released = session.release_provenance(&document, unbound)?;
    session.retire_save(&save)?;
    session.reclaim(&document, released, &facts)?;
    let mut records = 0;
    while session.pop_diagnostic().is_some() {
        records += 1;
    }
    println!(
        "reopen and document-owner cleanup verified; diagnostic records: {records}; drops: {}",
        session.dropped_diagnostics()
    );
    // This discharges the tracked document owners, not the advanced engine-wide
    // shutdown protocol. On errors above, no cleanup completion is asserted.
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("file_edit requires the currently supported macOS filesystem adapter");
    std::process::exit(1);
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::run;
    use std::{
        ffi::OsString,
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            loop {
                let path = std::env::temp_dir().join(format!(
                    "vellum-cli-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::create_dir(&path) {
                    Ok(()) => {
                        fs::write(path.join("source"), "\u{feff}A\r\nGrüße\n").unwrap();
                        return Self(path);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(e) => panic!("fixture: {e}"),
                }
            }
        }
        fn args(&self, expected: &str) -> Vec<OsString> {
            vec![
                self.0.join("source").into_os_string(),
                "0".into(),
                "1".into(),
                expected.into(),
                "Ä".into(),
            ]
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_file(self.0.join("source")).unwrap();
            fs::remove_dir(&self.0).unwrap();
        }
    }
    #[test]
    fn cli_workflow_preserves_bytes_and_discharges_document_owners() {
        let f = Fixture::new();
        run(f.args("A")).unwrap();
        assert_eq!(
            fs::read(f.0.join("source")).unwrap(),
            "\u{feff}Ä\r\nGrüße\n".as_bytes()
        );
    }
    #[test]
    fn refused_edit_never_saves_or_claims_cleanup_success() {
        let f = Fixture::new();
        let before = fs::read(f.0.join("source")).unwrap();
        assert!(run(f.args("wrong")).is_err());
        assert_eq!(fs::read(f.0.join("source")).unwrap(), before);
    }
    #[test]
    fn native_selection_cli_preserves_original_bytes_and_rejects_wrong_fragments() {
        for expected in ["A", "wrong"] {
            let f = Fixture::new();
            let mut args = f.args(expected);
            args.insert(0, "--logical".into());
            let result = run(args);
            assert_eq!(result.is_ok(), expected == "A");
            let first = if expected == "A" { "Ä" } else { "A" };
            assert_eq!(
                fs::read(f.0.join("source")).unwrap(),
                format!("\u{feff}{first}\r\nGrüße\n").as_bytes()
            );
        }
    }
    #[test]
    fn native_invocation_cli_replay_preserves_bytes_and_refused_work_never_saves() {
        for expected in ["A", "wrong"] {
            let f = Fixture::new();
            let mut args = f.args(expected);
            args.insert(0, "--invocation".into());
            assert_eq!(run(args).is_ok(), expected == "A");
            let first = if expected == "A" { "Ä" } else { "A" };
            assert_eq!(
                fs::read(f.0.join("source")).unwrap(),
                format!("\u{feff}{first}\r\nGrüße\n").as_bytes()
            );
        }
    }
}
