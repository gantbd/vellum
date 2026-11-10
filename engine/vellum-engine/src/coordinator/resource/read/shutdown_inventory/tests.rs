use super::*;
use crate::coordinator::resource::shutdown_bindings::tests::{capture, fence, read};

#[test]
fn restoration_and_attachment_keep_distinct_original_obligations_without_new_pins() {
    let (mut o, id, package, mut r) = super::super::super::loaded_baseline::commit_tests::ready(0);
    let plan = o
        .prepare_annotation_restoration_native(package, None, &mut r)
        .unwrap();
    o.commit_annotation_restoration_native(plan, &mut r)
        .unwrap();
    let detached = capture(&o, &mut r);
    let view = read(&o, &detached, &mut r);
    assert_eq!(view.reads[0].read, ShutdownReadIdV1::Annotation(id.clone()));
    assert!(view.reads[0].restoration_bytes > 0);
    assert!(!view.reads[0].loaded_baseline_adopted);
    assert!(view.read_writer_attachments.is_empty());
    let refs = view.reads[0].capture_references;
    o.attach_annotation_read_baselines(&mut r).unwrap();
    assert!(
        o.adopt_loaded_annotation_baseline(&id, None, 16384, &mut r)
            .unwrap()
    );
    let attached = capture(&o, &mut r);
    let view = read(&o, &attached, &mut r);
    assert_eq!(view.reads[0].capture_references, refs);
    assert!(view.reads[0].loaded_baseline_adopted);
    assert_eq!(view.read_writer_attachments.len(), 1);
    let link = &view.read_writer_attachments[0];
    assert_ne!(link.reader, link.writer);
    assert!(link.reader_endpoint_bytes > 0 && link.writer_endpoint_bytes > 0);
    assert!(view.dependencies.iter().any(|e| e.dependent
        == ShutdownInventoryTargetV1::AnnotationRead(id.clone())
        && e.required == ShutdownInventoryTargetV1::ReadWriterAttachment(link.attachment.clone())));
    assert!(view.coverage.all_owner_obligations);
    assert!(!read(&o, &detached, &mut r).reads[0].loaded_baseline_adopted);
}

#[test]
fn annotation_admission_is_fenced_without_disabling_original_delivery_or_cleanup() {
    let (mut o, p, id, mut r) = super::super::tests::fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let task = o.read_task(&id).unwrap();
    let source = task.source.clone();
    let set = task.set;
    fence(&mut o, &mut r);
    assert_eq!(
        o.admit_annotation_read(
            &source,
            set,
            p.annotation_read_target(b"comments").unwrap(),
            ReadStrength::CapturedRead,
            None,
            &mut r
        ),
        Err(ResourceError::Unavailable)
    );
    let dispatch = o.submit_annotation_read(&id, None, &mut r).unwrap();
    for e in [
        p.annotation_read_start(&dispatch, b"v0").unwrap(),
        p.annotation_read_end(&dispatch, b"v0", 0, 0).unwrap(),
        p.annotation_read_quiescence(&dispatch).unwrap(),
    ] {
        o.record_annotation_read(e, &mut r).unwrap();
    }
    drop(dispatch);
    o.retire_annotation_read(&id, &mut r).unwrap();
}

#[test]
fn annotation_operation_registry_acquisition_cannot_bypass_the_fence() {
    for configured in [false, true] {
        let (mut o, _, _, mut r) = super::super::tests::fixture();
        o.configure_retention_budget(4194304, &mut r).unwrap();
        let limits = AnnotationReadOperationLimitsV1 {
            max_capture_bytes: 8192,
            max_reply_bytes: 1024,
            max_label_bytes: 64,
            max_store_bytes: 1048576,
        };
        if configured {
            o.configure_annotation_read_operations_v1(limits, &mut r)
                .unwrap();
        }
        fence(&mut o, &mut r);
        let before = o.retention_usage();
        assert_eq!(
            o.configure_annotation_read_operations_v1(limits, &mut r),
            if configured {
                Ok(())
            } else {
                Err(ResourceError::Unavailable)
            }
        );
        assert_eq!(o.retention_usage(), before);
    }
}
