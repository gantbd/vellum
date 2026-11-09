//! Public, host-authenticated recipient read from configuration to final discharge.
use super::*;

#[test]
fn recipient_can_inspect_configuration_without_receiving_native_layer_identity() {
    for route in [None, Some(true), Some(false)] {
        exercise_recipient(route);
    }
}
fn exercise_recipient(route: Option<bool>) {
    let mut r = Recorder::disabled();
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 128,
        source: Limits {
            max_source_bytes: 128,
            max_edits: 8,
            max_edit_payload_bytes: 256,
        },
    });
    o.install_initial_settings(
        compile_settings_v1(&definition(), "engine", limits(), &mut r).unwrap(),
        &mut r,
    )
    .unwrap();
    let mut budget = SettingsReaderLimitsV1 {
        max_issued: 16,
        max_packets: 2,
        max_contexts: 1,
        max_origins: 4,
        max_packet_bytes: 65536,
        max_capture_bytes: 8192,
        max_reply_bytes: 8192,
        max_context_bytes: 4096,
        max_label_bytes: 128,
        max_depth: 64,
        max_work: 1_048_576,
        max_pool_bytes: usize::MAX,
    };
    budget.max_pool_bytes = budget.required_pool_bytes().unwrap();
    let decode_bytes = settings_read_decode_classification_bytes_v1() + 7 * 8192;
    o.configure_retention_budget(budget.max_pool_bytes + decode_bytes, &mut r)
        .unwrap();
    o.configure_settings_reader_v1("engine", "local", budget, &mut r)
        .unwrap();
    let native_target = o.settings_target(&id("document"), &mut r).unwrap();
    let scope = SettingsReadScopeV1 {
        execution: SettingsExecutionContextV1 {
            authority: "engine".into(),
            engine: "local".into(),
            recipient: "sdk-client".into(),
            purpose: "inspect".into(),
            destination: "client".into(),
            profile: SETTINGS_READ_PROFILE_V1.into(),
            continuity: "session".into(),
        },
        policy: "current-policy".into(),
    };
    let current = TrustedSettingsReadFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        retained: Some(true),
        outside_publication: Some(true),
        resources_ready: Some(true),
        transfer_ready: Some(true),
        reply_ready: Some(true),
    };
    let ids = [SettingProjectionOriginV1::Layer(id("host"))];
    let keys = [SettingProjectionGrantV1 {
        key: "unit",
        visible: true,
        value: true,
        origins: true,
        declaration: true,
        origin_ids: &ids,
    }];
    let attested = TrustedSettingsReadPreparationV1 {
        read: &current,
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        visibility_attested: Some(true),
        visibility_complete: Some(true),
        target_readable: Some(true),
        keys: &keys,
        capabilities: true,
    };
    let cut = o
        .prepare_settings_read_v1(&native_target, &scope, &attested, &mut r)
        .unwrap();
    let reader = o
        .acquire_settings_read_v1(&cut, &scope, &current, &mut r)
        .unwrap();
    let cut = o
        .prepare_settings_read_v1(&native_target, &scope, &attested, &mut r)
        .unwrap();
    let SettingsReadResultV1::Value(value) = read(
        &mut o,
        &reader,
        &scope,
        SettingsReadOperationV1::EffectiveConfiguration { key: "unit" },
        &cut,
        &current,
        route,
        &mut r,
    ) else {
        panic!()
    };
    assert_eq!(value.read().number(), Some(3.0));
    let cut = o
        .prepare_settings_read_v1(&native_target, &scope, &attested, &mut r)
        .unwrap();
    let origins = read(
        &mut o,
        &reader,
        &scope,
        SettingsReadOperationV1::PermittedOrigin { key: "unit" },
        &cut,
        &current,
        route,
        &mut r,
    );
    assert_eq!(origins.coverage(), "permitted-only");
    let SettingsReadResultV1::Origins(SettingsReadOriginsV1::Whole(refs)) = origins else {
        panic!()
    };
    assert_eq!(refs.len(), 1);
    assert!(!format!("{refs:?}").contains("host"));
    let controls = SettingsReadControlV1 {
        scope: &scope,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert!(
        o.stop_settings_read_v1(&reader.context, &controls, &mut r)
            .unwrap()
    );
    assert!(
        o.retire_settings_read_v1(&reader.context, &controls, &mut r)
            .unwrap()
    );
    assert!(o.retire_settings_reader_v1(&mut r).unwrap());
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(value.read().number(), Some(3.0));
}

#[allow(clippy::too_many_arguments)]
fn read(
    o: &mut SourceCoordinator,
    reader: &SettingsReadAcquisitionV1,
    scope: &SettingsReadScopeV1,
    operation: SettingsReadOperationV1<'_>,
    cut: &SettingsReadCutIdV1,
    facts: &TrustedSettingsReadFactsV1,
    route: Option<bool>,
    r: &mut Recorder,
) -> SettingsReadResultV1 {
    let Some(typed) = route else {
        return o
            .query_settings_read_v1(
                &reader.context,
                &reader.target,
                scope,
                operation,
                cut,
                facts,
                r,
            )
            .unwrap();
    };
    use SettingsReadEnvelopeValueV1 as V;
    use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
    let (operation, key) = match operation {
        SettingsReadOperationV1::EffectiveConfiguration { key } => ("effective-configuration", key),
        SettingsReadOperationV1::PermittedOrigin { key } => ("permitted-origin", key),
        SettingsReadOperationV1::Capabilities => ("capabilities", ""),
    };
    let args = [
        ("contract", V::Data(D::Text("configuration-query/1"))),
        ("operation", V::Data(D::Text(operation))),
        ("context", V::Context(&reader.context)),
        ("target", V::Target(&reader.target)),
        ("profile", V::Data(D::Text(SETTINGS_READ_PROFILE_V1))),
        ("key", V::Data(D::Text(key))),
    ];
    let header = [
        (
            if typed { "family" } else { "name" },
            if typed {
                V::Data(D::Number(6.0))
            } else {
                V::Data(D::Text("inspect-context"))
            },
        ),
        ("variant", V::Data(D::Text(operation))),
        ("version", V::Data(D::Number(1.0))),
        (
            "arguments",
            V::Object(&args[..if operation == "capabilities" { 5 } else { 6 }]),
        ),
    ];
    let envelope = [
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&header)),
    ];
    let result = o
        .query_decoded_settings_read_v1(
            &V::Object(&envelope),
            CaptureLimitsV1 {
                max_bytes: 8192,
                max_nodes: 128,
                max_depth: 8,
            },
            scope,
            cut,
            facts,
            r,
        )
        .unwrap();
    match result {
        SettingsReadReplyV1::Value {
            context,
            target,
            key: returned_key,
            value,
        } => {
            assert_eq!(context, reader.context);
            assert_eq!(target, reader.target);
            assert_eq!(returned_key, key);
            SettingsReadResultV1::Value(value)
        }
        SettingsReadReplyV1::Origins {
            context,
            target,
            key: returned_key,
            origins,
        } => {
            assert_eq!(context, reader.context);
            assert_eq!(target, reader.target);
            assert_eq!(returned_key, key);
            SettingsReadResultV1::Origins(origins)
        }
        SettingsReadReplyV1::Capabilities {
            context,
            target,
            capabilities,
        } => {
            assert_eq!(context, reader.context);
            assert_eq!(target, reader.target);
            SettingsReadResultV1::Capabilities(capabilities)
        }
    }
}
