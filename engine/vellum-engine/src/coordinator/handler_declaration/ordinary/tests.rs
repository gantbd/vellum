use super::*;
use crate::capture::CaptureLimitsV1;
use crate::instrumentation::Recorder;
fn property<'a>(v: D<'a>, key: &str) -> D<'a> {
    let D::Object(rows) = v else { panic!() };
    rows.iter().find(|(k, _)| *k == key).unwrap().1
}
#[test]
fn exact_ordinary_schema_rejects_nine_mutations_but_ignores_object_member_order() {
    with_schema(|expected| {
        let limits = CaptureLimitsV1 {
            max_bytes: 262144,
            max_nodes: 4096,
            max_depth: 16,
        };
        for mode in 0..=10 {
            let D::Object(root) = expected else { panic!() };
            let mut root = root.to_vec();
            let D::Array(branches) = property(expected, "oneOf") else {
                panic!()
            };
            let mut branches = branches.to_vec();
            let D::Object(first) = branches[0] else {
                panic!()
            };
            let mut first = first.to_vec();
            let D::Object(props) = property(branches[0], "properties") else {
                panic!()
            };
            let mut props = props.to_vec();
            let D::Array(required) = property(branches[0], "required") else {
                panic!()
            };
            let mut required = required.to_vec();
            let tight_text = [("type", D::Text("string")), ("minLength", D::Number(1.0))];
            let foreign_profile = [("const", D::Text("foreign/1"))];
            match mode {
                5 => {
                    first
                        .iter_mut()
                        .find(|(k, _)| *k == "additionalProperties")
                        .unwrap()
                        .1 = D::Boolean(true)
                }
                6 => {
                    props.iter_mut().find(|(k, _)| *k == "text").unwrap().1 = D::Object(&tight_text)
                }
                7 => required.clear(),
                8 => {
                    props.iter_mut().find(|(k, _)| *k == "profile").unwrap().1 =
                        D::Object(&foreign_profile)
                }
                9 => {
                    props.pop();
                }
                10 => {
                    props.reverse();
                    root.reverse();
                }
                _ => {}
            }
            first
                .iter_mut()
                .find(|(k, _)| *k == "properties")
                .unwrap()
                .1 = D::Object(&props);
            first.iter_mut().find(|(k, _)| *k == "required").unwrap().1 = D::Array(&required);
            branches[0] = D::Object(&first);
            match mode {
                1 => {
                    branches.pop();
                }
                2 => branches.reverse(),
                3 => branches[0] = branches[1],
                4 => root.push(("$ref", D::Text("https://not-loaded.invalid/schema"))),
                _ => {}
            }
            root.iter_mut().find(|(k, _)| *k == "oneOf").unwrap().1 = D::Array(&branches);
            let c =
                InertCaptureV1::copy(&D::Object(&root), limits, &mut Recorder::disabled()).unwrap();
            assert_eq!(
                ordinary_schema_matches_v1(&c, c.root()),
                mode == 0 || mode == 10,
                "mutation {mode}"
            );
        }
    })
    .unwrap();
}
