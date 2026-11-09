//! Explicit test-only registry capacity, not SDK defaults or policy authority.
#![allow(dead_code)]
use vellum_engine::capture::CaptureLimitsV1;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
pub fn limits() -> DiscoveryLimitsV1 {
    let mut l = DiscoveryLimitsV1 {
        max_contexts: 2,
        max_preparations: 2,
        max_entries: 3,
        max_variants: 4,
        max_selectors: 16,
        max_issued: 32,
        max_page_rows: 3,
        max_root_bytes: 1048576,
        max_packet_bytes: 1048576,
        max_context_bytes: 4096,
        max_capture_bytes: 1048576,
        max_row_bytes: 262144,
        max_selector_bytes: 1024,
        max_reply_bytes: 262144,
        metadata: CaptureLimitsV1 {
            max_bytes: 262144,
            max_nodes: 4096,
            max_depth: 16,
        },
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.declared_bytes().unwrap();
    l
}
pub fn facts() -> TrustedDiscoveryFactsV1 {
    TrustedDiscoveryFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        discovery_allowed: Some(true),
        interpretation_current: Some(true),
        accepted_registry_attested: Some(true),
        schema_descriptions_attested: Some(true),
        visibility_attested: Some(true),
        visibility_complete: Some(true),
        projection_order_attested: Some(true),
        outside_publication_guard: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
        retention_ready: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        registry_ready: Some(true),
        reply_ready: Some(true),
        target_binding_attested: Some(true),
        target_equality_observable: Some(true),
        advice_attested: Some(true),
        target_current: Some(true),
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn visibility() -> NativeBuiltinDiscoveryVisibilityV2 {
    fn show<const N: usize>(order: usize) -> NativeBuiltinEntryVisibilityV1<N> {
        NativeBuiltinEntryVisibilityV1 {
            order: Some(order),
            schema: true,
            presentation: false,
            advice: false,
            variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
                order: Some(i),
                enabled: None,
            }),
        }
    }
    NativeBuiltinDiscoveryVisibilityV2 {
        legacy: NativeBuiltinDiscoveryVisibilityV1 {
            annotation: show(0),
            history: show(1),
        },
        context: show(2),
    }
}
pub struct Route {
    scope: DiscoveryScopeV1,
    context: DiscoveryContextIdV1,
    entry: DiscoveryEntryIdV1,
    variants: Vec<DiscoveryVariantDescriptorV1>,
}
impl Route {
    pub fn install(
        o: &mut SourceCoordinator,
        c: &SettingsExecutionContextV1,
        r: &mut Recorder,
    ) -> Self {
        let mut scope = DiscoveryScopeV1 {
            context: c.clone(),
            policy_epoch: 7,
        };
        scope.context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V2.into();
        o.configure_discovery_v1(&scope, limits(), &facts(), r)
            .unwrap();
        let prepare = |o: &mut SourceCoordinator, r: &mut Recorder| {
            o.prepare_native_builtin_discovery_cut_v2(
                &scope,
                &visibility(),
                None,
                false,
                &facts(),
                r,
            )
            .unwrap()
        };
        let p = prepare(o, r);
        let context = o
            .acquire_discovery_v1(&scope, &p, &facts(), r)
            .unwrap()
            .context;
        let p = prepare(o, r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
                &p,
                DiscoveryQueryV1::List {
                    limit: 3,
                    continuation: None,
                },
                &facts(),
                r,
            )
            .unwrap()
        else {
            panic!()
        };
        let entry = entries[2].reference.clone();
        let p = prepare(o, r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
                &p,
                DiscoveryQueryV1::Inspect { entry: &entry },
                &facts(),
                r,
            )
            .unwrap()
        else {
            panic!()
        };
        Self {
            scope,
            context,
            entry,
            variants: descriptor.variants,
        }
    }
    pub fn submit_lifecycle_with_results(
        &self,
        o: &mut SourceCoordinator,
        c: &SettingsExecutionContextV1,
        q: &NormalizedNativeLifecycleArgumentsV1<'_>,
        f: &TrustedNativeSettingsLifecycleInvocationV1<'_>,
        result: &TrustedCreatedLayerResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let p = o
            .prepare_native_builtin_discovery_cut_v2(
                &self.scope,
                &visibility(),
                None,
                false,
                &facts(),
                r,
            )
            .unwrap();
        let reply = o.submit_discovered_native_lifecycle_with_created_results_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &self.context,
                scope: &self.scope,
                cut: &p,
                entry: &self.entry,
                variant: &self.variants[0].reference,
                facts: &facts(),
            },
            q,
            c,
            f,
            result,
            r,
        );
        assert!(matches!(
            o.retire_discovery_cut_v1(&p, &facts(), r),
            Ok(()) | Err(DiscoveryErrorV1::Unavailable)
        ));
        reply
    }
    pub fn submit(
        &self,
        o: &mut SourceCoordinator,
        c: &SettingsExecutionContextV1,
        command: NativeDiscoveredContextCommandV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let index = match command {
            NativeDiscoveredContextCommandV1::NativeDefinition { .. }
            | NativeDiscoveredContextCommandV1::ScopedDefinition { .. }
            | NativeDiscoveredContextCommandV1::NativeRegistration { .. }
            | NativeDiscoveredContextCommandV1::ScopedRegistration { .. } => 1,
            _ => 0,
        };
        self.submit_with(o, c, command, index, &facts(), r)
    }
    pub fn submit_with(
        &self,
        o: &mut SourceCoordinator,
        c: &SettingsExecutionContextV1,
        command: NativeDiscoveredContextCommandV1<'_, '_>,
        index: usize,
        current: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let p = o
            .prepare_native_builtin_discovery_cut_v2(
                &self.scope,
                &visibility(),
                None,
                false,
                &facts(),
                r,
            )
            .unwrap();
        let result = o.submit_discovered_context_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &self.context,
                scope: &self.scope,
                cut: &p,
                entry: &self.entry,
                variant: &self.variants[index].reference,
                facts: current,
            },
            c,
            command,
            r,
        );
        // A shape refusal leaves our cut live; a query already consumed it.
        assert!(matches!(
            o.retire_discovery_cut_v1(&p, &facts(), r),
            Ok(()) | Err(DiscoveryErrorV1::Unavailable)
        ));
        result
    }
}
