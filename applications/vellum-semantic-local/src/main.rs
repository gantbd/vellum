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
    use std::net::Shutdown;
    use std::num::NonZeroU64;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    use std::thread::JoinHandle;
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
        LocalOrderedManagedTimedStopResolutionV1, LocalOrderedMonotonicClockV1,
        LocalOrderedTimedStoppedRecoveryAdvanceV1, LocalSemanticRetirementStateV1,
        SemanticLocalListenerV1, SemanticProcessDeadlinesV1, SemanticProcessLimitsV1,
        SemanticProviderProfileV1, authenticate_semantic_stream_v1,
        launch_supervised_ordered_lsp_with_deadlines_rust_analyzer_standalone_v1,
        qualify_provider_launch_v1, serve_one_authenticated_semantic_exchange_v1,
    };
    use vellum_semantic_protocol::{
        SemanticSessionIncarnationV1, SemanticSessionLimitsV1, SemanticSessionRolesV1,
    };
    use vellum_semantic_service::SemanticClientRetirementStateV1;
    use vellum_semantic_service::SemanticServiceLimitsV1;
    use vellum_semantic_session_actor::{
        SemanticSessionActorEventAdmissionV1, SemanticSessionActorLimitsV1,
    };

    struct SignalSupervisor {
        requested: Arc<AtomicBool>,
        active_stream: Arc<Mutex<Option<UnixStream>>>,
        handle: signal_hook::iterator::Handle,
        thread: Option<JoinHandle<()>>,
    }

    impl SignalSupervisor {
        fn install() -> Result<Self, CliError> {
            let mut signals = signal_hook::iterator::Signals::new([
                signal_hook::consts::signal::SIGINT,
                signal_hook::consts::signal::SIGTERM,
            ])
            .map_err(|_| CliError("signal supervision refused"))?;
            let handle = signals.handle();
            let requested = Arc::new(AtomicBool::new(false));
            let active_stream = Arc::new(Mutex::new(None::<UnixStream>));
            let signal_requested = Arc::clone(&requested);
            let signal_stream = Arc::clone(&active_stream);
            let thread = std::thread::Builder::new()
                .name("vellum-semantic-signal".to_owned())
                .spawn(move || {
                    if signals.forever().next().is_some() {
                        signal_requested.store(true, Ordering::Release);
                        if let Some(stream) = signal_stream
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .as_ref()
                        {
                            let _ = stream.shutdown(Shutdown::Both);
                        }
                    }
                })
                .map_err(|_| CliError("signal supervision refused"))?;
            Ok(Self {
                requested,
                active_stream,
                handle,
                thread: Some(thread),
            })
        }

        fn requested(&self) -> bool {
            self.requested.load(Ordering::Acquire)
        }

        fn track(&self, stream: &UnixStream) -> Result<(), CliError> {
            let tracked = stream
                .try_clone()
                .map_err(|_| CliError("semantic connection tracking refused"))?;
            *self
                .active_stream
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(tracked);
            if self.requested()
                && let Some(stream) = self
                    .active_stream
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .as_ref()
            {
                let _ = stream.shutdown(Shutdown::Both);
            }
            Ok(())
        }

        fn clear(&self) {
            self.active_stream
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take();
        }

        fn finish(mut self) -> Result<(), CliError> {
            self.clear();
            self.handle.close();
            if self
                .thread
                .take()
                .is_some_and(|thread| thread.join().is_err())
            {
                return Err(CliError("signal supervisor join refused"));
            }
            Ok(())
        }
    }

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

    #[derive(Default)]
    struct StandaloneDocuments {
        assignments: BTreeMap<SemanticDocumentRefV1, LspVirtualDocumentV1>,
    }
    impl LspVirtualDocumentIssuerV1 for StandaloneDocuments {
        fn issue(&mut self, document: SemanticDocumentRefV1) -> Option<LspVirtualDocumentV1> {
            if let Some(locator) = self.assignments.get(&document) {
                return Some(locator.clone());
            }
            let relative = if self.assignments.is_empty() {
                "src/lib.rs".to_owned()
            } else {
                use std::fmt::Write as _;
                let mut relative = String::with_capacity(15 + 32 + 3);
                relative.push_str("src/vellum-doc-");
                for byte in document.as_bytes() {
                    write!(&mut relative, "{byte:02x}").ok()?;
                }
                relative.push_str(".rs");
                relative
            };
            let locator =
                LspVirtualDocumentV1::try_workspace_relative(relative, LspLimitsV1::reference())?;
            self.assignments.insert(document, locator.clone());
            Some(locator)
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
            StandaloneDocuments::default(),
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
        let shutdown = SignalSupervisor::install()?;
        eprintln!("vellum-semantic-local: endpoint_ready probe-profile provider_indexing");
        'serve: loop {
            let mut stream = loop {
                if shutdown.requested() {
                    break 'serve;
                }
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
            shutdown.track(&stream)?;
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
                    shutdown.clear();
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
                        shutdown.clear();
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
                    shutdown.clear();
                    if shutdown.requested() {
                        if let SemanticSessionActorEventAdmissionV1::Submitted(ticket) =
                            actor.try_event(dispatcher.authority_lost_event())
                        {
                            let _ = ticket.receive();
                        }
                        break 'serve;
                    }
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

        drop(listener);
        drop(sender);
        shutdown.finish()?;
        eprintln!("vellum-semantic-local: shutdown_draining");

        let _ = actor.begin_drain();
        let mut actor = Some(actor);
        let mut stopped = None;
        'stop: for _ in 0..64 {
            let current = actor
                .take()
                .ok_or(CliError("semantic actor ownership lost"))?;
            if let Ok(ticket) = current.try_drive_retirement() {
                let _ = ticket.receive();
            }
            let mut ticket = match current.try_begin_stop() {
                Ok(ticket) => ticket,
                Err(failure) => {
                    actor = Some(failure.into_actor());
                    std::thread::yield_now();
                    continue;
                }
            };
            for _ in 0..4_096 {
                match ticket.try_complete() {
                    Ok(LocalOrderedManagedTimedStopResolutionV1::Terminal(terminal)) => {
                        stopped = Some(terminal);
                        break 'stop;
                    }
                    Ok(LocalOrderedManagedTimedStopResolutionV1::Refused(refusal)) => {
                        actor = Some(refusal.into_actor());
                        continue 'stop;
                    }
                    Err(pending) => {
                        ticket = *pending;
                        std::thread::yield_now();
                    }
                }
            }
            stopped = Some(ticket.abandon());
            break;
        }
        let mut stopped = stopped.ok_or(CliError("semantic actor retirement exhausted"))?;
        let recovery_deadline = Instant::now() + Duration::from_secs(5);
        let timed_cleanup = loop {
            match stopped.try_advance() {
                LocalOrderedTimedStoppedRecoveryAdvanceV1::Pending(pending) => {
                    if Instant::now() >= recovery_deadline {
                        return Err(Box::new(CliError("semantic actor recovery exhausted")));
                    }
                    stopped = *pending;
                    std::thread::yield_now();
                }
                LocalOrderedTimedStoppedRecoveryAdvanceV1::Ready(cleanup) => break *cleanup,
                LocalOrderedTimedStoppedRecoveryAdvanceV1::Mismatch(mismatch) => {
                    break mismatch.into_cleanup();
                }
            }
        };
        let mut cleanup = match timed_cleanup.try_retire_deadlines() {
            Ok(cleanup) => cleanup,
            Err(unresolved) => unresolved.abandon_deadlines(),
        };
        for _ in 0..256 {
            if matches!(
                cleanup.drive_client_retirement_once().state,
                SemanticClientRetirementStateV1::Absent
                    | SemanticClientRetirementStateV1::Retired { .. }
            ) {
                break;
            }
        }
        let mut provider = match cleanup.try_into_provider_cleanup() {
            Ok(provider) => provider,
            Err(blocked) => blocked.into_owner().force_into_provider_cleanup(
                vellum_semantic_service::SemanticBridgeForcedEffectPolicyV1::AttemptBounded,
            ),
        };
        let graceful_deadline = Instant::now() + Duration::from_secs(7);
        let terminal_deadline = Instant::now() + Duration::from_secs(14);
        loop {
            let state = provider.drive_once();
            if matches!(
                state,
                LocalSemanticRetirementStateV1::GracefulConfirmed
                    | LocalSemanticRetirementStateV1::ForcedConfirmed
                    | LocalSemanticRetirementStateV1::CleanupFailed
            ) {
                break;
            }
            if Instant::now() >= graceful_deadline
                && state != LocalSemanticRetirementStateV1::ForcedShutdownPending
            {
                let _ = provider.escalate();
            }
            if Instant::now() >= terminal_deadline {
                return Err(Box::new(CliError("semantic provider retirement exhausted")));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let receipt = provider
            .try_into_terminal_receipt()
            .map_err(|_| CliError("semantic provider retirement incomplete"))?;
        eprintln!(
            "vellum-semantic-local: shutdown_complete provider={:?}",
            receipt.provider().terminal()
        );
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::{StandaloneDocuments, provider_version_token};
        use vellum_adapter_semantic_lsp::{
            LspLimitsV1, LspVirtualDocumentIssuerV1, LspVirtualDocumentV1,
        };
        use vellum_host::semantic::SemanticDocumentRefV1;

        #[test]
        fn observed_provider_version_has_a_valid_provenance_projection() {
            assert_eq!(
                provider_version_token("0.3.3113-standalone (e5e7830640 2026-11-08)"),
                "0.3.3113-standalone--e5e7830640-2026-11-08-"
            );
        }

        #[test]
        fn standalone_virtual_documents_are_repeatable_and_injective() {
            let first = SemanticDocumentRefV1::new([1; 16]);
            let second = SemanticDocumentRefV1::new([2; 16]);
            let mut documents = StandaloneDocuments::default();
            let first_locator = documents.issue(first).unwrap();
            assert_eq!(
                first_locator,
                LspVirtualDocumentV1::try_workspace_relative(
                    "src/lib.rs".to_owned(),
                    LspLimitsV1::reference(),
                )
                .unwrap()
            );
            assert_eq!(documents.issue(first), Some(first_locator.clone()));
            assert_ne!(documents.issue(second), Some(first_locator));
        }
    }
}
