//! Run: cargo run -p vellum-engine --example headless_edit
//!
//! A trusted, in-process embedding, not an SDK, UI, policy evaluator or filesystem
//! adapter. The explicit positive facts below are premises supplied by this demo
//! host. Real hosts must establish them independently at admission/service/release;
//! discovery visibility and opaque handle possession are not authorization.
use std::mem::size_of;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{NativeTimingProfileV1, Recorder};
use vellum_engine::source::{Limits, Replacement};

#[derive(Debug)]
pub struct Report {
    pub text: String,
    pub issued: usize,
    pub retained_invocations: usize,
    pub retained_history: usize,
    pub reserved_bytes: usize,
    pub dropped: u64,
}

fn discovery_facts() -> TrustedDiscoveryFactsV1 {
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

fn discovery_limits() -> DiscoveryLimitsV1 {
    let mut l = DiscoveryLimitsV1 {
        max_contexts: 1,
        max_preparations: 1,
        max_entries: 1,
        max_variants: 1,
        max_selectors: 16,
        max_issued: 32,
        max_page_rows: 1,
        max_root_bytes: 65536,
        max_packet_bytes: 65536,
        max_context_bytes: 256,
        max_capture_bytes: 65536,
        max_row_bytes: 32768,
        max_selector_bytes: 1024,
        max_reply_bytes: 65536,
        metadata: CaptureLimitsV1 {
            max_bytes: 8192,
            max_nodes: 512,
            max_depth: 12,
        },
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.declared_bytes().expect("finite discovery limits");
    l
}

fn prepare(
    owner: &mut SourceCoordinator,
    scope: &DiscoveryScopeV1,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    owner
        .prepare_native_exact_batch_discovery_cut_v1(
            scope,
            &NativeBuiltinEntryVisibilityV1 {
                order: Some(0),
                schema: true,
                presentation: false,
                advice: false,
                variants: [NativeBuiltinVariantVisibilityV1 {
                    order: Some(0),
                    enabled: None,
                }],
            },
            &discovery_facts(),
            r,
        )
        .expect("prepare installed command")
}

fn policy_facts(c: &SettingsExecutionContextV1) -> TrustedInvocationPolicyFactsV1<'_> {
    TrustedInvocationPolicyFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        resources_ready: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        preparation_reserved: Some(true),
        registry_ready: Some(true),
        retention_ready: Some(true),
    }
}

/// Execute and validate one complete session. Capture controls affect diagnostics
/// only. All limits are example-host budgets, not engine defaults or product SLOs.
pub fn run(capacity: usize, timed: bool) -> Report {
    let mut r = Recorder::try_with_capacity(capacity)
        .expect("capture budget")
        .with_native_timing(if timed {
            NativeTimingProfileV1::ProcessMonotonicV1
        } else {
            NativeTimingProfileV1::Off
        });
    let mut owner = SourceCoordinator::with_invocation_and_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 1,
            max_participants: 1,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 4,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 1,
            max_retained: 1,
            max_intent_bytes: 65536,
        },
        HistoryLimits {
            max_retained_groups: 1,
            max_journal_bytes: 65536,
            max_pins: 0,
        },
    );
    let dl = discovery_limits();
    owner
        .configure_retention_budget(dl.max_held_bytes + 1048576, &mut r)
        .expect("fund discovery and observation");
    let actor = SettingsExecutionContextV1 {
        authority: "example-host".into(),
        engine: "local".into(),
        recipient: "local-agent".into(),
        purpose: "edit".into(),
        destination: "in-process".into(),
        profile: BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1.into(),
        continuity: "session".into(),
    };
    let scope = DiscoveryScopeV1 {
        context: SettingsExecutionContextV1 {
            profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
            ..actor.clone()
        },
        policy_epoch: 1,
    };
    owner
        .configure_discovery_v1(&scope, dl, &discovery_facts(), &mut r)
        .expect("install discovery budget");
    let cut = prepare(&mut owner, &scope, &mut r);
    let discovery = owner
        .acquire_discovery_v1(&scope, &cut, &discovery_facts(), &mut r)
        .expect("acquire discovery")
        .context;
    let cut = prepare(&mut owner, &scope, &mut r);
    let DiscoveryReplyV1::Page { entries, .. } = owner
        .query_discovery_v1(
            &discovery,
            &scope,
            &cut,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
            &discovery_facts(),
            &mut r,
        )
        .expect("list installed commands")
    else {
        panic!("expected discovery page")
    };
    assert_eq!(entries.len(), 1);
    let entry = &entries[0].reference;
    let cut = prepare(&mut owner, &scope, &mut r);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = owner
        .query_discovery_v1(
            &discovery,
            &scope,
            &cut,
            DiscoveryQueryV1::Inspect { entry },
            &discovery_facts(),
            &mut r,
        )
        .expect("inspect installed command")
    else {
        panic!("expected descriptor")
    };
    assert_eq!(descriptor.variants.len(), 1);
    let variant = &descriptor.variants[0].reference;

    // Explicit trusted host creation and write scope; no filesystem or UI needed.
    let source = owner
        .create(b"Hello, world!\n", &mut r)
        .expect("create source");
    let id = owner.issue_invocation(&mut r).expect("one invocation");
    let policy = InvocationObservationPolicyV1 {
        recipient: actor.recipient.clone(),
        epoch: 1,
        grant: true,
        publication: true,
        control: true,
        effect: true,
        reasons: true,
        details: false,
        recovery: true,
    };
    owner
        .bind_invocation_observation_policy_v1(
            &id,
            &actor,
            &policy,
            InvocationObservationPolicyLimitsV1 {
                max_context_bytes: 2048,
                max_result_bytes: 512,
            },
            &policy_facts(&actor),
            &mut r,
        )
        .expect("bind original result authority");
    let writes = [DocumentEdits {
        expected: source.clone(),
        edits: vec![Replacement {
            start: 7,
            end: 12,
            expected: "world".into(),
            inserted: "agents".into(),
        }],
    }];
    let reads = ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 0,
            max_work: 4096,
        },
    };
    let scopes = [SourceOperationScope {
        expected: source.clone(),
        replacement_ranges: vec![ByteRange { start: 7, end: 12 }],
        insertion_sites: vec![],
    }];
    use ExactBatchEnvelopeValueV1 as V;
    let args = [
        ("contract", V::Data(D::Text("exact-source-batch/1"))),
        ("operation", V::Data(D::Text("exact-batch"))),
        ("invocation", V::Invocation(&id)),
        (
            "profile",
            V::Data(D::Text(BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1)),
        ),
        ("sources", V::Sources(&writes)),
        ("reads", V::Reads(&reads)),
        (
            "scopes",
            V::Scopes(
                &scopes,
                SourceScopeBudget {
                    max_scopes: 1,
                    max_records: 4,
                    max_work: 4096,
                },
            ),
        ),
    ];
    let q = owner
        .normalize_exact_batch_arguments_v1(
            V::Object(&args),
            &actor,
            CaptureLimitsV1 {
                max_bytes: 65536,
                max_nodes: 4096,
                max_depth: 16,
            },
            &mut r,
        )
        .expect("normalize closed batch vocabulary");
    let cut = prepare(&mut owner, &scope, &mut r);
    assert_eq!(
        owner.submit_discovered_native_exact_batch_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &discovery,
                scope: &scope,
                cut: &cut,
                entry,
                variant,
                facts: &discovery_facts(),
            },
            &q,
            &mut r,
        ),
        Ok(InvocationState::Pending)
    );
    // Admission is not execution. The trusted host separately permits service.
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        "Hello, world!\n"
    );
    assert_eq!(owner.run(&id, &mut r), Ok(InvocationState::Finished));
    let generation = owner.generation();
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&q, &mut r),
        Ok(InvocationState::Finished)
    );
    owner.run(&id, &mut r).expect("matching retry");
    assert_eq!(owner.generation(), generation, "retry cannot execute twice");

    // F06 ordinary result release uses its own current permissions, not the
    // privileged invocation_outcome diagnostic used later for owner cleanup.
    let mut limits = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 4,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationControlReferenceV1>()
            .max(size_of::<InvocationObservationReferenceV1>()),
        max_reply_bytes: 512 + size_of::<InvocationObservationReplyV1>(),
        max_control_reply_bytes: 512,
        max_held_bytes: 0,
    };
    limits.max_held_bytes = limits
        .required_hold_bytes()
        .expect("finite observation limits");
    let mut observer = owner
        .open_invocation_observation_v1(&id, &actor, &policy, limits, &policy_facts(&actor), &mut r)
        .expect("open original result observer");
    let current = TrustedInvocationObservationReleaseV1 {
        context: &actor,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        control_ready: Some(true),
    };
    let release = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &current,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let mut withheld = policy.clone();
    withheld.publication = false;
    let hidden = owner
        .query_invocation_observation_v1(
            &mut observer,
            InvocationObservationQueryV1::RetainedOutcome,
            &withheld,
            &release,
            &mut r,
        )
        .expect("permission-filtered result");
    assert_eq!(hidden.outcome.unwrap().publication, None);
    let result = owner
        .query_invocation_observation_v1(
            &mut observer,
            InvocationObservationQueryV1::RetainedOutcome,
            &policy,
            &release,
            &mut r,
        )
        .expect("currently authorized result");
    assert_eq!(
        result.outcome.unwrap().publication,
        Some(InvocationPublicationV1::Applied)
    );
    assert_eq!(
        owner.generation(),
        generation,
        "release cannot re-execute editing"
    );
    let text = owner
        .read(&source.document, &mut r)
        .expect("host read")
        .source
        .text()
        .to_owned();

    // Close/drop each original owner explicitly. This is session cleanup, not a
    // claim that the unfinished cooperative shutdown controller is implemented.
    let control = InvocationObservationPolicyControlV1 {
        context: &actor,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    owner
        .close_invocation_observation_v1(&mut observer, &control, &mut r)
        .expect("close observer");
    owner
        .retire_invocation_observation_v1(&mut observer, &control, &mut r)
        .expect("retire observer");
    owner
        .retire_invocation_observation_policy_v1(&id, &control, &mut r)
        .expect("retire result policy");
    let InvocationOutcome::Committed(committed) = owner
        .invocation_outcome(&id, &mut r)
        .expect("host cleanup outcome")
    else {
        panic!("expected committed edit")
    };
    let group = committed.history_group.clone().expect("one history group");
    owner
        .forget_invocation_outcome(&id, &mut r)
        .expect("forget retained outcome");
    owner
        .retire_invocation(&id, &mut r)
        .expect("retire invocation");
    owner
        .retire_history_group(&group, &mut r)
        .expect("retire history");
    owner
        .dispose(&source.document, &mut r)
        .expect("dispose unbound document");
    owner
        .stop_discovery_context_v1(&discovery, &scope, &discovery_facts(), &mut r)
        .expect("stop discovery");
    owner
        .retire_discovery_context_v1(&discovery, &discovery_facts(), &mut r)
        .expect("retire discovery");
    owner
        .stop_discovery_admission_v1(&discovery_facts(), &mut r)
        .expect("stop registry admission");
    owner
        .retire_discovery_registry_v1(&discovery_facts(), &mut r)
        .expect("retire registry");
    Report {
        text,
        issued: owner.invocation_usage().issued_lifetime,
        retained_invocations: owner.invocation_usage().retained,
        retained_history: owner.history_usage().retained_groups,
        reserved_bytes: owner.retention_usage().reserved,
        dropped: r.dropped(),
    }
}

fn main() {
    let report = run(128, true);
    assert_eq!(report.retained_invocations, 0);
    assert_eq!(report.retained_history, 0);
    assert_eq!(report.reserved_bytes, 0);
    println!(
        "{}One invocation: {}; all session owners released; diagnostic drops: {}",
        report.text, report.issued, report.dropped
    );
}
