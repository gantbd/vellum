use super::*;
use AnnotationAuthoringEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
pub(super) fn limits() -> AnnotationAuthoringArgumentLimitsV1 {
    AnnotationAuthoringArgumentLimitsV1 {
        max_body_bytes: 1024,
        max_profile_bytes: 128,
        max_dependencies: 16,
        max_owned_bytes: 16384,
    }
}
pub(super) fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 16,
            max_work: 256,
        },
    }
}
pub(super) fn header<'h>(
    ticket: &'h InvocationId,
    document: &'h DocumentId,
    dependencies: &'h ReadDependencyRequestV3,
    operation: &'h str,
) -> Vec<(&'h str, V<'h, 'h>)> {
    vec![
        ("contract", V::Data(D::Text("annotation-authoring/1"))),
        ("operation", V::Data(D::Text(operation))),
        ("invocation", V::Invocation(ticket)),
        ("document", V::Document(document)),
        (
            "profile",
            V::Data(D::Text("host-selected-annotation-profile")),
        ),
        ("dependencies", V::Dependencies(dependencies)),
    ]
}
#[test]
fn all_annotation_forms_preserve_body_native_versions_targets_and_original_dependencies() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let (annotation, _) = create(&mut o, target(&source, 0, 1), "old", &mut r);
    let ticket = o.issue_invocation(&mut r).unwrap();
    let dependencies = reads();
    let body = "Deutsch: Grüße\n```mermaid\ngraph LR\n```";
    let target = target(&source, 1, 2);
    for operation in ["create", "update-body", "reattach", "delete"] {
        let mut raw = header(&ticket, &source.document, &dependencies, operation);
        match operation {
            "create" => raw.extend([
                ("body", V::Data(D::Text(body))),
                ("target", V::Target(&target)),
            ]),
            "update-body" => raw.extend([
                ("annotation", V::Annotation(&annotation)),
                ("expectedContent", V::Version(u64::MAX)),
                ("body", V::Data(D::Text(body))),
            ]),
            "reattach" => raw.extend([
                ("annotation", V::Annotation(&annotation)),
                ("expectedTarget", V::Version(u64::MAX)),
                ("target", V::Target(&target)),
            ]),
            _ => raw.extend([
                ("annotation", V::Annotation(&annotation)),
                ("expectedContent", V::Version(u64::MAX)),
                ("expectedTarget", V::Version(u64::MAX)),
            ]),
        }
        let before = o.generation();
        let usage = o.annotation_usage();
        let q =
            normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
        assert!(std::ptr::eq(q.invocation(), &ticket));
        assert!(std::ptr::eq(q.document(), &source.document));
        assert!(std::ptr::eq(q.dependencies(), &dependencies));
        assert_eq!(q.execution_profile(), "host-selected-annotation-profile");
        assert_eq!(
            q.argument_declaration_v1().variant,
            *q.mutation().declaration_v1()
        );
        match q.mutation() {
            AnnotationMutation::Create {
                body: actual,
                target: actual_target,
            } => {
                assert_eq!(actual, body);
                assert_eq!(actual_target, &target);
            }
            AnnotationMutation::UpdateBody {
                expected_content,
                body: actual,
                ..
            } => {
                assert_eq!(*expected_content, u64::MAX);
                assert_eq!(actual, body);
            }
            AnnotationMutation::Reattach {
                expected_target,
                target: actual_target,
                ..
            } => {
                assert_eq!(*expected_target, u64::MAX);
                assert_eq!(actual_target, &target);
            }
            AnnotationMutation::Delete {
                expected_content,
                expected_target,
                ..
            } => {
                assert_eq!(*expected_content, u64::MAX);
                assert_eq!(*expected_target, u64::MAX);
            }
        }
        assert_eq!(o.generation(), before);
        assert_eq!(o.annotation_usage(), usage);
        assert_eq!(
            o.invocation_state(&ticket, &mut r),
            Ok(InvocationState::Issued)
        );
        assert!(!format!("{q:?}").contains("Deutsch"));
        assert!(!format!("{:?}", V::Object(&raw)).contains("host-selected"));
    }
}

#[test]
fn authoring_fields_and_opaque_roles_are_closed_without_current_registry_lookup() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let other = o.create(b"abc", &mut r).unwrap();
    let (annotation, _) = create(&mut o, target(&source, 0, 1), "old", &mut r);
    let ticket = o.issue_invocation(&mut r).unwrap();
    let dependencies = reads();
    let target = target(&source, 0, 1);
    let before = o.generation();
    let usage = o.annotation_usage();
    for case in 0..13 {
        let mut raw = header(&ticket, &source.document, &dependencies, "create");
        raw.extend([
            ("body", V::Data(D::Text("PRIVATE_BODY"))),
            ("target", V::Target(&target)),
        ]);
        match case {
            0 => {
                raw.pop();
            }
            1 => raw[7] = raw[6],
            2 => raw[0].1 = V::Data(D::Text("annotation-authoring/2")),
            3 => raw[1].1 = V::Data(D::Text("erase")),
            4 => raw[2].1 = V::Data(D::Text("not an invocation")),
            5 => raw[3].1 = V::Annotation(&annotation),
            6 => raw[4].1 = V::Data(D::Text("")),
            7 => raw[5].1 = V::Data(D::Array(&[])),
            8 => raw[6].1 = V::Data(D::Null),
            9 => raw[7].1 = V::Data(D::Text("not a target")),
            10 => raw[3].1 = V::Document(&other.document),
            11 => raw[7].0 = "allocation",
            _ => raw[7].0 = "permission",
        }
        assert_eq!(
            normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r)
                .unwrap_err(),
            InvocationError::MalformedIntent
        );
        assert_eq!(o.generation(), before);
        assert_eq!(o.annotation_usage(), usage);
        assert_eq!(
            o.invocation_state(&ticket, &mut r),
            Ok(InvocationState::Issued)
        );
    }
}

#[test]
fn native_version_roles_do_not_coerce_json_numbers_strings_or_zero() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let other = o.create(b"abc", &mut r).unwrap();
    let (annotation, _) = create(&mut o, target(&source, 0, 1), "old", &mut r);
    let ticket = o.issue_invocation(&mut r).unwrap();
    let dependencies = reads();
    for bad in [
        V::Version(0),
        V::Data(D::Number(1.0)),
        V::Data(D::Text("18446744073709551615")),
    ] {
        let mut raw = header(&ticket, &source.document, &dependencies, "delete");
        raw.extend([
            ("annotation", V::Annotation(&annotation)),
            ("expectedContent", bad),
            ("expectedTarget", V::Version(1)),
        ]);
        assert_eq!(
            normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r)
                .unwrap_err(),
            InvocationError::MalformedIntent
        );
    }
    let mut raw = header(&ticket, &other.document, &dependencies, "delete");
    raw.extend([
        ("annotation", V::Annotation(&annotation)),
        ("expectedContent", V::Version(1)),
        ("expectedTarget", V::Version(1)),
    ]);
    assert_eq!(
        normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap_err(),
        InvocationError::MalformedIntent
    );
}

#[test]
fn native_targets_preserve_document_range_both_point_profiles_and_original_source_basis() {
    use vellum_engine::attachment::{Affinity, PointProfile};
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let dependencies = reads();
    let targets = [
        AttachmentTarget::Document,
        AttachmentTarget::ContentRange { start: 0, end: 2 },
        AttachmentTarget::Point {
            offset: 1,
            affinity: Affinity::Before,
            profile: PointProfile::Position,
        },
        AttachmentTarget::Point {
            offset: 1,
            affinity: Affinity::After,
            profile: PointProfile::Content,
        },
    ];
    for basis in [TargetBasis::Before, TargetBasis::PreparedSource] {
        for target in targets {
            let mut version = source.clone();
            version.revision = u64::MAX;
            let target = SourceTarget {
                source: version,
                target,
                basis,
            };
            let mut raw = header(&ticket, &source.document, &dependencies, "create");
            raw.extend([
                ("body", V::Data(D::Text(""))),
                ("target", V::Target(&target)),
            ]);
            let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r)
                .unwrap();
            let AnnotationMutation::Create {
                body,
                target: actual,
            } = q.mutation()
            else {
                panic!("wrong variant")
            };
            assert!(body.is_empty());
            assert_eq!(actual, &target);
        }
    }
    // Preserving a native target is deliberately not a liveness, byte-boundary
    // or standalone PreparedSource eligibility assertion.
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Ok(InvocationState::Issued)
    );
}

#[test]
fn payload_profile_dependency_and_owned_bounds_are_independent_and_checked_before_copy() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let mut dependencies = reads();
    dependencies.predicates = vec![
        ReadDependencyV3::Base(ReadDependencyV2::Base(
            ReadDependencyV1::Source(source.clone())
        ));
        2
    ];
    let target = target(&source, 0, 1);
    let body = "Grüße";
    let mut raw = header(&ticket, &source.document, &dependencies, "create");
    raw.extend([
        ("body", V::Data(D::Text(body))),
        ("target", V::Target(&target)),
    ]);
    let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
    assert_eq!(q.dependencies().predicates.len(), 2);
    assert!(std::ptr::eq(q.dependencies(), &dependencies));
    assert_eq!(
        q.owned_bytes(),
        NormalizedAnnotationAuthoringArgumentsV1::required_owned_bytes(
            body.len(),
            q.execution_profile().len()
        )
        .unwrap()
    );
    let mut exact = limits();
    exact.max_owned_bytes = q.owned_bytes();
    assert!(normalize_annotation_authoring_arguments_v1(V::Object(&raw), exact, &mut r).is_ok());
    exact.max_owned_bytes -= 1;
    assert_eq!(
        normalize_annotation_authoring_arguments_v1(V::Object(&raw), exact, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    let mut small = limits();
    small.max_body_bytes = body.len() - 1;
    assert_eq!(
        normalize_annotation_authoring_arguments_v1(V::Object(&raw), small, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    small = limits();
    small.max_profile_bytes = q.execution_profile().len() - 1;
    assert_eq!(
        normalize_annotation_authoring_arguments_v1(V::Object(&raw), small, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    small = limits();
    small.max_dependencies = 1;
    assert_eq!(
        normalize_annotation_authoring_arguments_v1(V::Object(&raw), small, &mut r).unwrap_err(),
        InvocationError::Limit
    );
    assert_eq!(
        NormalizedAnnotationAuthoringArgumentsV1::required_owned_bytes(usize::MAX, 1),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Ok(InvocationState::Issued)
    );
}

#[test]
fn authoring_normalization_observations_are_optional_timed_bounded_and_non_authorizing() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let mut o = owner();
        let mut setup = Recorder::disabled();
        let source = o.create(b"abc", &mut setup).unwrap();
        let ticket = o.issue_invocation(&mut setup).unwrap();
        let dependencies = reads();
        let target = target(&source, 0, 1);
        let mut raw = header(&ticket, &source.document, &dependencies, "create");
        raw.extend([
            ("body", V::Data(D::Text("PRIVATE_BODY"))),
            ("target", V::Target(&target)),
        ]);
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 16 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        let before = o.generation();
        let usage = o.annotation_usage();
        normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
        raw[1].1 = V::Data(D::Text("unknown"));
        assert!(
            normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).is_err()
        );
        let mut seen = Vec::new();
        while let Some(event) = r.pop_front() {
            assert!(!format!("{event:?}").contains("PRIVATE_BODY"));
            if event.boundary == Boundary::AnnotationAuthoringArgumentNormalization {
                seen.push(event.disposition);
                assert_eq!(event.service_time.is_some(), mode == 2);
            }
        }
        if mode == 1 || mode == 2 {
            assert_eq!(seen, vec![Disposition::Completed, Disposition::Refused]);
        } else {
            assert!(seen.is_empty());
        }
        if mode == 3 {
            assert_eq!(r.dropped(), 2);
        }
        assert_eq!(o.generation(), before);
        assert_eq!(o.annotation_usage(), usage);
        assert_eq!(
            o.invocation_state(&ticket, &mut setup),
            Ok(InvocationState::Issued)
        );
    }
}
