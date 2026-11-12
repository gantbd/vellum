//! Transport-neutral native Vellum client.
//!
//! The first transport is in-process. A future remote transport must implement
//! the same port without weakening request identity or refusal semantics.
#![forbid(unsafe_code)]

use vellum_protocol::{
    ClientId, CommandV1, DurableEffectIntentV1, DurableExecuteOutcomeV1, DurableExecuteRequestV1,
    DurableOperationRefV1, DurableReceiptLookupOutcomeV1, DurableReceiptLookupV1,
    DurableReceiptRetirementOutcomeV1, DurableReceiptRetirementV1, DurableReceiptStatusRequestV1,
    DurableReceiptStatusV1, RefusalV1, ReplyV1, RequestId, RequestV1, ServiceAccessErrorV1,
    ServiceControlErrorV1, ServicePort, VersionWitness, WorkspaceChangeCursorV1,
    WorkspaceDocumentEditV1, WorkspaceDocumentIdV1, WorkspaceInventoryCursorV1, WorkspaceRootIdV1,
};

#[derive(Debug)]
pub struct Client<T> {
    transport: T,
    client: ClientId,
    next_request: u64,
    /// The single exact request whose effect may have happened without a
    /// trustworthy reply. This is a one-element bounded replay register.
    pending: Option<RequestV1>,
    /// One durable effect whose transport outcome is unresolved. Its stable
    /// identity survives only in caller persistence, but this live client
    /// fences unrelated effects until exact execution/lookup is definitive.
    pending_durable: Option<Box<PendingDurableOperation>>,
}

#[derive(Debug, PartialEq, Eq)]
struct PendingDurableOperation {
    operation: DurableOperationRefV1,
    intent: DurableEffectIntentV1,
}

impl<T: ServicePort> Client<T> {
    pub fn connect(mut transport: T) -> Result<Self, ServiceControlErrorV1> {
        let client = transport.connect()?;
        Ok(Self {
            transport,
            client,
            next_request: 1,
            pending: None,
            pending_durable: None,
        })
    }

    pub fn snapshot(&mut self) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::Snapshot)
    }

    pub fn workspace_inventory(&mut self) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::WorkspaceInventory)
    }

    pub fn workspace_changes(
        &mut self,
        after: Option<WorkspaceChangeCursorV1>,
        page_size: u16,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::WorkspaceChanges { after, page_size })
    }

    /// Requests one bounded page. The caller must pass the exact returned
    /// cursor; stale and foreign continuations are semantic refusals.
    pub fn workspace_inventory_page(
        &mut self,
        cursor: Option<WorkspaceInventoryCursorV1>,
        page_size: u16,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::WorkspaceInventoryPage { cursor, page_size })
    }

    pub fn open_view(&mut self) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::OpenView)
    }

    pub fn close_view(&mut self) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::CloseView)
    }

    pub fn add_file(&mut self, locator: String) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::AddFile { locator })
    }

    pub fn add_directory(&mut self, locator: String) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::AddDirectory { locator })
    }

    pub fn remove_document(
        &mut self,
        document: WorkspaceDocumentIdV1,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::RemoveDocument { document })
    }

    pub fn remove_root(
        &mut self,
        root: WorkspaceRootIdV1,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::RemoveRoot { root })
    }

    pub fn select_document(
        &mut self,
        document: WorkspaceDocumentIdV1,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::SelectDocument { document })
    }

    pub fn save(&mut self, expected: VersionWitness) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::Save { expected })
    }

    pub fn replace(
        &mut self,
        expected: VersionWitness,
        start: u64,
        end: u64,
        inserted: String,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::Replace {
            expected,
            start,
            end,
            inserted,
        })
    }

    /// Requests one revision-witnessed replacement followed by synchronized
    /// persistence under the same retained request identity. The accepted
    /// receipt distinguishes durable, not-established, indeterminate, and
    /// post-edit save-admission outcomes.
    pub fn replace_and_save(
        &mut self,
        expected: VersionWitness,
        start: u64,
        end: u64,
        inserted: String,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::ReplaceAndSave {
            expected,
            start,
            end,
            inserted,
        })
    }

    /// Publishes one bounded, revision-witnessed multi-document transaction in
    /// memory. Persistence remains a separate effect with separate receipts.
    pub fn workspace_edit(
        &mut self,
        documents: Vec<WorkspaceDocumentEditV1>,
    ) -> Result<ReplyV1, ServiceAccessErrorV1> {
        self.send(CommandV1::WorkspaceEdit { documents })
    }

    /// Executes one explicit restart-stable effect under a caller-persisted
    /// operation reference. The reference must be chosen before the first
    /// attempt; this client never mints one after uncertainty.
    pub fn execute_durable(
        &mut self,
        operation: DurableOperationRefV1,
        intent: DurableEffectIntentV1,
    ) -> Result<DurableExecuteOutcomeV1, ServiceAccessErrorV1> {
        if self.pending.is_some() {
            return Err(ServiceAccessErrorV1::OutcomeUnknown);
        }
        if self
            .pending_durable
            .as_ref()
            .is_some_and(|pending| pending.operation != operation || pending.intent != intent)
        {
            return Err(ServiceAccessErrorV1::OutcomeUnknown);
        }
        let request = self.next_request_id()?;
        let result = self.transport.execute_durable(DurableExecuteRequestV1 {
            operation,
            client: self.client,
            request,
            intent: intent.clone(),
        });
        match result {
            Ok(outcome) => {
                self.pending_durable = None;
                Ok(outcome)
            }
            Err(ServiceAccessErrorV1::OutcomeUnknown) => {
                self.pending_durable =
                    Some(Box::new(PendingDurableOperation { operation, intent }));
                Err(ServiceAccessErrorV1::OutcomeUnknown)
            }
            Err(error) => Err(error),
        }
    }

    /// Looks up historical evidence only. Absence is returned as the native
    /// closed outcome and never falls through to execution.
    pub fn lookup_durable_receipt(
        &mut self,
        operation: DurableOperationRefV1,
        intent: DurableEffectIntentV1,
    ) -> Result<DurableReceiptLookupOutcomeV1, ServiceAccessErrorV1> {
        if self.pending.is_some() {
            return Err(ServiceAccessErrorV1::OutcomeUnknown);
        }
        if self
            .pending_durable
            .as_ref()
            .is_some_and(|pending| pending.operation != operation || pending.intent != intent)
        {
            return Err(ServiceAccessErrorV1::OutcomeUnknown);
        }
        let request = self.next_request_id()?;
        let result = self
            .transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client: self.client,
                request,
                intent,
            });
        if matches!(
            result,
            Ok(DurableReceiptLookupOutcomeV1::Complete(_)
                | DurableReceiptLookupOutcomeV1::RequestConflict)
        ) {
            self.pending_durable = None;
        }
        result
    }

    /// Explicitly retires one completed durable receipt. This never clears or
    /// bypasses an unresolved effect fence; callers must first close any
    /// pending outcome by exact lookup.
    pub fn retire_durable_receipt(
        &mut self,
        operation: DurableOperationRefV1,
        intent: DurableEffectIntentV1,
    ) -> Result<DurableReceiptRetirementOutcomeV1, ServiceAccessErrorV1> {
        if self.pending.is_some() || self.pending_durable.is_some() {
            return Err(ServiceAccessErrorV1::OutcomeUnknown);
        }
        let request = self.next_request_id()?;
        self.transport
            .retire_durable_receipt(DurableReceiptRetirementV1 {
                operation,
                client: self.client,
                request,
                intent,
            })
    }

    /// Reads only the content-free current state of one stable operation. This
    /// does not clear an uncertain-operation fence and grants no execute or
    /// receipt-retirement authority.
    pub fn durable_receipt_status(
        &mut self,
        operation: DurableOperationRefV1,
    ) -> Result<DurableReceiptStatusV1, ServiceAccessErrorV1> {
        if self.pending.is_some() {
            return Err(ServiceAccessErrorV1::OutcomeUnknown);
        }
        let request = self.next_request_id()?;
        self.transport
            .durable_receipt_status(DurableReceiptStatusRequestV1 {
                operation,
                client: self.client,
                request,
            })
    }

    fn next_request_id(&mut self) -> Result<RequestId, ServiceAccessErrorV1> {
        let Some(request) = RequestId::new(self.next_request) else {
            return Err(ServiceAccessErrorV1::Limit);
        };
        self.next_request = self.next_request.checked_add(1).unwrap_or(0);
        Ok(request)
    }

    pub fn send(&mut self, command: CommandV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
        if let Some(pending) = &self.pending {
            if pending.command != command {
                // A different operation cannot cross an unresolved effect
                // boundary. No transport call is made and no new identity is
                // allocated.
                return Err(ServiceAccessErrorV1::OutcomeUnknown);
            }
            let request = pending.clone();
            return match self.transport.execute(request) {
                Ok(reply) => {
                    self.pending = None;
                    Ok(reply)
                }
                Err(error) => Err(error),
            };
        }
        let Some(request) = RequestId::new(self.next_request) else {
            return Ok(ReplyV1::Refused(RefusalV1::CounterExhausted));
        };
        self.next_request = self.next_request.checked_add(1).unwrap_or(0);
        let request = RequestV1 {
            client: self.client,
            request,
            command,
        };
        match self.transport.execute(request.clone()) {
            Err(ServiceAccessErrorV1::OutcomeUnknown) => {
                self.pending = Some(request);
                Err(ServiceAccessErrorV1::OutcomeUnknown)
            }
            result => result,
        }
    }

    /// The exact request retained after an outcome-unknown transport failure.
    /// Calling the same command replays this identity; any different command
    /// is fenced until a trustworthy reply closes it.
    pub fn pending_request(&self) -> Option<&RequestV1> {
        self.pending.as_ref()
    }

    pub fn disconnect(mut self) -> Result<T, (Self, ServiceControlErrorV1)> {
        if self.pending.is_some() || self.pending_durable.is_some() {
            return Err((
                self,
                ServiceControlErrorV1::Access(ServiceAccessErrorV1::OutcomeUnknown),
            ));
        }
        match self.transport.disconnect(self.client) {
            Ok(()) => Ok(self.transport),
            Err(error) => Err((self, error)),
        }
    }

    /// Abandon client-side replay and disconnect protection and recover the
    /// raw transport. If [`Self::pending_request`] is present, the caller is
    /// explicitly accepting that the uncertain request may remain applied and
    /// unreplayable; this is not an ordinary cleanup path.
    pub fn into_transport(self) -> T {
        self.transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use vellum_protocol::VersionWitness;

    #[derive(Debug)]
    struct RecordingTransport {
        requests: Vec<RequestV1>,
        disconnected: Vec<ClientId>,
    }

    impl ServicePort for RecordingTransport {
        fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
            Ok(ClientId::new(7).unwrap())
        }

        fn execute(&mut self, request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
            self.requests.push(request);
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable))
        }

        fn disconnect(&mut self, client: ClientId) -> Result<(), ServiceControlErrorV1> {
            self.disconnected.push(client);
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FaultTransport {
        requests: Vec<RequestV1>,
        outcomes: VecDeque<Result<ReplyV1, ServiceAccessErrorV1>>,
    }

    impl ServicePort for FaultTransport {
        fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
            Ok(ClientId::new(7).unwrap())
        }

        fn execute(&mut self, request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
            self.requests.push(request);
            self.outcomes.pop_front().unwrap()
        }

        fn disconnect(&mut self, _client: ClientId) -> Result<(), ServiceControlErrorV1> {
            Ok(())
        }
    }

    fn fault_client(
        outcomes: impl IntoIterator<Item = Result<ReplyV1, ServiceAccessErrorV1>>,
    ) -> Client<FaultTransport> {
        Client::connect(FaultTransport {
            requests: Vec::new(),
            outcomes: outcomes.into_iter().collect(),
        })
        .unwrap()
    }

    #[derive(Debug)]
    struct DurableFaultTransport {
        executes: Vec<DurableExecuteRequestV1>,
        lookups: Vec<DurableReceiptLookupV1>,
        statuses: Vec<DurableReceiptStatusRequestV1>,
        execute_outcomes: VecDeque<Result<DurableExecuteOutcomeV1, ServiceAccessErrorV1>>,
        lookup_outcomes: VecDeque<Result<DurableReceiptLookupOutcomeV1, ServiceAccessErrorV1>>,
    }

    impl ServicePort for DurableFaultTransport {
        fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
            Ok(ClientId::new(7).unwrap())
        }

        fn execute(&mut self, _request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
            panic!("durable APIs must not fall through to ordinary execute")
        }

        fn disconnect(&mut self, _client: ClientId) -> Result<(), ServiceControlErrorV1> {
            Ok(())
        }

        fn execute_durable(
            &mut self,
            request: DurableExecuteRequestV1,
        ) -> Result<DurableExecuteOutcomeV1, ServiceAccessErrorV1> {
            self.executes.push(request);
            self.execute_outcomes.pop_front().unwrap()
        }

        fn lookup_durable_receipt(
            &mut self,
            lookup: DurableReceiptLookupV1,
        ) -> Result<DurableReceiptLookupOutcomeV1, ServiceAccessErrorV1> {
            self.lookups.push(lookup);
            self.lookup_outcomes.pop_front().unwrap()
        }

        fn durable_receipt_status(
            &mut self,
            request: DurableReceiptStatusRequestV1,
        ) -> Result<DurableReceiptStatusV1, ServiceAccessErrorV1> {
            self.statuses.push(request);
            Ok(DurableReceiptStatusV1::Pending)
        }
    }

    #[test]
    fn durable_uncertainty_fences_other_effects_and_only_definitive_lookup_clears() {
        let mut client = Client::connect(DurableFaultTransport {
            executes: Vec::new(),
            lookups: Vec::new(),
            statuses: Vec::new(),
            execute_outcomes: [
                Err(ServiceAccessErrorV1::OutcomeUnknown),
                Ok(DurableExecuteOutcomeV1::Refused(RefusalV1::Stale)),
            ]
            .into_iter()
            .collect(),
            lookup_outcomes: [
                Ok(DurableReceiptLookupOutcomeV1::OutcomeUnknown),
                Ok(DurableReceiptLookupOutcomeV1::Complete(
                    DurableExecuteOutcomeV1::Refused(RefusalV1::Stale),
                )),
            ]
            .into_iter()
            .collect(),
        })
        .unwrap();
        let operation = DurableOperationRefV1::new([1; 16]).unwrap();
        let other = DurableOperationRefV1::new([2; 16]).unwrap();
        let intent = DurableEffectIntentV1::AddFile {
            locator: "one".into(),
        };
        assert_eq!(
            client.execute_durable(operation, intent.clone()),
            Err(ServiceAccessErrorV1::OutcomeUnknown)
        );
        assert_eq!(
            client.execute_durable(other, intent.clone()),
            Err(ServiceAccessErrorV1::OutcomeUnknown)
        );
        assert_eq!(
            client.durable_receipt_status(operation),
            Ok(DurableReceiptStatusV1::Pending)
        );
        assert_eq!(client.transport.statuses.len(), 1);
        assert!(client.pending_durable.is_some());
        assert_eq!(
            client.lookup_durable_receipt(operation, intent.clone()),
            Ok(DurableReceiptLookupOutcomeV1::OutcomeUnknown)
        );
        assert_eq!(
            client.execute_durable(
                operation,
                DurableEffectIntentV1::AddFile {
                    locator: "changed".into(),
                },
            ),
            Err(ServiceAccessErrorV1::OutcomeUnknown)
        );
        assert_eq!(
            client.lookup_durable_receipt(operation, intent.clone()),
            Ok(DurableReceiptLookupOutcomeV1::Complete(
                DurableExecuteOutcomeV1::Refused(RefusalV1::Stale),
            ))
        );
        assert_eq!(
            client.execute_durable(other, intent),
            Ok(DurableExecuteOutcomeV1::Refused(RefusalV1::Stale))
        );
        let transport = client.into_transport();
        assert_eq!(transport.executes.len(), 2);
        assert_eq!(transport.lookups.len(), 2);
    }

    #[test]
    fn client_issues_monotone_request_ids_without_interpreting_refusals() {
        let transport = RecordingTransport {
            requests: Vec::new(),
            disconnected: Vec::new(),
        };
        let mut client = Client::connect(transport).unwrap();
        assert_eq!(
            client.snapshot(),
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable))
        );
        assert_eq!(
            client.replace(VersionWitness::new(3).unwrap(), 1, 2, "x".into()),
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable))
        );
        assert_eq!(
            client.replace_and_save(VersionWitness::new(4).unwrap(), 2, 3, "y".into()),
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable))
        );
        let transport = client.disconnect().unwrap();
        assert_eq!(transport.requests.len(), 3);
        assert_eq!(transport.requests[0].request.get(), 1);
        assert_eq!(transport.requests[1].request.get(), 2);
        assert_eq!(transport.requests[2].request.get(), 3);
        assert!(matches!(
            transport.requests[2].command,
            CommandV1::ReplaceAndSave { .. }
        ));
        assert_eq!(transport.disconnected, vec![ClientId::new(7).unwrap()]);
    }

    #[test]
    fn client_projects_workspace_inventory_as_a_native_command() {
        let transport = RecordingTransport {
            requests: Vec::new(),
            disconnected: Vec::new(),
        };
        let mut client = Client::connect(transport).unwrap();

        assert_eq!(
            client.workspace_inventory(),
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable))
        );

        let transport = client.into_transport();
        assert_eq!(transport.requests.len(), 1);
        assert_eq!(transport.requests[0].command, CommandV1::WorkspaceInventory);
    }

    #[test]
    fn client_projects_inventory_cursor_without_interpreting_it() {
        let transport = RecordingTransport {
            requests: Vec::new(),
            disconnected: Vec::new(),
        };
        let mut client = Client::connect(transport).unwrap();
        let workspace = vellum_protocol::WorkspaceIdV1::new(11).unwrap();
        let cursor = WorkspaceInventoryCursorV1::new(workspace, 13, 17).unwrap();

        client.workspace_inventory_page(Some(cursor), 3).unwrap();

        let transport = client.into_transport();
        assert_eq!(
            transport.requests[0].command,
            CommandV1::WorkspaceInventoryPage {
                cursor: Some(cursor),
                page_size: 3,
            }
        );
    }

    #[test]
    fn client_projects_change_cursor_without_interpreting_it() {
        let transport = RecordingTransport {
            requests: Vec::new(),
            disconnected: Vec::new(),
        };
        let mut client = Client::connect(transport).unwrap();
        let cursor =
            WorkspaceChangeCursorV1::new(vellum_protocol::WorkspaceIdV1::new(11).unwrap(), 3, 13)
                .unwrap();
        client.workspace_changes(Some(cursor), 1).unwrap();
        let transport = client.into_transport();
        assert_eq!(
            transport.requests[0].command,
            CommandV1::WorkspaceChanges {
                after: Some(cursor),
                page_size: 1
            }
        );
    }

    #[test]
    fn client_projects_view_lifecycle_without_interpreting_results() {
        let transport = RecordingTransport {
            requests: Vec::new(),
            disconnected: Vec::new(),
        };
        let mut client = Client::connect(transport).unwrap();
        client.open_view().unwrap();
        client.close_view().unwrap();

        let transport = client.into_transport();
        assert_eq!(transport.requests.len(), 2);
        assert_eq!(transport.requests[0].command, CommandV1::OpenView);
        assert_eq!(transport.requests[1].command, CommandV1::CloseView);
    }

    #[test]
    fn client_projects_workspace_mutations_without_interpreting_results() {
        let transport = RecordingTransport {
            requests: Vec::new(),
            disconnected: Vec::new(),
        };
        let mut client = Client::connect(transport).unwrap();
        let workspace = vellum_protocol::WorkspaceIdV1::new(11).unwrap();
        let document = WorkspaceDocumentIdV1::new(workspace, 13).unwrap();

        client.add_file("private-file".into()).unwrap();
        client.add_directory("private-directory".into()).unwrap();
        client.select_document(document).unwrap();
        client.save(VersionWitness::new(17).unwrap()).unwrap();
        client
            .workspace_edit(vec![WorkspaceDocumentEditV1 {
                document,
                expected_revision: 19,
                replacements: vec![vellum_protocol::WorkspaceReplacementV1 {
                    start: 0,
                    end: 0,
                    inserted: "private insertion".into(),
                }],
            }])
            .unwrap();

        let transport = client.into_transport();
        assert_eq!(transport.requests.len(), 5);
        assert_eq!(
            transport.requests[0].command,
            CommandV1::AddFile {
                locator: "private-file".into()
            }
        );
        assert_eq!(
            transport.requests[1].command,
            CommandV1::AddDirectory {
                locator: "private-directory".into()
            }
        );
        assert_eq!(
            transport.requests[2].command,
            CommandV1::SelectDocument { document }
        );
        assert_eq!(
            transport.requests[3].command,
            CommandV1::Save {
                expected: VersionWitness::new(17).unwrap()
            }
        );
        assert_eq!(transport.requests[0].request.get(), 1);
        assert_eq!(transport.requests[3].request.get(), 4);
        assert!(matches!(
            transport.requests[4].command,
            CommandV1::WorkspaceEdit { .. }
        ));
        assert_eq!(transport.requests[4].request.get(), 5);
    }

    #[test]
    fn outcome_unknown_replays_only_the_equal_command_under_the_exact_identity() {
        let expected = VersionWitness::new(17).unwrap();
        let mut client = fault_client([
            Err(ServiceAccessErrorV1::OutcomeUnknown),
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable)),
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable)),
        ]);

        assert_eq!(
            client.save(expected),
            Err(ServiceAccessErrorV1::OutcomeUnknown)
        );
        assert_eq!(client.pending_request().unwrap().request.get(), 1);
        assert_eq!(
            client.save(expected),
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable))
        );
        assert!(client.pending_request().is_none());
        assert_eq!(
            client.snapshot(),
            Ok(ReplyV1::Refused(RefusalV1::EditorUnavailable))
        );

        let transport = client.into_transport();
        assert_eq!(transport.requests.len(), 3);
        assert_eq!(transport.requests[0], transport.requests[1]);
        assert_eq!(transport.requests[0].request.get(), 1);
        assert_eq!(transport.requests[2].request.get(), 2);
    }

    #[test]
    fn outcome_unknown_fences_a_different_command_without_touching_transport() {
        let expected = VersionWitness::new(17).unwrap();
        let mut client = fault_client([Err(ServiceAccessErrorV1::OutcomeUnknown)]);

        assert_eq!(
            client.save(expected),
            Err(ServiceAccessErrorV1::OutcomeUnknown)
        );
        assert_eq!(client.snapshot(), Err(ServiceAccessErrorV1::OutcomeUnknown));
        assert_eq!(client.pending_request().unwrap().request.get(), 1);

        let transport = client.into_transport();
        assert_eq!(transport.requests.len(), 1);
    }

    #[test]
    fn not_sent_after_unknown_does_not_clear_the_original_uncertainty() {
        let expected = VersionWitness::new(17).unwrap();
        let mut client = fault_client([
            Err(ServiceAccessErrorV1::OutcomeUnknown),
            Err(ServiceAccessErrorV1::NotSent),
        ]);

        assert_eq!(
            client.save(expected),
            Err(ServiceAccessErrorV1::OutcomeUnknown)
        );
        assert_eq!(client.save(expected), Err(ServiceAccessErrorV1::NotSent));
        assert_eq!(client.pending_request().unwrap().request.get(), 1);
        assert_eq!(client.snapshot(), Err(ServiceAccessErrorV1::OutcomeUnknown));

        let transport = client.into_transport();
        assert_eq!(transport.requests.len(), 2);
        assert_eq!(transport.requests[0], transport.requests[1]);
    }

    #[test]
    fn pending_uncertainty_fences_disconnect_and_returns_the_intact_client() {
        let expected = VersionWitness::new(17).unwrap();
        let mut client = fault_client([Err(ServiceAccessErrorV1::OutcomeUnknown)]);
        assert_eq!(
            client.save(expected),
            Err(ServiceAccessErrorV1::OutcomeUnknown)
        );

        let (client, error) = client.disconnect().unwrap_err();
        assert_eq!(
            error,
            ServiceControlErrorV1::Access(ServiceAccessErrorV1::OutcomeUnknown)
        );
        assert_eq!(client.pending_request().unwrap().request.get(), 1);
        let transport = client.into_transport();
        assert_eq!(transport.requests.len(), 1);
    }
}
