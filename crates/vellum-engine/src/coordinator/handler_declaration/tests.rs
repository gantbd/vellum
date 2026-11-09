use super::*;
use crate::coordinator::*;

fn arguments() -> [&'static NativeArgumentDeclarationV1; 10] {
    [
        NormalizedResourceRegistrationArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedScopedRegistrationArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedSourceSerializationArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedScopedSerializationArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedNativeSettingsArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedScopedSettingsArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedNativeDefinitionArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedScopedDefinitionArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedNativeLifecycleArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedScopedLifecycleArgumentsV1::ARGUMENT_DECLARATION,
    ]
}

#[test]
fn ten_handler_owned_argument_products_keep_the_specified_native_and_scoped_headers_distinct() {
    let expected = [
        ("registration-invocation/1", "activate-context", 5),
        ("scoped-registration-invocation/1", "activate-context", 5),
        ("source-serialization/1", "semantic-settings", 8),
        ("scoped-source-serialization/1", "semantic-settings", 6),
        ("semantic-settings-activation/1", "semantic-settings", 6),
        ("scoped-semantic-settings/1", "semantic-settings", 6),
        ("engine-settings-definition/1", "supported-registry", 7),
        (
            "scoped-engine-settings-definition/1",
            "supported-registry",
            6,
        ),
        ("engine-settings-layers/1", "semantic-settings", 9),
        ("scoped-engine-settings-layers/1", "semantic-settings", 8),
    ];
    let fields: [&[&str]; 10] = [
        &["contract", "operation", "invocation", "profile", "request"],
        &["contract", "operation", "invocation", "profile", "request"],
        &[
            "contract",
            "operation",
            "invocation",
            "profile",
            "document",
            "expectedSerialization",
            "serialization",
            "dependencies",
        ],
        &[
            "contract",
            "operation",
            "invocation",
            "profile",
            "serialization",
            "dependencies",
        ],
        &[
            "contract",
            "operation",
            "invocation",
            "profile",
            "changes",
            "dependencies",
        ],
        &[
            "contract",
            "operation",
            "invocation",
            "profile",
            "changes",
            "dependencies",
        ],
        &[
            "contract",
            "operation",
            "invocation",
            "profile",
            "expected",
            "definition",
            "dependencies",
        ],
        &[
            "contract",
            "operation",
            "invocation",
            "profile",
            "definition",
            "dependencies",
        ],
        &[
            "contract",
            "operation",
            "invocation",
            "profile",
            "expected",
            "create",
            "remove",
            "targets",
            "dependencies",
        ],
        &[
            "contract",
            "operation",
            "invocation",
            "profile",
            "create",
            "remove",
            "targets",
            "dependencies",
        ],
    ];
    for ((declaration, (contract, operation, count)), fields) in
        arguments().into_iter().zip(expected).zip(fields)
    {
        assert_eq!(declaration.contract, contract);
        assert_eq!(declaration.variant.operation, operation);
        assert_eq!(declaration.variant.fields.len(), count);
        assert_eq!(
            declaration
                .variant
                .fields
                .iter()
                .map(|(k, _)| *k)
                .collect::<Vec<_>>(),
            fields
        );
        assert!(
            declaration
                .variant
                .fields
                .iter()
                .all(|(k, v)| !k.is_empty() && !v.is_empty())
        );
        assert!(
            !declaration
                .variant
                .fields
                .iter()
                .any(|(k, _)| *k == "basis")
        );
    }
}

#[test]
fn all_headers_reject_missing_extra_duplicate_and_ui_fields_before_unbounded_work() {
    for declaration in arguments() {
        let names: Vec<_> = declaration.variant.fields.iter().map(|(k, _)| *k).collect();
        assert!(declaration.fields_match(names.iter().copied()));
        assert!(declaration.fields_match(names.iter().rev().copied()));
        assert!(!declaration.fields_match(std::iter::repeat_n("contract", usize::MAX)));
        for i in 0..names.len() {
            let mut missing = names.clone();
            missing.remove(i);
            assert!(!declaration.fields_match(missing.iter().copied()));
            let mut unexpected = names.clone();
            unexpected[i] = "keymap";
            assert!(!declaration.fields_match(unexpected.iter().copied()));
            let mut duplicate = names.clone();
            duplicate[i] = names[(i + 1) % names.len()];
            assert!(!declaration.fields_match(duplicate.iter().copied()));
        }
        let mut extra = names;
        extra.push("basis");
        assert!(!declaration.fields_match(extra.iter().copied()));
    }
}
