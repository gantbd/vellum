#![cfg(target_os = "macos")]
#[path = "../examples/support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vellum_annotation_json::*;
use vellum_engine::attachment::{Affinity, AttachmentTarget, PointProfile};
use vellum_engine::coordinator::{
    AnnotationLimits, AnnotationMutation, AnnotationSnapshot, DocumentVersion, SourceTarget,
    TargetBasis,
};
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::resource::{
    Acknowledgment, Application, Protection, SaveRequirements, Synchronization,
};
use vellum_engine::source::Replacement;
use vellum_session::host::{LocalHost, SessionConfig};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let p = std::env::temp_dir().join(format!(
                "vellum-sdk-json-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&p) {
                Ok(()) => {
                    fs::write(p.join("source"), "\u{feff}ABC\r\n".as_bytes()).unwrap();
                    fs::write(p.join("notes"), b"old destination").unwrap();
                    return Self(p);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture: {e}"),
            }
        }
    }
    fn source(&self) -> PathBuf {
        self.0.join("source")
    }
    fn notes(&self) -> PathBuf {
        self.0.join("notes")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(self.source()).unwrap();
        fs::remove_file(self.notes()).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn config(capacity: usize) -> SessionConfig {
    let mut c = support::config(capacity, true);
    c.filesystem.max_resources = 2;
    c.engine.annotations = Some(AnnotationLimits {
        max_issued: 16,
        max_records: 16,
        max_body_bytes: 1024,
        max_retained_body_bytes: 8192,
        max_mapping_steps: 8192,
        max_live_snapshots: 8,
        max_snapshot_bytes: 16384,
    });
    c
}
fn limits() -> CodecLimits {
    CodecLimits {
        max_input_bytes: 16384,
        max_output_bytes: 16384,
        max_nodes: 2048,
        max_depth: 32,
        max_string_bytes: 16384,
        max_records: 16,
        max_integer_digits: 20,
        max_body_bytes: 1024,
        max_total_body_bytes: 8192,
        max_associations: 32,
        max_source_bytes: 65536,
    }
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Synchronized,
    }
}
fn populate(host: &mut LocalHost, version: &DocumentVersion) -> AnnotationSnapshot {
    let mutations: Vec<_> = [PointProfile::Position, PointProfile::Content]
        .into_iter()
        .map(|profile| AnnotationMutation::Create {
            body: "Inert comment: Grüße".into(),
            target: SourceTarget {
                source: version.clone(),
                target: AttachmentTarget::Point {
                    offset: 1,
                    affinity: Affinity::Before,
                    profile,
                },
                basis: TargetBasis::Before,
            },
        })
        .collect();
    host.commit_compound(&[], &[], &mutations).unwrap();
    // This example's initial set has exactly this one compound publication.
    host.capture_annotations(version, 1).unwrap()
}
fn associations(snapshot: &AnnotationSnapshot) -> CaptureAssociations {
    CaptureAssociations {
        document: DocumentAssociation {
            native: snapshot.source().document.clone(),
            identity: DocumentIdentity {
                namespace: "test-host".into(),
                value: "document".into(),
            },
        },
        records: snapshot
            .records()
            .iter()
            .enumerate()
            .map(|(i, r)| RecordAssociation {
                native: r.annotation.clone(),
                identity: RecordIdentity {
                    namespace: "test-host".into(),
                    value: format!("note-{i}"),
                    incarnation: 1,
                },
            })
            .collect(),
        sources: vec![SourceAssociation {
            native: snapshot.source().clone(),
            evidence: SourceEvidence {
                domain: "test-host-canonical".into(),
                token: "source-0".into(),
                byte_length: snapshot.source_text().len() as u64,
            },
            text: snapshot.source_text().into(),
        }],
    }
}

#[test]
fn separate_json_save_preserves_profiles_and_does_not_turn_source_save_into_pair_atomicity() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new();
        let original = fs::read(f.source()).unwrap();
        let mut host = LocalHost::new(config(capacity)).unwrap();
        let doc = host.open(&f.source()).unwrap();
        let destination = host.bind_annotations(&doc, &f.notes()).unwrap();
        let version = host.read(&doc).unwrap().version;
        let snapshot = populate(&mut host, &version);
        let aliases = associations(&snapshot);
        let encoder = host.register_annotation_encoder().unwrap();
        let encoded = host
            .encode_annotation_json(&snapshot, &aliases, limits(), &encoder)
            .unwrap();
        let task = host
            .queue_annotation_save(&destination, encoded, requirements())
            .unwrap();
        assert!(host.annotation_baseline(&doc).unwrap().is_none());
        let report = host.service_annotation_save(&destination, &task).unwrap();
        assert!(report.failure.is_none() && !report.cleanup_failed);
        assert_eq!(report.outcome.application, Application::Written);
        assert_eq!(report.outcome.synchronization, Synchronization::Succeeded);
        assert!(report.outcome.quiescent && report.outcome.no_future_effects);
        assert!(host.publish_save_baseline(&task).is_err());
        assert!(
            host.publish_annotation_save_baseline(&destination, &task)
                .unwrap()
        );
        let fact = host.annotation_baseline(&doc).unwrap().unwrap();
        assert_eq!(fact.producer, task);
        assert_eq!(fact.source, version);
        assert_eq!(fs::read(f.source()).unwrap(), original);
        let bytes = fs::read(f.notes()).unwrap();
        let context = DecodeContext {
            document: aliases.document.identity.clone(),
            source: aliases.sources[0].evidence.clone(),
            text: snapshot.source_text().into(),
        };
        let decoded = decode(&bytes, &context, limits(), &mut Recorder::disabled()).unwrap();
        assert_eq!(decoded.records.len(), 2);
        assert!(matches!(
            decoded.records[0].authored_target,
            Target::Point {
                profile: PointMeaning::Position,
                ..
            }
        ));
        assert!(matches!(
            decoded.records[1].authored_target,
            Target::Point {
                profile: PointMeaning::Content,
                ..
            }
        ));
        assert_eq!(decoded.records[0].body, "Inert comment: Grüße");
        // Ordinary source save must now provide real distinct-resource evidence
        // at admission and dispatch. The JSON is not rewritten with the source.
        host.edit(
            &version,
            vec![Replacement {
                start: 0,
                end: 1,
                expected: "A".into(),
                inserted: "Ä".into(),
            }],
        )
        .unwrap();
        let current = host.read(&doc).unwrap().version;
        let source_task = host.queue_save(&current, requirements()).unwrap();
        let source_report = host.service_save(&doc, &source_task).unwrap();
        assert_eq!(source_report.outcome.application, Application::Written);
        assert_eq!(
            source_report.outcome.synchronization,
            Synchronization::Succeeded
        );
        host.publish_save_baseline(&source_task).unwrap();
        assert_eq!(fs::read(f.source()).unwrap(), "\u{feff}ÄBC\r\n".as_bytes());
        assert_eq!(fs::read(f.notes()).unwrap(), bytes);
        assert!(
            host.service_annotation_save(&destination, &source_task)
                .is_err()
        );
        assert!(host.service_annotation_save(&destination, &task).is_err());
        if capacity == 1 {
            assert!(host.dropped_diagnostics() > 0);
        }
        if capacity == 256 {
            let mut boundaries = Vec::new();
            while let Some(o) = host.pop_diagnostic() {
                boundaries.push(o.boundary);
            }
            for b in [
                Boundary::SubstrateReplacement,
                Boundary::SubstrateSynchronization,
            ] {
                assert!(boundaries.contains(&b));
            }
        }
    }
}

#[test]
fn alias_and_missing_destinations_refuse_without_creating_or_overwriting_source() {
    let f = Fixture::new();
    let mut host = LocalHost::new(config(128)).unwrap();
    let doc = host.open(&f.source()).unwrap();
    let original = fs::read(f.source()).unwrap();
    assert!(host.bind_annotations(&doc, &f.source()).is_err());
    let missing = f.0.join("missing");
    assert!(host.bind_annotations(&doc, &missing).is_err());
    assert!(!missing.exists());
    assert_eq!(fs::read(f.source()).unwrap(), original);
    assert_eq!(fs::read(f.notes()).unwrap(), b"old destination");
}

#[test]
fn annotation_binding_clones_share_only_actual_written_checkpoints_not_fresh_probe_adoption() {
    let f = Fixture::new();
    let mut host = LocalHost::new(config(256)).unwrap();
    let document = host.open(&f.source()).unwrap();
    let destination = host.bind_annotations(&document, &f.notes()).unwrap();
    let clone = destination.clone();
    let version = host.read(&document).unwrap().version;
    let initial = populate(&mut host, &version);
    let encoder = host.register_annotation_encoder().unwrap();
    let encoded = host
        .encode_annotation_json(&initial, &associations(&initial), limits(), &encoder)
        .unwrap();
    let first = host
        .queue_annotation_save(&destination, encoded, requirements())
        .unwrap();
    let report = host.service_annotation_save(&clone, &first).unwrap();
    assert_eq!(report.outcome.application, Application::Written);
    host.publish_annotation_save_baseline(&destination, &first)
        .unwrap();
    assert!(host.service_annotation_save(&destination, &first).is_err());
    drop(initial);
    let current = host.capture_annotations(&version, 1).unwrap();
    let encoded = host
        .encode_annotation_json(&current, &associations(&current), limits(), &encoder)
        .unwrap();
    let second = host
        .queue_annotation_save(&clone, encoded, requirements())
        .unwrap();
    let report = host.service_annotation_save(&destination, &second).unwrap();
    assert_eq!(report.outcome.application, Application::Written);
    host.publish_annotation_save_baseline(&clone, &second)
        .unwrap();
    drop(current);
    fs::write(f.notes(), b"external update").unwrap();
    let current = host.capture_annotations(&version, 1).unwrap();
    let encoded = host
        .encode_annotation_json(&current, &associations(&current), limits(), &encoder)
        .unwrap();
    assert!(
        host.queue_annotation_save(&destination, encoded, requirements())
            .is_err()
    );
    assert_eq!(fs::read(f.notes()).unwrap(), b"external update");
    assert_eq!(fs::read(f.source()).unwrap(), "\u{feff}ABC\r\n".as_bytes());
}

#[test]
fn changed_destination_preserves_original_attempt_and_never_silently_retries() {
    let f = Fixture::new();
    let mut host = LocalHost::new(config(128)).unwrap();
    let doc = host.open(&f.source()).unwrap();
    let destination = host.bind_annotations(&doc, &f.notes()).unwrap();
    let version = host.read(&doc).unwrap().version;
    let snapshot = populate(&mut host, &version);
    let encoder = host.register_annotation_encoder().unwrap();
    let encoded = host
        .encode_annotation_json(&snapshot, &associations(&snapshot), limits(), &encoder)
        .unwrap();
    let task = host
        .queue_annotation_save(&destination, encoded, requirements())
        .unwrap();
    fs::write(f.notes(), b"external author").unwrap();
    assert!(host.service_annotation_save(&destination, &task).is_err());
    assert!(host.save_outcome(&task).is_ok());
    assert!(
        host.publish_annotation_save_baseline(&destination, &task)
            .is_err()
    );
    assert_eq!(fs::read(f.notes()).unwrap(), b"external author");
    host.stop_save(&task).unwrap();
    let original = host.save_outcome(&task).unwrap();
    assert_eq!(original.application, Application::NotWritten);
    assert!(original.quiescent && original.no_future_effects);
    host.retire_save(&task).unwrap();
}

fn saved_notes(f: &Fixture) -> CaptureAssociations {
    let mut host = LocalHost::new(config(0)).unwrap();
    let doc = host.open(&f.source()).unwrap();
    let destination = host.bind_annotations(&doc, &f.notes()).unwrap();
    let version = host.read(&doc).unwrap().version;
    let snapshot = populate(&mut host, &version);
    let aliases = associations(&snapshot);
    let encoder = host.register_annotation_encoder().unwrap();
    let encoded = host
        .encode_annotation_json(&snapshot, &aliases, limits(), &encoder)
        .unwrap();
    let task = host
        .queue_annotation_save(&destination, encoded, requirements())
        .unwrap();
    let report = host.service_annotation_save(&destination, &task).unwrap();
    assert_eq!(report.outcome.application, Application::Written);
    assert_eq!(report.outcome.synchronization, Synchronization::Succeeded);
    assert!(
        host.publish_annotation_save_baseline(&destination, &task)
            .unwrap()
    );
    // A fresh owner below is not a protected shutdown certificate for this one.
    aliases
}

#[test]
fn fresh_owner_restores_original_profiles_with_explicit_approval_and_loaded_baseline() {
    use vellum_engine::resource::{AnnotationBaselineEvidence, ContentComparison};
    for capacity in [0, 1, 256] {
        let f = Fixture::new();
        let aliases = saved_notes(&f);
        let original_source = fs::read(f.source()).unwrap();
        let original_notes = fs::read(f.notes()).unwrap();
        let mut host = LocalHost::new(config(capacity)).unwrap();
        let doc = host.open(&f.source()).unwrap();
        let destination = host.bind_annotations(&doc, &f.notes()).unwrap();
        let version = host.read(&doc).unwrap().version;
        let witness = host.capture_annotations(&version, 0).unwrap();
        let attempt = host
            .queue_annotation_read(&destination, &version, 0)
            .unwrap();
        let report = host.service_annotation_read(&attempt).unwrap();
        assert!(report.failure.is_none());
        assert!(report.status.complete && report.status.quiescent && !report.status.restored);
        assert!(host.service_annotation_read(&attempt).is_err());
        let context = DecodeContext {
            document: aliases.document.identity.clone(),
            source: aliases.sources[0].evidence.clone(),
            text: witness.source_text().into(),
        };
        let mut approved = RestorationAssociations {
            document: DocumentAssociation {
                native: version.document.clone(),
                identity: aliases.document.identity.clone(),
            },
            records: aliases.records.iter().map(|r| r.identity.clone()).collect(),
            sources: vec![RestorationSourceAssociation {
                witness,
                evidence: aliases.sources[0].evidence.clone(),
            }],
        };
        let encoder = host.register_annotation_encoder().unwrap();
        let restorer = host.register_annotation_restorer(&encoder).unwrap();
        approved.records[0].incarnation += 1;
        let decoded = host
            .decode_annotation_json(&attempt, &context, limits())
            .unwrap();
        assert!(
            host.restore_annotation_json(decoded, &approved, &restorer, limits())
                .is_err()
        );
        assert_eq!(host.annotation_usage().present_records, 0);
        assert!(!host.annotation_read_status(&attempt).unwrap().restored);
        approved.records[0].incarnation -= 1;
        let decoded = host
            .decode_annotation_json(&attempt, &context, limits())
            .unwrap();
        let delivery_loan = host
            .decode_annotation_json(&attempt, &context, limits())
            .unwrap();
        let outcome = host
            .restore_annotation_json(decoded, &approved, &restorer, limits())
            .unwrap();
        assert!(outcome.changed && !outcome.loaded_baseline_adopted);
        assert_eq!(outcome.source, version);
        assert_eq!(
            host.annotation_restoration_outcome(&attempt).unwrap(),
            outcome
        );
        assert!(host.annotation(&aliases.records[0].native).is_err());
        let restored = host
            .capture_annotations(&version, outcome.set_version)
            .unwrap();
        assert_eq!(restored.records().len(), 2);
        assert!(matches!(
            restored.records()[0].authored.target,
            AttachmentTarget::Point {
                profile: PointProfile::Position,
                ..
            }
        ));
        assert!(matches!(
            restored.records()[1].authored.target,
            AttachmentTarget::Point {
                profile: PointProfile::Content,
                ..
            }
        ));
        assert_eq!(restored.records()[0].body(), "Inert comment: Grüße");
        assert!(host.annotation_baseline_evidence(&doc).unwrap().is_none());
        assert!(host.enable_annotation_read_baselines().unwrap());
        assert!(
            host.adopt_loaded_annotation_baseline(&attempt, 65536)
                .unwrap()
        );
        let Some(AnnotationBaselineEvidence::Loaded(fact)) =
            host.annotation_baseline_evidence(&doc).unwrap()
        else {
            panic!("expected loaded evidence")
        };
        assert_eq!(fact.origin, *attempt.native_id());
        let encoded = host
            .encode_annotation_json(&restored, &associations(&restored), limits(), &encoder)
            .unwrap();
        assert_eq!(
            host.compare_annotations(&doc, &encoded, 65536).unwrap(),
            ContentComparison::Unchanged
        );
        assert!(host.retire_annotation_read(&attempt).is_err());
        assert_eq!(host.resource_usage().retained_reads, 1);
        assert_eq!(host.resource_usage().outstanding, 0);
        assert!(host.retire_checkpointed_annotation_read(&attempt).is_err());
        assert!(host.disable_annotation_read_baselines().is_err());
        // A fresh snapshot captures publication after loaded-baseline adoption.
        // Write acknowledgment and baseline publication remain separate steps.
        let current = host
            .capture_annotations(&version, outcome.set_version)
            .unwrap();
        let checkpoint = host
            .encode_annotation_json(&current, &associations(&current), limits(), &encoder)
            .unwrap();
        let save = host
            .queue_annotation_save(&destination, checkpoint, requirements())
            .unwrap();
        assert!(host.retire_checkpointed_annotation_read(&attempt).is_err());
        let report = host.service_annotation_save(&destination, &save).unwrap();
        assert_eq!(report.outcome.application, Application::Written);
        assert_eq!(report.outcome.synchronization, Synchronization::Succeeded);
        assert!(host.retire_checkpointed_annotation_read(&attempt).is_err());
        host.publish_annotation_save_baseline(&destination, &save)
            .unwrap();
        host.stop_annotation_read(&attempt).unwrap();
        assert!(host.retire_checkpointed_annotation_read(&attempt).is_err());
        drop(delivery_loan);
        let writer = host.save_outcome(&save).unwrap();
        host.retire_checkpointed_annotation_read(&attempt).unwrap();
        assert!(host.annotation_read_status(&attempt).is_err());
        assert_eq!(host.save_outcome(&save).unwrap(), writer);
        assert!(host.disable_annotation_read_baselines().unwrap());
        assert_eq!(host.resource_usage().retained_reads, 0);
        assert_eq!(host.resource_usage().held_read_bytes, 0);
        assert_eq!(fs::read(f.source()).unwrap(), original_source);
        assert_eq!(fs::read(f.notes()).unwrap(), original_notes);
        if capacity == 1 {
            assert!(host.dropped_diagnostics() > 0);
        }
        if capacity == 256 {
            let mut boundaries = Vec::new();
            while let Some(o) = host.pop_diagnostic() {
                boundaries.push(o.boundary);
            }
            for b in [
                Boundary::AnnotationReadCapture,
                Boundary::AnnotationRestorationPublication,
                Boundary::LoadedBaselineAdoption,
            ] {
                assert!(boundaries.contains(&b), "missing {b:?}");
            }
        }
    }
}

#[test]
fn source_change_after_decode_refuses_native_activation_without_overwriting_authored_text() {
    let f = Fixture::new();
    let aliases = saved_notes(&f);
    let mut host = LocalHost::new(config(128)).unwrap();
    let doc = host.open(&f.source()).unwrap();
    let destination = host.bind_annotations(&doc, &f.notes()).unwrap();
    let version = host.read(&doc).unwrap().version;
    let witness = host.capture_annotations(&version, 0).unwrap();
    let attempt = host
        .queue_annotation_read(&destination, &version, 0)
        .unwrap();
    host.service_annotation_read(&attempt).unwrap();
    let context = DecodeContext {
        document: aliases.document.identity.clone(),
        source: aliases.sources[0].evidence.clone(),
        text: witness.source_text().into(),
    };
    let approved = RestorationAssociations {
        document: DocumentAssociation {
            native: version.document.clone(),
            identity: aliases.document.identity.clone(),
        },
        records: aliases.records.iter().map(|r| r.identity.clone()).collect(),
        sources: vec![RestorationSourceAssociation {
            witness,
            evidence: aliases.sources[0].evidence.clone(),
        }],
    };
    let encoder = host.register_annotation_encoder().unwrap();
    let restorer = host.register_annotation_restorer(&encoder).unwrap();
    let decoded = host
        .decode_annotation_json(&attempt, &context, limits())
        .unwrap();
    host.edit(
        &version,
        vec![Replacement {
            start: 0,
            end: 1,
            expected: "A".into(),
            inserted: "Z".into(),
        }],
    )
    .unwrap();
    assert!(
        host.decode_annotation_json(&attempt, &context, limits())
            .is_err()
    );
    assert!(
        host.restore_annotation_json(decoded, &approved, &restorer, limits())
            .is_err()
    );
    assert!(host.annotation_restoration_outcome(&attempt).is_err());
    assert_eq!(host.annotation_usage().present_records, 0);
    assert_eq!(host.read(&doc).unwrap().source.text(), "ZBC\r\n");
    assert_eq!(fs::read(f.source()).unwrap(), "\u{feff}ABC\r\n".as_bytes());
    host.stop_annotation_read(&attempt).unwrap();
    host.retire_annotation_read(&attempt).unwrap();
}

#[test]
fn malformed_json_and_foreign_attempts_never_become_restoration_or_saved_evidence() {
    let f = Fixture::new();
    fs::write(f.notes(), b"{not valid JSON").unwrap();
    let mut host = LocalHost::new(config(128)).unwrap();
    let doc = host.open(&f.source()).unwrap();
    let destination = host.bind_annotations(&doc, &f.notes()).unwrap();
    let version = host.read(&doc).unwrap().version;
    let attempt = host
        .queue_annotation_read(&destination, &version, 0)
        .unwrap();
    let mut foreign = LocalHost::new(config(0)).unwrap();
    assert!(foreign.service_annotation_read(&attempt).is_err());
    host.service_annotation_read(&attempt).unwrap();
    let context = DecodeContext {
        document: DocumentIdentity {
            namespace: "test-host".into(),
            value: "document".into(),
        },
        source: SourceEvidence {
            domain: "test-host-canonical".into(),
            token: "source-0".into(),
            byte_length: 5,
        },
        text: "ABC\r\n".into(),
    };
    assert!(
        host.decode_annotation_json(&attempt, &context, limits())
            .is_err()
    );
    assert!(host.annotation_restoration_outcome(&attempt).is_err());
    assert!(host.annotation_baseline_evidence(&doc).unwrap().is_none());
    assert_eq!(host.annotation_usage().present_records, 0);
    host.stop_annotation_read(&attempt).unwrap();
    host.retire_annotation_read(&attempt).unwrap();
    assert_eq!(host.resource_usage().retained_reads, 0);
    assert_eq!(host.resource_usage().issued_reads_lifetime, 1);
    assert_eq!(fs::read(f.notes()).unwrap(), b"{not valid JSON");
}

#[test]
fn disposed_restored_documents_release_both_bindings_original_read_and_actual_payloads() {
    use vellum_engine::coordinator::TrustedDocumentReclamationV1;
    let f = Fixture::new();
    let aliases = saved_notes(&f);
    let source_bytes = fs::read(f.source()).unwrap();
    let note_bytes = fs::read(f.notes()).unwrap();
    let mut host = LocalHost::new(config(128)).unwrap();
    let doc = host.open(&f.source()).unwrap();
    let destination = host.bind_annotations(&doc, &f.notes()).unwrap();
    let version = host.read(&doc).unwrap().version;
    let witness = host.capture_annotations(&version, 0).unwrap();
    let attempt = host
        .queue_annotation_read(&destination, &version, 0)
        .unwrap();
    host.service_annotation_read(&attempt).unwrap();
    let context = DecodeContext {
        document: aliases.document.identity.clone(),
        source: aliases.sources[0].evidence.clone(),
        text: witness.source_text().into(),
    };
    let approved = RestorationAssociations {
        document: DocumentAssociation {
            native: version.document.clone(),
            identity: aliases.document.identity.clone(),
        },
        records: aliases.records.iter().map(|r| r.identity.clone()).collect(),
        sources: vec![RestorationSourceAssociation {
            witness,
            evidence: aliases.sources[0].evidence.clone(),
        }],
    };
    let encoder = host.register_annotation_encoder().unwrap();
    let restorer = host.register_annotation_restorer(&encoder).unwrap();
    let decoded = host
        .decode_annotation_json(&attempt, &context, limits())
        .unwrap();
    let delivery_loan = host
        .decode_annotation_json(&attempt, &context, limits())
        .unwrap();
    host.restore_annotation_json(decoded, &approved, &restorer, limits())
        .unwrap();
    host.enable_annotation_read_baselines().unwrap();
    host.adopt_loaded_annotation_baseline(&attempt, 65536)
        .unwrap();
    assert!(host.retire_disposed_annotation_read(&attempt).is_err());
    let disposed = host.dispose(&version).unwrap();
    assert!(host.retire_disposed_annotation_read(&attempt).is_err());
    let unbound = host.release_source_binding(&doc, disposed).unwrap();
    assert!(host.release_provenance(&doc, unbound).is_err());
    let unbound = host.release_annotation_binding(&doc, unbound).unwrap();
    let released = host.release_provenance(&doc, unbound).unwrap();
    host.stop_annotation_read(&attempt).unwrap();
    assert!(host.retire_disposed_annotation_read(&attempt).is_err());
    drop(delivery_loan);
    host.retire_disposed_annotation_read(&attempt).unwrap();
    host.disable_annotation_read_baselines().unwrap();
    assert_eq!(host.resource_usage().held_read_bytes, 0);
    drop(approved);
    assert_eq!(host.history_usage().retained_groups, 0);
    host.reclaim(
        &doc,
        released,
        &TrustedDocumentReclamationV1 {
            authorized: Some(true),
            authored_disposition_ready: Some(true),
            session_references_released: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
    )
    .unwrap();
    assert!(host.read(&doc).is_err());
    assert_eq!(host.annotation_usage().retained_records, 0);
    assert_eq!(fs::read(f.source()).unwrap(), source_bytes);
    assert_eq!(fs::read(f.notes()).unwrap(), note_bytes);
}
