use super::*;
#[test]
fn native_result_guard_preserves_old_source_stamp_and_does_not_confuse_ordinary_refusal_with_fault()
{
    let (mut o, mut r, d, s) = ready("é\r\nbody".as_bytes());
    let id = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 2,
                    expected: "é".into(),
                    inserted: "new".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    let reply = o
        .query_source_read_context(
            &id,
            &s,
            SourceContextQueryV1::Line { index: 0 },
            &facts(&s),
            &mut r,
        )
        .unwrap();
    let SourceContextContentV1::Line(line) = reply.content else {
        panic!("wrong product")
    };
    assert_eq!(line.source, d);
    assert_eq!(line.body(), "é".as_bytes());
    assert_eq!(line.delimiter(), b"\r\n");
    assert!(
        o.query_source_read_context(
            &id,
            &s,
            SourceContextQueryV1::Line { index: 999 },
            &facts(&s),
            &mut r
        )
        .is_none()
    );
    assert!(!o.is_faulted());
    assert_eq!(o.read_result_invariant(), None);
    assert_eq!(
        ReadResultInvariantV1::NativeSourceResultInvariant.code(),
        "NATIVE_SOURCE_RESULT_INVARIANT"
    );
}
