use vellum_adapter_annotation_json::*;
use vellum_engine::attachment::{Affinity, AttachmentTarget, PointProfile};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::source::{Limits, Replacement};

#[test]
fn header_inspection_is_bounded_and_duplicate_checked_not_record_or_source_approval() {
    let (snapshot, associations) = fixture(PointProfile::Position);
    let artifact = encode_capture(
        &snapshot,
        &associations,
        limits(),
        &mut Recorder::disabled(),
    )
    .unwrap();
    let declared = inspect_context(artifact.bytes(), limits()).unwrap();
    assert_eq!(declared.document, associations.document.identity);
    assert_eq!(declared.source, associations.sources[0].evidence);
    let mut too_small = limits();
    too_small.max_input_bytes = artifact.bytes().len() - 1;
    assert!(matches!(
        inspect_context(artifact.bytes(), too_small),
        Err(CodecError::Limit)
    ));
    let duplicate = std::str::from_utf8(artifact.bytes()).unwrap().replacen(
        "\"version\":\"1\"",
        "\"version\":\"1\",\"version\":\"1\"",
        1,
    );
    assert!(matches!(
        inspect_context(duplicate.as_bytes(), limits()),
        Err(CodecError::DuplicateMember)
    ));
}

fn limits() -> CodecLimits {
    CodecLimits {
        max_input_bytes: 65536,
        max_output_bytes: 65536,
        max_nodes: 2048,
        max_depth: 32,
        max_string_bytes: 32768,
        max_records: 16,
        max_integer_digits: 20,
        max_body_bytes: 4096,
        max_total_body_bytes: 8192,
        max_associations: 64,
        max_source_bytes: 8192,
    }
}
fn fixture(profile: PointProfile) -> (AnnotationSnapshot, CaptureAssociations) {
    let (_, snapshot, associations) = live_fixture(AttachmentTarget::Point {
        offset: 1,
        affinity: Affinity::After,
        profile,
    });
    (snapshot, associations)
}
fn live_fixture(
    target: AttachmentTarget,
) -> (SourceCoordinator, AnnotationSnapshot, CaptureAssociations) {
    let mut owner = SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 8192,
            },
        },
        invocation: None,
        resources: None,
        search: None,
        history: None,
        annotations: Some(AnnotationLimits {
            max_issued: 16,
            max_records: 16,
            max_body_bytes: 1024,
            max_retained_body_bytes: 8192,
            max_mapping_steps: 8192,
            max_live_snapshots: 4,
            max_snapshot_bytes: 16384,
        }),
    });
    let mut recorder = Recorder::disabled();
    let source = owner.create("AÄ\r\n".as_bytes(), &mut recorder).unwrap();
    let prepared = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                body: "PRIVATE_BODY\0🙂\r\n".into(),
                target: SourceTarget {
                    source: source.clone(),
                    basis: TargetBasis::Before,
                    target,
                },
            }],
            &mut recorder,
        )
        .unwrap();
    let annotation = prepared.created_annotations().next().unwrap().clone();
    owner.commit(prepared, &mut recorder).unwrap();
    let set = owner
        .annotation_set_version(&source.document, &mut recorder)
        .unwrap();
    let snapshot = owner
        .capture_annotations(&source, set, &mut recorder)
        .unwrap();
    let associations = CaptureAssociations {
        document: DocumentAssociation {
            native: source.document.clone(),
            identity: DocumentIdentity {
                namespace: "host".into(),
                value: "doc".into(),
            },
        },
        records: vec![RecordAssociation {
            native: annotation,
            identity: RecordIdentity {
                namespace: "notes".into(),
                value: "comment".into(),
                incarnation: 1,
            },
        }],
        sources: vec![SourceAssociation {
            native: source,
            evidence: SourceEvidence {
                domain: "host-source".into(),
                token: "s0".into(),
                byte_length: 5,
            },
            text: "AÄ\r\n".into(),
        }],
    };
    (owner, snapshot, associations)
}
fn encoded(profile: PointProfile) -> (Vec<u8>, DecodeContext) {
    let (snapshot, associations) = fixture(profile);
    let context = DecodeContext {
        document: associations.document.identity.clone(),
        source: associations.sources[0].evidence.clone(),
        text: snapshot.source_text().into(),
    };
    (
        encode_capture(
            &snapshot,
            &associations,
            limits(),
            &mut Recorder::disabled(),
        )
        .unwrap()
        .bytes()
        .to_vec(),
        context,
    )
}
#[test]
fn both_point_profiles_affinity_scalar_bodies_and_decimal_versions_round_trip_without_normalization()
 {
    for profile in [PointProfile::Position, PointProfile::Content] {
        let (bytes, context) = encoded(profile);
        let value = decode(&bytes, &context, limits(), &mut Recorder::disabled()).unwrap();
        assert_eq!(value.records[0].body, "PRIVATE_BODY\0🙂\r\n");
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let expected = if profile == PointProfile::Position {
            "position"
        } else {
            "content"
        };
        assert_eq!(json["records"][0]["authoredTarget"]["profile"], expected);
        assert_eq!(
            json["records"][0]["placement"]["target"]["profile"],
            expected
        );
        assert_eq!(json["records"][0]["contentVersion"], "1");
        assert_eq!(json["source"]["byteLength"], "5");
        assert!(!format!("{value:?}").contains("PRIVATE_"));
    }
}
#[test]
fn duplicate_plain_or_escape_decoded_keys_at_every_depth_refuse() {
    let (bytes, context) = encoded(PointProfile::Content);
    let text = String::from_utf8(bytes).unwrap();
    for changed in [
        text.replacen(
            "\"format\":",
            "\"format\":\"vellum.annotations\",\"format\":",
            1,
        ),
        text.replacen("\"body\":", "\"b\\u006fdy\":\"duplicate\",\"body\":", 1),
        text.replacen("\"domain\":", "\"domain\":\"duplicate\",\"domain\":", 1),
    ] {
        assert_eq!(
            decode(
                changed.as_bytes(),
                &context,
                limits(),
                &mut Recorder::disabled()
            )
            .unwrap_err(),
            CodecError::DuplicateMember
        );
    }
}
#[test]
fn malformed_utf8_bom_unpaired_surrogates_trailing_data_and_unknown_fields_refuse() {
    let (bytes, context) = encoded(PointProfile::Position);
    let mut bom = b"\xef\xbb\xbf".to_vec();
    bom.extend(&bytes);
    let mut trailing = bytes.clone();
    trailing.extend(b" {}");
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["metadata"] = serde_json::json!({});
    let unknown = serde_json::to_vec(&value).unwrap();
    let invalid_surrogate = String::from_utf8(bytes.clone())
        .unwrap()
        .replace("PRIVATE_BODY", "\\ud800")
        .into_bytes();
    for malformed in [vec![0xff], bom, trailing, unknown, invalid_surrogate] {
        assert!(decode(&malformed, &context, limits(), &mut Recorder::disabled()).is_err());
    }
}
#[test]
fn noncanonical_or_rounded_number_representations_are_not_edit_versions() {
    let (bytes, context) = encoded(PointProfile::Position);
    for invalid in [
        serde_json::json!(1),
        serde_json::json!("01"),
        serde_json::json!("+1"),
        serde_json::json!("1e0"),
        serde_json::json!("1 "),
        serde_json::json!("0"),
        serde_json::json!("18446744073709551616"),
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["records"][0]["contentVersion"] = invalid;
        assert!(
            decode(
                &serde_json::to_vec(&value).unwrap(),
                &context,
                limits(),
                &mut Recorder::disabled()
            )
            .is_err()
        );
    }
}
#[test]
fn missing_or_unknown_point_profiles_never_infer_a_default() {
    let (bytes, context) = encoded(PointProfile::Position);
    for bad in [None, Some("other")] {
        for target in ["authoredTarget", "placement"] {
            let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let value = if target == "placement" {
                &mut value["records"][0][target]["target"]
            } else {
                &mut value["records"][0][target]
            };
            if let Some(bad) = bad {
                value["profile"] = bad.into();
            } else {
                value.as_object_mut().unwrap().remove("profile");
            }
            let mut original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            if target == "placement" {
                original["records"][0][target]["target"] = value.clone();
            } else {
                original["records"][0][target] = value.clone();
            }
            assert!(
                decode(
                    &serde_json::to_vec(&original).unwrap(),
                    &context,
                    limits(),
                    &mut Recorder::disabled()
                )
                .is_err()
            );
        }
    }
}
#[test]
fn current_source_aliases_cannot_hide_invalid_scalar_boundaries_or_wrong_lengths() {
    let (bytes, context) = encoded(PointProfile::Position);
    for change_length in [false, true] {
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["records"][0]["authoredTarget"]["offset"] = "2".into();
        if change_length {
            value["records"][0]["authoredTarget"]["source"]["byteLength"] = "99".into();
        }
        assert!(
            decode(
                &serde_json::to_vec(&value).unwrap(),
                &context,
                limits(),
                &mut Recorder::disabled()
            )
            .is_err()
        );
    }
}
#[test]
fn record_identity_collisions_and_unsupported_versions_refuse_without_rewriting() {
    let (bytes, context) = encoded(PointProfile::Content);
    for version in [false, true] {
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        if version {
            value["version"] = "2".into();
        } else {
            let mut duplicate = value["records"][0].clone();
            duplicate["identity"]["incarnation"] = "2".into();
            value["records"].as_array_mut().unwrap().push(duplicate);
        }
        let input = serde_json::to_vec(&value).unwrap();
        let unchanged = input.clone();
        assert!(decode(&input, &context, limits(), &mut Recorder::disabled()).is_err());
        assert_eq!(input, unchanged);
    }
}
#[test]
fn each_parser_budget_refuses_before_accepting_a_complete_snapshot() {
    let (bytes, context) = encoded(PointProfile::Content);
    for bound in 0..8 {
        let mut profile = limits();
        match bound {
            0 => profile.max_input_bytes = 1,
            1 => profile.max_nodes = 1,
            2 => profile.max_depth = 1,
            3 => profile.max_string_bytes = 1,
            4 => profile.max_records = 0,
            5 => profile.max_integer_digits = 0,
            6 => profile.max_body_bytes = 1,
            _ => profile.max_total_body_bytes = 1,
        }
        assert!(decode(&bytes, &context, profile, &mut Recorder::disabled()).is_err());
    }
}
#[test]
fn export_requires_complete_injective_native_aliases_and_the_actual_captured_source() {
    for bad in 0..4 {
        let (snapshot, mut associations) = fixture(PointProfile::Content);
        match bad {
            0 => associations.records.clear(),
            1 => associations.records.push(associations.records[0].clone()),
            2 => associations.sources[0].text = "wrong".into(),
            _ => associations.sources.clear(),
        }
        assert!(
            encode_capture(
                &snapshot,
                &associations,
                limits(),
                &mut Recorder::disabled()
            )
            .is_err()
        );
    }
}
#[test]
fn output_bounds_and_capture_modes_do_not_change_successful_encoded_meaning() {
    let mut expected = None;
    for capacity in [None, Some(64), Some(1)] {
        let (snapshot, associations) = fixture(PointProfile::Content);
        let mut recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let artifact = encode_capture(&snapshot, &associations, limits(), &mut recorder).unwrap();
        if let Some(expected) = &expected {
            assert_eq!(artifact.bytes(), expected);
        } else {
            expected = Some(artifact.bytes().to_vec());
        }
        let context = DecodeContext {
            document: associations.document.identity.clone(),
            source: associations.sources[0].evidence.clone(),
            text: snapshot.source_text().into(),
        };
        decode(artifact.bytes(), &context, limits(), &mut recorder).unwrap();
        let mut small = limits();
        small.max_output_bytes = 1;
        assert!(encode_capture(&snapshot, &associations, small, &mut recorder).is_err());
        assert!(!format!("{artifact:?}").contains("PRIVATE_"));
        while let Some(record) = recorder.pop_front() {
            assert!(!format!("{record:?}").contains("PRIVATE_"));
        }
        if capacity == Some(1) {
            assert!(recorder.dropped() > 0);
        }
    }
}

#[test]
fn all_native_target_forms_keep_their_explicit_meaning() {
    let mut targets = vec![
        AttachmentTarget::Document,
        AttachmentTarget::ContentRange { start: 0, end: 3 },
    ];
    for profile in [PointProfile::Position, PointProfile::Content] {
        for affinity in [Affinity::Before, Affinity::After] {
            targets.push(AttachmentTarget::Point {
                offset: 1,
                affinity,
                profile,
            });
        }
    }
    for native in targets {
        let (_, snapshot, associations) = live_fixture(native);
        let artifact = encode_capture(
            &snapshot,
            &associations,
            limits(),
            &mut Recorder::disabled(),
        )
        .unwrap();
        let context = DecodeContext {
            document: associations.document.identity.clone(),
            source: associations.sources[0].evidence.clone(),
            text: snapshot.source_text().into(),
        };
        let parsed = decode(
            artifact.bytes(),
            &context,
            limits(),
            &mut Recorder::disabled(),
        )
        .unwrap();
        let StoredPlacement::Attached { target } = &parsed.records[0].placement else {
            panic!("missing attached target")
        };
        assert_eq!(&parsed.records[0].authored_target, target);
        match (native, target) {
            (AttachmentTarget::Document, Target::Document { .. }) => {}
            (
                AttachmentTarget::ContentRange { start: a, end: b },
                Target::Range { start, end, .. },
            ) => assert_eq!((*start, *end), (a as u64, b as u64)),
            (
                AttachmentTarget::Point {
                    offset: native,
                    affinity: a,
                    profile: p,
                },
                Target::Point {
                    offset,
                    affinity,
                    profile,
                    ..
                },
            ) => {
                assert_eq!(*offset, native as u64);
                assert_eq!(
                    *affinity,
                    if a == Affinity::Before {
                        PointAffinity::Before
                    } else {
                        PointAffinity::After
                    }
                );
                assert_eq!(
                    *profile,
                    if p == PointProfile::Content {
                        PointMeaning::Content
                    } else {
                        PointMeaning::Position
                    }
                );
            }
            _ => panic!("target kind changed"),
        }
    }
}

#[test]
fn historical_targets_require_actual_aliases_and_canonical_feff_text_does_not_enter_the_file_decoder()
 {
    let (mut owner, old_snapshot, mut associations) =
        live_fixture(AttachmentTarget::ContentRange { start: 0, end: 3 });
    let before = old_snapshot.source().clone();
    drop(old_snapshot);
    let mut recorder = Recorder::disabled();
    let edit = owner
        .prepare(
            &[DocumentEdits {
                expected: before.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "\u{feff}".into(),
                }],
            }],
            &[],
            &mut recorder,
        )
        .unwrap();
    owner.commit(edit, &mut recorder).unwrap();
    let current = owner.read(&before.document, &mut recorder).unwrap().version;
    let text = owner
        .read(&before.document, &mut recorder)
        .unwrap()
        .source
        .text()
        .to_owned();
    assert!(text.starts_with('\u{feff}'));
    assert!(
        owner
            .read(&before.document, &mut recorder)
            .unwrap()
            .source
            .encode()
            .is_err()
    );
    associations.sources.push(SourceAssociation {
        native: current.clone(),
        evidence: SourceEvidence {
            domain: "host-source".into(),
            token: "s1".into(),
            byte_length: text.len() as u64,
        },
        text: text.clone(),
    });
    let set = owner
        .annotation_set_version(&before.document, &mut recorder)
        .unwrap();
    let snapshot = owner
        .capture_annotations(&current, set, &mut recorder)
        .unwrap();
    let artifact = encode_capture(&snapshot, &associations, limits(), &mut recorder).unwrap();
    let context = DecodeContext {
        document: associations.document.identity.clone(),
        source: associations.sources[1].evidence.clone(),
        text,
    };
    let value = decode(artifact.bytes(), &context, limits(), &mut recorder).unwrap();
    assert!(value.needs_historical_evidence());
    assert_eq!(value.records[0].authored_target.source().token, "s0");
    let StoredPlacement::Attached { target } = &value.records[0].placement else {
        panic!("wrong placement")
    };
    assert_eq!(target.source().token, "s1");
    drop(snapshot);
    assert_eq!(owner.annotation_usage().retained_snapshots, 1);
    associations.sources.remove(0);
    assert_eq!(
        encode_capture(artifact.snapshot(), &associations, limits(), &mut recorder).unwrap_err(),
        CodecError::HistoricalEvidenceMissing
    );
    drop(artifact);
    assert_eq!(owner.annotation_usage().retained_snapshots, 0);
}

#[test]
fn producer_budget_matches_an_independent_json_tree_count_at_the_exact_boundary() {
    fn count(value: &serde_json::Value) -> (usize, usize) {
        match value {
            serde_json::Value::String(text) => (1, text.len()),
            serde_json::Value::Array(values) => values
                .iter()
                .map(count)
                .fold((1, 0), |(n, s), (a, b)| (n + a, s + b)),
            serde_json::Value::Object(values) => values.iter().fold((1, 0), |(n, s), (k, v)| {
                let (a, b) = count(v);
                (n + a, s + k.len() + b)
            }),
            _ => (1, 0),
        }
    }
    let (snapshot, associations) = fixture(PointProfile::Content);
    let baseline = encode_capture(
        &snapshot,
        &associations,
        limits(),
        &mut Recorder::disabled(),
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_slice(baseline.bytes()).unwrap();
    let (nodes, strings) = count(&value);
    let mut exact = limits();
    exact.max_nodes = nodes;
    exact.max_string_bytes = strings;
    assert!(encode_capture(&snapshot, &associations, exact, &mut Recorder::disabled()).is_ok());
    for strings_bound in [false, true] {
        let mut short = exact;
        if strings_bound {
            short.max_string_bytes -= 1;
        } else {
            short.max_nodes -= 1;
        }
        assert_eq!(
            encode_capture(&snapshot, &associations, short, &mut Recorder::disabled()).unwrap_err(),
            CodecError::Limit
        );
    }
}

#[test]
fn unknown_members_at_every_product_depth_are_rejected() {
    let (bytes, context) = encoded(PointProfile::Content);
    for product in 0..7 {
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let object = match product {
            0 => &mut value["document"],
            1 => &mut value["source"],
            2 => &mut value["records"][0],
            3 => &mut value["records"][0]["identity"],
            4 => &mut value["records"][0]["authoredTarget"],
            5 => &mut value["records"][0]["placement"],
            _ => &mut value["records"][0]["placement"]["target"]["source"],
        };
        object["extra"] = true.into();
        assert_eq!(
            decode(
                &serde_json::to_vec(&value).unwrap(),
                &context,
                limits(),
                &mut Recorder::disabled()
            )
            .unwrap_err(),
            CodecError::Shape
        );
    }
}
