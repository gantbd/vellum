use super::*;
use std::collections::VecDeque;
use vellum_host::semantic::{
    SemanticCapabilitySetV1, SemanticDiagnosticSeverityV1, SemanticDiagnosticV1,
    SemanticDocumentRefV1, SemanticIndexGenerationV1, SemanticLimitsV1, SemanticLocationV1,
    SemanticProviderProvenanceV1, SemanticProviderRefV1, SemanticQueryV1, SemanticResultV1,
    SemanticSourceRevisionV1, SemanticSpanV1, SemanticTargetV1, SemanticTextEditV1, SemanticTextV1,
};
use vellum_semantic_protocol::{
    SemanticProviderObservationV1, SemanticProviderReadinessProjectionV1, SemanticSessionLimitsV1,
    SemanticSessionReadinessV1, SemanticSessionRefusalV1,
};

#[derive(Default)]
struct FaultPort {
    outcomes: VecDeque<SemanticExchangeDispositionV1<SemanticSessionReplyV1>>,
    commands: Vec<SemanticSessionCommandV1>,
}

impl SemanticSessionExchangePortV1 for FaultPort {
    fn exchange(
        &mut self,
        command: &SemanticSessionCommandV1,
    ) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
        self.commands.push(command.clone());
        self.outcomes.pop_front().expect("fault outcome")
    }
}

fn bytes(tag: u8) -> [u8; 16] {
    [tag; 16]
}

fn incarnation(tag: u8) -> SemanticSessionIncarnationV1 {
    SemanticSessionIncarnationV1::new(bytes(tag)).unwrap()
}

fn membership(tag: u8) -> SemanticMembershipRefV1 {
    SemanticMembershipRefV1::new(bytes(tag)).unwrap()
}

fn exchange(tag: u8) -> SemanticExchangeRefV1 {
    SemanticExchangeRefV1::new(bytes(tag)).unwrap()
}

fn operation(tag: u8) -> SemanticSessionOperationRefV1 {
    SemanticSessionOperationRefV1::new(bytes(tag)).unwrap()
}

fn epoch(tag: u8) -> SemanticSynchronizationEpochV1 {
    SemanticSynchronizationEpochV1::new(bytes(tag)).unwrap()
}

fn target(tag: u8, revision: u64) -> SemanticTargetV1 {
    SemanticTargetV1 {
        document: SemanticDocumentRefV1::new(bytes(tag)),
        source_revision: SemanticSourceRevisionV1::new(revision),
    }
}

fn snapshot(target: SemanticTargetV1, source: &str) -> SemanticSessionSnapshotV1 {
    SemanticSessionSnapshotV1::try_new(
        target,
        "rust".into(),
        source.into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap()
}

fn description(incarnation: SemanticSessionIncarnationV1) -> SemanticSessionDescriptionV1 {
    SemanticSessionDescriptionV1::try_new(
        incarnation,
        SemanticSessionRolesV1::top(),
        SemanticSessionReadinessV1::Unavailable,
        SemanticProviderObservationV1::new(
            SemanticProviderReadinessProjectionV1::Unavailable,
            SemanticCapabilitySetV1::top(),
        ),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap()
}

fn ready_description(
    incarnation: SemanticSessionIncarnationV1,
    epoch: SemanticSynchronizationEpochV1,
) -> SemanticSessionDescriptionV1 {
    ready_description_with_limits(incarnation, epoch, SemanticSessionLimitsV1::reference())
}

fn ready_description_with_limits(
    incarnation: SemanticSessionIncarnationV1,
    epoch: SemanticSynchronizationEpochV1,
    limits: SemanticSessionLimitsV1,
) -> SemanticSessionDescriptionV1 {
    SemanticSessionDescriptionV1::try_new(
        incarnation,
        SemanticSessionRolesV1::top(),
        SemanticSessionReadinessV1::Ready { epoch },
        SemanticProviderObservationV1::new(
            SemanticProviderReadinessProjectionV1::Ready {
                generation: SemanticIndexGenerationV1::new(3).unwrap(),
            },
            SemanticCapabilitySetV1::top(),
        ),
        limits,
    )
    .unwrap()
}

fn mark_query_ready(client: &mut SemanticSessionClientV1<FaultPort>) {
    client.description = Some(ready_description(incarnation(1), epoch(4)));
    client.membership.as_mut().unwrap().epoch = Some(epoch(4));
}

fn member_client(
    outcomes: impl IntoIterator<Item = SemanticExchangeDispositionV1<SemanticSessionReplyV1>>,
) -> SemanticSessionClientV1<FaultPort> {
    let inc = incarnation(1);
    let member = membership(2);
    let mut client = SemanticSessionClientV1::try_new(
        FaultPort {
            outcomes: outcomes.into_iter().collect(),
            commands: Vec::new(),
        },
        SemanticSessionClientLimitsV1::reference(),
    )
    .unwrap();
    client.description = Some(description(inc));
    client.membership = Some(MembershipProjectionV1 {
        incarnation: inc,
        membership: member,
        roles: SemanticSessionRolesV1::top(),
        validation: SemanticReplyValidationV1::Unsealed,
        epoch: None,
        documents: BTreeMap::new(),
        witness_bytes: 0,
        staged: None,
        requests: BTreeMap::new(),
        result_reservation: 0,
        remaining_operations: 100,
        definitive_operations: BTreeSet::new(),
    });
    client.update_gauges();
    client
}

fn reply(
    command: &SemanticSessionCommandV1,
    outcome: SemanticSessionOutcomeV1,
) -> SemanticSessionReplyV1 {
    SemanticSessionReplyV1::try_new(
        command.kind(),
        command.exchange(),
        incarnation(1),
        command.binding().map(|(_, membership)| membership),
        command.operation(),
        outcome,
    )
    .unwrap()
}

#[test]
fn outcome_unknown_is_one_exact_replay_slot_and_not_sent_does_not_clear_it() {
    let mut client = member_client([SemanticExchangeDispositionV1::OutcomeUnknown]);
    let leave = SemanticSessionCommandV1::LeaveSession {
        exchange: exchange(3),
        operation: operation(3),
        incarnation: incarnation(1),
        membership: membership(2),
    };
    assert_eq!(
        client.attempt(leave.clone()).unwrap(),
        SemanticExchangeDispositionV1::OutcomeUnknown
    );
    assert_eq!(
        client.usage().replay_intent_bytes,
        u64::try_from(leave.retained_intent_bytes_v1().unwrap()).unwrap()
    );
    assert_eq!(
        client.attempt(leave.clone()),
        Err(SemanticSessionClientRefusalV1::PendingMutation)
    );
    let describe = SemanticSessionCommandV1::Describe {
        exchange: exchange(4),
    };
    client
        .port
        .outcomes
        .push_back(SemanticExchangeDispositionV1::Reply(
            SemanticSessionReplyV1::try_new(
                describe.kind(),
                describe.exchange(),
                incarnation(1),
                None,
                None,
                SemanticSessionOutcomeV1::Description(description(incarnation(1))),
            )
            .unwrap(),
        ));
    assert!(matches!(
        client.attempt(describe),
        Ok(SemanticExchangeDispositionV1::Reply(_))
    ));
    assert!(client.pending_mutation().is_some());
    let not_sent_exchange = exchange(5);
    client
        .port
        .outcomes
        .push_back(SemanticExchangeDispositionV1::NotSent);
    assert_eq!(
        client.replay_pending(not_sent_exchange).unwrap(),
        SemanticExchangeDispositionV1::NotSent
    );
    assert_eq!(
        client.port.commands,
        vec![
            leave.clone(),
            SemanticSessionCommandV1::Describe {
                exchange: exchange(4)
            },
            command_with_exchange(leave.clone(), not_sent_exchange)
        ]
    );
    assert_eq!(
        client.pending_mutation(),
        Some(PendingSemanticMutationV1 {
            kind: leave.kind(),
            exchange: leave.exchange(),
            operation: leave.operation().unwrap(),
        })
    );
    let reply_exchange = exchange(6);
    let replayed = command_with_exchange(leave.clone(), reply_exchange);
    client
        .port
        .outcomes
        .push_back(SemanticExchangeDispositionV1::Reply(reply(
            &replayed,
            SemanticSessionOutcomeV1::Accepted,
        )));
    assert!(matches!(
        client.replay_pending(reply_exchange),
        Ok(SemanticExchangeDispositionV1::Reply(_))
    ));
    assert!(client.pending_mutation().is_none());
    assert_eq!(
        client.port.commands.last().unwrap().exchange(),
        reply_exchange
    );
}

#[test]
fn changed_incarnation_describe_clears_unrelated_uncertainty_and_volatile_state() {
    let mut client = member_client([SemanticExchangeDispositionV1::OutcomeUnknown]);
    let leave = SemanticSessionCommandV1::LeaveSession {
        exchange: exchange(3),
        operation: operation(3),
        incarnation: incarnation(1),
        membership: membership(2),
    };
    client.attempt(leave).unwrap();
    let describe = SemanticSessionCommandV1::Describe {
        exchange: exchange(4),
    };
    let changed = incarnation(9);
    client
        .port
        .outcomes
        .push_back(SemanticExchangeDispositionV1::Reply(
            SemanticSessionReplyV1::try_new(
                describe.kind(),
                describe.exchange(),
                changed,
                None,
                None,
                SemanticSessionOutcomeV1::Description(description(changed)),
            )
            .unwrap(),
        ));
    client.attempt(describe).unwrap();
    assert!(client.pending_mutation().is_none());
    assert!(client.membership.is_none());
    assert_eq!(client.usage().replay_intent_bytes, 0);
}

#[test]
fn transplanted_membership_is_pre_effect_refusal() {
    let mut client = member_client([]);
    let refusal = client.attempt(SemanticSessionCommandV1::LeaveSession {
        exchange: exchange(3),
        operation: operation(3),
        incarnation: incarnation(1),
        membership: membership(9),
    });
    assert_eq!(refusal, Err(SemanticSessionClientRefusalV1::SessionBinding));
    assert!(client.port.commands.is_empty());
}

fn join_attempt(
    requested: SemanticSessionRolesV1,
    ceiling: SemanticSessionRolesV1,
    granted: SemanticSessionRolesV1,
    described_max: u32,
    remaining: u32,
) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
    let mut limits = SemanticSessionLimitsV1::reference();
    limits.max_operations_per_membership = described_max;
    let observed = SemanticSessionDescriptionV1::try_new(
        incarnation(1),
        ceiling,
        SemanticSessionReadinessV1::Unavailable,
        SemanticProviderObservationV1::new(
            SemanticProviderReadinessProjectionV1::Unavailable,
            SemanticCapabilitySetV1::top(),
        ),
        limits,
    )
    .unwrap();
    let command = SemanticSessionCommandV1::JoinSession {
        exchange: exchange(20),
        operation: operation(20),
        incarnation: incarnation(1),
        requested_roles: requested,
    };
    let joined_membership = membership(20);
    let response = SemanticSessionReplyV1::try_new(
        command.kind(),
        command.exchange(),
        incarnation(1),
        Some(joined_membership),
        command.operation(),
        SemanticSessionOutcomeV1::Joined {
            membership: joined_membership,
            granted_roles: granted,
            remaining_operations: remaining,
        },
    )
    .unwrap();
    let mut client = SemanticSessionClientV1::try_new(
        FaultPort {
            outcomes: VecDeque::from([SemanticExchangeDispositionV1::Reply(response)]),
            commands: Vec::new(),
        },
        SemanticSessionClientLimitsV1::reference(),
    )
    .unwrap();
    client.description = Some(observed);
    client.attempt(command).unwrap()
}

#[test]
fn join_rejects_role_amplification_bottom_grants_and_budget_increases() {
    assert_eq!(
        join_attempt(
            SemanticSessionRolesV1::query(),
            SemanticSessionRolesV1::top(),
            SemanticSessionRolesV1::top(),
            10,
            5,
        ),
        SemanticExchangeDispositionV1::OutcomeUnknown
    );
    assert_eq!(
        join_attempt(
            SemanticSessionRolesV1::top(),
            SemanticSessionRolesV1::query(),
            SemanticSessionRolesV1::synchronize(),
            10,
            5,
        ),
        SemanticExchangeDispositionV1::OutcomeUnknown
    );
    assert_eq!(
        join_attempt(
            SemanticSessionRolesV1::top(),
            SemanticSessionRolesV1::top(),
            SemanticSessionRolesV1::bottom(),
            10,
            5,
        ),
        SemanticExchangeDispositionV1::OutcomeUnknown
    );
    assert_eq!(
        join_attempt(
            SemanticSessionRolesV1::top(),
            SemanticSessionRolesV1::top(),
            SemanticSessionRolesV1::query(),
            2,
            3,
        ),
        SemanticExchangeDispositionV1::OutcomeUnknown
    );
}

#[test]
fn definitive_operation_budget_is_monotone_and_matching_replay_is_inert() {
    let request = query(target(5, 8), "rust");
    let refused = SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::AbsentOrForeign);
    let commands = [
        SemanticSessionCommandV1::ReleaseRequest {
            exchange: exchange(30),
            operation: operation(30),
            incarnation: incarnation(1),
            membership: membership(2),
            request: request.request,
        },
        SemanticSessionCommandV1::ReleaseRequest {
            exchange: exchange(31),
            operation: operation(30),
            incarnation: incarnation(1),
            membership: membership(2),
            request: request.request,
        },
        SemanticSessionCommandV1::ReleaseRequest {
            exchange: exchange(32),
            operation: operation(32),
            incarnation: incarnation(1),
            membership: membership(2),
            request: request.request,
        },
    ];
    let outcomes = commands
        .iter()
        .map(|command| SemanticExchangeDispositionV1::Reply(reply(command, refused.clone())))
        .collect::<Vec<_>>();
    let mut client = member_client(outcomes);
    let state = client.membership.as_mut().unwrap();
    state.requests.insert(request.request, request);
    state.remaining_operations = 2;
    client.update_gauges();

    client.attempt(commands[0].clone()).unwrap();
    assert_eq!(client.usage().remaining_operations, 1);
    client.attempt(commands[1].clone()).unwrap();
    assert_eq!(client.usage().remaining_operations, 1);
    client.attempt(commands[2].clone()).unwrap();
    assert_eq!(client.usage().remaining_operations, 0);
    assert_eq!(
        client.attempt(SemanticSessionCommandV1::ReleaseRequest {
            exchange: exchange(33),
            operation: operation(33),
            incarnation: incarnation(1),
            membership: membership(2),
            request: SemanticRequestRefV1::new(bytes(7)),
        }),
        Err(SemanticSessionClientRefusalV1::Capacity)
    );
}

#[test]
fn compact_witness_has_exact_language_and_canonical_utf8_boundaries() {
    let snapshot = SemanticSessionSnapshotV1::try_new(
        target(5, 8),
        "rust".into(),
        "aéz".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let witness =
        DocumentWitnessV1::try_from_snapshot(&snapshot, SemanticSessionLimitsV1::reference())
            .unwrap();
    assert_eq!(witness.language, "rust");
    assert!(witness.contains_span(0, 1));
    assert!(witness.contains_span(1, 3));
    assert!(witness.contains_span(4, 4));
    assert!(!witness.contains_span(2, 3));
    assert_eq!(witness.utf8_boundaries.len(), 1);
    assert_eq!(witness.utf8_boundaries[0] & !0b0001_1011, 0);
}

fn accepted(
    command: &SemanticSessionCommandV1,
) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
    SemanticExchangeDispositionV1::Reply(reply(command, SemanticSessionOutcomeV1::Accepted))
}

#[test]
fn stage_publishes_only_after_exact_leaf_root_completion() {
    let snapshot = SemanticSessionSnapshotV1::try_new(
        target(5, 8),
        "rust".into(),
        "aéz".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        std::slice::from_ref(&snapshot),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let epoch = epoch(4);
    let begin = SemanticSessionCommandV1::BeginSynchronization {
        exchange: exchange(4),
        operation: operation(4),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch,
        manifest,
    };
    let open = SemanticSessionCommandV1::SyncOpen {
        exchange: exchange(5),
        operation: operation(5),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch,
        snapshot,
    };
    let complete = SemanticSessionCommandV1::CompleteSynchronization {
        exchange: exchange(6),
        operation: operation(6),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch,
        manifest,
    };
    let mut client = member_client([accepted(&begin), accepted(&open), accepted(&complete)]);
    client.attempt(begin).unwrap();
    client.attempt(open).unwrap();
    assert_eq!(
        client.validation_state(),
        SemanticReplyValidationV1::Unsealed
    );
    client.attempt(complete).unwrap();
    assert_eq!(
        client.validation_state(),
        SemanticReplyValidationV1::WitnessValidated
    );
    let store = client.export_witness_store().unwrap();
    let debug = format!("{store:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("rust"));
    assert!(client.usage().witness_bytes > 0);
}

fn ready_committed_client(
    outcomes: impl IntoIterator<Item = SemanticExchangeDispositionV1<SemanticSessionReplyV1>>,
    committed_epoch: SemanticSynchronizationEpochV1,
    snapshots: &[SemanticSessionSnapshotV1],
) -> SemanticSessionClientV1<FaultPort> {
    let mut client = member_client(outcomes);
    client.description = Some(ready_description(incarnation(1), committed_epoch));
    client.seen_epochs.insert(committed_epoch);
    let state = client.membership.as_mut().unwrap();
    state.validation = SemanticReplyValidationV1::WitnessValidated;
    state.epoch = Some(committed_epoch);
    for snapshot in snapshots {
        let witness =
            DocumentWitnessV1::try_from_snapshot(snapshot, SemanticSessionLimitsV1::reference())
                .unwrap();
        state.witness_bytes = state
            .witness_bytes
            .checked_add(witness.retained_bytes())
            .unwrap();
        state.documents.insert(snapshot.target().document, witness);
    }
    client.update_gauges();
    client
}

#[test]
fn ready_epoch_change_open_close_complete_atomically_replaces_committed_projection() {
    let prior_a = snapshot(target(5, 8), "a");
    let prior_b = snapshot(target(6, 3), "b");
    let changed_a = snapshot(target(5, 9), "aa");
    let opened_c = snapshot(target(7, 1), "cé");
    let final_manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        &[changed_a.clone(), opened_c.clone()],
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let next_epoch = epoch(5);
    let commands = [
        SemanticSessionCommandV1::BeginSynchronization {
            exchange: exchange(40),
            operation: operation(40),
            incarnation: incarnation(1),
            membership: membership(2),
            epoch: next_epoch,
            manifest: final_manifest,
        },
        SemanticSessionCommandV1::SyncChange {
            exchange: exchange(41),
            operation: operation(41),
            incarnation: incarnation(1),
            membership: membership(2),
            epoch: next_epoch,
            snapshot: changed_a.clone(),
        },
        SemanticSessionCommandV1::SyncOpen {
            exchange: exchange(42),
            operation: operation(42),
            incarnation: incarnation(1),
            membership: membership(2),
            epoch: next_epoch,
            snapshot: opened_c.clone(),
        },
        SemanticSessionCommandV1::SyncClose {
            exchange: exchange(43),
            operation: operation(43),
            incarnation: incarnation(1),
            membership: membership(2),
            epoch: next_epoch,
            target: prior_b.target(),
        },
        SemanticSessionCommandV1::CompleteSynchronization {
            exchange: exchange(44),
            operation: operation(44),
            incarnation: incarnation(1),
            membership: membership(2),
            epoch: next_epoch,
            manifest: final_manifest,
        },
    ];
    let outcomes = commands.iter().map(accepted).collect::<Vec<_>>();
    let mut client =
        ready_committed_client(outcomes, epoch(4), &[prior_a.clone(), prior_b.clone()]);
    for command in commands {
        client.attempt(command).unwrap();
    }
    let state = client.membership.as_ref().unwrap();
    assert_eq!(state.epoch, Some(next_epoch));
    assert_eq!(state.documents.len(), 2);
    assert_eq!(
        state
            .documents
            .get(&prior_a.target().document)
            .unwrap()
            .seal
            .target(),
        changed_a.target()
    );
    assert!(!state.documents.contains_key(&prior_b.target().document));
    assert_eq!(
        state
            .documents
            .get(&opened_c.target().document)
            .unwrap()
            .seal
            .target(),
        opened_c.target()
    );
    assert!(state.staged.is_none());
    let describe = SemanticSessionCommandV1::Describe {
        exchange: exchange(45),
    };
    client
        .port
        .outcomes
        .push_back(SemanticExchangeDispositionV1::Reply(
            SemanticSessionReplyV1::try_new(
                describe.kind(),
                describe.exchange(),
                incarnation(1),
                None,
                None,
                SemanticSessionOutcomeV1::Description(ready_description(
                    incarnation(1),
                    next_epoch,
                )),
            )
            .unwrap(),
        ));
    assert!(matches!(
        client.attempt(describe),
        Ok(SemanticExchangeDispositionV1::Reply(_))
    ));
}

#[test]
fn epoch_refusal_discards_stage_without_false_publishing_over_committed_projection() {
    let prior = snapshot(target(5, 8), "a");
    let changed = snapshot(target(5, 9), "aa");
    let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        std::slice::from_ref(&changed),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let next_epoch = epoch(5);
    let begin = SemanticSessionCommandV1::BeginSynchronization {
        exchange: exchange(50),
        operation: operation(50),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: next_epoch,
        manifest,
    };
    let change = SemanticSessionCommandV1::SyncChange {
        exchange: exchange(51),
        operation: operation(51),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: next_epoch,
        snapshot: changed,
    };
    let complete = SemanticSessionCommandV1::CompleteSynchronization {
        exchange: exchange(52),
        operation: operation(52),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: next_epoch,
        manifest,
    };
    let outcomes = [
        accepted(&begin),
        accepted(&change),
        SemanticExchangeDispositionV1::Reply(reply(
            &complete,
            SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::SynchronizationConflict),
        )),
    ];
    let mut client = ready_committed_client(outcomes, epoch(4), std::slice::from_ref(&prior));
    client.attempt(begin).unwrap();
    client.attempt(change).unwrap();
    client.attempt(complete).unwrap();
    let state = client.membership.as_ref().unwrap();
    assert_eq!(state.epoch, Some(epoch(4)));
    assert_eq!(
        state
            .documents
            .get(&prior.target().document)
            .unwrap()
            .seal
            .target(),
        prior.target()
    );
    assert!(state.staged.is_none());
    assert_eq!(state.validation, SemanticReplyValidationV1::Unsealed);
}

#[test]
fn unsealed_multi_document_projection_reseals_without_reopening_sources() {
    let first = snapshot(target(5, 8), "a");
    let second = snapshot(target(6, 3), "bb");
    let snapshots = [first, second];
    let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        &snapshots,
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let describe = SemanticSessionCommandV1::Describe {
        exchange: exchange(53),
    };
    let begin = SemanticSessionCommandV1::BeginSynchronization {
        exchange: exchange(54),
        operation: operation(54),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: epoch(6),
        manifest,
    };
    let complete = SemanticSessionCommandV1::CompleteSynchronization {
        exchange: exchange(55),
        operation: operation(55),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: epoch(6),
        manifest,
    };
    let description = SemanticSessionReplyV1::try_new(
        describe.kind(),
        describe.exchange(),
        incarnation(1),
        None,
        None,
        SemanticSessionOutcomeV1::Description(ready_description(incarnation(1), epoch(5))),
    )
    .unwrap();
    let mut client = ready_committed_client(
        [
            SemanticExchangeDispositionV1::Reply(description),
            accepted(&begin),
            accepted(&complete),
        ],
        epoch(4),
        &snapshots,
    );

    client.attempt(describe).unwrap();
    assert_eq!(
        client.validation_state(),
        SemanticReplyValidationV1::Unsealed
    );
    client.attempt(begin).unwrap();
    client.attempt(complete).unwrap();

    let state = client.membership.as_ref().unwrap();
    assert_eq!(state.documents.len(), 2);
    assert_eq!(state.epoch, Some(epoch(6)));
    assert_eq!(
        state.validation,
        SemanticReplyValidationV1::WitnessValidated
    );
}

#[test]
fn ready_epoch_aba_and_begin_reuse_are_rejected_across_readiness_transitions() {
    let prior = snapshot(target(5, 8), "a");
    let unavailable = SemanticSessionCommandV1::Describe {
        exchange: exchange(70),
    };
    let repeated = SemanticSessionCommandV1::Describe {
        exchange: exchange(71),
    };
    let unavailable_reply = SemanticSessionReplyV1::try_new(
        unavailable.kind(),
        unavailable.exchange(),
        incarnation(1),
        None,
        None,
        SemanticSessionOutcomeV1::Description(description(incarnation(1))),
    )
    .unwrap();
    let repeated_reply = SemanticSessionReplyV1::try_new(
        repeated.kind(),
        repeated.exchange(),
        incarnation(1),
        None,
        None,
        SemanticSessionOutcomeV1::Description(ready_description(incarnation(1), epoch(4))),
    )
    .unwrap();
    let mut client = ready_committed_client(
        [
            SemanticExchangeDispositionV1::Reply(unavailable_reply),
            SemanticExchangeDispositionV1::Reply(repeated_reply),
        ],
        epoch(4),
        std::slice::from_ref(&prior),
    );
    let old_store = client.export_witness_store().unwrap();
    client.attempt(unavailable).unwrap();
    assert_eq!(
        client.attempt(repeated).unwrap(),
        SemanticExchangeDispositionV1::OutcomeUnknown
    );
    assert_eq!(
        client.install_witness_store(old_store),
        Err(SemanticSessionClientRefusalV1::SessionBinding)
    );
    let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        std::slice::from_ref(&prior),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    assert_eq!(
        client.attempt(SemanticSessionCommandV1::BeginSynchronization {
            exchange: exchange(72),
            operation: operation(72),
            incarnation: incarnation(1),
            membership: membership(2),
            epoch: epoch(4),
            manifest,
        }),
        Err(SemanticSessionClientRefusalV1::SynchronizationState)
    );
}

#[test]
fn ambiguous_epoch_change_preserves_committed_and_stage_until_explicit_resolution() {
    let prior = snapshot(target(5, 8), "a");
    let changed = snapshot(target(5, 9), "aa");
    let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        std::slice::from_ref(&changed),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let next_epoch = epoch(5);
    let begin = SemanticSessionCommandV1::BeginSynchronization {
        exchange: exchange(60),
        operation: operation(60),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: next_epoch,
        manifest,
    };
    let change = SemanticSessionCommandV1::SyncChange {
        exchange: exchange(61),
        operation: operation(61),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: next_epoch,
        snapshot: changed.clone(),
    };
    let mut client = ready_committed_client(
        [
            accepted(&begin),
            SemanticExchangeDispositionV1::OutcomeUnknown,
        ],
        epoch(4),
        std::slice::from_ref(&prior),
    );
    client.attempt(begin).unwrap();
    client.attempt(change.clone()).unwrap();
    {
        let state = client.membership.as_ref().unwrap();
        assert_eq!(
            state
                .documents
                .get(&prior.target().document)
                .unwrap()
                .seal
                .target(),
            prior.target()
        );
        assert_eq!(
            state
                .staged
                .as_ref()
                .unwrap()
                .documents
                .get(&prior.target().document)
                .unwrap()
                .seal
                .target(),
            prior.target()
        );
    }
    assert_eq!(
        client.attempt(SemanticSessionCommandV1::CompleteSynchronization {
            exchange: exchange(63),
            operation: operation(63),
            incarnation: incarnation(1),
            membership: membership(2),
            epoch: next_epoch,
            manifest,
        }),
        Err(SemanticSessionClientRefusalV1::PendingMutation)
    );
    let replay_exchange = exchange(62);
    let replayed = command_with_exchange(change, replay_exchange);
    client
        .port
        .outcomes
        .push_back(SemanticExchangeDispositionV1::Reply(reply(
            &replayed,
            SemanticSessionOutcomeV1::Accepted,
        )));
    client.replay_pending(replay_exchange).unwrap();
    let state = client.membership.as_ref().unwrap();
    assert_eq!(
        state
            .documents
            .get(&prior.target().document)
            .unwrap()
            .seal
            .target(),
        prior.target()
    );
    assert_eq!(
        state
            .staged
            .as_ref()
            .unwrap()
            .documents
            .get(&changed.target().document)
            .unwrap()
            .seal
            .target(),
        changed.target()
    );
}

#[test]
fn staged_projection_enforces_exact_document_and_source_byte_limits_before_transport() {
    let prior_a = snapshot(target(5, 8), "a");
    let prior_b = snapshot(target(6, 3), "bbb");
    let exact_a = snapshot(target(5, 9), "aa");
    let over_a = snapshot(target(5, 9), "aaa");
    let extra = snapshot(target(7, 1), "c");
    let mut limits = SemanticSessionLimitsV1::reference();
    limits.max_snapshot_bytes = 5;
    limits.max_total_snapshot_bytes = 5;
    limits.max_documents = 2;
    assert!(limits.is_valid());

    let final_manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        &[exact_a.clone(), prior_b.clone()],
        limits,
    )
    .unwrap();
    let begin = SemanticSessionCommandV1::BeginSynchronization {
        exchange: exchange(80),
        operation: operation(80),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: epoch(5),
        manifest: final_manifest,
    };
    let exact_change = SemanticSessionCommandV1::SyncChange {
        exchange: exchange(81),
        operation: operation(81),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: epoch(5),
        snapshot: exact_a,
    };
    let mut exact_client = ready_committed_client(
        [accepted(&begin), accepted(&exact_change)],
        epoch(4),
        &[prior_a.clone(), prior_b.clone()],
    );
    exact_client.description = Some(ready_description_with_limits(
        incarnation(1),
        epoch(4),
        limits,
    ));
    exact_client.attempt(begin.clone()).unwrap();
    exact_client.attempt(exact_change).unwrap();
    assert_eq!(exact_client.port.commands.len(), 2);

    let mut over_client = ready_committed_client([accepted(&begin)], epoch(4), &[prior_a, prior_b]);
    over_client.description = Some(ready_description_with_limits(
        incarnation(1),
        epoch(4),
        limits,
    ));
    over_client.attempt(begin).unwrap();
    let over_change = SemanticSessionCommandV1::SyncChange {
        exchange: exchange(82),
        operation: operation(82),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: epoch(5),
        snapshot: over_a,
    };
    assert_eq!(
        over_client.attempt(over_change),
        Err(SemanticSessionClientRefusalV1::Capacity)
    );
    let over_open = SemanticSessionCommandV1::SyncOpen {
        exchange: exchange(83),
        operation: operation(83),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: epoch(5),
        snapshot: extra,
    };
    assert_eq!(
        over_client.attempt(over_open),
        Err(SemanticSessionClientRefusalV1::Capacity)
    );
    assert_eq!(over_client.port.commands.len(), 1);
}

fn query(target: SemanticTargetV1, language: &str) -> SemanticRequestV1 {
    SemanticRequestV1 {
        request: SemanticRequestRefV1::new(bytes(7)),
        target,
        expected_index: Some(SemanticIndexGenerationV1::new(3).unwrap()),
        language: SemanticTextV1::try_new(language.into(), 512).unwrap(),
        query: SemanticQueryV1::References { offset: 0 },
    }
}

fn provenance(tag: u8) -> SemanticProviderProvenanceV1 {
    SemanticProviderProvenanceV1::try_new(
        SemanticProviderRefV1::new(bytes(tag)),
        "profile".into(),
        "1".into(),
    )
    .unwrap()
}

#[test]
fn collect_complete_during_active_synchronization_is_quarantined_and_reserved() {
    let prior = snapshot(target(5, 8), "abc");
    let request = query(prior.target(), "rust");
    let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        std::slice::from_ref(&prior),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let begin = SemanticSessionCommandV1::BeginSynchronization {
        exchange: exchange(84),
        operation: operation(84),
        incarnation: incarnation(1),
        membership: membership(2),
        epoch: epoch(5),
        manifest,
    };
    let collect = SemanticSessionCommandV1::Collect {
        exchange: exchange(85),
        incarnation: incarnation(1),
        membership: membership(2),
        request: request.request,
    };
    let complete = SemanticReplyV1 {
        request: request.request,
        target: request.target,
        index_generation: SemanticIndexGenerationV1::new(3).unwrap(),
        provider: provenance(8),
        result: SemanticResultV1::try_locations(
            vec![SemanticLocationV1 {
                target: request.target,
                span: SemanticSpanV1::try_new(0, 1).unwrap(),
            }],
            SemanticLimitsV1::reference(),
        )
        .unwrap(),
    };
    let mut client = ready_committed_client(
        [
            accepted(&begin),
            SemanticExchangeDispositionV1::Reply(reply(
                &collect,
                SemanticSessionOutcomeV1::Complete(complete),
            )),
        ],
        epoch(4),
        std::slice::from_ref(&prior),
    );
    let reservation = semantic_session_result_reservation_bytes_v1(
        request.query.kind(),
        SemanticLimitsV1::reference(),
    )
    .unwrap() as u64;
    {
        let state = client.membership.as_mut().unwrap();
        state.requests.insert(request.request, request.clone());
        state.result_reservation = reservation;
    }
    client.attempt(begin).unwrap();
    assert_eq!(
        client.attempt(collect).unwrap(),
        SemanticExchangeDispositionV1::OutcomeUnknown
    );
    let state = client.membership.as_ref().unwrap();
    assert!(state.requests.contains_key(&request.request));
    assert_eq!(state.result_reservation, reservation);
    assert_eq!(state.validation, SemanticReplyValidationV1::Unsealed);
}

#[test]
fn complete_result_requires_exact_request_target_generation_and_utf8_locations() {
    let snapshot = SemanticSessionSnapshotV1::try_new(
        target(5, 8),
        "rust".into(),
        "aéz".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let witness =
        DocumentWitnessV1::try_from_snapshot(&snapshot, SemanticSessionLimitsV1::reference())
            .unwrap();
    let request = query(snapshot.target(), "rust");
    let collect = SemanticSessionCommandV1::Collect {
        exchange: exchange(8),
        incarnation: incarnation(1),
        membership: membership(2),
        request: request.request,
    };
    let result = SemanticReplyV1 {
        request: request.request,
        target: request.target,
        index_generation: SemanticIndexGenerationV1::new(3).unwrap(),
        provider: provenance(8),
        result: SemanticResultV1::try_locations(
            vec![SemanticLocationV1 {
                target: request.target,
                span: SemanticSpanV1::try_new(2, 3).unwrap(),
            }],
            SemanticLimitsV1::reference(),
        )
        .unwrap(),
    };
    let response = reply(&collect, SemanticSessionOutcomeV1::Complete(result));
    let mut client = member_client([SemanticExchangeDispositionV1::Reply(response)]);
    let state = client.membership.as_mut().unwrap();
    state.validation = SemanticReplyValidationV1::WitnessValidated;
    state.epoch = Some(epoch(4));
    state.witness_bytes = witness.retained_bytes();
    state.documents.insert(request.target.document, witness);
    state.requests.insert(request.request, request);
    state.result_reservation = 1024;
    assert_eq!(
        client.attempt(collect).unwrap(),
        SemanticExchangeDispositionV1::OutcomeUnknown
    );
    assert!(client.pending_mutation().is_none());
    assert_eq!(client.usage().binding_refusals, 1);
}

#[test]
fn exact_language_is_required_and_provider_is_not_preselected() {
    let snapshot = SemanticSessionSnapshotV1::try_new(
        target(5, 8),
        "rust".into(),
        "abc".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let witness =
        DocumentWitnessV1::try_from_snapshot(&snapshot, SemanticSessionLimitsV1::reference())
            .unwrap();
    let mut client = member_client([]);
    mark_query_ready(&mut client);
    let state = client.membership.as_mut().unwrap();
    state.validation = SemanticReplyValidationV1::WitnessValidated;
    state.documents.insert(snapshot.target().document, witness);
    let submit = SemanticSessionCommandV1::Submit {
        exchange: exchange(7),
        operation: operation(7),
        incarnation: incarnation(1),
        membership: membership(2),
        request: query(snapshot.target(), "go"),
    };
    assert_eq!(
        client.attempt(submit),
        Err(SemanticSessionClientRefusalV1::RequestState)
    );
    assert_ne!(provenance(8), provenance(9));
}

#[test]
fn split_utf8_query_offset_refuses_before_transport() {
    let snapshot = SemanticSessionSnapshotV1::try_new(
        target(5, 8),
        "rust".into(),
        "aéz".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let witness =
        DocumentWitnessV1::try_from_snapshot(&snapshot, SemanticSessionLimitsV1::reference())
            .unwrap();
    let mut request = query(snapshot.target(), "rust");
    request.query = SemanticQueryV1::Definition { offset: 2 };
    let mut client = member_client([]);
    mark_query_ready(&mut client);
    let state = client.membership.as_mut().unwrap();
    state.validation = SemanticReplyValidationV1::WitnessValidated;
    state.documents.insert(snapshot.target().document, witness);
    let submit = SemanticSessionCommandV1::Submit {
        exchange: exchange(7),
        operation: operation(7),
        incarnation: incarnation(1),
        membership: membership(2),
        request,
    };
    assert_eq!(
        client.attempt(submit),
        Err(SemanticSessionClientRefusalV1::RequestState)
    );
    assert!(client.port.commands.is_empty());
}

#[test]
fn result_reservation_is_checked_logical_bytes_not_item_count() {
    let snapshot = SemanticSessionSnapshotV1::try_new(
        target(5, 8),
        "rust".into(),
        "abc".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let witness =
        DocumentWitnessV1::try_from_snapshot(&snapshot, SemanticSessionLimitsV1::reference())
            .unwrap();
    let mut client = member_client([]);
    client.limits.max_result_reservation = 1;
    mark_query_ready(&mut client);
    let state = client.membership.as_mut().unwrap();
    state.validation = SemanticReplyValidationV1::WitnessValidated;
    state.documents.insert(snapshot.target().document, witness);
    let submit = SemanticSessionCommandV1::Submit {
        exchange: exchange(7),
        operation: operation(7),
        incarnation: incarnation(1),
        membership: membership(2),
        request: query(snapshot.target(), "rust"),
    };
    assert_eq!(
        client.attempt(submit),
        Err(SemanticSessionClientRefusalV1::Capacity)
    );
    assert_eq!(client.usage().capacity_refusals, 1);
}

#[test]
fn witness_install_requires_exact_ready_epoch_and_query_role() {
    let snapshot = SemanticSessionSnapshotV1::try_new(
        target(5, 8),
        "rust".into(),
        "abc".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let witness =
        DocumentWitnessV1::try_from_snapshot(&snapshot, SemanticSessionLimitsV1::reference())
            .unwrap();
    let retained_bytes = witness.retained_bytes();
    let store = SemanticSynchronizationWitnessStoreV1 {
        incarnation: incarnation(1),
        epoch: epoch(4),
        documents: BTreeMap::from([(snapshot.target().document, witness)]),
        retained_bytes,
    };
    let mut client = member_client([]);
    assert_eq!(
        client.install_witness_store(store.clone()),
        Err(SemanticSessionClientRefusalV1::SessionBinding)
    );
    client.description = Some(ready_description(incarnation(1), epoch(4)));
    client.install_witness_store(store).unwrap();
    assert_eq!(
        client.validation_state(),
        SemanticReplyValidationV1::WitnessValidated
    );
}

#[test]
fn diagnostics_are_target_local_and_rename_edits_are_globally_nonoverlapping() {
    let first = SemanticSessionSnapshotV1::try_new(
        target(5, 8),
        "rust".into(),
        "abcdef".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let second = SemanticSessionSnapshotV1::try_new(
        target(6, 3),
        "rust".into(),
        "abcdef".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let documents = BTreeMap::from([
        (
            first.target().document,
            DocumentWitnessV1::try_from_snapshot(&first, SemanticSessionLimitsV1::reference())
                .unwrap(),
        ),
        (
            second.target().document,
            DocumentWitnessV1::try_from_snapshot(&second, SemanticSessionLimitsV1::reference())
                .unwrap(),
        ),
    ]);
    let request = query(first.target(), "rust");
    let diagnostics = SemanticResultV1::try_diagnostics(
        vec![SemanticDiagnosticV1 {
            severity: SemanticDiagnosticSeverityV1::Error,
            location: SemanticLocationV1 {
                target: second.target(),
                span: SemanticSpanV1::try_new(0, 1).unwrap(),
            },
            message: SemanticTextV1::try_new("bad".into(), 4096).unwrap(),
            code: None,
        }],
        SemanticLimitsV1::reference(),
    )
    .unwrap();
    assert!(!result_locations_valid(&diagnostics, &request, &documents));

    let replacement = SemanticTextV1::try_new("x".into(), 512).unwrap();
    let rename = SemanticResultV1::try_rename_plan(
        vec![
            SemanticTextEditV1 {
                location: SemanticLocationV1 {
                    target: first.target(),
                    span: SemanticSpanV1::try_new(0, 3).unwrap(),
                },
                replacement: replacement.clone(),
            },
            SemanticTextEditV1 {
                location: SemanticLocationV1 {
                    target: first.target(),
                    span: SemanticSpanV1::try_new(2, 4).unwrap(),
                },
                replacement,
            },
        ],
        SemanticLimitsV1::reference(),
    )
    .unwrap();
    assert!(!result_locations_valid(&rename, &request, &documents));
}

#[test]
fn newer_incarnation_description_invalidates_membership_and_witnesses() {
    let command = SemanticSessionCommandV1::Describe {
        exchange: exchange(9),
    };
    let newer = incarnation(9);
    let response = SemanticSessionReplyV1::try_new(
        command.kind(),
        command.exchange(),
        newer,
        None,
        None,
        SemanticSessionOutcomeV1::Description(description(newer)),
    )
    .unwrap();
    let mut client = member_client([SemanticExchangeDispositionV1::Reply(response)]);
    client.attempt(command).unwrap();
    assert!(client.membership.is_none());
    assert_eq!(
        client.validation_state(),
        SemanticReplyValidationV1::Unsealed
    );
}
