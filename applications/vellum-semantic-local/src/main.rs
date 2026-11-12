#![forbid(unsafe_code)]

#[cfg(unix)]
fn main() {
    if let Err(error) = unix::run() {
        eprintln!("vellum-semantic-local: {error}");
        std::process::exit(2);
    }
}

#[cfg(not(unix))]
fn main() {
    eprintln!("vellum-semantic-local: the local semantic service requires Unix");
    std::process::exit(2);
}

#[cfg(unix)]
mod unix {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::num::NonZeroU64;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use vellum_adapter_local_authentication::{LocalCredentialStoreV1, LocalCredentialVerifierV1};
    use vellum_adapter_semantic_lsp::{
        LspIndexReadinessPortV1, LspIndexReadinessV1, LspLimitsV1, LspProviderProtocolProfileV1,
        LspReadinessNotificationSchemaV1, LspReadinessNotificationV1, LspServerHealthV1,
        LspVirtualDocumentIssuerV1, LspVirtualDocumentV1, SemanticGenerationIssuerV1,
    };
    use vellum_host::{
        authentication::AuthenticationGateV1,
        semantic::{
            SemanticDocumentRefV1, SemanticIndexGenerationV1, SemanticLimitsV1,
            SemanticProviderProvenanceV1, SemanticProviderRefV1,
        },
    };
    use vellum_semantic_client::SemanticClientRefV1;
    use vellum_semantic_local::{
        AuthenticatedSemanticConnectionLimitsV1, AuthenticatedSemanticDispatchLimitsV1,
        AuthenticatedSemanticRoleDecisionV1, AuthenticatedSemanticRolePortV1,
        LocalOrderedClockFailureV1, LocalOrderedDeadlinePolicyV1, LocalOrderedInstantV1,
        LocalOrderedMonotonicClockV1, SemanticLocalListenerV1, SemanticProcessDeadlinesV1,
        SemanticProcessLimitsV1, SemanticProviderProfileV1, authenticate_semantic_stream_v1,
        launch_supervised_ordered_lsp_with_deadlines_rust_analyzer_standalone_v1,
        qualify_provider_launch_v1, serve_one_authenticated_semantic_exchange_v1,
    };
    use vellum_semantic_protocol::{
        SemanticSessionIncarnationV1, SemanticSessionLimitsV1, SemanticSessionRolesV1,
    };
    use vellum_semantic_service::SemanticServiceLimitsV1;
    use vellum_semantic_session_actor::SemanticSessionActorLimitsV1;

    const HELP: &str = concat!(
        "vellum-semantic-local 0.1.0\n",
        "Supervised authenticated local semantic service.\n\n",
        "USAGE:\n",
        "  vellum-semantic-local --serve-probe --socket PRIVATE_SOCKET ",
        "--credential PRIVATE_CREDENTIAL \\\n",
        "    --provider ABSOLUTE_EXECUTABLE --provider-version EXACT_VERSION \\\n",
        "    --workspace-root ABSOLUTE_DIRECTORY ",
        "--private-cwd ABSOLUTE_PRIVATE_DIRECTORY \\\n",
        "    [--provider-arg ARG]... [--provider-env NAME=VALUE]...\n\n",
        "--serve-probe explicitly runs with observed launch evidence. It does not claim the\n",
        "closed production assurance lattice. The credential establishes attribution; this\n",
        "operator profile independently grants synchronize and query semantic roles.\n",
    );

    #[derive(Debug)]
    struct CliError(&'static str);
    impl std::fmt::Display for CliError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(self.0)
        }
    }
    impl std::error::Error for CliError {}

    struct Arguments {
        socket: PathBuf,
        credential: PathBuf,
        provider: PathBuf,
        provider_version: String,
        workspace_root: PathBuf,
        private_cwd: PathBuf,
        provider_arguments: Vec<String>,
        provider_environment: BTreeMap<String, String>,
    }

    fn parse() -> Result<Option<Arguments>, CliError> {
        let values = std::env::args_os().skip(1).collect::<Vec<OsString>>();
        if values.len() == 1 && values[0] == "--probe" {
            println!("vellum-semantic-local/0.1 unix-authenticated-session/1 probe-profile");
            return Ok(None);
        }
        if values.len() == 1 && (values[0] == "--help" || values[0] == "-h") {
            print!("{HELP}");
            return Ok(None);
        }
        if values.first().and_then(|value| value.to_str()) != Some("--serve-probe") {
            return Err(CliError("use --help for the explicit service profile"));
        }
        let mut socket = None;
        let mut credential = None;
        let mut provider = None;
        let mut provider_version = None;
        let mut workspace_root = None;
        let mut private_cwd = None;
        let mut provider_arguments = Vec::new();
        let mut provider_environment = BTreeMap::new();
        let mut cursor = 1;
        while cursor < values.len() {
            let flag = values[cursor]
                .to_str()
                .ok_or(CliError("non-UTF-8 option refused"))?;
            let value = values
                .get(cursor + 1)
                .ok_or(CliError("missing option value"))?;
            match flag {
                "--socket" if socket.is_none() => socket = Some(PathBuf::from(value)),
                "--credential" if credential.is_none() => credential = Some(PathBuf::from(value)),
                "--provider" if provider.is_none() => provider = Some(PathBuf::from(value)),
                "--provider-version" if provider_version.is_none() => {
                    provider_version = Some(
                        value
                            .to_str()
                            .ok_or(CliError("non-UTF-8 provider version refused"))?
                            .to_owned(),
                    )
                }
                "--workspace-root" if workspace_root.is_none() => {
                    workspace_root = Some(PathBuf::from(value))
                }
                "--private-cwd" if private_cwd.is_none() => {
                    private_cwd = Some(PathBuf::from(value))
                }
                "--provider-arg" => provider_arguments.push(
                    value
                        .to_str()
                        .ok_or(CliError("non-UTF-8 provider argument refused"))?
                        .to_owned(),
                ),
                "--provider-env" => {
                    let value = value
                        .to_str()
                        .ok_or(CliError("non-UTF-8 provider environment refused"))?;
                    let (name, value) = value
                        .split_once('=')
                        .ok_or(CliError("provider environment refused"))?;
                    if provider_environment
                        .insert(name.to_owned(), value.to_owned())
                        .is_some()
                    {
                        return Err(CliError("duplicate provider environment refused"));
                    }
                }
                _ => return Err(CliError("unknown or duplicate option")),
            }
            cursor += 2;
        }
        Ok(Some(Arguments {
            socket: socket.ok_or(CliError("missing socket"))?,
            credential: credential.ok_or(CliError("missing credential"))?,
            provider: provider.ok_or(CliError("missing provider"))?,
            provider_version: provider_version.ok_or(CliError("missing provider version"))?,
            workspace_root: workspace_root.ok_or(CliError("missing workspace root"))?,
            private_cwd: private_cwd.ok_or(CliError("missing private cwd"))?,
            provider_arguments,
            provider_environment,
        }))
    }

    struct Generations(u64);
    impl SemanticGenerationIssuerV1 for Generations {
        fn issue(&mut self) -> Option<SemanticIndexGenerationV1> {
            self.0 = self.0.checked_add(1)?;
            SemanticIndexGenerationV1::new(self.0)
        }
    }

    struct RustAnalyzerReadiness(LspIndexReadinessV1);
    impl LspIndexReadinessPortV1 for RustAnalyzerReadiness {
        fn poll_readiness(&mut self) -> LspIndexReadinessV1 {
            self.0
        }
        fn observe_notification(
            &mut self,
            notification: LspReadinessNotificationV1,
        ) -> Option<LspIndexReadinessV1> {
            self.0 = match (notification.health, notification.quiescent) {
                (LspServerHealthV1::Ok, true) => LspIndexReadinessV1::Current,
                (LspServerHealthV1::Error, _) => LspIndexReadinessV1::Unavailable,
                _ => LspIndexReadinessV1::Indexing,
            };
            Some(self.0)
        }
    }

    struct StandaloneDocument {
        issued: bool,
    }
    impl LspVirtualDocumentIssuerV1 for StandaloneDocument {
        fn issue(&mut self, _document: SemanticDocumentRefV1) -> Option<LspVirtualDocumentV1> {
            if self.issued {
                return None;
            }
            self.issued = true;
            LspVirtualDocumentV1::try_workspace_relative(
                "src/lib.rs".to_owned(),
                LspLimitsV1::reference(),
            )
        }
    }

    struct MonotonicClock {
        origin: Instant,
    }
    impl LocalOrderedMonotonicClockV1 for MonotonicClock {
        fn sample(&mut self) -> Result<LocalOrderedInstantV1, LocalOrderedClockFailureV1> {
            let ticks = u64::try_from(self.origin.elapsed().as_nanos())
                .map_err(|_| LocalOrderedClockFailureV1::Unavailable)?;
            Ok(LocalOrderedInstantV1::from_composition_tick(
                ticks.saturating_add(1),
            ))
        }
    }

    struct OperatorRolePolicy;
    impl AuthenticatedSemanticRolePortV1 for OperatorRolePolicy {
        type Context = ();
        fn derive_roles(
            &mut self,
            _authenticated: vellum_host::authority::AuthorityContextV1,
            _context: &(),
        ) -> AuthenticatedSemanticRoleDecisionV1 {
            AuthenticatedSemanticRoleDecisionV1::Grant {
                roles: SemanticSessionRolesV1::top(),
                policy_epoch: 1,
            }
        }
    }

    fn random_nonzero() -> Result<[u8; 16], CliError> {
        let mut value = [0_u8; 16];
        getrandom::fill(&mut value).map_err(|_| CliError("randomness unavailable"))?;
        if value == [0; 16] {
            return Err(CliError("randomness unavailable"));
        }
        Ok(value)
    }

    fn provider_version_token(value: &str) -> String {
        value
            .bytes()
            .map(|byte| {
                if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-') {
                    char::from(byte)
                } else {
                    '-'
                }
            })
            .collect()
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let Some(arguments) = parse()? else {
            return Ok(());
        };
        let record = LocalCredentialStoreV1::default()
            .load(&arguments.credential)
            .map_err(|_| CliError("credential loading refused"))?;
        let mut authentication =
            AuthenticationGateV1::new(LocalCredentialVerifierV1::new(record), u64::MAX);
        let provenance = SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new(random_nonzero()?),
            "rust-analyzer-standalone".to_owned(),
            provider_version_token(&arguments.provider_version),
        )
        .map_err(|_| CliError("provider provenance refused"))?;
        let profile = SemanticProviderProfileV1::try_new(
            provenance,
            arguments.provider,
            arguments.provider_arguments,
            arguments.provider_environment,
            arguments.private_cwd,
            arguments.workspace_root,
            SemanticProcessDeadlinesV1 {
                launch: Duration::from_secs(20),
                graceful_shutdown: Duration::from_secs(5),
                kill_and_reap: Duration::from_secs(5),
            },
            SemanticProcessLimitsV1::reference(),
        )
        .map_err(|_| CliError("provider profile refused"))?;
        let qualified = qualify_provider_launch_v1(profile)
            .map_err(|_| CliError("provider qualification refused"))?;
        let protocol = LspProviderProtocolProfileV1::try_new_with_readiness_schema(
            qualified.profile_identity().digest(),
            "rust-analyzer".to_owned(),
            Some(arguments.provider_version),
            "experimental/serverStatus".to_owned(),
            LspReadinessNotificationSchemaV1::HealthAndQuiescentWithOptionalMessage,
        )
        .ok_or(CliError("provider protocol profile refused"))?;
        let session_limits = SemanticSessionLimitsV1::reference();
        let mut service_limits = SemanticServiceLimitsV1::reference();
        service_limits.max_clients = 1;
        service_limits.max_provider_polls_per_pump = 1;
        let prepared = launch_supervised_ordered_lsp_with_deadlines_rust_analyzer_standalone_v1(
            qualified,
            Generations(0),
            RustAnalyzerReadiness(LspIndexReadinessV1::Indexing),
            StandaloneDocument { issued: false },
            LspLimitsV1::reference(),
            SemanticLimitsV1::reference(),
            SemanticClientRefV1::new(random_nonzero()?),
            SemanticSessionIncarnationV1::new(random_nonzero()?)
                .ok_or(CliError("session incarnation refused"))?,
            session_limits,
            service_limits,
            MonotonicClock {
                origin: Instant::now(),
            },
            LocalOrderedDeadlinePolicyV1 {
                relative_budget_ticks: NonZeroU64::new(30_000_000_000).unwrap(),
                max_deadlines: service_limits.max_pending,
            },
            &protocol,
        )
        .map_err(|_| CliError("semantic provider launch refused"))?;
        let (actor, sender) = prepared
            .spawn_managed(SemanticSessionActorLimitsV1::reference())
            .map_err(|_| CliError("semantic actor launch refused"))?;
        let listener = SemanticLocalListenerV1::try_bind(arguments.socket)
            .map_err(|_| CliError("semantic socket bind refused"))?;
        eprintln!("vellum-semantic-local: endpoint_ready probe-profile provider_indexing");
        loop {
            let mut stream = loop {
                match listener
                    .try_accept()
                    .map_err(|_| CliError("semantic endpoint fenced"))?
                {
                    Some(stream) => break stream,
                    None => {
                        if let Ok(ticket) = actor.try_drive_provider() {
                            let _ = ticket.receive();
                        }
                        if let Ok(ticket) = actor.try_drive_expiry() {
                            let _ = ticket.receive();
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
            };
            stream.set_read_timeout(Some(Duration::from_secs(30)))?;
            stream.set_write_timeout(Some(Duration::from_secs(30)))?;
            let challenge = random_nonzero()?;
            let session = random_nonzero()?;
            let authenticated = match authenticate_semantic_stream_v1(
                &mut authentication,
                &mut stream,
                challenge,
                session,
            ) {
                Ok(authenticated) => authenticated,
                Err(refusal) => {
                    eprintln!("vellum-semantic-local: authentication_refused reason={refusal:?}");
                    continue;
                }
            };
            // The short deadline bounds credential exchange only. A persistent
            // MCP backend must survive ordinary human/agent think time.
            stream.set_read_timeout(None)?;
            let mut role_policy = OperatorRolePolicy;
            let mut dispatcher =
                match vellum_semantic_local::AuthenticatedSemanticDispatcherV1::try_new(
                    authenticated,
                    &mut role_policy,
                    &(),
                    sender.clone(),
                    AuthenticatedSemanticDispatchLimitsV1::reference(),
                ) {
                    Ok(dispatcher) => dispatcher,
                    Err(failure) => {
                        eprintln!(
                            "vellum-semantic-local: dispatch_refused reason={:?} usage={:?}",
                            failure.reason(),
                            failure.usage()
                        );
                        continue;
                    }
                };
            loop {
                if let Err(refusal) = serve_one_authenticated_semantic_exchange_v1(
                    &mut dispatcher,
                    &mut stream,
                    AuthenticatedSemanticConnectionLimitsV1::reference(),
                ) {
                    eprintln!(
                        "vellum-semantic-local: exchange_refused reason={refusal:?} usage={:?}",
                        dispatcher.usage()
                    );
                    break;
                }
                if let Ok(ticket) = actor.try_drive_provider() {
                    let _ = ticket.receive();
                }
                if let Ok(ticket) = actor.try_drive_expiry() {
                    let _ = ticket.receive();
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::provider_version_token;

        #[test]
        fn observed_provider_version_has_a_valid_provenance_projection() {
            assert_eq!(
                provider_version_token("0.3.3113-standalone (e5e7830640 2026-11-08)"),
                "0.3.3113-standalone--e5e7830640-2026-11-08-"
            );
        }
    }
}
