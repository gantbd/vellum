#![forbid(unsafe_code)]

use std::process::ExitCode;

use serde_json::json;
use tokio::io::BufReader;
use vellum_semantic_client::{
    SemanticCapabilitySetV1, SemanticClientRefV1, SemanticProviderProvenanceV1,
    SemanticProviderRefV1, SemanticProviderStateV1, SemanticRequestRefV1, SemanticRequestV1,
    bind_semantic_client_v1,
};
use vellum_semantic_mcp::{
    SemanticMcpLimitsV1, SemanticMcpServerV1, SemanticMcpStdioFailureV1, SemanticMcpStdioLimitsV1,
    serve_stdio_v1,
};
use vellum_semantic_service::{
    SemanticClientRetirementStateV1, SemanticProviderAdmissionV1, SemanticProviderCompletionV1,
    SemanticProviderPortV1, SemanticProviderSubmissionV1, SemanticServiceLimitsV1,
    SemanticServiceV1, SemanticSnapshotUpdateV1,
};

const HELP: &str = "vellum-semantic-mcp 0.1.0\n\
Bounded stdio MCP peer for Vellum semantic queries.\n\n\
USAGE:\n\
    vellum-semantic-mcp --unavailable [--metrics-stderr]\n\
    vellum-semantic-mcp --probe\n\n\
OPTIONS:\n\
    --unavailable      Run an explicit no-provider composition for MCP discovery and\n\
                       harness qualification. Semantic queries fail closed.\n\
    --metrics-stderr   Emit one content-safe JSON usage record after clean EOF.\n\
    --probe            Print the executable protocol identity and exit.\n\
    -h, --help         Print this help.\n\n\
The default invocation refuses: a production semantic-session transport and\n\
sovereign-validation bridge have not been implemented. --unavailable never\n\
claims provider readiness or workspace membership.\n";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["--probe"] => {
            println!("vellum-semantic-mcp/0.1 stdio/2026-07-28");
            ExitCode::SUCCESS
        }
        ["-h"] | ["--help"] => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        ["--unavailable"] => run_unavailable(false),
        ["--unavailable", "--metrics-stderr"] | ["--metrics-stderr", "--unavailable"] => {
            run_unavailable(true)
        }
        [] => {
            eprintln!(
                "vellum-semantic-mcp: no semantic composition selected; use --unavailable only for protocol qualification"
            );
            ExitCode::from(2)
        }
        _ => {
            eprintln!("vellum-semantic-mcp: invalid arguments; use --help");
            ExitCode::from(2)
        }
    }
}

fn run_unavailable(metrics: bool) -> ExitCode {
    let provider = UnavailableProvider::new();
    let mut service =
        match SemanticServiceV1::try_new(provider, SemanticServiceLimitsV1::reference()) {
            Ok(service) => service,
            Err(_) => {
                eprintln!("vellum-semantic-mcp: local composition refused");
                return ExitCode::from(2);
            }
        };
    let client_ref = SemanticClientRefV1::new([0x6d; 16]);
    if service.admit_client(client_ref).is_err() {
        eprintln!("vellum-semantic-mcp: client admission refused");
        return ExitCode::from(2);
    }

    let runtime = match tokio::runtime::Builder::new_current_thread().build() {
        Ok(runtime) => runtime,
        Err(_) => {
            eprintln!("vellum-semantic-mcp: runtime initialization failed");
            return ExitCode::from(2);
        }
    };
    let (stdio_result, mcp_usage) = {
        let client = bind_semantic_client_v1(&mut service, client_ref);
        let mut server =
            match SemanticMcpServerV1::try_new(client, SemanticMcpLimitsV1::reference()) {
                Ok(server) => server,
                Err(_) => {
                    eprintln!("vellum-semantic-mcp: MCP composition refused");
                    return ExitCode::from(2);
                }
            };
        let mut input = BufReader::new(tokio::io::stdin());
        let mut output = tokio::io::stdout();
        let result = runtime.block_on(serve_stdio_v1(
            &mut server,
            &mut input,
            &mut output,
            SemanticMcpStdioLimitsV1::reference(),
        ));
        let mcp_usage = server.usage();
        (result, mcp_usage)
    };

    let retirement = service.retire_client(client_ref);
    let stdio_usage = match stdio_result {
        Ok(usage) => usage,
        Err(failure) => {
            report_failure(&failure);
            return ExitCode::FAILURE;
        }
    };
    if metrics {
        eprintln!(
            "{}",
            json!({
                "event":"vellum_semantic_mcp_exit",
                "backend":"unavailable",
                "stdio":{
                    "frames":stdio_usage.frames,
                    "frame_bytes":stdio_usage.frame_bytes,
                    "responses":stdio_usage.responses,
                    "response_bytes":stdio_usage.response_bytes,
                    "notifications":stdio_usage.notifications,
                    "oversized_frames":stdio_usage.oversized_frames,
                    "truncated_frames":stdio_usage.truncated_frames,
                    "io_failures":stdio_usage.io_failures,
                    "saturated":stdio_usage.saturated
                },
                "mcp":{
                    "requests":mcp_usage.requests,
                    "tool_calls":mcp_usage.tool_calls,
                    "queries_submitted":mcp_usage.queries_submitted,
                    "refused":mcp_usage.refused,
                    "protocol_errors":mcp_usage.protocol_errors,
                    "saturated":mcp_usage.saturated
                },
                "retirement":retirement_state(retirement.state)
            })
        );
    }
    ExitCode::SUCCESS
}

const fn retirement_state(state: SemanticClientRetirementStateV1) -> &'static str {
    match state {
        SemanticClientRetirementStateV1::Absent => "absent",
        SemanticClientRetirementStateV1::Pending { .. } => "pending",
        SemanticClientRetirementStateV1::Retired { .. } => "retired",
    }
}

fn report_failure(failure: &SemanticMcpStdioFailureV1) {
    match failure {
        SemanticMcpStdioFailureV1::InvalidLimits => {
            eprintln!("vellum-semantic-mcp: invalid stdio limits");
        }
        SemanticMcpStdioFailureV1::Io { .. } => {
            eprintln!("vellum-semantic-mcp: stdio failed");
        }
    }
}

struct UnavailableProvider {
    state: SemanticProviderStateV1,
}

impl UnavailableProvider {
    fn new() -> Self {
        let provenance = SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([0x75; 16]),
            "unavailable-composition".to_owned(),
            env!("CARGO_PKG_VERSION").to_owned(),
        )
        .expect("fixed unavailable provider provenance is valid");
        Self {
            state: SemanticProviderStateV1 {
                provider: provenance,
                readiness: vellum_semantic_client::SemanticProviderReadinessV1::Unavailable,
                capabilities: SemanticCapabilitySetV1::bottom(),
            },
        }
    }
}

impl SemanticProviderPortV1 for UnavailableProvider {
    fn state(&self) -> SemanticProviderStateV1 {
        self.state.clone()
    }

    fn synchronize(&mut self, _update: &SemanticSnapshotUpdateV1) -> SemanticProviderAdmissionV1 {
        SemanticProviderAdmissionV1::Unavailable
    }

    fn submit(&mut self, _request: &SemanticRequestV1) -> SemanticProviderSubmissionV1 {
        SemanticProviderSubmissionV1::Unavailable
    }

    fn cancel(&mut self, _request: SemanticRequestRefV1) {}

    fn poll(&mut self) -> Option<SemanticProviderCompletionV1> {
        None
    }
}
