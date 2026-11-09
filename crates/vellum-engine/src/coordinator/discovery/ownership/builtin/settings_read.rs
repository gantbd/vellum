//! Installed F07 descriptors over the existing funded recipient reader.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredSettingsReadErrorV1 {
    Discovery(DiscoveryErrorV1),
    Native(crate::settings::SettingsError),
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[(&str, D<'_>); 11]>(),
        size_of::<[&str; 18]>(),
        size_of::<[[(&str, D<'_>); 6]; 3]>() * 2,
        size_of::<[[(&str, D<'_>); 5]; 3]>(),
        size_of::<[[(&str, D<'_>); 3]; 3]>() * 2,
        size_of::<[D<'_>; 6]>(),
        size_of::<[DiscoveryVariantInputV1<'_>; 3]>(),
        size_of::<[DiscoveryEntryInputV1<'_>; 1]>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
        size_of::<TrustedDiscoveryFactsV1>(),
    ]
    .into_iter()
    .try_fold(0, add)
}
fn with_input<T>(
    authority: &str,
    command: &str,
    visibility: &NativeBuiltinEntryVisibilityV1<3>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let names = [
        "effective-configuration",
        "permitted-origin",
        "capabilities",
    ];
    let keys = [
        "contract",
        "operation",
        "context",
        "target",
        "profile",
        "key",
    ];
    let roles = [
        "configuration-query/1",
        "",
        "OriginalOpaqueSettingsReadContextV1",
        "OriginalOpaqueSettingsReadTargetV1",
        SETTINGS_READ_PROFILE_V1,
        "NonemptySettingKey",
    ];
    let contract = [("const", D::Text("configuration-query/1"))];
    let operations = names.map(|name| [("const", D::Text(name))]);
    let opaque = [("type", D::Text("object"))];
    let profile = [("const", D::Text(SETTINGS_READ_PROFILE_V1))];
    let key = [("type", D::Text("string")), ("minLength", D::Number(1.0))];
    let count = |i| if i == 2 { 5 } else { 6 };
    let properties: [[(&str, D<'_>); 6]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (
                keys[j],
                match j {
                    0 => D::Object(&contract),
                    1 => D::Object(&operations[i]),
                    4 => D::Object(&profile),
                    5 => D::Object(&key),
                    _ => D::Object(&opaque),
                },
            )
        })
    });
    let required = keys.map(D::Text);
    let schemas: [[(&str, D<'_>); 5]; 3] = std::array::from_fn(|i| {
        [
            (
                "$schema",
                D::Text("https://json-schema.org/draft/2020-12/schema"),
            ),
            ("type", D::Text("object")),
            ("properties", D::Object(&properties[i][..count(i)])),
            ("required", D::Array(&required[..count(i)])),
            ("additionalProperties", D::Boolean(false)),
        ]
    });
    let fields: [[(&str, D<'_>); 6]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| (keys[j], D::Text(if j == 1 { names[i] } else { roles[j] })))
    });
    let inputs: [[(&str, D<'_>); 3]; 3] = std::array::from_fn(|i| {
        [
            ("contract", D::Text("configuration-query/1")),
            ("variant", D::Text(names[i])),
            ("fields", D::Object(&fields[i][..count(i)])),
        ]
    });
    let requirements: [[(&str, D<'_>); 3]; 3] = std::array::from_fn(|i| {
        [
            ("schema", D::Object(&schemas[i])),
            ("coverage", D::Text("outer-structure-only")),
            (
                "runtime",
                D::Text(
                    "Original context/target and captured scope; independently current native release and fresh one-use settings-root cut; complete attested key facts and retained ceiling; metadata grants no authority",
                ),
            ),
        ]
    });
    let variants: [DiscoveryVariantInputV1<'_>; 3] = std::array::from_fn(|i| {
        DiscoveryVariantInputV1 {
            id: names[i],
            input: D::Object(&inputs[i]),
            requirements: D::Object(&requirements[i]),
            outcomes: D::Text(
                "Owned filtered value, context-local permitted-only origins or accepted declarations; unavailable/limit/fault explicit",
            ),
            limits: D::Text(
                "Closed 2/4/5-or-6 borrowed envelope; bounded owned native capture; actual shared-parent classification plus seven request frames; existing read pool/capture/reply/work limits",
            ),
            cancellation: D::Text(
                "Immediate read only; query cancellation or temporary refusal does not stop the context; protected stop, cut disposal and retirement remain native",
            ),
            order: visibility.variants[i].order,
            enabled: visibility.variants[i].enabled,
        }
    });
    let entries = [DiscoveryEntryInputV1 {
        command: DiscoveryCommandV1 {
            authority,
            id: command,
            incarnation: 1,
        },
        contract: "configuration-query/1",
        version: 1,
        kind: DiscoveryExecutionKindV1::Query,
        variants: &variants,
        presentation: D::Object(&[]),
        order: visibility.order,
        schema_visible: visibility.schema,
        presentation_visible: visibility.presentation,
        advice_visible: visibility.advice,
    }];
    consume(DiscoveryCutInputV1 {
        entries: &entries,
        target: None,
        advice_present: false,
    })
}
impl SourceCoordinator {
    pub fn prepare_native_settings_read_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<3>,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, facts, r)?;
            let s = self.discovery_store()?;
            if s.limits.max_entries < 1 || s.limits.max_variants < 3 {
                return Err(Error::Limit);
            }
            let authority = text(&s.authority)?;
            let command = command(
                &s.engine,
                "F07-native-settings-read-v1",
                s.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            with_input(&authority, &command, visibility, |i| {
                self.prepare_discovery_cut_from_source(
                    scope,
                    i,
                    &facts,
                    Origin::NativeSettingsReadV1,
                    r,
                )
            })
        })();
        self.discovery_finish_timed(
            r,
            Boundary::DiscoveryBuiltinPreparation,
            &result,
            timing.finish(),
        );
        result
    }
    #[allow(clippy::too_many_arguments)]
    pub fn query_discovered_settings_read_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &SettingsReadEnvelopeValueV1<'_, '_>,
        limits: crate::capture::CaptureLimitsV1,
        scope: &SettingsReadScopeV1,
        cut: &SettingsReadCutIdV1,
        facts: &TrustedSettingsReadFactsV1,
        r: &mut Recorder,
    ) -> Result<SettingsReadReplyV1, DiscoveredSettingsReadErrorV1> {
        type E = DiscoveredSettingsReadErrorV1;
        let timing = r.begin_service_timing();
        let result = (|| {
            let operation = self
                .recognize_settings_read_request_v1(envelope)
                .map_err(E::Native)?;
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::settings_read_selector_matches(c, route.entry, route.variant, operation)
            });
            observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
            recognized.map_err(E::Discovery)?;
            self.query_discovery_v1(
                route.context,
                route.scope,
                route.cut,
                DiscoveryQueryV1::Inspect { entry: route.entry },
                route.facts,
                r,
            )
            .map_err(E::Discovery)?;
            self.query_decoded_settings_read_v1(envelope, limits, scope, cut, facts, r)
                .map_err(E::Native)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoverySettingsReadDispatch,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(E::Discovery(Error::Faulted))
                | Err(E::Native(crate::settings::SettingsError::Faulted)) => Disposition::Unknown,
                _ => Disposition::Refused,
            },
            service_time: timing.finish(),
        });
        result
    }
}
