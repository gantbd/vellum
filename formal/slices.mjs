// Finite exploration bounds are harness parameters, not product capacity choices.
const tx = {module:'Transactions', constants:{Bound:7,LeakStage:false},
  invariants:['TypeOK','SerialRoot','NoPartialObservation','DiagnosticIsolation'],
  properties:['PublishValidated','RejectedUnchanged']};
const history = {module:'History', constants:{Bound:6,MixedGroup:false,PartialUndo:false},
  invariants:['NoSplitUndo','NoSplitRedo','HistoryMatchesContent','NoInvalidRedo','DisjointPositions'],
  properties:['BlockedUnchanged','FreshVersions','ReversalValidated']};
const disclosure = {module:'Disclosure', constants:{Bound:5,LeakCache:false},
  invariants:['FunctionalNoninterference','EqualPublicState'],
  properties:['BodySurvivesDeletion','HiddenPreservesSource','NoUnauthorizedRelease']};
const persistence = {module:'Persistence', constants:{Bound:6,ConditionalCapable:true,IgnoreBinding:false,IgnoreCapability:false,LooseReceiptIdentity:false},
  invariants:['BaselineAttribution','NoUndispatchedEvidence','StrongAdmission','ConditionalWrite','MalformedReceiptsRejected'],
  properties:['EvidenceMonotone','CaptureImmutable','StopPreservesEvidence','BadReceiptNoEffect','SafeRebind','SourceDoesNotSaveAnnotations']};
const bridges = {module:'Bridges',constants:{Bound:6,Fuel:6,SplitHistory:false,RestoreBody:false,LeakCapture:false,LeakProjection:false,UnchargedBuild:false},
  invariants:['KernelTypes','HistoryTextCoherent','MappingCurrent','CapturePublished','EffectCaptured','ProjectionErasesHidden','NoCaptureBeforePrepare'],
  properties:['KernelRefinement','WorkCharged','SourcePreservesAuthoredBody','CommentPreservesSourceHistory','RejectedRootUnchanged','CaptureImmutable','ReleaseAtCurrentBoundary','TerminalAndEvidenceStable']};
const saves = {module:'ConcurrentSaves',constants:{Bound:8,SingleFlight:false,OrderedEvidence:false,FreshTokens:true,ProfileOnly:false,
  ReceiptOrderSelection:false,ForgetStopped:false,FreezeToken:false,UnsafeTokenOrder:false},
  invariants:['ExactReceipts','OneWinnerPerToken','EvidenceOrderSound','SelectionRespectsEvidence','SingleFlightBound'],
  properties:['ReceiptsMonotone','CaptureImmutable','StopPreservesObligations','DuplicateSelectionStable']};
const admission = {module:'SaveAdmission',constants:{Bound:6,QueuePolicy:true,Capacity:2,
  ForgetStopped:false,Recapture:false,DropQueued:false,Bypass:false,
  Lifecycle:false,BudgetUnits:3,CompletionSlots:2,IgnoreCost:false,IgnoreSlots:false,ReleaseUnknown:false},
  invariants:['TypeOK','QueueLedger','CapacityBound','SingleFlight','ExactDispatch','FIFO',
    'ReservationLedger','CostBound','CompletionBound','ExpiredNotDispatched'],
  properties:['CaptureImmutable','EvidenceMonotone','StoppedStable','ExpiredStable']};
const dispatch = {module:'DispatchGate',constants:{Bound:5,ProviderSafe:true,
  IgnoreAuthority:false,IgnoreBinding:false,IgnoreEpoch:false,TrustAnything:false,
  MatchAttemptOnly:false,PromoteProgress:false,IgnoreCapability:false,RequireCurrentReceipt:false},
  invariants:['TypeOK','AuthorityAtLaunch','BindingAtLaunch','EpochAtLaunch',
    'ExactTrustedEvidence','ValidEvidenceRetained','SafeResolution','SingleFlight'],
  properties:['EvidenceMonotone','CaptureImmutable','StoppedStable','TerminalStable']};
const composition = {module:'SaveComposition',constants:{Bound:5,EarlyRefund:false,SkipRejectRefund:false,
  HistoricalBaseline:false,QuiescentBaseline:false,ReceiptOrderBaseline:false,LeakRead:false,DropOnWait:false,IgnoreContradiction:false},
  invariants:['RejectedRefund','ResolutionCoherent','ControlCoherent','CaptureCoherent','KernelSafety',
    'AckEvidence','BaselineAcknowledged','BaselineUncontested','BaselineAttributed','LatestKnownAck','ReadSafe'],
  properties:['AdmissionSteps','GateSteps','EvidenceMonotone','ReleaseProjection','AdmissionClosed','WaitIsNotQuiescence','ReportStable']};
const execution = {module:'SaveExecution',constants:{Bound:5,Fuel:6,UnchargedTick:false,ForgetCleanup:false,QuiescenceAsFailure:false},
  invariants:['ControlType','NoOrphanControl','CompositeSafety','ReceiptGrades'],
  properties:['KernelCorrespondence','RankDecreases','NoRefill','TerminalStable','CompositeSteps','LateFactsMonotone']};
const storage = {module:'StorageBridge',constants:{Bound:6,HostCapable:true,ForgeStored:false,FakeEarly:false,IgnoreConflict:false},
  invariants:['LiveAgreement','CaptureAgreement','AckBackedByWrite','OutcomeAgreement','BaselineAgreement',
    'DispatchAgreement','ResolutionBackedByQuiet','CompositeSafety','PersistenceSafety'],
  properties:['CompositeSteps','PersistenceSteps','EvidenceMonotone','SafeRebind']};
const recipient = {module:'RecipientBridge',constants:{Bound:5,LeakHidden:false,LeakEvidence:false,BypassGate:false},
  invariants:['SourceAgreement','FunctionalNoninterference','NoPrivateEvidence','ReleaseSafe','StorageSafety'],
  properties:['StorageSteps','DisclosureSteps','OutputMonotone']};
const editor = {module:'EditorContracts',constants:{Bound:8,SourceLimit:3,InsertLimit:2,BatchInsertLimit:1,
  UnsafeScope:false,SplitState:false,
  LeakRelease:false,SkipGap:false,BadMap:false,TrustDiscovery:false,
  IgnoreEventGrant:false,IgnoreEventRetention:false},
  invariants:['TypeOK','Coherent','Frozen','BodyAndOwner','SafeRelease','GapSafe','EventReleaseSafe'],
  properties:['ScopeAtPublish','RejectedUnchanged','DiscoveryAuthorized']};
const derived = {module:'DerivedRetention',constants:{Bound:6,Capacity:2,
  ForgetOnCancel:false,IgnoreContext:false,IgnoreRevision:false,IgnoreGrant:false,
  TransferOwner:false,IgnoreCapacity:false},
  invariants:['TypeOK','NoDangling','ExactAccounting','Budget','OwnerStable',
    'SnapshotStable','CurrentEvidence'],
  properties:['CancelRetains','WorkerCannotPublish']};
const accounting = {module:'AccountingBridge',constants:{Bound:5,
  SkipAdmission:false,DoubleRefund:false,PrematureRefund:false,WrongCost:false},
  invariants:['KernelSafety','LogicalPartition','CountConservation','CostConservation','Nonnegative'],
  properties:['AdmissionSteps']};
const privacySignals = {module:'PrivacySignals',constants:{Bound:4,
  LeakTag:false,LeakCount:false,SkipEpoch:false},
  invariants:['EqualAuthorizedState','FunctionalPrivacy','CurrentDiagnosticRelease','DeliveryMatchesOutput'],
  properties:['CoreSteps','DiagnosticIsolation','DeliveryStable']};
const useful = {module:'UsefulProgress',constants:{Bound:2,
  SilentDrop:false,AcceptStale:false,CancelDischarges:false},
  invariants:['DispositionPartition','CurrentAcceptance'],
  properties:['TerminalStable','ServiceDecreases','EventualDrain']};
const positive = (id,base,constants={})=>({id,...base,constants:{...base.constants,...constants},expected:'success'});
const handoff = {module:'ContextHandoff',constants:{Bound:6,CompatibleSemantics:false,
  IgnoreCompatibility:false,ReinterpretAtCommit:false,ForgetContext:false,RewriteCapture:false,
  IgnoreAuthority:false,IgnoreRevision:false,DiagnosticsOn:true,DiagnosticFull:false,
  DiagnosticFault:false,DiagnosticWrites:false,DiagnosticRejects:false},
  invariants:['TypeOK','NoDanglingContext','CaptureConsistent','PublishedAuthority','PublishedRevision',
    'PublishedCompatibility','CapturedMeaning','AtomicCandidate','OwnerStable'],
  properties:['CaptureStable','DiagnosticIsolation','RejectedUnchanged','TerminalStable']};
const negative = (id,base,constants,violation)=>({...positive(id,base,constants),expected:'counterexample',violation});
const witness = (base,target,constants={})=>({...positive(`${base.module}-${target}`,base,constants),
  expected:'witness',invariants:[...base.invariants,`Not${target}`],violation:`Invariant Not${target} is violated`});
const noop = {module:'NoOpPublication',constants:{Bound:6,Kind:0,DiagnosticMode:1,
  SkipValidation:false,TextOnly:false,ForceChange:false,BookkeepNoop:false,RestoreCaptured:false,
  DiagnosticWrites:false,DiagnosticRejects:false},
  invariants:['TypeOK','SuccessValidated','NoopMeaning','NoopComplete'],
  properties:['NoopFrame','RejectedFrame','CandidatePublished','DiagnosticIsolation','TerminalStable']};
export const sliceCases = [
  ...Array.from({length:9},(_,Kind)=>positive(`noop-kind-${Kind}-safety`,noop,{Kind})),
  ...[0,2,3].map(DiagnosticMode=>positive(`noop-diagnostics-${DiagnosticMode}-safety`,noop,{DiagnosticMode})),
  {...negative('noop-validation-bypass-control',noop,{SkipValidation:true},'Invariant SuccessValidated is violated'),
    invariants:['TypeOK','SuccessValidated'],properties:[]},
  {...negative('noop-fabricated-change-control',noop,{ForceChange:true},'Invariant NoopComplete is violated'),
    invariants:['TypeOK','NoopComplete'],properties:[]},
  ...[2,3,4,5,7].map(Kind=>({...negative(`noop-text-only-${Kind}-control`,noop,{Kind,TextOnly:true},'Invariant NoopMeaning is violated'),
    invariants:['TypeOK','NoopMeaning'],properties:[]})),
  {...negative('noop-bookkeeping-control',noop,{BookkeepNoop:true},'Action property NoopFrame is violated'),
    invariants:['TypeOK'],properties:['NoopFrame']},
  {...negative('noop-stale-root-control',noop,{RestoreCaptured:true},'Action property NoopFrame is violated'),
    invariants:['TypeOK'],properties:['NoopFrame']},
  {...negative('noop-diagnostic-write-control',noop,{DiagnosticWrites:true},'Action property DiagnosticIsolation is violated'),
    invariants:['TypeOK'],properties:['DiagnosticIsolation']},
  {...negative('noop-diagnostic-relabel-control',noop,{DiagnosticMode:3,DiagnosticRejects:true},'Action property TerminalStable is violated'),
    invariants:['TypeOK'],properties:['TerminalStable']},
  ...['EmptyUnchanged','SameTextUnchanged','MappedApplied','SelectionApplied','AnnotationApplied',
    'HistoryApplied','TextApplied','EffectPreserved','MalformedRejected'].map((x,Kind)=>witness(noop,x,{Kind})),
  ...['GrantNoopRejected','ScopeNoopRejected','ReadonlyNoopRejected','CapacityNoopRejected',
    'CapabilityNoopRejected','ContextNoopRejected','StaleEqualNoopRejected',
    'CancelledBeforeNoop','CancelledAfterNoop','UnrelatedBeforeNoop'].map(x=>witness(noop,x)),
  witness(noop,'PostNoopFault',{DiagnosticMode:3}),
  positive('context-handoff-safety',handoff),
  positive('context-handoff-compatible-safety',handoff,{CompatibleSemantics:true}),
  positive('context-handoff-diagnostics-off',handoff,{DiagnosticsOn:false}),
  positive('context-handoff-diagnostics-full',handoff,{DiagnosticFull:true}),
  positive('context-handoff-diagnostics-fault',handoff,{DiagnosticFault:true}),
  {...negative('context-handoff-compatibility-control',handoff,{IgnoreCompatibility:true},'Invariant PublishedCompatibility is violated'),
    invariants:['TypeOK','PublishedCompatibility'],properties:[]},
  {...negative('context-handoff-reinterpret-control',handoff,{IgnoreCompatibility:true,ReinterpretAtCommit:true},'Invariant CapturedMeaning is violated'),
    invariants:['TypeOK','CapturedMeaning'],properties:[]},
  {...negative('context-handoff-retention-control',handoff,{ForgetContext:true},'Invariant NoDanglingContext is violated'),
    invariants:['TypeOK','NoDanglingContext'],properties:[]},
  {...negative('context-handoff-authority-control',handoff,{IgnoreAuthority:true},'Invariant PublishedAuthority is violated'),
    invariants:['TypeOK','PublishedAuthority'],properties:[]},
  {...negative('context-handoff-revision-control',handoff,{IgnoreRevision:true},'Invariant PublishedRevision is violated'),
    invariants:['TypeOK','PublishedRevision'],properties:[]},
  {...negative('context-handoff-diagnostic-write-control',handoff,{DiagnosticWrites:true},'Action property DiagnosticIsolation is violated'),
    invariants:['TypeOK'],properties:['DiagnosticIsolation']},
  {...negative('context-handoff-capture-rewrite-control',handoff,{RewriteCapture:true},'Action property CaptureStable is violated'),
    invariants:['TypeOK'],properties:['CaptureStable']},
  {...negative('context-handoff-postcommit-fault-control',handoff,{DiagnosticFault:true,DiagnosticRejects:true},'Action property TerminalStable is violated'),
    invariants:['TypeOK'],properties:['TerminalStable']},
  ...['FreshApplied','CosmeticApplied','IncompatibleRefused','StaleRefused',
    'RevokedRefused','HeldAcrossActivation','CancelledNoPublication'].map(x=>witness(handoff,x)),
  witness(handoff,'CompatibleChangedApplied',{CompatibleSemantics:true}),
  witness(handoff,'DiagnosticDrop',{DiagnosticFull:true}),
  witness(handoff,'DiagnosticFailure',{DiagnosticFault:true}),
  witness(handoff,'DiagnosticCaptured'),
  witness(handoff,'PostCommitFault',{DiagnosticFault:true}),
  positive('transactions-safety',tx),
  negative('transactions-private-stage-control',tx,{LeakStage:true},'Invariant NoPartialObservation is violated'),
  ...['SuccessfulX','BothCommit','StaleLoser','ReadDependencyRejected','ContextRejected','DeniedRejected','StoppedBeforePublish','PartialStagePrivate','SuccessfulSaturated','SuccessfulFailedCapture'].map(x=>witness(tx,x)),
  positive('history-source-safety',history),
  positive('history-compound-safety',history,{MixedGroup:true}),
  negative('history-partial-undo-control',history,{PartialUndo:true},'Invariant NoSplitUndo is violated'),
  ...['LinkedBlocked','IndependentUndo','CommentPreserved','RoundTrip','RedoInvalidated','DependentInvalidated','RetentionBlocks','RightsBlock'].map(x=>witness(history,x)),
  witness(history,'CompoundBlocked',{MixedGroup:true}),
  positive('disclosure-paired-safety',disclosure),
  negative('disclosure-rich-cache-control',disclosure,{LeakCache:true},'Invariant FunctionalNoninterference is violated'),
  ...['SuccessfulRelease','RevokedQueuedDenied','RegrantGapDenied','DetachedBodyRetained','HiddenDifferenceUnobservable'].map(x=>witness(disclosure,x)),
  positive('persistence-safety',persistence),
  positive('persistence-unsupported-capability',persistence,{ConditionalCapable:false}),
  negative('persistence-stale-binding-control',persistence,{IgnoreBinding:true},'Invariant BaselineAttribution is violated'),
  negative('persistence-capability-control',persistence,{ConditionalCapable:false,IgnoreCapability:true},'Invariant StrongAdmission is violated'),
  negative('persistence-loose-identity-control',persistence,{LooseReceiptIdentity:true},'Invariant MalformedReceiptsRejected is violated'),
  ...['SavedSource','SavedAnnotations','OldRevisionReceipt','UndoEqualFreshRevision','StoppedUnknown','LateReceipt','HistoricalAfterRebind','PartialSave','CASRaceRejected','ConflictingEvidence','MismatchedReceiptRejected'].map(x=>witness(persistence,x)),
  {...witness(persistence,'BothArtifactsSaved',{Bound:8}),specification:'BothSaveSpec'},
  witness(persistence,'UnsupportedRejected',{ConditionalCapable:false}),
  positive('bridges-shared-safety',bridges),
  {...positive('bridges-body-script-safety',bridges,{Bound:11}),specification:'BodySpec'},
  {...positive('bridges-late-script-safety',bridges,{Bound:9}),specification:'LateSpec'},
  {...positive('bridges-save-script-safety',bridges,{Bound:8}),specification:'SaveSpec'},
  negative('bridges-split-history-control',bridges,{SplitHistory:true},'Invariant HistoryTextCoherent is violated'),
  {...negative('bridges-old-body-control',bridges,{RestoreBody:true,Bound:11},'Action property SourcePreservesAuthoredBody is violated'),specification:'BodySpec'},
  negative('bridges-private-capture-control',bridges,{LeakCapture:true},'Invariant CapturePublished is violated'),
  negative('bridges-owner-generation-control',bridges,{LeakProjection:true},'Invariant ProjectionErasesHidden is violated'),
  {...negative('bridges-uncharged-build-control',bridges,{UnchargedBuild:true},'Action property KernelRefinement is violated'),properties:['KernelRefinement']},
  ...['CoherentEdit','BudgetStopped','SurpriseFaulted','SafeRelease'].map(x=>witness(bridges,x)),
  {...witness(bridges,'CommentSurvivesUndo',{Bound:11}),specification:'BodySpec'},
  {...witness(bridges,'FrozenSaveDuringEdit',{Bound:8}),specification:'SaveSpec'},
  {...witness(bridges,'LateReceiptRetainedDenied',{Bound:9}),specification:'LateSpec'},
  {...witness(bridges,'PublishedCapture',{Bound:7}),specification:'CaptureSpec'},
  positive('saves-overlap-fresh-safety',saves),
  positive('saves-single-flight-safety',saves,{SingleFlight:true}),
  positive('saves-reusable-opaque-safety',saves,{FreshTokens:false}),
  positive('saves-host-ordered-safety',saves,{FreshTokens:false,OrderedEvidence:true}),
  {...positive('saves-reverse-delivery-script',saves,{Bound:10}),specification:'ChainSpec'},
  {...positive('saves-foreign-gap-script',saves,{Bound:11}),specification:'GapSpec'},
  {...negative('saves-receipt-order-control',saves,{Bound:10,ReceiptOrderSelection:true},'Invariant SelectionRespectsEvidence is violated'),specification:'ChainSpec'},
  negative('saves-cancelled-slot-control',saves,{SingleFlight:true,ForgetStopped:true},'Invariant SingleFlightBound is violated'),
  negative('saves-fixed-token-control',saves,{FreezeToken:true},'Invariant OneWinnerPerToken is violated'),
  {...negative('saves-aba-order-control',saves,{Bound:10,FreshTokens:false,UnsafeTokenOrder:true},'Invariant EvidenceOrderSound is violated'),specification:'ChainSpec'},
  ...['ConcurrentOutstanding','StoppedOutstanding','NewWinsRace'].map(x=>witness(saves,x)),
  witness(saves,'StoppedBlocksNew',{SingleFlight:true}),
  {...witness(saves,'ReverseDeliverySafe',{Bound:10}),specification:'ChainSpec'},
  {...witness(saves,'SameRevisionDifferentOutput',{Bound:10,ProfileOnly:true}),specification:'ChainSpec'},
  {...witness(saves,'TokenChainGapUnresolved',{Bound:11}),specification:'GapSpec'},
  {...witness(saves,'OrderedGapResolved',{Bound:11,OrderedEvidence:true}),specification:'GapSpec'},
  {...witness(saves,'ReusableTokensUnordered',{Bound:10,FreshTokens:false}),specification:'ChainSpec'},
  positive('admission-bounded-fifo-safety',admission),
  positive('admission-busy-safety',admission,{QueuePolicy:false}),
  negative('admission-stopped-slot-control',admission,{ForgetStopped:true},'Invariant SingleFlight is violated'),
  negative('admission-recapture-control',admission,{Recapture:true},'Invariant ExactDispatch is violated'),
  negative('admission-silent-drop-control',admission,{DropQueued:true},'Invariant QueueLedger is violated'),
  negative('admission-fifo-bypass-control',admission,{Bypass:true},'Invariant FIFO is violated'),
  witness(admission,'BusyRejected',{QueuePolicy:false}),
  ...['CapacityRejected','QueuedCancelled','FrozenDispatch','StoppedBlocks','LateRelease','IndependentProgress','FIFOProgress'].map(x=>witness(admission,x)),
  positive('admission-lifecycle-safety',admission,{Lifecycle:true}),
  positive('admission-cost-pressure-safety',admission,{Lifecycle:true,BudgetUnits:2,CompletionSlots:3}),
  positive('admission-slot-pressure-safety',admission,{Lifecycle:true,BudgetUnits:8,CompletionSlots:1}),
  negative('admission-cost-overrun-control',admission,{Lifecycle:true,BudgetUnits:2,CompletionSlots:3,IgnoreCost:true},'Invariant CostBound is violated'),
  negative('admission-slot-overrun-control',admission,{Lifecycle:true,BudgetUnits:8,CompletionSlots:1,IgnoreSlots:true},'Invariant CompletionBound is violated'),
  negative('admission-expired-unknown-release-control',admission,{Lifecycle:true,ReleaseUnknown:true},'Invariant ReservationLedger is violated'),
  ...['QueuedExpiry','ActiveExpiryHeld','LateExpiredResolution','ReservationReused'].map(x=>witness(admission,x,{Lifecycle:true})),
  ...['BudgetRejected','UnknownBlocksBudget'].map(x=>witness(admission,x,{Lifecycle:true,BudgetUnits:2,CompletionSlots:3})),
  witness(admission,'SlotsRejected',{Lifecycle:true,BudgetUnits:8,CompletionSlots:1}),
  positive('dispatch-boundary-safety',dispatch),
  positive('dispatch-unattested-provider-safety',dispatch,{ProviderSafe:false}),
  negative('dispatch-stale-authority-control',dispatch,{IgnoreAuthority:true},'Invariant AuthorityAtLaunch is violated'),
  negative('dispatch-stale-binding-control',dispatch,{IgnoreBinding:true},'Invariant BindingAtLaunch is violated'),
  negative('dispatch-stale-epoch-control',dispatch,{IgnoreEpoch:true},'Invariant EpochAtLaunch is violated'),
  negative('dispatch-forged-source-control',dispatch,{TrustAnything:true},'Invariant ExactTrustedEvidence is violated'),
  negative('dispatch-loose-tuple-control',dispatch,{MatchAttemptOnly:true},'Invariant ExactTrustedEvidence is violated'),
  negative('dispatch-progress-is-quiescence-control',dispatch,{PromoteProgress:true},'Invariant SafeResolution is violated'),
  negative('dispatch-unsupported-guarantee-control',dispatch,{ProviderSafe:false,IgnoreCapability:true},'Invariant SafeResolution is violated'),
  negative('dispatch-current-receipt-gate-control',dispatch,{RequireCurrentReceipt:true},'Invariant ValidEvidenceRetained is violated'),
  ...['AcceptedDisposition','DeniedLaunch','StaleBinding','StaleEpoch','FrozenAllowed',
    'BadIgnored','ProgressHeld','LateBindingEvidence','LateRevokedEvidence','StoppedLateCompletion'].map(x=>witness(dispatch,x)),
  positive('composition-shared-safety',composition),
  {...positive('composition-late-script-safety',composition,{Bound:9}),specification:'LateSpec'},
  {...positive('composition-order-script-safety',composition,{Bound:9}),specification:'OrderSpec'},
  negative('composition-progress-refund-control',composition,{EarlyRefund:true},'Invariant ResolutionCoherent is violated'),
  negative('composition-reject-leak-control',composition,{SkipRejectRefund:true},'Invariant RejectedRefund is violated'),
  negative('composition-historical-baseline-control',composition,{HistoricalBaseline:true},'Invariant BaselineAttributed is violated'),
  negative('composition-quiescence-is-save-control',composition,{QuiescentBaseline:true},'Invariant BaselineAcknowledged is violated'),
  {...negative('composition-receipt-order-control',composition,{Bound:9,ReceiptOrderBaseline:true},'Invariant LatestKnownAck is violated'),specification:'OrderSpec'},
  negative('composition-release-authority-control',composition,{LeakRead:true},'Invariant ReadSafe is violated'),
  ...['SuccessfulSave','OldCaptureAcknowledged','ProgressHeld','BadHeld','RejectedRefund','QueuedTogether','StoppedHeld','ProjectedKnownAck'].map(x=>witness(composition,x)),
  {...witness(composition,'DeniedLateComposite',{Bound:9}),specification:'LateSpec'},
  {...witness(composition,'OrderedDuplicateSafe',{Bound:9}),specification:'OrderSpec'},
  {...positive('composition-shutdown-script-safety',composition,{Bound:8}),specification:'ShutdownSpec'},
  negative('composition-wait-refund-control',composition,{DropOnWait:true},'Invariant ResolutionCoherent is violated'),
  ...['QueuedExpiredRefund','QueuedCancelledRefund','ActiveExpiredHeld','WaitReportsUnknown'].map(x=>witness(composition,x)),
  {...witness(composition,'LateAfterWait',{Bound:8}),specification:'ShutdownSpec'},
  {...positive('composition-conflict-script-safety',composition,{Bound:6}),specification:'ConflictSpec'},
  {...negative('composition-ignore-conflict-control',composition,{Bound:6,IgnoreContradiction:true},'Invariant BaselineUncontested is violated'),specification:'ConflictSpec'},
  {...witness(composition,'ConflictRetained',{Bound:6}),specification:'ConflictSpec'},
  positive('save-execution-safety',execution),
  positive('save-execution-low-fuel-safety',execution,{Fuel:2}),
  {...positive('save-execution-late-script-safety',execution,{Fuel:4}),specification:'LateSpec'},
  negative('save-execution-uncharged-control',execution,{UnchargedTick:true},'Action property KernelCorrespondence is violated'),
  negative('save-execution-orphan-control',execution,{Fuel:2,ForgetCleanup:true},'Invariant NoOrphanControl is violated'),
  ...['Succeeded','FaultRefund','DeniedLaunch'].map(x=>witness(execution,x)),
  witness(execution,'BudgetBeforeDispatch',{Fuel:2}),
  {...witness(execution,'StoppedUnknown',{Fuel:4}),specification:'LateSpec'},
  {...witness(execution,'LateBudgetEvidence',{Fuel:4}),specification:'LateSpec'},
  negative('save-execution-quiescence-grade-control',execution,{QuiescenceAsFailure:true},'Invariant ReceiptGrades is violated'),
  witness(execution,'QuiescentWithoutSave'),
  positive('storage-bridge-safety',storage),
  positive('storage-bridge-unsupported-safety',storage,{HostCapable:false}),
  {...positive('storage-bridge-conflict-script-safety',storage),specification:'ConflictSpec'},
  negative('storage-bridge-false-success-control',storage,{ForgeStored:true},'Invariant AckBackedByWrite is violated'),
  negative('storage-bridge-premature-receipt-control',storage,{FakeEarly:true},'Invariant AckBackedByWrite is violated'),
  {...negative('storage-bridge-ignored-conflict-control',storage,{IgnoreConflict:true},'Invariant BaselineAgreement is violated'),specification:'ConflictSpec'},
  ...['Saved','RaceFailed','PrelaunchRefused','LateStopped','HistoricalBinding','OldSnapshot','MalformedHeld'].map(x=>witness(storage,x)),
  witness(storage,'UnsupportedRefused',{HostCapable:false}),
  {...witness(storage,'ConflictRetained'),specification:'ConflictSpec'},
  positive('recipient-bridge-safety',recipient),
  {...positive('recipient-saved-read-script-safety',recipient),specification:'SavedReadSpec'},
  {...positive('recipient-regrant-gap-script-safety',recipient,{Bound:4}),specification:'GapSpec'},
  negative('recipient-hidden-cache-control',recipient,{LeakHidden:true},'Invariant FunctionalNoninterference is violated'),
  {...negative('recipient-evidence-count-control',recipient,{LeakEvidence:true},'Invariant NoPrivateEvidence is violated'),specification:'SavedReadSpec'},
  negative('recipient-current-authority-control',recipient,{BypassGate:true},'Invariant ReleaseSafe is violated'),
  ...['AuthorizedRead','HiddenUnobservable','SourceDeletionProjected','RevokedReadDenied'].map(x=>witness(recipient,x)),
  {...witness(recipient,'SavedEvidenceErased'),specification:'SavedReadSpec'},
  {...witness(recipient,'QueuedGapDenied',{Bound:4}),specification:'GapSpec'},
  positive('editor-boundary-safety',editor),
  {...positive('editor-sequence-algebra',editor),specification:'AlgebraSpec',
    invariants:['SpliceLaw','InverseLaw','PointLaw','ChunkLaw'],properties:[]},
  {...negative('editor-unmapped-anchor-control',editor,{BadMap:true},'Invariant PointLaw is violated'),
    specification:'AlgebraSpec',invariants:['SpliceLaw','InverseLaw','PointLaw','ChunkLaw'],properties:[]},
  {...positive('editor-batch-composition',editor),specification:'BatchSpec',
    invariants:['BatchComposition','BatchInverse'],properties:[]},
  {...negative('editor-unshifted-batch-control',editor,{BadMap:true},'Invariant BatchComposition is violated'),
    specification:'BatchSpec',invariants:['BatchComposition'],properties:[]},
  negative('editor-scope-bypass-control',editor,{UnsafeScope:true},'Action property ScopeAtPublish is violated'),
  negative('editor-split-state-control',editor,{SplitState:true},'Invariant Coherent is violated'),
  negative('editor-release-bypass-control',editor,{LeakRelease:true},'Invariant SafeRelease is violated'),
  negative('editor-gap-bypass-control',editor,{SkipGap:true},'Invariant GapSafe is violated'),
  negative('editor-event-grant-control',editor,{IgnoreEventGrant:true},'Invariant EventReleaseSafe is violated'),
  negative('editor-event-retention-control',editor,{IgnoreEventRetention:true},'Invariant EventReleaseSafe is violated'),
  negative('editor-discovery-grant-control',editor,{TrustDiscovery:true},'Action property ScopeAtPublish is violated'),
  ...['Committed','ScopeRejected','DeniedAfterDiscovery','Handoff','GapRecovered','RetainedSnapshot'].map(x=>witness(editor,x)),
  positive('derived-retention-safety',derived),
  positive('derived-one-cell-pressure-safety',derived,{Capacity:1}),
  negative('derived-cancel-refund-control',derived,{ForgetOnCancel:true},'Invariant NoDangling is violated'),
  negative('derived-stale-context-control',derived,{IgnoreContext:true},'Invariant CurrentEvidence is violated'),
  negative('derived-stale-revision-control',derived,{IgnoreRevision:true},'Invariant CurrentEvidence is violated'),
  negative('derived-revoked-grant-control',derived,{IgnoreGrant:true},'Invariant CurrentEvidence is violated'),
  {...negative('derived-owner-transfer-control',derived,{TransferOwner:true},'Action property WorkerCannotPublish is violated'),
    invariants:['TypeOK','NoDangling','ExactAccounting','Budget','SnapshotStable','CurrentEvidence']},
  negative('derived-overbudget-control',derived,{Capacity:1,IgnoreCapacity:true},'Invariant Budget is violated'),
  ...['LiveOldAfterEdit','CancelledStillHeld','CompletionReclaims','FreshAccepted',
    'StaleContextHeld','StaleRevisionHeld'].map(x=>witness(derived,x)),
  witness(derived,'CapacityBlocksEdit',{Capacity:1}),
  positive('accounting-imported-admission-safety',accounting),
  {...negative('accounting-missing-admission-control',accounting,{SkipAdmission:true},'Invariant CountConservation is violated'),invariants:['KernelSafety','CountConservation']},
  negative('accounting-double-refund-control',accounting,{DoubleRefund:true},'Invariant CountConservation is violated'),
  negative('accounting-premature-refund-control',accounting,{PrematureRefund:true},'Invariant CountConservation is violated'),
  negative('accounting-wrong-cost-control',accounting,{WrongCost:true},'Invariant CostConservation is violated'),
  ...['StoppedHeld','RefundedOnce','RejectedOffer','LateDischarge'].map(x=>witness(accounting,x)),
  positive('privacy-signal-paired-safety',privacySignals),
  negative('privacy-stable-tag-control',privacySignals,{LeakTag:true},'Invariant FunctionalPrivacy is violated'),
  negative('privacy-hidden-count-control',privacySignals,{LeakCount:true},'Invariant FunctionalPrivacy is violated'),
  negative('privacy-diagnostic-epoch-control',privacySignals,{SkipEpoch:true},'Invariant CurrentDiagnosticRelease is violated'),
  ...['AuthorizedHealth','GapDenied','DisposedDenied','HiddenExistenceMasked'].map(x=>witness(privacySignals,x)),
  ...[
    ['late-regrant','LateRegrantSpec','NoFalseGap','LateRegrantRejected'],
    ['late-disposal','LateDisposeSpec','NoFalseDisposal','LateDisposeRejected'],
    ['late-hidden','LateHiddenSpec','NoFalseMask','LateHiddenRejected'],
    ['denied-mask','DeniedMaskSpec','NoFalseMask','DeniedMaskRejected']
  ].flatMap(([id,specification,guard,target])=>[
    {...positive(`privacy-${id}-regression`,privacySignals),specification,
      invariants:[...privacySignals.invariants,guard]},
    {...witness(privacySignals,target),specification,
      invariants:[...privacySignals.invariants,guard,`Not${target}`]}
  ]),
  positive('useful-fair-drain',useful),
  positive('useful-three-job-drain',useful,{Bound:3}),
  {...negative('useful-no-service-fairness-control',useful,{},'Temporal properties were violated'),specification:'NoFairSpec'},
  negative('useful-silent-drop-control',useful,{SilentDrop:true},'Invariant DispositionPartition is violated'),
  negative('useful-stale-success-control',useful,{AcceptStale:true},'Invariant CurrentAcceptance is violated'),
  negative('useful-cancel-discharge-control',useful,{CancelDischarges:true},'Invariant DispositionPartition is violated'),
  ...['AppliedDone','SupersededDone','CancelledDone','CancelStillPending'].map(x=>({...witness(useful,x),properties:['TerminalStable','ServiceDecreases']}))
];
export const parametricCases = [
  'RANK-BASE','EDIT-LENGTH','EDIT-POINT-DOMAIN','EDIT-SURVIVOR-ROUNDTRIP',
  'EDIT-SCOPE-FRAME','EDIT-COLLAPSED-TARGET-CONTROL',
  'RANK-STEP','RANK-NO-REFILL-CONTROL','LATTICE-LAWS',
  'JOIN-CLOSURE','JOIN-UPPER','JOIN-LEAST','MEET-LOWER','MEET-GREATEST',
  'EMPTY-FAMILY-BOUNDS','PROJECTION-PRESERVATION','HIDDEN-LEAK-CONTROL',
  'CLOSURE-RANK','CONTRACT-COMPOSITION','MISSING-BRIDGE-CONTROL',
  'BRIDGE-SOURCE-FRAME','BRIDGE-COMMENT-FRAME','BRIDGE-PUBLIC-ERASURE',
  'SAVE-CAS-SINGLE-WINNER','SAVE-ORDER-MERGE','SAVE-ABA-CONTROL','SAVE-OPAQUE-AMBIGUITY-CONTROL',
  'TEXT-INVERSE-CONTENT','TEXT-PREFIX-SUFFIX-FRAME','TEXT-BATCH-COMPOSITION',
  'TEXT-UNSHIFTED-BATCH-CONTROL',
  'TEXT-ADJACENT-COMPOSITION','TEXT-ADJACENT-UNSHIFTED-CONTROL',
  'ANCHOR-INSERTION-AFFINITY','ANCHOR-IGNORED-AFFINITY-CONTROL',
  'SNAPSHOT-FUNCTIONAL-FRAME','SNAPSHOT-ALIASED-WRITE-CONTROL',
  'HISTORY-GROUP-ISOLATION','HISTORY-TIMER-ONLY-CONTROL',
  'CONTEXT-COHERENT-ACTIVATION','CONTEXT-PARTIAL-ACTIVATION-CONTROL',
  'TEXT-CHUNK-READ-REFINEMENT','TEXT-CHUNK-OFFSET-CONTROL',
  'REGION-SOURCE-MAP','UTF8-SCALAR-ROUNDTRIP','UTF8-ENCODING-VALIDITY',
  'UTF8-DECODE-SCALAR','UTF8-CANONICAL-REENCODE','UTF8-CONTINUATION-NOT-LEAD',
  'UTF8-SURROGATE-CONTROL','UTF8-OVERLONG-CONTROL','UTF8-OUT-OF-RANGE-CONTROL',
  'LEDGER-CONSERVATION','LEDGER-OMITTED-DISCHARGE-CONTROL',
  'DRAIN-RANK','DRAIN-NO-SERVICE-CONTROL','PRIVACY-PROJECTION','PRIVACY-TAG-CONTROL'
].map(name=>({id:`SMT-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

// General-state local preservation lemmas; see precedent-proofs.md for the
// authored induction arguments and exact premises/refinement boundaries.
export const precedentCases = [
  'SNAPSHOT-PUBLISH','SNAPSHOT-REUSE-CONTROL',
  'BATCH-PREFIX-STEP','BATCH-INVERSE-STEP','BATCH-UNSHIFTED-CONTROL',
  'NAMESPACE-FRAME','NAMESPACE-ID-ONLY-CONTROL',
  'ANCHOR-RANGE-ORDER','ANCHOR-OPPOSED-AFFINITY-CONTROL',
  'ATOMIC-ROOT-READ','ATOMIC-ABORT-FRAME','ATOMIC-TORN-READ-CONTROL',
  'GROUP-APPEND-STEP','GROUP-CLOSE-STABLE','GROUP-PLAN-OMITTED-CONTROL',
  'COMMAND-REJECT-FRAME','COMMAND-SURFACE-EQUIVALENCE','COMMAND-DISCOVERY-AUTHORITY-CONTROL',
  'REGION-INDEPENDENT-FRAME','REGION-CONCATENATION-CONTROL',
  'REGION-UNIQUE-MEMBERSHIP','REGION-SPLICE-SUFFIX',
  'CONFIG-UNRELATED-SETTING-FRAME','CONFIG-DEPENDENCY-FRAME','CONFIG-MISSING-DEPENDENCY-CONTROL',
  'SYNC-EVIDENCE-STEP','SYNC-FAILURE-PRESERVES-WRITE','SYNC-OUTOFORDER-RETAINED','SYNC-PREMATURE-ACK-CONTROL'
].map(name=>({id:`PRE-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

// Conditional resource-profile laws; no numeric/default/provider adoption.
export const resourceCases = [
  'ADMIT-PROFILE','ACTUAL-REFINEMENT','DISCHARGE','TRANSFER','QUOTA-STEP','PRODUCT-LATTICE',
  'MISSING-DIMENSION-CONTROL','EARLY-RELEASE-CONTROL','FANOUT-CONTROL','TRANSFER-FORGET-CONTROL',
  'SPENT-ORIGIN-STEP','SPENT-ORIGIN-OMITTED-CONTROL'
].map(name=>({id:`RES-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const noopCases = [
  'UNCHANGED-FRAME','VALIDATION-GATE','MEANINGFUL-PARTICIPANTS','EFFECT-INTENT',
  'BYPASS-CONTROL','SOURCE-ONLY-CONTROL','EFFECT-DROP-CONTROL','BOOKKEEPING-CONTROL'
].map(name=>({id:`NOP-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const normalizationCases = [
  'TWO-IDEMPOTENT','TWO-PERMUTATION','RECORD-FRAME','ADJACENT-MAP-SEPARATION',
  'INPUT-ORDER-CONTROL','METADATA-SWAP-CONTROL','TIE-ORDER-CONTROL'
].map(name=>({id:`NRM-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const replayCases = [
  'DUPLICATE-FRAME','INTENT-BINDING','RETIRED-FENCE','RELEASE-GATE','AT-MOST-ONCE-STEP',
  'FORGOTTEN-APPLIED-CONTROL','CACHE-MISS-FRESH-CONTROL','RETIREMENT-DISCHARGE-CONTROL','INTENT-MISMATCH-CONTROL'
].map(name=>({id:`RPL-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const mappingCases = [
  'POINT-BOUNDS','SURVIVING-RANGE-ORDER','EXCLUSIVE-BOUNDARIES','NO-AUTOMATIC-RESURRECTION','EXACT-PROPOSAL-GATE',
  'COLLAPSED-OPPOSED-CONTROL','CLAMP-SURVIVAL-CONTROL','MISSING-HISTORY-CONTROL','PREVIEW-AUTHORITY-CONTROL'
].map(name=>({id:`MAP-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const outcomeCases = [
  'PROJECTION-NONINTERFERENCE','FILTER-IDEMPOTENCE','CURRENT-RELEASE-GATE','OBSERVATION-FRAME','RECOVERY-BASIS-GATE',
  'RAW-REASON-CONTROL','HIDDEN-TOP-KIND-CONTROL','UNAVAILABLE-NONAPPLICATION-CONTROL','REASON-RETRY-CONTROL'
].map(name=>({id:`OUT-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const capacityCases = [
  'PROTECTED-PARTITION','ADMISSION-HEADROOM','ACTIVATION-COMMITMENTS','SOFT-NO-GRANT','REFUSAL-FRAME','CONSUMED-ACTIVATION',
  'BORROW-PROTECTED-CONTROL','SHRINK-HELD-CONTROL','ESTIMATE-CEILING-CONTROL','RESET-QUOTA-CONTROL'
].map(name=>({id:`CAP-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const lifecycleCases = [
  'FRESH-INCARNATION','DISPOSED-MUTATION','DISPOSAL-GUARD','DISCHARGE-WITNESS',
  'REUSED-INCARNATION-CONTROL','SOURCE-ONLY-DISPOSAL-CONTROL','CANCEL-REFUND-CONTROL'
].map(name=>({id:`LIFE-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const scopedReleaseCases = [
  'CURRENT-GATE','CAPTURE-MEET','INVALIDATED-CONTEXT','STREAM-CONTINUITY','VISIBLE-COUNT',
  'POSSESSION-GRANT-CONTROL','ENQUEUE-GRANT-CONTROL','GLOBAL-COUNT-CONTROL','REGRANT-RESUME-CONTROL',
  'ANNOTATION-CAPABILITY-CLOSURE','ANNOTATION-INTERSECTION-BOUND','ANNOTATION-DECISION-BINDING',
  'ANNOTATION-STALE-EPOCH','ANNOTATION-CURRENT-ONLY-CONTROL','ANNOTATION-ROW-LOOKUP-CONTROL',
  'HISTORY-PARTICIPANT-BASIS','HISTORY-UNRELATED-PUBLICATION','HISTORY-OBSERVED-INVALIDATION',
  'HISTORY-LIFETIME-ISSUANCE','HISTORY-VISIBLE-ONLY-CONTROL','HISTORY-RESET-ISSUANCE-CONTROL',
  'REVERSAL-TARGET-MEET','REVERSAL-ORIGINAL-SCOPE','REVERSAL-HISTORICAL-DECISION',
  'REVERSAL-KNOWN-NONPUBLICATION','REVERSAL-CURRENT-ONLY-CONTROL','REVERSAL-UNKNOWN-REFUSAL-CONTROL',
  'NATIVE-CONTROL-ISOLATION','NATIVE-CONTROL-RESERVE','NATIVE-WAIT-RANK','NATIVE-WAIT-NONEXECUTION',
  'NATIVE-RICH-PROGRESS-CONTROL','NATIVE-POLL-TIME-CONTROL',
  'SETTING-OBSERVED-GAP-ABSORPTION','SETTING-UNOBSERVED-FRAME',
  'SETTING-REGRANT-CONTROL','SETTING-FOREIGN-INVALIDATION-CONTROL'
].map(name=>({id:`REL-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const baselineCases = [
  'HISTORY-PINS','GROUP-ISOLATION','CAUSAL-PROGRESSION','BOM-INJECTIVE','INHERITED-BUDGET',
  'RETAINED-CONTEXT','NATIVE-SESSION','CUT-GATE','LOSS-SATURATION','COMPLETE-EVIDENCE',
  'CONFIG-NONAMPLIFICATION','ACCEPTANCE-GATE','BOM-AMBIGUITY-CONTROL','DELIVERY-ORDER-CONTROL',
  'LOSS-WRAP-CONTROL','PARTIAL-COMPLETENESS-CONTROL',
  'DIAG-COUNTER-RANGE','DIAG-COUNTER-EXACT','DIAG-ZERO-CAPACITY','DIAG-BYTE-ADMISSION',
  'DIAG-DROP-FRAME','DIAG-DRAIN-REFUSAL','DIAG-AGE-RANK','DIAG-RESET-BOUNDARY',
  'DIAG-WRAP-CONTROL','DIAG-OLD-GRANT-CONTROL',
  'DRAIN-AT-MOST-ONCE','DRAIN-TERMINAL-KNOWLEDGE','DRAIN-RESULT-CEILING',
  'DRAIN-QUEUE-HOLD','DRAIN-LIFETIME-ISSUANCE','DRAIN-RETRY-FRAME',
  'DRAIN-REDRAIN-CONTROL','DRAIN-PARENT-FLAG-CONTROL',
  'SCOPE-INSERTION-INDEPENDENCE','SCOPE-SITE-ENDPOINTS','SCOPE-HOLE-REFUSAL',
  'SCOPE-TOUCHING-COVERAGE','SCOPE-NO-GRANT','SCOPE-REVISION-FENCE',
  'SCOPE-MISSING-NOT-EMPTY','SCOPE-MEET-NONAMPLIFICATION',
  'SCOPE-HULL-WIDENING-CONTROL','SCOPE-IMPLICIT-SITES-CONTROL',
  'MEM-FRESH-VERSIONS','MEM-STALE-COMMAND','MEM-RESTORATION-EXHAUSTION','MEM-FLOOR-PRESERVATION',
  'MEM-RETIRED-ABSORPTION','MEM-RETIREMENT-OBLIGATIONS','MEM-OLD-VERSION-CONTROL','MEM-RETIRED-REUSE-CONTROL',
  'SETTING-COUNTER-RANGE','SETTING-STALE-LAYER','SETTING-ORIGIN-FRAME','SETTING-PARENT-BUDGET',
  'SETTING-ISSUANCE-NONREFUND','SETTING-CLOSED-ABSORPTION','SETTING-OVERFLOW-CONTROL','SETTING-ORIGIN-INVALIDATION-CONTROL',
  'WSREAD-CURRENT','WSREAD-ABA-FENCE','WSREAD-DISJOINT-FRAME','WSREAD-VALUE-ONLY-CONTROL',
  'DEF-AUTHORED-FRAME','DEF-VERSION-RANGE','DEF-FACET-AUTHORITY','DEF-NOOP-EXHAUSTED',
  'DEF-ASSOCIATION-ABA','DEF-TARGET-ABSENCE','DEF-TARGET-ABSORPTION',
  'DEF-VALUE-ONLY-CONTROL','DEF-PACKET-ONLY-CONTROL',
  'LAYER-ISSUER-RANGE','LAYER-FRESH-INCARNATION','LAYER-REMOVE-NONREFUND',
  'LAYER-EXHAUSTION','LAYER-NOOP-FRAME','LAYER-DISJOINT-FRAME',
  'LAYER-CALLER-INCARNATION-CONTROL','LAYER-REFUND-CONTROL'
].map(name=>({id:`BASE-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const catalogCases = [
  'QUERY-ROUTE','UNKNOWN-REFUSAL','CANONICAL-CONGRUENCE','EVIDENCE-OBSERVER-SEPARATION',
  'PROTECTED-RETENTION','DERIVED-CURRENT','PROTECTED-CONTROL-LANE','QUERY-MUTATION-CONTROL','DISCOVERY-GRANT-CONTROL',
  'HISTORICAL-LIVE-GATE-CONTROL'
].map(name=>({id:`CAT-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const ordinaryBindingCases = [
  'KNOWN-DISTINCTION','KNOWN-EQUIVALENCE','UNKNOWN','INCOMPARABLE','CURRENT-PAIR',
  'STALE-DISTINCTION','UNBOUND-ABSENCE','SERIALIZED-PAIR','WRITE-SKEW-CONTROL',
  'LOCATOR-DIFFERENCE-CONTROL','EXPORT-FRAME'
].map(name=>({id:`OD-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));

export const analysisRangeCases = [
  'POSITIVE-INTERSECTION','CLIP-CONTAINMENT','EMPTY-QUERY','OLD-EMPTY-CONTROL'
].map(name=>({id:`AR-${name}`,expected:name.endsWith('CONTROL')?['sat']:['sat','unsat']}));
