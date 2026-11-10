use vellum_engine::coordinator::*;
pub fn facts() -> TrustedResourceCatalogFactsV1 {
    TrustedResourceCatalogFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        registration_allowed: Some(true),
        identity_allowed: Some(true),
        registry_coverage_complete: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
    }
}
pub fn limits() -> ResourceCatalogLimitsV1 {
    ResourceCatalogLimitsV1 {
        max_relations: 8,
        max_tag_bytes: 128,
        max_request_bytes: 16384,
        max_result_bytes: 8192,
        max_plan_bytes: 131072,
    }
}
pub fn address(table: ResourceCatalogTableV1, id: &str) -> ResourceCatalogAddressV1 {
    ResourceCatalogAddressV1 {
        table,
        provider: ResourceCatalogIdentityV1 {
            authority: "a".into(),
            id: "provider".into(),
            incarnation: 1,
        },
        resource: ResourceCatalogIdentityV1 {
            authority: "a".into(),
            id: id.into(),
            incarnation: 1,
        },
    }
}
pub fn configuration(table: ResourceCatalogTableV1) -> ResourceCatalogConfigurationV1 {
    ResourceCatalogConfigurationV1 {
        identity_domain: "domain".into(),
        identity_key: "private identity".into(),
        identity_known: false,
        locator: "private opaque locator".into(),
        capabilities: match table {
            ResourceCatalogTableV1::Read => {
                ResourceCatalogCapabilitiesV1::AnnotationsV1 { atomic_read: false }
            }
            ResourceCatalogTableV1::Write => ResourceCatalogCapabilitiesV1::Write {
                procedure: "opaque-procedure".into(),
                logical_continuity: false,
                synchronization_supported: false,
                conditional_write: false,
                causal_receipts: false,
                non_reuse_tokens: false,
                exclusive_create: false,
            },
        },
    }
}
pub fn install(table: ResourceCatalogTableV1, id: &str) -> ResourceCatalogChangeV1 {
    ResourceCatalogChangeV1 {
        address: address(table, id),
        expected: CatalogExpectationV1::Absent,
        action: ResourceCatalogActionV1::Install(configuration(table)),
    }
}
