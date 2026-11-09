use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::selection::{Selection, SelectionBudget};
use vellum_engine::source::Limits;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;
fn owner() -> SourceCoordinator {
    SourceCoordinator::with_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 8192,
            source: Limits {
                max_source_bytes: 2048,
                max_edits: 32,
                max_edit_payload_bytes: 4096,
            },
        },
        HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 8,
        },
    )
}
fn attach(o: &mut SourceCoordinator, d: &DocumentVersion, r: &mut Recorder) -> LogicalViewContext {
    o.configure_logical_views(
        LogicalViewLimits {
            max_live_views: 8,
            max_issued_views: 8,
            max_total_selections: 32,
            normalization: SelectionBudget {
                max_input_selections: 16,
                max_output_selections: 16,
                max_work: 65536,
            },
            max_mapping_steps: 65536,
        },
        r,
    )
    .unwrap();
    o.configure_line_edits(
        LineEditBudget {
            max_lines: 16,
            max_prefix_bytes: 1024,
            max_indent_columns: 128,
            max_unit_bytes: 8,
            max_derived_bytes: 4096,
            max_source_scan_bytes: 1024,
        },
        r,
    )
    .unwrap();
    let v = o.create_logical_view(r).unwrap();
    let stamp = o.logical_view(&v, r).unwrap().stamp;
    o.attach_logical_view(
        &v,
        stamp,
        d,
        &[Selection {
            anchor: vellum_engine::coordinates::BytePosition(1),
            head: vellum_engine::coordinates::BytePosition(1),
            affinity: vellum_engine::attachment::Affinity::After,
        }],
        0,
        r,
    )
    .unwrap();
    o.logical_view(&v, r).unwrap().context().unwrap()
}
#[test]
fn new_documents_default_to_lf_and_host_birth_policy_drives_only_generated_newlines() {
    for (policy, newline) in [
        (GeneratedNewlineV1::Lf, "\n"),
        (GeneratedNewlineV1::CrLf, "\r\n"),
        (GeneratedNewlineV1::Cr, "\r"),
    ] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let d = o
            .create_with_insertion_policy_v1(b"A\r\nB\rC\n", policy, &mut r)
            .unwrap();
        assert_eq!(o.source_insertion_policy_v1(&d, &mut r).unwrap(), policy);
        let c = attach(&mut o, &d, &mut r);
        let p = o
            .prepare_document_newline_v1(&c, NewlineIndentV1::None, &mut r)
            .unwrap();
        o.commit(p, &mut r).unwrap();
        let read = o.read(&d.document, &mut r).unwrap();
        assert_eq!(read.source.text(), format!("A{newline}\r\nB\rC\n"));
    }
    let mut o = owner();
    let mut r = Recorder::disabled();
    let d = o.create(b"A\r\n", &mut r).unwrap();
    assert_eq!(
        o.source_insertion_policy_v1(&d, &mut r).unwrap(),
        GeneratedNewlineV1::Lf
    );
}

#[test]
fn loaded_policy_is_explicit_and_alias_join_does_not_replace_the_existing_owners_policy_or_bytes() {
    let mut f = support::Fixture::new();
    let raw = "\u{feff}English\r\nDeutsch\rEnd\n".as_bytes();
    let d = f
        .owner
        .load_resource_with_insertion_policy_v1(
            f.port.read_result(b"new", b"v0", raw).unwrap(),
            GeneratedNewlineV1::Cr,
            &mut f.recorder,
        )
        .unwrap();
    let before = f.owner.generation();
    let joined = f
        .owner
        .load_resource_with_insertion_policy_v1(
            f.port.read_result(b"new", b"v1", b"different").unwrap(),
            GeneratedNewlineV1::CrLf,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(joined, d);
    assert_eq!(f.owner.generation(), before);
    assert_eq!(
        f.owner
            .source_insertion_policy_v1(&d, &mut f.recorder)
            .unwrap(),
        GeneratedNewlineV1::Cr
    );
    assert_eq!(
        f.owner
            .read(&d.document, &mut f.recorder)
            .unwrap()
            .source
            .encode()
            .unwrap(),
        raw
    );
}
#[test]
fn explicit_newlines_and_literal_insertions_override_no_existing_bytes_and_history_keeps_the_birth_default()
 {
    for literal in [false, true] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let d = o
            .create_with_insertion_policy_v1(b"AB\r\n", GeneratedNewlineV1::Cr, &mut r)
            .unwrap();
        let c = attach(&mut o, &d, &mut r);
        let p = if literal {
            o.prepare_literal_edit(&c, "\n\r\n", SelectionAfterV1::CaretsAfter, &mut r)
                .unwrap()
        } else {
            o.prepare_line_edit(
                &c,
                &LineEditV1::Newline {
                    delimiter: GeneratedNewlineV1::CrLf,
                    indent: NewlineIndentV1::None,
                },
                &mut r,
            )
            .unwrap()
        };
        let changed = o.commit(p, &mut r).unwrap();
        let group = changed.history_group.unwrap();
        let read = o.read(&d.document, &mut r).unwrap();
        assert_eq!(
            read.source.text(),
            if literal {
                "A\n\r\nB\r\n"
            } else {
                "A\r\nB\r\n"
            }
        );
        let v = read.version;
        assert_eq!(
            o.source_insertion_policy_v1(&v, &mut r).unwrap(),
            GeneratedNewlineV1::Cr
        );
        let undo = o
            .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
            .unwrap();
        o.commit(undo, &mut r).unwrap();
        let read = o.read(&d.document, &mut r).unwrap();
        assert_eq!(read.source.text(), "AB\r\n");
        let v = read.version;
        assert_eq!(
            o.source_insertion_policy_v1(&v, &mut r).unwrap(),
            GeneratedNewlineV1::Cr
        );
    }
}
#[test]
fn default_newline_uses_existing_validation_and_optional_instrumentation_without_publication_on_refusal()
 {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for capacity in [0, 1, 128] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let d = o
            .create_with_insertion_policy_v1(b"AB", GeneratedNewlineV1::CrLf, &mut r)
            .unwrap();
        let c = attach(&mut o, &d, &mut r);
        r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let generation = o.generation();
        assert_eq!(
            o.source_insertion_policy_v1(&d, &mut r).unwrap(),
            GeneratedNewlineV1::CrLf
        );
        assert_eq!(o.generation(), generation);
        let p = o
            .prepare_document_newline_v1(&c, NewlineIndentV1::None, &mut r)
            .unwrap();
        o.commit(p, &mut r).unwrap();
        let read = o.read(&d.document, &mut r).unwrap();
        assert_eq!(read.source.text(), "A\r\nB");
        let current = read.version;
        let generation = o.generation();
        assert_eq!(
            o.source_insertion_policy_v1(&d, &mut r),
            Err(CoordinatorError::Stale)
        );
        assert!(
            o.prepare_document_newline_v1(&c, NewlineIndentV1::None, &mut r)
                .is_err()
        );
        assert_eq!(o.generation(), generation);
        o.set_read_only(&d.document, true, &mut r).unwrap();
        let snapshot = o.logical_view(&c.view, &mut r).unwrap();
        assert!(
            o.prepare_document_newline_v1(
                &snapshot.context().unwrap(),
                NewlineIndentV1::None,
                &mut r
            )
            .is_err()
        );
        assert_eq!(
            o.source_insertion_policy_v1(&current, &mut r).unwrap(),
            GeneratedNewlineV1::CrLf
        );
        let mut queried = false;
        let mut prepared = false;
        while let Some(record) = r.pop_front() {
            if matches!(
                record.boundary,
                Boundary::DocumentInsertionPolicyRead | Boundary::DocumentNewlinePreparation
            ) {
                assert!(record.service_time.is_some());
                queried |= record.boundary == Boundary::DocumentInsertionPolicyRead;
                prepared |= record.boundary == Boundary::DocumentNewlinePreparation;
            }
        }
        if capacity == 128 {
            assert!(queried && prepared);
        }
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
    }
}
