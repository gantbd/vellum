use super::*;
fn owner() -> (SourceCoordinator, DocumentVersion, Recorder) {
    let mut owner = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 4,
        max_participants: 4,
        max_staged_bytes: 256,
        source: Limits {
            max_source_bytes: 128,
            max_edits: 4,
            max_edit_payload_bytes: 128,
        },
    });
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"line\r\n", &mut recorder).unwrap();
    (owner, source, recorder)
}
#[test]
fn current_query_requires_no_unused_publication_counter_or_lifetime_slot() {
    let (mut owner, source, mut recorder) = owner();
    owner.generation = u64::MAX;
    owner.limits.max_document_incarnations = owner.documents.len();
    let before = owner.lifecycle_usage();
    assert_eq!(
        owner
            .read_source_line(
                &source,
                1,
                SourceQueryBudget {
                    max_scan_bytes: 6,
                    max_result_bytes: 0
                },
                &mut recorder
            )
            .unwrap()
            .body(),
        b""
    );
    owner
        .read_source_range(&source, BytePosition(6), BytePosition(6), 0, &mut recorder)
        .unwrap();
    owner
        .convert_source_coordinate(
            &source,
            CoordinateRequestV1::ByteToScalar(BytePosition(0)),
            CoordinateBudget { max_scan_bytes: 0 },
            &mut recorder,
        )
        .unwrap();
    assert_eq!(owner.generation, u64::MAX);
    assert_eq!(owner.lifecycle_usage(), before);
}
#[test]
fn a_faulted_owner_refuses_new_queries_but_does_not_recall_already_delivered_content() {
    let (mut owner, source, mut recorder) = owner();
    let line = owner
        .read_source_line(
            &source,
            0,
            SourceQueryBudget {
                max_scan_bytes: 6,
                max_result_bytes: 6,
            },
            &mut recorder,
        )
        .unwrap();
    owner.faulted = true;
    assert_eq!(
        owner.read_source_range(&source, BytePosition(0), BytePosition(0), 0, &mut recorder),
        Err(SourceQueryError::Owner(CoordinatorError::Faulted))
    );
    assert_eq!(
        owner.read_source_line(
            &source,
            0,
            SourceQueryBudget {
                max_scan_bytes: 6,
                max_result_bytes: 6
            },
            &mut recorder
        ),
        Err(SourceQueryError::Owner(CoordinatorError::Faulted))
    );
    assert_eq!(
        owner.convert_source_coordinate(
            &source,
            CoordinateRequestV1::ByteToScalar(BytePosition(0)),
            CoordinateBudget { max_scan_bytes: 0 },
            &mut recorder
        ),
        Err(SourceQueryError::Owner(CoordinatorError::Faulted))
    );
    assert_eq!(line.body(), b"line");
}
