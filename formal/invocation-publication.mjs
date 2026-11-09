import {normalizeSelectionArguments,sameSelectionArguments,selectionProducerRequest} from './selection-arguments.mjs';
import {normalizeLifecycleArguments,lifecycleProducerRequest} from './lifecycle-arguments.mjs';
import {normalizeLifecycleActivationArguments,lifecycleActivationProducerRequest} from './lifecycle-activation-arguments.mjs';
import {normalizeReadAcquisitionArguments,readAcquisitionProducerRequest,captureReadAcquisitionLimits,captureReadPreparationBinding,readAcquisitionClassificationBytes,readAcquisitionResultValid} from './read-acquisition-arguments.mjs';
import {normalizeAnnotationQueryArguments,annotationQueryProducerRequest,captureAnnotationQueryLimits,annotationQueryClassificationBytes} from './annotation-query-arguments.mjs';
import {annotationQueryResultValid} from './annotation-queries.mjs';
import {normalizeSourceReadArguments,sourceReadProducerRequest,captureSourceReadArgumentLimits,sourceReadArgumentClassificationBytes} from './source-read-arguments.mjs';
import {sourceReadResultValid} from './source-products.mjs';
import {normalizeSearchArguments,searchProducerRequest,captureSearchArgumentLimits,captureSearchCurrentInputs,searchArgumentClassificationBytes} from './search-arguments.mjs';
import {nativeVersionSetStopTargetOwned,openNativeVersionSetContexts,installNativeVersionSetContext,acquireNativeRetainedVersionSetContext,inspectNativeVersionSetContext,validateNativeVersionSetContext,queryNativeVersionSetMember,stopNativeVersionSetContext,retireNativeVersionSetContext,retireNativeVersionSetContexts,nativeVersionSetContextUsage,nativeVersionSetContextPoolAvailable} from './native-version-set-contexts.mjs';
import {openNativeVersionSetCapture,prepareNativeVersionSetCapture,validateNativeVersionSetCapture,inspectNativeVersionSetCapture,discardNativeVersionSetCapture,retireNativeVersionSetCapture,nativeVersionSetCaptureUsage,nativeVersionSetCutMatchesRequest} from './native-version-set-capture.mjs';
import {openNativeSourceSearch,queryNativeSourceSearch,releaseNativeSourceSearchResult,retireNativeSourceSearchResult,retireNativeSourceSearch,nativeSourceSearchUsage,nativeSourceSearchAllowsPreparation,nativeSourceSearchAllowsSelectionPreparation} from './native-source-search.mjs';
import {nativeSourceStopTargetOwned,openNativeSourceContexts,acquireNativeSourceContext,queryNativeSourceContext,stopNativeSourceContext,holdNativeSourceContext,releaseNativeSourceContextHold,retireNativeSourceContext,retireNativeSourceContexts,nativeSourceContextUsage,nativeSourceContextPoolAvailable} from './native-source-contexts.mjs';
import {normalizeReplacementArguments,sameReplacementArguments,replacementProducerRequest} from './replacement-arguments.mjs';
import {openNativeAnnotationCapture,prepareNativeAnnotationCapture,validateNativeAnnotationCapture,inspectNativeAnnotationCapture,discardNativeAnnotationCapture,retireNativeAnnotationCapture,nativeAnnotationCaptureUsage,nativeAnnotationCutMatchesRequest} from './native-annotation-capture.mjs';
import {nativeAnnotationStopTargetOwned,openNativeAnnotationContexts,installNativeAnnotationContext,acquireNativeRetainedAnnotationContext,inspectNativeAnnotationContext,queryNativeAnnotationContext,queryNativeCombinedSource,stopNativeAnnotationContext,retireNativeAnnotationContext,retireNativeAnnotationContexts,nativeAnnotationContextUsage,nativeAnnotationContextPoolAvailable} from './native-annotation-contexts.mjs';
import {prepareReplacement,discardReplacement,replacementResultFootprint} from './literal-search.mjs';
import {normalizeOrdinaryEditArguments,sameOrdinaryEditArguments,ordinaryEditProducerRequest} from './ordinary-edit-arguments.mjs';
import {normalizeCompositionArguments,sameCompositionArguments} from './composition-arguments.mjs';
import {normalizeExactBatchArguments,sameExactBatchArguments} from './exact-batch-arguments.mjs';
import {normalizeAtomicAuthoringArguments} from './atomic-authoring-arguments.mjs';
import {normalizeNativeLocalCommandArguments,captureNativeLocalCommandLimits} from './native-local-command-arguments.mjs';
import {prepareAnnotationEdit} from './annotation-editing.mjs';
import {prepareAtomicAuthoringIntent,atomicAuthoringIntentCurrent,atomicAuthoringCandidateFits,atomicAuthoringMembershipChecks} from './atomic-authoring-intent.mjs';
import {prepareAtomicAnnotationResultHold,atomicAnnotationResultPlanMatches,atomicAnnotationResultHoldUsage,stageAtomicAnnotationResults,releaseAtomicAnnotationResult,retireAtomicAnnotationResultHold} from './atomic-annotation-results.mjs';
import {validateCanonicalBatch} from './text-oracles.mjs';
import {copySettingsData} from './semantic-settings.mjs';
import {prepareSelectionEdit,prepareCompositionCommit,discardSelectionEdit} from './selection-products.mjs';
import {prepareLineEdit,discardLineEdit} from './line-edits.mjs';
import {prepareBoundaryCommand,discardBoundaryCommand} from './boundary-commands.mjs';
import {captureNativeEngineSettingsLifecycleBasis,inspectNativeEngineSettingsLifecycleBasis,resolveNativeEngineSettingsLifecycleBasis,stopNativeEngineSettingsLifecycleBasis,releaseNativeEngineSettingsLifecycleBasis} from './engine-settings-state.mjs';
import {prepareScopedNativeEngineSettingsLifecycle} from './engine-settings-state.mjs';
import {prepareBasisNativeEngineSettingsLifecycle} from './engine-settings-state.mjs';
import {prepareConfiguredLine,forgetConfiguredLine} from './configured-line-state.mjs';
import {prepareSourceSerialization,forgetSourceSerialization} from './source-serialization-state.mjs';
import {openSerializationBasisPool,nativeSerializationBasisPoolHandle,captureSerializationBasis,inspectSerializationBasis,resolveSerializationBasis,stopSerializationBasis,releaseSerializationBasis,retireSerializationBasisPool,serializationBasisUsage} from './source-serialization-state.mjs';
import {prepareScopedSourceSerialization} from './source-serialization-state.mjs';
import {captureNativeEngineSettingsWriteBasis,inspectNativeEngineSettingsWriteBasis,resolveNativeEngineSettingsWriteBasis,stopNativeEngineSettingsWriteBasis,releaseNativeEngineSettingsWriteBasis} from './engine-settings-state.mjs';
import {prepareScopedNativeEngineSettings} from './engine-settings-state.mjs';
import {prepareScopedNativeEngineSettingsDefinition} from './engine-settings-state.mjs';
import {captureNativeEngineSettingsDefinitionBasis,inspectNativeEngineSettingsDefinitionBasis,resolveNativeEngineSettingsDefinitionBasis,stopNativeEngineSettingsDefinitionBasis,releaseNativeEngineSettingsDefinitionBasis} from './engine-settings-state.mjs';
// Closed normalized local jobs. Not a public token codec or production dispatcher.
import {reserveCreatedSettingsResult,admitCreatedSettingsResult,stageCreatedSettingsPublication,commitCreatedSettingsPublication,discardCreatedSettingsPublication,releaseCreatedSettingsResult,captureCreatedSettingsWriteBasis,dischargeCreatedSettingsResult,createdSettingsResultUsage} from './settings-created-results.mjs';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {publishAnnotationPlan,decideAnnotationPlan,compoundPlanShape} from './annotation-compounds.mjs';
import {isReadDependencyPlan,normalizeReadDependencyPlan} from './read-dependencies.mjs';
import {prepareReversal,publishReversal,admitReversalResult,captureReversalResultReceipt,releaseReversalResult} from './history-reversal.mjs';
import {prepareForwardViews,validateReversalViews,completeForwardViews as publishForwardViews,decideForwardViews} from './history-views.mjs';
import {carrySourceJournal,carryBaselineSourceJournal,carryRestoredAnnotationSourceJournal} from './parser-source.mjs';
import {prepareOutcome,releaseOutcome} from './outcome-projection.mjs';
import {captureEditingIntent,publishEditingIntent,admitAnnotationResult,captureAnnotationEditResult,releaseAnnotationResult} from './editing-intents.mjs';
import {prepareNativeEngineSettings,forgetNativeEngineSettings,acquireNativeEngineSettingsContext,readNativeEngineSettingsContext,releaseNativeEngineSettingsContext,retireNativeEngineSettingsPool,nativeEngineSettingsUsage,nativeEngineSettingsPoolHandle} from './engine-settings-state.mjs';
import {nativeSettingsStopTargetOwned,queryDecodedNativeSettingsRead,nativeSettingsReadDecodeUsage,openNativeSettingsReads,prepareNativeSettingsRead,acquireNativeSettingsRead,queryNativeSettingsRead,discardNativeSettingsReadCut,stopNativeSettingsRead,retireNativeSettingsRead,retireNativeSettingsReads,nativeSettingsReadUsage} from './engine-settings-reads.mjs';
import {clipboardInvocationIntent,claimClipboardInvocation,releaseClipboardInvocation,clipboardInvocationEffect,stageClipboardInvocation,discardClipboardStage,validateClipboardStage,commitClipboardStage,faultClipboardInvocation} from './clipboard-products.mjs';
import {annotationRestorationCandidate,discardAnnotationRestoration} from './annotation-restoration-state.mjs';
import {annotationLoadedBaselineGates,readLoadedAnnotationBaseline,completeLoadedAnnotationBaseline} from './annotation-loaded-baseline-state.mjs';
import {readNativeRetainedLifecycle,consumeNativeRetainedLifecycle} from './retained-lifecycle-state.mjs';
import {readSourceDestinationTransition,commitSourceDestinationTransition} from './source-destination-state.mjs';
import {readNativeSourceRead,commitNativeSourceRead} from './source-read-state.mjs';
import {prepareNativeResourceRegistration,readNativeResourceRegistration,commitNativeResourceRegistration,discardNativeResourceRegistration} from './resource-registration-state.mjs';
import {prepareNativeRelationLifecycle,readNativeRelationLifecycle,commitNativeRelationLifecycle,discardNativeRelationLifecycle} from './relation-lifecycle-state.mjs';
import {prepareRelationInvocation,captureRelationInvocation,stageRelationInvocation,commitRelationInvocation,discardRelationInvocationStage,releaseRelationInvocationResult,forgetRelationInvocation} from './relation-invocation-state.mjs';
import {prepareLifecycleInvocation,captureLifecycleInvocation,stageLifecycleInvocation,commitLifecycleInvocation,discardLifecycleInvocationStage,releaseLifecycleInvocationResult,forgetLifecycleInvocation} from './lifecycle-invocation-state.mjs';
import {prepareSaveAsInvocation} from './saveas-invocation-state.mjs';
import {prepareLoadInvocation} from './load-invocation-state.mjs';
import {prepareAnnotationLoadInvocation} from './annotation-load-invocation-state.mjs';
import {prepareScopedRegistrationInvocation,registrationInvocationIsScoped,releaseScopedRegistrationInvocationResult,prepareRegistrationInvocation,captureRegistrationInvocation,stageRegistrationInvocation,commitRegistrationInvocation,discardRegistrationInvocationStage,releaseRegistrationInvocationResult,forgetRegistrationInvocation} from './registration-invocation-state.mjs';
import {carryRetainedLifecycleSourceJournal,carryScratchSourceJournal} from './parser-source.mjs';
import {openBaselineReservationPool,reserveBaselineCapacity,reserveBaselineWriteCapacity,baselineReservationsAllow,releaseBaselineCapacity,stopBaselineReservations,baselineReservationUsage,stageAnnotationBaselineInstallation,commitAnnotationBaselineInstallation,prepareOrdinaryBaselineCapacity,validateOrdinaryBaselineCapacity,commitOrdinaryBaselineCapacity,discardOrdinaryBaselineCapacity,releaseIneligibleBaselineCapacity} from './baseline-reservations.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./invocation-publication.json',import.meta.url))),domains=new WeakMap(),exposedRootOrigins=new WeakMap(),copy=structuredClone;
const natural=n=>Number.isSafeInteger(n)&&n>=0,exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort()),size=v=>Buffer.byteLength(JSON.stringify(v));
const label=s=>typeof s==='string'&&s.length>0&&new TextDecoder('utf-8',{fatal:true}).decode(Buffer.from(s))===s;
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;},frozen=v=>!v||typeof v!=='object'||Object.isFrozen(v)&&Object.values(v).every(frozen);
const flags=['publication','control','effect','reasons','details','recovery'];
const contextValid=c=>exact(c,catalog.contextFields)&&Object.values(c).every(label);
const lifecycleActivationPortFields=Object.freeze(Object.fromEntries([
 ['source-load',['authority']],['annotation-restore',['authority','inspection']],
 ['annotation-baseline',['authority','inspection']],['source-saveas',['authority']],
 ['source-rebind',['inspection']],['view-container-create',[]],['view-container-retire',[]],
 ['workspace-container-create',[]],['workspace-container-retire',[]]
].map(([key,value])=>[key,Object.freeze(value)])));
// Three bounded capture frames (envelope, context, canonical record), plus
// one producer reconstruction frame. Existing native retained-plan funds remain
// separate. These declared serialized units are not JS/RSS accounting.
function beginLifecycleEnvelopeWorkspace(o){const units=4*o.limits.maxIntentBytes;if(!natural(units)||units>o.limits.maxHeldBytes-o.charge)return null;o.charge+=units;return units;}
function endLifecycleEnvelopeWorkspace(o,units){if(units!==null)o.charge-=units;}
function captureLifecycleActivationPorts(input,form){
 const expected=lifecycleActivationPortFields[form];if(!expected||!input||typeof input!=='object'||Array.isArray(input)||![Object.prototype,null].includes(Object.getPrototypeOf(input)))return null;
 const descriptors=Object.getOwnPropertyDescriptors(input),keys=Reflect.ownKeys(descriptors);
 if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(descriptors[k],'value')||descriptors[k].enumerable!==true)||!equal(keys.toSorted(),expected.toSorted()))return null;
 return Object.freeze(Object.fromEntries(keys.map(k=>[k,descriptors[k].value])));
}
// Fixed existing native ports: these handles and host facts are not JSON arguments.
export function prepareInvocationLifecycleActivationEnvelope(domain,ticket,envelope,inputLimits,profile,ports,c,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket);
 if(!o||!entry||o.faulted||entry.ordinaryPrepared||entry.ordinaryPreparing||entry.phase!=='issued')return null;
 entry.ordinaryPreparing=true;
 let workspace=null;
 try{
  workspace=beginLifecycleEnvelopeWorkspace(o);if(workspace===null)return null;
  // Lifecycle activation envelope capture checkpoint.
  const n=normalizeLifecycleActivationArguments(envelope,c,inputLimits,o.limits.maxIntentBytes);
  if(!n||!entryCurrent(o,entry,n.record.context)||n.record.request.invocation!==entry.reference||!gates(g,['outsidePublicationGuard','preparationReserved']))return null;
  const bound=captureLifecycleActivationPorts(ports,n.form);if(!bound)return null;
  const request=lifecycleActivationProducerRequest(n);let plan=null,kind='lifecycle';
  // Lifecycle activation envelope native checkpoint.
  switch(n.form){
   case 'source-load':plan=prepareLoadIntent(o,domain,ticket,entry,request,profile,bound.authority,n.record.context,g);break;
   case 'annotation-restore':case 'annotation-baseline':plan=prepareAnnotationLoadIntent(o,domain,ticket,entry,request,profile,bound.authority,bound.inspection,n.record.context,g);break;
   case 'source-saveas':plan=prepareSaveAsIntent(o,domain,ticket,entry,request,profile,bound.authority,n.record.context,g);break;
   case 'source-rebind':plan=prepareLifecycleIntent(o,domain,ticket,entry,request,profile,bound.inspection,n.record.context,g);break;
   case 'view-container-create':case 'view-container-retire':case 'workspace-container-create':case 'workspace-container-retire':kind='relation';plan=prepareRelationIntent(o,domain,ticket,entry,request,profile,n.record.context,g);break;
   default:return null;
  }
  if(!plan)return null;
  // Lifecycle activation envelope release checkpoint.
  return Object.freeze({job:Object.freeze({kind,plan}),route:n.route});
 }catch{o.faulted=true;return null;}finally{endLifecycleEnvelopeWorkspace(o,workspace);entry.ordinaryPreparing=false;}
}
// Selected data-envelope adapter. Profile, writer inspection and host facts are
// existing trusted native ports, not a JSON ingress schema or copied capabilities.
export function prepareInvocationLifecycleEnvelope(domain,ticket,envelope,inputLimits,profile,inspection,c,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket);
 if(!o||!entry||o.faulted||entry.ordinaryPrepared||entry.ordinaryPreparing||entry.phase!=='issued')return null;
 entry.ordinaryPreparing=true;
 let workspace=null;
 try{
  workspace=beginLifecycleEnvelopeWorkspace(o);if(workspace===null)return null;
  // Lifecycle envelope capture checkpoint.
  const n=normalizeLifecycleArguments(envelope,c,inputLimits,o.limits.maxIntentBytes);
  if(!n||!entryCurrent(o,entry,n.record.context)||n.record.request.invocation!==entry.reference||!gates(g,['outsidePublicationGuard','preparationReserved']))return null;
  // Lifecycle envelope native checkpoint.
  const prepared=prepareLifecycleIntent(o,domain,ticket,entry,lifecycleProducerRequest(n),profile,inspection,n.record.context,g);
  // Lifecycle envelope release checkpoint.
  return prepared;
 }catch{o.faulted=true;return null;}finally{endLifecycleEnvelopeWorkspace(o,workspace);entry.ordinaryPreparing=false;}
}
const policyShape=p=>exact(p,[...flags,'grant','epoch','recipient'])&&flags.every(k=>typeof p[k]==='boolean')&&typeof p.grant==='boolean'&&natural(p.epoch)&&label(p.recipient);
const policyValid=(p,c)=>policyShape(p)&&p.details===false&&p.grant===true&&p.recipient===c.recipient;
const current=(o,c)=>o&&contextValid(c)&&catalog.domainFields.every(k=>equal(o.context[k],c[k]));
const entryCurrent=(o,entry,c)=>current(o,c)&&entry&&equal(entry.context,c);
const gates=(g,ks)=>ks.every(k=>g?.[k]===true),unavailable=()=>({status:'unavailable'});
const facts=(publication,control,reference,effect='none')=>({publication,control,effect,basis:'none',causes:[],participants:[],publicCorrelation:reference});
const terminals=[['applied','done'],['unchanged','done'],['notApplied','rejected'],['notApplied','stopped'],['unknown','faulted']];
// Qualified internal local-command forms; recipient projection is independent.
export function invocationTerminalForms(){return Object.freeze(terminals.map(([publication,control])=>Object.freeze({publication,control})));}
function resultFits(o,reference,p,effects=['none']){
 for(const effect of effects)for(const [publication,control] of terminals){const prepared=prepareOutcome(facts(publication,control,reference,effect),p);if(size(prepared)>o.limits.maxResultBytes)return false;
  for(let mask=0;mask<32;mask++){const q={...p};flags.filter(k=>k!=='details').forEach((k,i)=>{q[k]=!!(mask&(1<<i));});if(size({status:'outcome',outcome:releaseOutcome(prepared,q)})>o.limits.maxResultBytes)return false;}
 }return size({status:'outcome',outcome:releaseOutcome(null,p)})<=o.limits.maxResultBytes;
}
export function openInvocationDomain(root,context,limits,g){
 if(!contextValid(context)||!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||!gates(g,catalog.admissionGates)||!g.uniqueDomainAttested||root?.authority!==context.authority||!natural(root.publication)||size(root)>limits.maxRootBytes)return null;
 // Declared before/candidate roots, owned intent/reference/scope and policy/result/frame maxima.
 const charge=2*limits.maxRootBytes+size(context)+limits.maxSlots*(4*limits.maxIntentBytes+5*limits.maxResultBytes);if(!natural(charge)||charge>limits.maxHeldBytes)return null;
 const domain=Object.freeze({kind:'invocation-domain/1'}),ownedRoot=freeze(copy(root));carrySourceJournal(root,ownedRoot);domains.set(domain,{context:copy(context),limits:copy(limits),root:ownedRoot,issued:0,starts:0,slots:new Map(),faulted:false,charge});return domain;
}
export function issueInvocation(domain,c,g){
 const section=beginInvocationAdmissionMutation(domain);if(!section)return null;try{
 const o=domains.get(domain);if(!current(o,c)||o.faulted||o.admissionFence||!gates(g,catalog.admissionGates)||o.slots.size>=o.limits.maxSlots||o.issued>=o.limits.maxIssued)return null;
 const reference=`${o.context.engine}:invocation:${o.issued+1}`,ticket=Object.freeze({kind:'invocation-ticket/1'});if(size(reference)>o.limits.maxIntentBytes||size(c)>o.limits.maxIntentBytes)return null;
 o.issued++;o.slots.set(ticket,{reference,context:freeze(copy(c)),phase:'issued',intent:null,plan:null,prepared:null,annotationCeiling:null,annotationReceipt:null,reversalCeiling:null,reversalReceipt:null,relationPrepared:null,relationReceipt:null,relationCeiling:false,lifecyclePrepared:null,lifecycleReceipt:null,lifecycleCeiling:false,registrationPrepared:null,registrationReceipt:null,registrationCeiling:false,stop:false});return {ticket,reference};
 }finally{endInvocationAdmissionMutation(domain,section);}
}
// Fixed first-party resource-catalog funding ports, not facade commands or a
// generic allocator. The catalog keeps the returned lease private. One slot per
// domain preserves its existing single-registry, non-reopening lifetime.
export function reserveInvocationRegistrationReadCharge(domain,units,g){
 const o=domains.get(domain);
 if(!o||o.faulted||o.registrationReadFunding||!natural(units)||!gates(g,['ownerControlAllowed','resourcesReady','preparationReserved'])||units>o.limits.maxHeldBytes-o.charge)return null;
 const lease=Object.freeze({kind:'registration-read-funding/1'});
 o.charge+=units;o.registrationReadFunding={lease,units,held:true,phase:'idle',temporary:0};return lease;
}
export function releaseInvocationRegistrationReadCharge(domain,lease,g){
 const o=domains.get(domain),f=o?.registrationReadFunding;
 if(!f||!f.held||f.lease!==lease||f.phase!=='idle'||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;
 o.charge-=f.units;f.held=false;return true;
}
// Two bounded decoder reservations on that same private funding lease. Neither
// completion nor transient refund discharges the native registry reservation.
export function beginInvocationRegistrationReadDecode(domain,lease,units){
 const o=domains.get(domain),f=o?.registrationReadFunding;
 if(!o||o.faulted||!f?.held||f.lease!==lease||f.phase!=='idle'||!natural(units)||units>o.limits.maxHeldBytes-o.charge)return null;
 o.charge+=units;f.temporary=units;f.phase='classification';return o.limits.maxIntentBytes;
}
export function extendInvocationRegistrationReadDecode(domain,lease,units){
 const o=domains.get(domain),f=o?.registrationReadFunding;
 if(!f?.held||f.lease!==lease||f.phase!=='classification'||!natural(units)||units>o.limits.maxHeldBytes-o.charge)return false;
 o.charge+=units;f.temporary+=units;f.phase='workspace';return true;
}
export function finishInvocationRegistrationReadDecode(domain,lease){
 const o=domains.get(domain),f=o?.registrationReadFunding;
 if(!f?.held||f.lease!==lease||!['classification','workspace'].includes(f.phase))return false;
 o.charge-=f.temporary;f.temporary=0;f.phase='idle';return true;
}
export function prepareInvocationLifecycleIntent(domain,ticket,request,profile,inspection,c,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;return prepareLifecycleIntent(o,domain,ticket,entry,request,profile,inspection,c,g);}
function prepareLifecycleIntent(o,domain,ticket,entry,request,profile,inspection,c,g){if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.registrationPrepared||entry.relationPrepared||entry.settingsPrepared||entry.configuredLinePrepared||!gates(g,catalog.admissionGates))return null;if(entry.lifecyclePrepared){const captured=captureLifecycleInvocation(entry.lifecyclePrepared,o,domain,ticket,o.limits.maxIntentBytes);return captured&&captured.inspection===inspection&&equal(captured.value,{request,profile})?entry.lifecyclePrepared:null;}const plan=prepareLifecycleInvocation(o,domain,ticket,entry.reference,o.root,request,profile,inspection,g);if(plan)entry.lifecyclePrepared=plan;return plan;}
export function prepareInvocationSaveAsIntent(domain,ticket,request,profile,authority,c,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;return prepareSaveAsIntent(o,domain,ticket,entry,request,profile,authority,c,g);}
function prepareSaveAsIntent(o,domain,ticket,entry,request,profile,authority,c,g){if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.registrationPrepared||entry.relationPrepared||entry.settingsPrepared||entry.configuredLinePrepared||!gates(g,catalog.admissionGates))return null;if(entry.lifecyclePrepared){const captured=captureLifecycleInvocation(entry.lifecyclePrepared,o,domain,ticket,o.limits.maxIntentBytes);return captured&&captured.inspection===authority&&equal(captured.value,{request,profile})?entry.lifecyclePrepared:null;}const plan=prepareSaveAsInvocation(o,domain,ticket,entry.reference,o.root,request,profile,authority,g);if(plan)entry.lifecyclePrepared=plan;return plan;}
export function prepareInvocationLoadIntent(domain,ticket,request,profile,authority,c,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;return prepareLoadIntent(o,domain,ticket,entry,request,profile,authority,c,g);}
function prepareLoadIntent(o,domain,ticket,entry,request,profile,authority,c,g){if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.registrationPrepared||entry.relationPrepared||entry.settingsPrepared||entry.configuredLinePrepared||!gates(g,catalog.admissionGates))return null;if(entry.lifecyclePrepared){const captured=captureLifecycleInvocation(entry.lifecyclePrepared,o,domain,ticket,o.limits.maxIntentBytes);return captured&&captured.inspection===authority&&equal(captured.value,{request,profile})?entry.lifecyclePrepared:null;}const plan=prepareLoadInvocation(o,domain,ticket,entry.reference,o.root,request,profile,authority,g);if(plan)entry.lifecyclePrepared=plan;return plan;}
export function prepareInvocationAnnotationLoadIntent(domain,ticket,request,profile,authority,inspection,c,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;return prepareAnnotationLoadIntent(o,domain,ticket,entry,request,profile,authority,inspection,c,g);}
function prepareAnnotationLoadIntent(o,domain,ticket,entry,request,profile,authority,inspection,c,g){if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.registrationPrepared||entry.relationPrepared||entry.settingsPrepared||entry.configuredLinePrepared||!gates(g,catalog.admissionGates))return null;if(entry.lifecyclePrepared){const captured=captureLifecycleInvocation(entry.lifecyclePrepared,o,domain,ticket,o.limits.maxIntentBytes);return captured&&captured.inspection===authority&&captured.companion===inspection&&equal(captured.value,{request,profile})?entry.lifecyclePrepared:null;}const plan=prepareAnnotationLoadInvocation(o,domain,ticket,entry.reference,o.root,request,profile,authority,inspection,g);if(plan)entry.lifecyclePrepared=plan;return plan;}
export function prepareInvocationRelationIntent(domain,ticket,request,profile,c,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;return prepareRelationIntent(o,domain,ticket,entry,request,profile,c,g);}
function prepareRelationIntent(o,domain,ticket,entry,request,profile,c,g){if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.registrationPrepared||entry.lifecyclePrepared||entry.settingsPrepared||entry.configuredLinePrepared||!gates(g,catalog.admissionGates))return null;if(entry.relationPrepared){const captured=captureRelationInvocation(entry.relationPrepared,o,domain,ticket,o.limits.maxIntentBytes);return captured&&equal(captured.value,{request,profile})?entry.relationPrepared:null;}const plan=prepareRelationInvocation(o,domain,ticket,entry.reference,o.root,request,profile,g);if(plan)entry.relationPrepared=plan;return plan;}
export function prepareInvocationRegistrationIntent(domain,ticket,request,profile,c,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.lifecyclePrepared||entry.relationPrepared||entry.settingsPrepared||entry.configuredLinePrepared||!gates(g,catalog.admissionGates))return null;if(entry.registrationPrepared){const captured=captureRegistrationInvocation(entry.registrationPrepared,o,domain,ticket,o.limits.maxIntentBytes);return captured&&equal(captured.value,{request,profile})?entry.registrationPrepared:null;}const plan=prepareRegistrationInvocation(o,domain,ticket,entry.reference,o.root,request,profile,g);if(plan)entry.registrationPrepared=plan;return plan;}
export function prepareInvocationScopedRegistrationIntent(domain,ticket,request,profile,basisOwner,bases,scope,c,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.lifecyclePrepared||entry.relationPrepared||entry.settingsPrepared||entry.configuredLinePrepared||!gates(g,catalog.admissionGates))return null;const plan=prepareScopedRegistrationInvocation(o,domain,ticket,entry.reference,o.root,request,profile,basisOwner,bases,scope,c,g,entry.registrationPrepared);if(plan)entry.registrationPrepared=plan;return plan;}
export function prepareInvocationSemanticSettings(domain,ticket,envelope,profile,c,h){
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;
 if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.lifecyclePrepared||entry.relationPrepared||entry.registrationPrepared||entry.configuredLinePrepared||!gates(h,catalog.admissionGates))return null;
 const plan=withAttachedNativeSettingsPool(domain,()=>prepareNativeEngineSettings(o,domain,ticket,entry.reference,o.root,envelope,c,profile,h,entry.settingsPrepared),!entry.settingsPrepared);if(plan)entry.settingsPrepared=plan;return plan;
}
// Trusted engine-profile consumers; these are not recipient F07 projections.
export function prepareInvocationSourceSerialization(domain,ticket,envelope,profile,c,h){
 const admissionSection=beginInvocationAdmissionMutation(domain,Boolean(domains.get(domain)?.slots.get(ticket)?.settingsPrepared));if(!admissionSection)return null;try{const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.lifecyclePrepared||entry.relationPrepared||entry.registrationPrepared||entry.configuredLinePrepared||!gates(h,catalog.admissionGates))return null;const plan=prepareSourceSerialization(o,domain,ticket,entry.reference,o.root,envelope,c,profile,h,entry.settingsPrepared);if(plan)entry.settingsPrepared=plan;return plan;
 }finally{endInvocationAdmissionMutation(domain,admissionSection);}
}

// These consumers use the independent configuration owner profile.
export function acquireInvocationSemanticContext(domain,c,profile,g){const o=domains.get(domain);return current(o,c)?withAttachedNativeSettingsPool(domain,()=>acquireNativeEngineSettingsContext(o,domain,c,profile,g),true):null;}
export function readInvocationSemanticContext(domain,handle,c,g){const o=domains.get(domain);return current(o,c)?readNativeEngineSettingsContext(o,domain,handle,c,g):null;}
export function releaseInvocationSemanticContext(domain,handle,c,g){const o=domains.get(domain);return current(o,c)?releaseNativeEngineSettingsContext(o,domain,handle,c,g):false;}
export function retireInvocationSemanticSettingsPool(domain,g){const o=domains.get(domain);return retireAttachedNativePool(domain,o?nativeEngineSettingsPoolHandle(o,domain):null,'settings',g,(owner,facts)=>retireNativeEngineSettingsPool(owner,domain,facts));}
export function invocationSemanticSettingsUsage(domain){const o=domains.get(domain);return o?nativeEngineSettingsUsage(o):null;}
// Shared admission meaning for the existing finite compound vocabulary. This
// groups consistent participant headers and orders complete edit records;
// source/context/expected-byte truth remains
// final native validation. Private evaluators continue to require ordered edits.
function normalizeCompoundAdmission(plan,maximum){
 if(!natural(maximum)||!compoundPlanShape(plan)||size(plan)>maximum)return null;
 const readPlan=isReadDependencyPlan(plan),n=readPlan?normalizeReadDependencyPlan(plan,maximum):copy(plan);
 if(!n)return null;
 const dense=a=>Array.isArray(a)&&Object.keys(a).length===a.length&&Object.keys(a).every((k,i)=>k===String(i));
 if(!dense(n.sources))return null;
 const participants=[],byDocument=new Map();
 for(const source of n.sources){
  if(!exact(source,['document','expectedSource','expectedAnnotationSet','edits'])||!dense(source.edits))return null;
  if(!source.edits.every(e=>exact(e,['lo','hi','expected','ins'])&&natural(e.lo)&&natural(e.hi)&&e.lo<=e.hi))return null;
  if(!exact(source.document,['authority','id','incarnation'])||!label(source.document.authority)||!label(source.document.id)||!natural(source.document.incarnation))return null;
  const key=JSON.stringify([source.document.authority,source.document.id,source.document.incarnation]),prior=byDocument.get(key);
  if(prior){
   if(prior.expectedSource!==source.expectedSource||prior.expectedAnnotationSet!==source.expectedAnnotationSet)return null;
   for(const edit of source.edits)prior.edits.push(edit);
  }else{const owned={...source,edits:source.edits.slice()};byDocument.set(key,owned);participants.push(owned);}
 }
 n.sources=participants;
 for(const source of n.sources){
  source.edits.sort((a,b)=>a.lo-b.lo);
  for(let i=1;i<source.edits.length;i++){
   const a=source.edits[i-1],b=source.edits[i];
   if(a.hi>b.lo||a.hi===b.lo&&(a.lo===a.hi||b.lo===b.hi))return null;
  }
 }
 return n;
}
export function submitInvocation(domain,ticket,job,route,c,p,g){
 const section=beginInvocationAdmissionMutation(domain,true);if(!section)return unavailable();try{
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPreparing)return unavailable();if(!entryCurrent(o,entry,c)||o.faulted||!catalog.routes.includes(route)||!gates(g,catalog.admissionGates)||!exact(job,['kind','plan'])||!catalog.jobKinds.includes(job.kind)||size(job)>o.limits.maxIntentBytes)return unavailable();
 if(o.admissionFence&&entry.phase==='issued')return unavailable();
 const ordinaryOwner=ordinaryPlans.get(job.plan);if(ordinaryOwner&&(ordinaryOwner.retired||ordinaryOwner.domain!==domain||ordinaryOwner.ticket!==ticket||entry.ordinaryPrepared?.job.plan!==job.plan))return unavailable();
 if(entry.ordinaryPrepared&&(job.kind!==entry.ordinaryPrepared.job.kind||job.plan!==entry.ordinaryPrepared.job.plan))return unavailable();
 if(entry.configuredLinePrepared&&(job.kind!=='line'||job.plan!==entry.configuredLinePrepared))return unavailable();
 if(entry.settingsPrepared&&(job.kind!=='settings'||job.plan!==entry.settingsPrepared)||entry.relationPrepared&&(job.kind!=='relation'||job.plan!==entry.relationPrepared)||entry.lifecyclePrepared&&(job.kind!=='lifecycle'||job.plan!==entry.lifecyclePrepared)||entry.registrationPrepared&&(job.kind!=='registration'||job.plan!==entry.registrationPrepared))return unavailable();
 const clipboard=job.kind==='clipboard',relation=job.kind==='relation',lifecycle=job.kind==='lifecycle',registration=job.kind==='registration',editing=catalog.ownedEditingKinds.includes(job.kind)||clipboard||relation||lifecycle||registration,captured=registration?captureRegistrationInvocation(job.plan,o,domain,ticket,o.limits.maxIntentBytes):lifecycle?captureLifecycleInvocation(job.plan,o,domain,ticket,o.limits.maxIntentBytes):relation?captureRelationInvocation(job.plan,o,domain,ticket,o.limits.maxIntentBytes):clipboard?clipboardInvocationIntent(job.plan,o.limits.maxIntentBytes):editing?captureEditingIntent(job.kind,job.plan,o.limits.maxIntentBytes,o,domain,ticket):null;
 if(!editing&&!['compound','reversal'].includes(job.kind))return unavailable();
 if(editing&&!captured)return unavailable();
 const compoundNormalized=job.kind==='compound'?normalizeCompoundAdmission(job.plan,o.limits.maxIntentBytes):null;
 if(job.kind==='compound'&&!compoundNormalized)return unavailable();
 const normalized=editing?{kind:job.kind,plan:captured.value}:job.kind==='compound'?{kind:job.kind,plan:compoundNormalized}:job;if(size(normalized)>o.limits.maxIntentBytes)return unavailable();
 if(entry.phase!=='issued')return equal(entry.intent,normalized)&&(!editing||entry.plan===job.plan)?{status:entry.phase==='pending'?'pending':'observe-existing'}:unavailable();
 const bound=editing?captured.invocation:job.kind==='compound'?job.plan?.invocation:job.plan?.request?.invocation;
 if(bound!==entry.reference||job.kind==='reversal'&&!frozen(job.plan)||!policyValid(p,c)||(clipboard?!resultFits(o,entry.reference,p,['confirmed','unknown']):!resultFits(o,entry.reference,p)))return unavailable();
 const intent=freeze(copy(normalized)),plan=job.kind==='compound'?intent.plan:job.plan,capturedPolicy=freeze(copy(p));
 if(clipboard&&!claimClipboardInvocation(plan,ticket))return unavailable();
 const annotationCeiling=job.kind==='annotation'?admitAnnotationResult(plan,c,g,o.limits.maxResultBytes):null;if(job.kind==='annotation'&&!annotationCeiling)return unavailable();
 const reversalCeiling=job.kind==='reversal'?admitReversalResult(plan,c,g,o.limits.maxResultBytes):null;if(job.kind==='reversal'&&!reversalCeiling)return unavailable();
 if(entry.settingsCreatedHold&&!admitCreatedSettingsResult(entry.settingsCreatedHold,o,domain,ticket,c,g))return unavailable();
 entry.intent=intent;entry.plan=plan;entry.policy=capturedPolicy;entry.registrationCeiling=registration&&(registrationInvocationIsScoped(plan)?g.scopedRegistrationResultAllowed===true:g.registrationHostResultAllowed===true);entry.relationCeiling=relation&&g.relationResultIdentifiersAllowed===true;entry.lifecycleCeiling=lifecycle&&g.lifecycleResultIdentifiersAllowed===true;entry.annotationCeiling=annotationCeiling;entry.reversalCeiling=reversalCeiling;entry.phase='pending';return {status:'accepted'};
 }finally{endInvocationAdmissionMutation(domain,section);}
}
function compound(state,plan,host,readContext,advisory=false){
 const completeForwardViews=advisory?decideForwardViews:publishForwardViews;
 const refuse=()=>({status:'not-published',state}),parts=[];
 if(!Array.isArray(plan.sources)||plan.sources.length>host.maxParticipants)return refuse();
 const charges={batches:0,edits:0,bytes:0,planBytes:0},keys={batches:'maxViewBatches',edits:'maxViewEdits',bytes:'maxViewBytes',planBytes:'maxViewPlanBytes'};
 for(const input of plan.sources){const part=prepareForwardViews(state,input.document,input.expectedSource,input.edits,host.viewLimits,host.viewWorkLimits);if(!part||!validateReversalViews(state,part,host.viewLimits,host.viewWorkLimits,host.viewVersionsAvailable)||part.viewChanged&&!host.versionsAvailable)return refuse();
  parts.push(part);for(const [key,limit] of Object.entries(keys)){charges[key]+=part.charges[key];if(!natural(charges[key])||charges[key]>host.viewWorkLimits[limit])return refuse();}
 }
 let result=advisory?decideAnnotationPlan(state,plan,host,readContext):publishAnnotationPlan(state,plan,host,readContext);if(result.status==='not-published')return result;
 for(const part of parts){result=completeForwardViews(state,part,result,host.viewLimits,host.viewWorkLimits,host.viewVersionsAvailable);if(result.status==='not-published')return refuse();}return result;
}
// Trusted bounded query facet. It neither issues nor runs an invocation.
const proposalValidationDefinition=freeze({family:'F10',contract:'proposal-validation/1',operation:'advisory-validation',profile:'native-compound-current-checks/1',requestFields:['contract','operation','plan'],basisFields:['root','context','request'],replyFields:['status','profile','checksSatisfied'],gates:['authorized','scope','continuous','live','notStopped','proposalReadable','validationObservable','outsidePublicationGuard','resourcesReady','replyReady']});
export const proposalValidationGates=proposalValidationDefinition.gates;
// Lazy declaration access; no authority, dispatcher or query execution.
export function proposalValidationDeclaration(){return proposalValidationDefinition;}
export function validateInvocationProposal(domain,request,c,g,host){
 const o=domains.get(domain);
 if(!current(o,c)||o.faulted||!gates(g,proposalValidationGates)||!exact(request,proposalValidationDefinition.requestFields)||request.contract!==proposalValidationDefinition.contract||request.operation!==proposalValidationDefinition.operation||!compoundPlanShape(request.plan)||!label(request.plan.invocation)||!label(request.plan.descriptor)||size(request)>o.limits.maxIntentBytes)return unavailable();
 const basis=host?.proposalValidationBasis;
 if(!exact(basis,proposalValidationDefinition.basisFields)||basis.root!==o.root||!equal(basis.context,c)||!equal(basis.request,request))return unavailable();
 const plan=normalizeCompoundAdmission(request.plan,o.limits.maxIntentBytes);
 if(!plan)return unavailable();
 const before=o.root;let result;
 try{result=compound(before,plan,host,c,true);}catch{o.faulted=true;return unavailable();}
 if(o.root!==before||o.faulted)return unavailable();
 const checksSatisfied=['applied','unchanged'].includes(result.status)&&size(result.state)<=o.limits.maxRootBytes&&(!o.baselineReservations||baselineReservationsAllow(o.baselineReservations,result.state));
 const reply={status:'advice',profile:proposalValidationDefinition.profile,checksSatisfied};
 return size(reply)<=o.limits.maxResultBytes?reply:unavailable();
}
export function runInvocation(domain,ticket,c,g,host){
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(!entryCurrent(o,entry,c)||o.faulted||entry?.phase!=='pending'||!g.schedulerOwned||!g.reservationHeld)return unavailable();
 if(o.admissionFence&&!entry.stop)return unavailable();
 entry.phase='running';o.starts++;const before=o.root,clipboard=entry.intent.kind==='clipboard',relation=entry.intent.kind==='relation',lifecycle=entry.intent.kind==='lifecycle',registration=entry.intent.kind==='registration';let settingsCreatedStage=null,registrationReceipt=null,registrationStage=null,result,prepared,annotationReceipt=null,reversalReceipt=null,relationReceipt=null,relationStage=null,lifecycleReceipt=null,lifecycleStage=null,lifecycleScratch=null,stage=null,atomicAnnotationReceipts=null,effect='none',faulted=false;
 try{
  if(clipboard){const captured=clipboardInvocationIntent(entry.plan,o.limits.maxIntentBytes);if(!captured||!equal(entry.intent,{kind:'clipboard',plan:captured.value}))throw Error('owned clipboard intent changed');effect=clipboardInvocationEffect(entry.plan);const staged=stageClipboardInvocation(entry.plan,ticket,before,host,entry.stop);if(!staged)throw Error('clipboard staging unavailable');stage=staged.stage;result=staged.result;}
  else
  if(entry.stop)result={status:'not-published',state:before};
  else if(registration){const captured=captureRegistrationInvocation(entry.plan,o,domain,ticket,o.limits.maxIntentBytes);if(!captured||!equal(entry.intent,{kind:'registration',plan:captured.value}))throw Error('owned registration intent changed');const staged=stageRegistrationInvocation(entry.plan,o,domain,ticket,before,host);registrationStage=staged.stage;registrationReceipt=staged.receipt;result=staged.result;}
  else if(lifecycle){const captured=captureLifecycleInvocation(entry.plan,o,domain,ticket,o.limits.maxIntentBytes);if(!captured||!equal(entry.intent,{kind:'lifecycle',plan:captured.value}))throw Error('owned lifecycle intent changed');const staged=stageLifecycleInvocation(entry.plan,o,domain,ticket,before,host);lifecycleStage=staged.stage;lifecycleReceipt=staged.receipt;lifecycleScratch=staged.scratch;result=staged.result;}
  else if(relation){const captured=captureRelationInvocation(entry.plan,o,domain,ticket,o.limits.maxIntentBytes);if(!captured||!equal(entry.intent,{kind:'relation',plan:captured.value}))throw Error('owned relation intent changed');const staged=stageRelationInvocation(entry.plan,o,domain,ticket,before,host);relationStage=staged.stage;relationReceipt=staged.receipt;result=staged.result;}
  else if(catalog.ownedEditingKinds.includes(entry.intent.kind)){
   const captured=captureEditingIntent(entry.intent.kind,entry.plan,o.limits.maxIntentBytes,o,domain,ticket);
   if(!captured)throw Error('owned editing intent unavailable');
else{if(!equal(entry.intent,{kind:entry.intent.kind,plan:captured.value}))throw Error('owned editing intent changed');result=entry.ordinaryPrepared?.inputFamily==='F13'?(nativeLocalCommandPermissions(host,'F13',entry.context)&&nativeLocalCommandLimitsCurrent(entry.ordinaryPrepared,host)?publishEditingIntent(entry.intent.kind,before,entry.plan,host.annotationHost,o,domain,ticket):{status:'not-published',state:before}):entry.ordinaryPrepared&&(!nativeSearchConsumerCurrent(o,domain,entry.ordinaryPrepared,host)||!(entry.ordinaryPrepared.inputFamily==='F12'?selectionPermissions(host,entry.ordinaryPrepared.job.kind,entry.ordinaryPrepared.capture.arguments.request):entry.ordinaryPrepared.job.kind==='replacement'?replacementPermissions(host):ordinaryPermissions(host,entry.ordinaryPrepared.job.kind)))?{status:'not-published',state:before}:publishEditingIntent(entry.intent.kind,before,entry.plan,host,o,domain,ticket);}
  }else if(entry.intent.kind==='compound'){
   const held=entry.ordinaryPrepared;
   if(held?.inputFamily==='F11-F13-mixed'){
    if(!atomicAuthoringPermissions(host,entry.plan.sources)||!atomicAuthoringIntentCurrent(held.mixedBundle,o,domain,ticket,before,host,entry.context))result={status:'not-published',state:before};
    else{const checks=atomicAuthoringMembershipChecks(held.mixedBundle,o,domain,ticket,host);if(!checks)throw Error('mixed authoring owner missing');result=compound(before,entry.plan,checks,entry.context);if(['applied','unchanged'].includes(result.status)&&!atomicAuthoringCandidateFits(held.mixedBundle,o,domain,ticket,result.state))result={status:'not-published',state:before};}
   }else result=entry.ordinaryPrepared?.inputFamily==='F11-exact-batch'&&!exactBatchPermissions(host)?{status:'not-published',state:before}:compound(before,entry.plan,host,entry.context);
  }
  else if(entry.intent.kind==='reversal')result=entry.ordinaryPrepared?.inputFamily==='F14'?(nativeLocalCommandPermissions(host,'F14',entry.context)&&nativeLocalCommandLimitsCurrent(entry.ordinaryPrepared,host)?publishReversal(before,entry.plan,host.historyHost):{status:'not-published',state:before}):publishReversal(before,entry.plan,host);
  else throw Error('unknown invocation job');
  if(!['applied','unchanged','not-published'].includes(result?.status)||result.state?.authority!==before.authority||
    (result.status==='applied'?result.state===before||!natural(result.state.publication)||result.state.publication!==before.publication+1:result.state!==before))throw Error('publisher protocol fault');
  if(size(result.state)>o.limits.maxRootBytes||o.baselineReservations&&!baselineReservationsAllow(o.baselineReservations,result.state)){
   result={status:'not-published',state:before};if(registration){discardRegistrationInvocationStage(registrationStage);registrationStage=null;registrationReceipt=null;}if(lifecycle){discardLifecycleInvocationStage(lifecycleStage);lifecycleStage=null;lifecycleReceipt=null;}if(relation){discardRelationInvocationStage(relationStage);relationStage=null;relationReceipt=null;}
   if(clipboard){if(!discardClipboardStage(entry.plan,ticket,stage))throw Error('clipboard discard failed');const staged=stageClipboardInvocation(entry.plan,ticket,before,host,true);if(!staged)throw Error('clipboard refusal unavailable');stage=staged.stage;result=staged.result;}
  }
  if(entry.settingsCreatedHold&&['applied','unchanged'].includes(result.status)){settingsCreatedStage=stageCreatedSettingsPublication(entry.settingsCreatedHold,o,domain,ticket,before,result,entry.context,host);if(!settingsCreatedStage)result={status:'not-published',state:before};}
  const publication=result.status==='not-published'?'notApplied':result.status,control=entry.stop?'stopped':result.status==='not-published'?'rejected':'done';
  prepared=prepareOutcome(facts(publication,control,entry.reference,effect),entry.policy);
  if(size(prepared)>o.limits.maxResultBytes)throw Error('reserved outcome exceeded');
  if(entry.intent.kind==='annotation'){annotationReceipt=captureAnnotationEditResult(entry.plan,result);if(!annotationReceipt||annotationReceipt.status!==result.status||annotationReceipt.invocation!==entry.reference||size(annotationReceipt)>o.limits.maxResultBytes)throw Error('reserved annotation result exceeded');}
  if(entry.intent.kind==='reversal'){reversalReceipt=captureReversalResultReceipt(entry.plan,result);if(!reversalReceipt||reversalReceipt.status!==result.status||reversalReceipt.invocation!==entry.reference||size(reversalReceipt)>o.limits.maxResultBytes)throw Error('reserved reversal result exceeded');}
  if(entry.ordinaryPrepared?.atomicReceiptHold){if(!atomicAnnotationResultPlanMatches(entry.ordinaryPrepared.atomicReceiptHold,o,domain,ticket,entry.plan))throw Error('atomic annotation receipt plan changed');atomicAnnotationReceipts=stageAtomicAnnotationResults(entry.ordinaryPrepared.atomicReceiptHold,o,domain,ticket,result);}
  freeze(result.state);
  if(entry.intent.kind==='settings'&&result.status==='applied')carryRetainedLifecycleSourceJournal(before,result.state);
  if(registration&&['applied','unchanged'].includes(result.status)){if(result.state!==before)carryRetainedLifecycleSourceJournal(before,result.state);if(!commitRegistrationInvocation(registrationStage,entry.plan,o,domain,ticket,before,result,registrationReceipt,host))throw Error('registration commit protocol fault');}
  if(lifecycle&&['applied','unchanged'].includes(result.status)){if(result.state!==before){if(lifecycleScratch)carryScratchSourceJournal(before,result.state,lifecycleScratch);else carryRetainedLifecycleSourceJournal(before,result.state);}if(!commitLifecycleInvocation(lifecycleStage,entry.plan,o,domain,ticket,before,result,lifecycleReceipt,host))throw Error('lifecycle commit protocol fault');}
  if(relation&&result.status==='applied'){carryRetainedLifecycleSourceJournal(before,result.state);if(!commitRelationInvocation(relationStage,entry.plan,o,domain,ticket,before,result,relationReceipt,host))throw Error('relation commit protocol fault');}
  if(clipboard&&(!validateClipboardStage(entry.plan,ticket,stage,before,result,host)||!commitClipboardStage(entry.plan,ticket,stage,before,result,host)))throw Error('clipboard commit protocol fault');
  if(settingsCreatedStage&&!commitCreatedSettingsPublication(settingsCreatedStage,entry.settingsCreatedHold,o,domain,ticket,before,result,entry.context,host))throw Error('created settings commit protocol fault');
 }catch{faulted=true;atomicAnnotationReceipts=null;registrationReceipt=null;annotationReceipt=null;reversalReceipt=null;relationReceipt=null;lifecycleReceipt=null;result={status:'not-published',state:before};if(clipboard)faultClipboardInvocation(entry.plan,ticket);prepared=prepareOutcome(clipboard?facts('unknown','faulted',entry.reference,'unknown'):facts('unknown','faulted',entry.reference),entry.policy);}
 if(settingsCreatedStage)discardCreatedSettingsPublication(settingsCreatedStage,faulted);
 if(registration)discardRegistrationInvocationStage(registrationStage);
 if(lifecycle)discardLifecycleInvocationStage(lifecycleStage);
 if(relation)discardRelationInvocationStage(relationStage);
 // Only bounded trusted assignments remain: install root and terminal bookkeeping together.
 o.root=result.state;entry.prepared=prepared;entry.annotationReceipt=annotationReceipt;entry.reversalReceipt=reversalReceipt;entry.relationReceipt=relationReceipt;entry.lifecycleReceipt=lifecycleReceipt;entry.registrationReceipt=registrationReceipt;entry.atomicAnnotationReceipts=atomicAnnotationReceipts;entry.phase='done';o.faulted||=faulted;return {status:faulted?'faulted':'settled'};
}
export function observeInvocation(domain,ticket,c,p,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(!entryCurrent(o,entry,c)||!policyShape(p)||!gates(g,catalog.releaseGates)||entry?.phase!=='done')return {status:'outcome',outcome:releaseOutcome(null,p)};
 return {status:'outcome',outcome:releaseOutcome(entry.prepared,p)};
}
export function stopInvocation(domain,ticket,g){const entry=domains.get(domain)?.slots.get(ticket);if(entry?.phase!=='pending'||!g.ownerControlAllowed)return unavailable();entry.stop=true;return {status:'stop-requested'};}
export function observeAnnotationResult(domain,ticket,c,p,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(!entryCurrent(o,entry,c)||entry.intent?.kind!=='annotation'||entry.phase!=='done'||!policyShape(p)||!gates(g,catalog.releaseGates))return unavailable();const summary=releaseOutcome(entry.prepared,p);if(summary.observation!=='available'||!['applied','unchanged','notApplied'].includes(summary.publication))return unavailable();return releaseAnnotationResult(entry.annotationReceipt,entry.annotationCeiling,g,o.limits.maxResultBytes);}
export function observeReversalResult(domain,ticket,c,p,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(!entryCurrent(o,entry,c)||entry.intent?.kind!=='reversal'||entry.phase!=='done'||!policyShape(p)||!gates(g,catalog.releaseGates))return unavailable();const summary=releaseOutcome(entry.prepared,p);if(summary.observation!=='available'||!['applied','notApplied'].includes(summary.publication))return unavailable();return releaseReversalResult(entry.reversalReceipt,entry.reversalCeiling,g,o.limits.maxResultBytes);}
export function observeRelationResult(domain,ticket,c,p,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(!entryCurrent(o,entry,c)||entry.intent?.kind!=='relation'||entry.phase!=='done'||!policyShape(p)||!gates(g,catalog.releaseGates))return unavailable();const summary=releaseOutcome(entry.prepared,p);if(summary.observation!=='available'||summary.publication!=='applied')return unavailable();return releaseRelationInvocationResult(entry.relationReceipt,entry.relationCeiling,g,o.limits.maxResultBytes);}
export function observeLifecycleResult(domain,ticket,c,p,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(!entryCurrent(o,entry,c)||entry.intent?.kind!=='lifecycle'||entry.phase!=='done'||!policyShape(p)||!gates(g,catalog.releaseGates))return unavailable();const summary=releaseOutcome(entry.prepared,p);if(summary.observation!=='available'||!['applied','unchanged'].includes(summary.publication))return unavailable();return releaseLifecycleInvocationResult(entry.lifecycleReceipt,entry.lifecycleCeiling,g,o.limits.maxResultBytes);}
export function observeRegistrationResult(domain,ticket,c,p,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(!entryCurrent(o,entry,c)||entry.intent?.kind!=='registration'||registrationInvocationIsScoped(entry.plan)||entry.phase!=='done'||!policyShape(p)||!gates(g,catalog.releaseGates))return unavailable();const summary=releaseOutcome(entry.prepared,p);if(summary.observation!=='available'||!['applied','unchanged'].includes(summary.publication))return unavailable();return releaseRegistrationInvocationResult(entry.registrationReceipt,entry.registrationCeiling,g,o.limits.maxResultBytes);}
export function observeScopedRegistrationResult(domain,ticket,c,p,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(!entryCurrent(o,entry,c)||entry.intent?.kind!=='registration'||entry.phase!=='done'||!policyShape(p)||!gates(g,catalog.releaseGates))return unavailable();const summary=releaseOutcome(entry.prepared,p);if(summary.observation!=='available'||!['applied','unchanged'].includes(summary.publication))return unavailable();return releaseScopedRegistrationInvocationResult(entry.plan,o,domain,ticket,o.root,entry.registrationReceipt,entry.registrationCeiling,g,o.limits.maxResultBytes);}
export function forgetInvocationResult(domain,ticket,g){const entry=domains.get(domain)?.slots.get(ticket);if(entry?.ordinaryPrepared?.atomicReceiptHold&&!gates(g,['retentionReleased','deliveryDrained']))return unavailable();if(entry?.phase!=='done'||!g.retentionReleased||!g.deliveryDrained)return unavailable();if(entry.settingsCreatedHold&&!dischargeCreatedSettingsResult(entry.settingsCreatedHold,domains.get(domain),domain,ticket,entry.context,g))return unavailable();entry.settingsCreatedHold=null;entry.prepared=null;entry.annotationReceipt=null;entry.reversalReceipt=null;entry.relationReceipt=null;entry.lifecycleReceipt=null;entry.registrationReceipt=null;entry.atomicAnnotationReceipts=null;return {status:'forgotten'};}
export function retireInvocation(domain,ticket,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPreparing)return unavailable();if((entry?.ordinaryPrepared||entry?.ordinaryCharge)&&!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return unavailable();if(!entry||!['issued','done'].includes(entry.phase)||!g.ownerControlAllowed||!g.retentionReleased||!g.deliveryDrained)return unavailable();if(entry.settingsCreatedHold&&!dischargeCreatedSettingsResult(entry.settingsCreatedHold,o,domain,ticket,entry.context,g))return unavailable();entry.settingsCreatedHold=null;if(entry.intent?.kind==='clipboard'&&!releaseClipboardInvocation(entry.plan,ticket,g))return unavailable();if(entry.configuredLinePrepared&&!forgetConfiguredLine(entry.configuredLinePrepared,o,domain,ticket,g))return unavailable();if(entry.settingsPrepared&&!forgetSourceSerialization(entry.settingsPrepared,o,domain,ticket))forgetNativeEngineSettings(entry.settingsPrepared,o,domain,ticket);if(entry.registrationPrepared)forgetRegistrationInvocation(entry.registrationPrepared);if(entry.lifecyclePrepared)forgetLifecycleInvocation(entry.lifecyclePrepared);if(entry.relationPrepared)forgetRelationInvocation(entry.relationPrepared);if(entry.ordinaryPrepared){const held=entry.ordinaryPrepared;if(held.atomicReceiptHold&&!retireAtomicAnnotationResultHold(held.atomicReceiptHold,o,domain,ticket,g))return unavailable();if(!held.configured&&!(held.job.kind==='compound'?true:held.job.kind==='reversal'?true:held.job.kind==='annotation'?true:held.job.kind==='selection'?discardSelectionEdit(held.job.plan,g):held.job.kind==='line'?discardLineEdit(held.job.plan,g):held.job.kind==='replacement'?discardReplacement(held.job.plan,g):discardBoundaryCommand(held.job.plan,g)))return unavailable();ordinaryPlans.set(held.job.plan,{retired:true});entry.ordinaryPrepared=null;}if(entry.ordinaryCharge){o.charge-=entry.ordinaryCharge;entry.ordinaryCharge=0;}if(entry.replacementResultCharge){o.charge-=entry.replacementResultCharge;entry.replacementResultCharge=0;}o.slots.delete(ticket);return {status:'retired'};}
// Fixed first-party result ports. Public wire/Rust encoding is not selected.
export function prepareInvocationSettingsCreatedResult(domain,ticket,c,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||!entry.settingsPrepared||!gates(g,catalog.admissionGates))return null;const hold=reserveCreatedSettingsResult(o,domain,ticket,entry.settingsPrepared,c,true);if(hold)entry.settingsCreatedHold=hold;return hold;}
export function observeSettingsCreatedResult(domain,ticket,c,p,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(!entryCurrent(o,entry,c)||entry.phase!=='done'||!entry.settingsCreatedHold||!policyShape(p)||!gates(g,catalog.releaseGates))return unavailable();return releaseCreatedSettingsResult(entry.settingsCreatedHold,o,domain,ticket,c,p,g);}
export function captureInvocationCreatedSettingsWriteBasis(domain,ticket,reference,c,p,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(observeSettingsCreatedResult(domain,ticket,c,p,g).status!=='result')return null;return captureCreatedSettingsWriteBasis(entry.settingsCreatedHold,o,domain,ticket,reference,c,p,g);}
export function invocationSettingsCreatedUsage(domain,ticket,c){const o=domains.get(domain),entry=o?.slots.get(ticket);return entryCurrent(o,entry,c)&&entry.settingsCreatedHold?createdSettingsResultUsage(entry.settingsCreatedHold,o,domain,ticket,c):null;}
// Trusted engine inspection only, never a recipient raw-root/status disclosure API.
export function invocationAuthorityRoot(domain){const root=domains.get(domain)?.root??null;if(root)exposedRootOrigins.set(root,domain);return root;}
// First-party identity witness for already exposed immutable roots. Weak keys do
// not retain historical roots or turn a copied state into an owned snapshot.
export function nativeInvocationRootOrigin(root){return root&&typeof root==='object'?exposedRootOrigins.get(root)??null:null;}
// Trusted first-party primitive, not F18 shutdown acceptance. Complete owner
// inventory, consent and eligible-effect dispatch must be composed separately.
// A fence is absorbing: ending a wait cannot reopen ordinary admission.
export function sealInvocationOrdinaryAdmission(domain,before,c,g){
 const o=domains.get(domain);if(!o||!contextValid(c)||!equal(o.context,c)||!gates(g,['ownerControlAllowed','controlContextCurrent','controlScopeValid','protectedControlReady']))return null;
 if(o.admissionFence)return o.admissionFence.before===before?o.admissionFence.token:null;
 if(o.faulted||o.admissionMutation||o.root!==before||[...o.slots.values()].some(e=>e.phase==='running'||e.ordinaryPreparing))return null;
 // Retaining the original root for exact retry is a distinct bounded logical
 // frame. It is not covered by a caller assertion or a physical-memory claim.
 const token=Object.freeze({kind:'ordinary-admission-fence/1'}),charge=o.limits.maxRootBytes+size(token);
 if(!natural(charge)||charge>o.limits.maxHeldBytes-o.charge)return null;
 o.charge+=charge;o.admissionFence={token,before,charge};return token;
}
export function invocationOrdinaryAdmissionOpen(domain){const o=domains.get(domain);return Boolean(o&&!o.faulted&&!o.admissionFence);}
// A single serialized mutation section excludes fence installation during fixed
// native admission, including controlled reentry through normalized fact reads.
// The fenced form supports admitted-invocation comparison and protected
// source-destination bookkeeping; each caller still checks its exact operation.
export function beginInvocationAdmissionMutation(domain,allowFenced=false){const o=domains.get(domain);if(!o||o.faulted||o.admissionMutation||o.admissionFence&&allowFenced!==true)return null;const token=Object.freeze({kind:'native-admission-section/1'});o.admissionMutation=token;return token;}
export function endInvocationAdmissionMutation(domain,token){const o=domains.get(domain);if(!o||o.admissionMutation!==token)return false;delete o.admissionMutation;return true;}
// Fixed first-party ownership edges, not public allocation/discovery commands.
// Each edge reserves its declared child envelope and one positive metadata frame
// in the actual parent. Native factories retain the lease privately. Attachment
// coverage is deliberately separate from the catalog of permitted owner kinds.
const nativeOwnerKinds=Object.freeze(['source-read','annotation-read','source-writer','baseline-reservations','source-contexts','annotation-capture','annotation-contexts','version-set-capture','version-set-contexts','source-search','settings','settings-reads','serialization','registration-bases','registration-reads','registration-events','discovery','history','analysis','observation','observation-admission','diagnostic-capture','diagnostic-drain','publication-observer']);
const ownerHandleShape=v=>(exact(v,['kind'])&&label(v.kind)||exact(v,['profile'])&&label(v.profile))&&Object.isFrozen(v);
const emptyOwnerInventory=Object.freeze({entries:Object.freeze([]),pending:false,declaredHeldBytes:0,completeFactoryCoverageProved:false,wholeOwnerDischargeProved:false});
// The complete serialized envelope conservatively funds simultaneously retained
// preparation, Map key/edge, and one internal captured-report representation.
// Numeric fields use their maximum encoded width. Native handle templates contain
// only their inert marker; equal markers never establish actual owner identity.
export function invocationOwnerAttachmentFrame(kind,ownerTemplate){
 if(!nativeOwnerKinds.includes(kind)||!ownerHandleShape(ownerTemplate))return null;
 const lease={kind:'native-owner-attachment/1'},section={kind:'native-admission-section/1'},maximum=Number.MAX_SAFE_INTEGER;
 return size({pending:{section,lease,kind,units:maximum,ownerTemplate,frame:maximum,charge:maximum},retained:{owner:ownerTemplate,lease,kind,units:maximum,frame:maximum,charge:maximum},inventory:{entries:[{owner:ownerTemplate,kind,additionalChildReservationBytes:maximum,declaredFrameBytes:maximum}],pending:false,declaredHeldBytes:maximum,completeFactoryCoverageProved:false,wholeOwnerDischargeProved:false}});
}
export function beginInvocationOwnerAttachment(domain,kind,units,ownerTemplate){
 const o=domains.get(domain),frame=invocationOwnerAttachmentFrame(kind,ownerTemplate);if(!o||frame===null||!natural(units))return null;
 const section=beginInvocationAdmissionMutation(domain);if(!section)return null;
 const charge=units+frame;
 if(!natural(charge)||charge>o.limits.maxHeldBytes-o.charge){endInvocationAdmissionMutation(domain,section);return null;}
 const lease=Object.freeze({kind:'native-owner-attachment/1'});o.charge+=charge;o.pendingOwnerAttachment={section,lease,kind,units,ownerTemplate,frame,charge};return lease;
}
export function commitInvocationOwnerAttachment(domain,lease,owner){
 const o=domains.get(domain),p=o?.pendingOwnerAttachment;
 if(!p||p.lease!==lease||o.admissionMutation!==p.section||!ownerHandleShape(owner)||!equal(owner,p.ownerTemplate)||o.ownerAttachments?.has(owner))return false;
 o.ownerAttachments??=new Map();o.ownerAttachments.set(owner,{lease,kind:p.kind,units:p.units,frame:p.frame,charge:p.charge});delete o.pendingOwnerAttachment;endInvocationAdmissionMutation(domain,p.section);return true;
}
export function abortInvocationOwnerAttachment(domain,lease){
 const o=domains.get(domain),p=o?.pendingOwnerAttachment;if(!p||p.lease!==lease||o.admissionMutation!==p.section)return false;
 o.charge-=p.charge;delete o.pendingOwnerAttachment;endInvocationAdmissionMutation(domain,p.section);return true;
}
export function releaseInvocationOwnerAttachment(domain,lease,g){
 const o=domains.get(domain);if(!o||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;
 const row=[...(o.ownerAttachments??[])].find(([,edge])=>edge.lease===lease);if(!row)return false;
 o.charge-=row[1].charge;o.ownerAttachments.delete(row[0]);return true;
}
// Protected internal supervision. Exact original authority context is mandatory;
// this exposes actual handles to the trusted controller, never recipient data.
// additionalChildReservationBytes is additional edge funding only. Parent-funded
// native pools keep their original separate charge; this is not total owner cost.
export function invocationOwnerAttachmentInventory(domain,c,g){
 const o=domains.get(domain);if(!o||!contextValid(c)||!equal(o.context,c)||g?.ownerControlAllowed!==true)return null;
 if(!o.ownerAttachments?.size&&!o.pendingOwnerAttachment)return emptyOwnerInventory;
 const entries=[...(o.ownerAttachments??[])].map(([owner,e])=>Object.freeze({owner,kind:e.kind,additionalChildReservationBytes:e.units,declaredFrameBytes:e.frame}));
 return Object.freeze({entries:Object.freeze(entries),pending:Boolean(o.pendingOwnerAttachment),declaredHeldBytes:[...(o.ownerAttachments??new Map()).values()].reduce((n,e)=>n+e.charge,0)+(o.pendingOwnerAttachment?.charge??0),completeFactoryCoverageProved:false,wholeOwnerDischargeProved:false});
}
// Pure first-party membership guard: no inventory allocation, native calls or
// authority issuance. The caller must separately obtain current control facts.
export function invocationOwnerAttachmentMatches(domain,c,owner,kind){const o=domains.get(domain);return Boolean(o&&contextValid(c)&&equal(o.context,c)&&o.ownerAttachments?.get(owner)?.kind===kind);}
// Pure context-role membership for the four parent-funded native read pools.
// No caller scope is inferred and no native stop/query/discharge is attempted.
export function invocationReadStopTargetOwned(domain,handle,reference,kind){const o=domains.get(domain);if(!o)return false;switch(kind){case 'source':return nativeSourceStopTargetOwned(o,domain,handle,reference);case 'annotation':return nativeAnnotationStopTargetOwned(o,domain,handle,reference);case 'version-set':return nativeVersionSetStopTargetOwned(o,domain,handle,reference);case 'settings':return nativeSettingsStopTargetOwned(o,domain,handle,reference);default:return false;}}
// Protected supervision only, not a recipient result or a shutdown status.
export function invocationAdmissionFenceUsage(domain,c,g){const o=domains.get(domain);return o&&contextValid(c)&&equal(o.context,c)&&g?.ownerControlAllowed===true?{fenced:Boolean(o.admissionFence),declaredHeldBytes:o.admissionFence?.charge??0,completeOwnerInventoryProved:false,shutdownAccepted:false}:null;}
// These fixed native pools already debit/refund the actual parent themselves.
// The edge adds only its complete metadata envelope, never a second child debit.
function openAttachedNativePool(domain,kind,marker,create){
 const o=domains.get(domain);if(!o)return null;const attachment=beginInvocationOwnerAttachment(domain,kind,0,Object.freeze({kind:marker}));if(!attachment)return null;
 try{const handle=create(o);if(!handle)return null;if(!commitInvocationOwnerAttachment(domain,attachment,handle))throw Error('parent-funded native pool attachment invariant');return handle;}
 finally{abortInvocationOwnerAttachment(domain,attachment);}
}
function retireAttachedNativePool(domain,handle,kind,g,retire){
 const o=domains.get(domain),edge=o?.ownerAttachments?.get(handle);if(!edge||edge.kind!==kind)return false;
 const facts=Object.fromEntries(['ownerControlAllowed','retentionReleased','deliveryDrained'].map(k=>[k,g?.[k]===true]));if(!Object.values(facts).every(Boolean)||!retire(o,facts))return false;
 if(!releaseInvocationOwnerAttachment(domain,edge.lease,facts))throw Error('parent-funded native pool discharge invariant');return true;
}
// All fixed lazy-settings creators share this acquisition transaction. Invalid
// requests create no pool and refund the pending metadata. If a native creator
// retains its pool before a later exception, the real pool stays attached/funded.
// These callbacks are fixed first-party ports, never caller-supplied operations.
function withAttachedNativeSettingsPool(domain,invoke,newProduct=false){
 const o=domains.get(domain);if(!o)return null;if(nativeEngineSettingsPoolHandle(o,domain)){
  if(!newProduct)return invoke();
  const admissionSection=beginInvocationAdmissionMutation(domain);if(!admissionSection)return null;
  try{return invoke();}finally{endInvocationAdmissionMutation(domain,admissionSection);}
 }
 const attachment=beginInvocationOwnerAttachment(domain,'settings',0,Object.freeze({kind:'native-engine-settings-pool/1'}));if(!attachment)return null;
 try{return invoke();}finally{const handle=nativeEngineSettingsPoolHandle(o,domain);try{if(handle&&!commitInvocationOwnerAttachment(domain,attachment,handle))throw Error('native lazy settings attachment invariant');}finally{abortInvocationOwnerAttachment(domain,attachment);}}
}
export function openInvocationVersionSetCapture(domain,p,g){return openAttachedNativePool(domain,'version-set-capture','native-version-set-preparation-pool/1',o=>openNativeVersionSetCapture(o,domain,p,g));}
export function prepareInvocationVersionSetCapture(domain,handle,r,h,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return null;try{const o=domains.get(domain);return o?prepareNativeVersionSetCapture(o,domain,handle,r,h,g):null;
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function validateInvocationVersionSetCapture(domain,handle,cut,h,g){const o=domains.get(domain);return o?validateNativeVersionSetCapture(o,domain,handle,cut,h,g):false;}
export function inspectInvocationVersionSetCapture(domain,handle,cut,g){const o=domains.get(domain);return o?inspectNativeVersionSetCapture(o,domain,handle,cut,g):null;}
export function discardInvocationVersionSetCapture(domain,handle,cut,g){const o=domains.get(domain);return o?discardNativeVersionSetCapture(o,domain,handle,cut,g):false;}
export function retireInvocationVersionSetCapture(domain,handle,g){return retireAttachedNativePool(domain,handle,'version-set-capture',g,(o,facts)=>retireNativeVersionSetCapture(o,domain,handle,facts));}
export function invocationVersionSetCaptureUsage(domain){const o=domains.get(domain);return o?nativeVersionSetCaptureUsage(o):null;}
export function openInvocationVersionSetContexts(domain,p,g){return openAttachedNativePool(domain,'version-set-contexts','native-version-set-context-pool/1',o=>openNativeVersionSetContexts(o,domain,p,g));}
export function installInvocationVersionSetContext(domain,handle,capture,cut,h,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=domains.get(domain);return o?installNativeVersionSetContext(o,domain,handle,capture,cut,h,g):unavailable();
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function acquireInvocationRetainedVersionSetContext(domain,handle,r,h,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=domains.get(domain);return o?acquireNativeRetainedVersionSetContext(o,domain,handle,r,h,g):unavailable();
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function inspectInvocationVersionSetContext(domain,handle,reference,g){const o=domains.get(domain);return o?inspectNativeVersionSetContext(o,domain,handle,reference,g):null;}
export function validateInvocationVersionSetContext(domain,handle,reference,h,g){const o=domains.get(domain);return o?validateNativeVersionSetContext(o,domain,handle,reference,h,g):false;}
export function queryInvocationVersionSetMember(domain,handle,r,h,g){const o=domains.get(domain);return o?queryNativeVersionSetMember(o,domain,handle,r,h,g):unavailable();}
export function stopInvocationVersionSetContext(domain,handle,r,c,g){const o=domains.get(domain);return o?stopNativeVersionSetContext(o,domain,handle,r,c,g):unavailable();}
export function retireInvocationVersionSetContext(domain,handle,reference,g){const o=domains.get(domain);return o?retireNativeVersionSetContext(o,domain,handle,reference,g):false;}
export function retireInvocationVersionSetContexts(domain,handle,g){return retireAttachedNativePool(domain,handle,'version-set-contexts',g,(o,facts)=>retireNativeVersionSetContexts(o,domain,handle,facts));}
export function invocationVersionSetContextUsage(domain){const o=domains.get(domain);return o?nativeVersionSetContextUsage(o):null;}
export function openInvocationAnnotationCapture(domain,p,g){return openAttachedNativePool(domain,'annotation-capture','native-annotation-preparation-pool/1',o=>openNativeAnnotationCapture(o,domain,p,g));}
export function prepareInvocationAnnotationCapture(domain,handle,r,c,rows,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return null;try{const o=domains.get(domain);return o?prepareNativeAnnotationCapture(o,domain,handle,r,c,rows,g):null;
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function validateInvocationAnnotationCapture(domain,handle,cut,c,g){const o=domains.get(domain);return o?validateNativeAnnotationCapture(o,domain,handle,cut,c,g):false;}
export function inspectInvocationAnnotationCapture(domain,handle,cut,g){const o=domains.get(domain);return o?inspectNativeAnnotationCapture(o,domain,handle,cut,g):null;}
export function discardInvocationAnnotationCapture(domain,handle,cut,g){const o=domains.get(domain);return o?discardNativeAnnotationCapture(o,domain,handle,cut,g):false;}
export function retireInvocationAnnotationCapture(domain,handle,g){return retireAttachedNativePool(domain,handle,'annotation-capture',g,(o,facts)=>retireNativeAnnotationCapture(o,domain,handle,facts));}
export function invocationAnnotationCaptureUsage(domain){const o=domains.get(domain);return o?nativeAnnotationCaptureUsage(o):null;}
export function openInvocationAnnotationContexts(domain,p,g){return openAttachedNativePool(domain,'annotation-contexts','native-annotation-context-pool/1',o=>openNativeAnnotationContexts(o,domain,p,g));}
export function installInvocationAnnotationContext(domain,handle,capturePool,cut,c,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=domains.get(domain);return o?installNativeAnnotationContext(o,domain,handle,capturePool,cut,c,g):unavailable();
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function acquireInvocationRetainedAnnotationContext(domain,handle,r,c,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=domains.get(domain);return o?acquireNativeRetainedAnnotationContext(o,domain,handle,r,c,g):unavailable();
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function inspectInvocationAnnotationContext(domain,handle,context,g){const o=domains.get(domain);return o?inspectNativeAnnotationContext(o,domain,handle,context,g):null;}
export function queryInvocationAnnotationContext(domain,handle,r,c,g){const o=domains.get(domain);return o?queryNativeAnnotationContext(o,domain,handle,r,c,g):unavailable();}
export function queryInvocationCombinedSource(domain,handle,r,c,g){const o=domains.get(domain);return o?queryNativeCombinedSource(o,domain,handle,r,c,g):unavailable();}
export function stopInvocationAnnotationContext(domain,handle,r,c,g){const o=domains.get(domain);return o?stopNativeAnnotationContext(o,domain,handle,r,c,g):unavailable();}
export function retireInvocationAnnotationContext(domain,handle,context,g){const o=domains.get(domain);return o?retireNativeAnnotationContext(o,domain,handle,context,g):false;}
export function retireInvocationAnnotationContexts(domain,handle,g){return retireAttachedNativePool(domain,handle,'annotation-contexts',g,(o,facts)=>retireNativeAnnotationContexts(o,domain,handle,facts));}
export function invocationAnnotationContextUsage(domain){const o=domains.get(domain);return o?nativeAnnotationContextUsage(o):null;}
export function openInvocationSourceSearch(domain,sourceHandle,p,g){return openAttachedNativePool(domain,'source-search','native-source-search-pool/1',o=>openNativeSourceSearch(o,domain,sourceHandle,p,g));}
export function queryInvocationSourceSearch(domain,handle,r,c,q,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=domains.get(domain);return o?queryNativeSourceSearch(o,domain,handle,r,c,q,g):unavailable();
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
// Selected synchronous decoded entry; existing search owns every returned hold.
// Eight bounded logical frames cover envelope, authority, canonical record,
// limits, descriptor, current/facts packet, producer and restored-current frames.
// This reservation is not a physical-memory or hostile-Proxy bound.
export function queryInvocationDecodedSearch(domain,handle,envelope,inputLimits,authorityContext,sourceContext,currentQuery,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{
 const o=domains.get(domain);if(!o||o.faulted||o.searchDecoding||searchArgumentClassificationBytes>o.limits.maxHeldBytes-o.charge)return unavailable();
 let charged=searchArgumentClassificationBytes;o.searchDecoding=true;o.searchDecodeCharge=charged;o.charge+=charged;
 try{
  const p=captureSearchArgumentLimits(inputLimits);if(!p)return unavailable();
  const maximum=Math.min(p.maxRequestBytes,o.limits.maxIntentBytes),workspace=8*maximum;if(!natural(workspace)||workspace>o.limits.maxHeldBytes-o.charge)return unavailable();
  charged+=workspace;o.searchDecodeCharge=charged;o.charge+=workspace;
  const n=normalizeSearchArguments(envelope,authorityContext,p,maximum);if(!n||!current(o,n.record.context))return unavailable();
  const captured=captureSearchCurrentInputs(sourceContext,currentQuery,g,p,maximum);if(!captured)return unavailable();
  return queryNativeSourceSearch(o,domain,handle,searchProducerRequest(n),captured.sourceContext,captured.current,captured.facts);
 }catch{o.faulted=true;return unavailable();}finally{o.charge-=charged;o.searchDecodeCharge=0;o.searchDecoding=false;}

 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function invocationSearchDecodeUsage(domain){const o=domains.get(domain);return o?{busy:o.searchDecoding===true,declaredWorkspaceBytes:o.searchDecodeCharge??0}:null;}
export function releaseInvocationSourceSearchResult(domain,handle,result,c,q,g){const o=domains.get(domain);return o?releaseNativeSourceSearchResult(o,domain,handle,result,c,q,g):unavailable();}
export function retireInvocationSourceSearchResult(domain,handle,result,g){const o=domains.get(domain);return o?retireNativeSourceSearchResult(o,domain,handle,result,g):false;}
export function retireInvocationSourceSearch(domain,handle,g){return retireAttachedNativePool(domain,handle,'source-search',g,(o,facts)=>retireNativeSourceSearch(o,domain,handle,facts));}
export function invocationSourceSearchUsage(domain){const o=domains.get(domain);return o?nativeSourceSearchUsage(o):null;}
export function openInvocationSourceContexts(domain,p,g){return openAttachedNativePool(domain,'source-contexts','native-source-context-pool/1',o=>openNativeSourceContexts(o,domain,p,g));}
export function acquireInvocationSourceContext(domain,handle,r,c,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=domains.get(domain);return o?acquireNativeSourceContext(o,domain,handle,r,c,g):unavailable();
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function queryInvocationSourceContext(domain,handle,r,c,g){const o=domains.get(domain);return o?queryNativeSourceContext(o,domain,handle,r,c,g):unavailable();}
// Fixed first-party read ports selected by an actual pool handle, never a caller
// issuer label or callback. Current scope/host meanings stay native to each port.
export function queryInvocationDecodedSourceRead(domain,handle,envelope,inputLimits,authorityContext,nativeCurrent,g){
 const o=domains.get(domain);if(!o||o.faulted||o.sourceReadDecoding||sourceReadArgumentClassificationBytes>o.limits.maxHeldBytes-o.charge)return unavailable();
 const port=nativeSourceContextPoolAvailable(o,domain,handle)?queryNativeSourceContext:nativeAnnotationContextPoolAvailable(o,domain,handle)?queryNativeCombinedSource:nativeVersionSetContextPoolAvailable(o,domain,handle)?queryNativeVersionSetMember:null;if(!port)return unavailable();
 let charged=sourceReadArgumentClassificationBytes;o.sourceReadDecoding=true;o.sourceReadDecodeCharge=charged;o.charge+=charged;
 try{
  const p=captureSourceReadArgumentLimits(inputLimits);if(!p)return unavailable();
  // Seven bounded logical frames: envelope, authority, canonical record, limits,
  // descriptor, current/facts packet and producer request. Replies stay charged
  // under the existing native owner's envelope; no physical-memory claim.
  const maximum=Math.min(p.maxRequestBytes,o.limits.maxIntentBytes),workspace=7*maximum;if(!natural(workspace)||workspace>o.limits.maxHeldBytes-o.charge)return unavailable();
  charged+=workspace;o.sourceReadDecodeCharge=charged;o.charge+=workspace;
  const n=normalizeSourceReadArguments(envelope,authorityContext,p,maximum);if(!n||!current(o,n.record.context))return unavailable();
  const packet=copySettingsData({current:nativeCurrent,facts:g},{...p,maxRequestBytes:maximum});if(!packet)return unavailable();
  const readRequest=sourceReadProducerRequest(n),readResult=port(o,domain,handle,readRequest,packet.value.current,packet.value.facts);
  // The port owns revision/authority truth; this boundary checks its shape and
  // original request/context binding without authenticating its own reply.
  if(!sourceReadResultValid(readResult,readRequest)){o.faulted=true;return unavailable();}return readResult;
 }catch{o.faulted=true;return unavailable();}finally{o.charge-=charged;o.sourceReadDecodeCharge=0;o.sourceReadDecoding=false;}
}
export function invocationSourceReadDecodeUsage(domain){const o=domains.get(domain);return o?{busy:o.sourceReadDecoding===true,declaredWorkspaceBytes:o.sourceReadDecodeCharge??0}:null;}
// Fixed native F03 ports. Record and continuation identities are not JSON values.
export function queryInvocationDecodedAnnotation(domain,handle,envelope,inputLimits,authorityContext,nativeCurrent,g){
 const o=domains.get(domain);if(!o||o.faulted||o.annotationQueryDecoding||annotationQueryClassificationBytes>o.limits.maxHeldBytes-o.charge)return unavailable();
 const annotationPort=nativeAnnotationContextPoolAvailable(o,domain,handle)?queryNativeAnnotationContext:nativeVersionSetContextPoolAvailable(o,domain,handle)?queryNativeVersionSetMember:null;if(!annotationPort)return unavailable();
 let annotationCharge=annotationQueryClassificationBytes;o.annotationQueryDecoding=true;o.annotationQueryDecodeCharge=annotationCharge;o.charge+=annotationCharge;
 try{
  const p=captureAnnotationQueryLimits(inputLimits);if(!p)return unavailable();
  // Seven logical frames, including the descriptor with both opaque bindings.
  // Actual native reply and selector reservations remain separately owned.
  const maximum=Math.min(p.maxRequestBytes,o.limits.maxIntentBytes),annotationWorkspace=7*maximum;if(!natural(annotationWorkspace)||annotationWorkspace>o.limits.maxHeldBytes-o.charge)return unavailable();
  annotationCharge+=annotationWorkspace;o.annotationQueryDecodeCharge=annotationCharge;o.charge+=annotationWorkspace;
  const annotationQuery=normalizeAnnotationQueryArguments(envelope,authorityContext,p,maximum);if(!annotationQuery||!current(o,annotationQuery.record.context))return unavailable();
  const annotationPacket=copySettingsData({current:nativeCurrent,facts:g},{...p,maxRequestBytes:maximum});if(!annotationPacket)return unavailable();
  const annotationRequest=annotationQueryProducerRequest(annotationQuery),annotationResult=annotationPort(o,domain,handle,annotationRequest,annotationPacket.value.current,annotationPacket.value.facts);
  if(!annotationQueryResultValid(annotationResult,annotationRequest)){o.faulted=true;return unavailable();}
  return annotationResult;
 }catch{o.faulted=true;return unavailable();}finally{o.charge-=annotationCharge;o.annotationQueryDecodeCharge=0;o.annotationQueryDecoding=false;}
}
export function invocationAnnotationQueryDecodeUsage(domain){const o=domains.get(domain);return o?{busy:o.annotationQueryDecoding===true,declaredWorkspaceBytes:o.annotationQueryDecodeCharge??0}:null;}
// Existing snapshot-handoff products. Prepared capabilities are a private
// companion port, not caller-supplied roots, rows, issuer labels or callbacks.
function checkedReadAcquisitionReply(owner,request,result){if(!readAcquisitionResultValid(result,request)){owner.faulted=true;return unavailable();}return result;}
export function acquireInvocationDecodedReadContext(domain,handle,envelope,inputLimits,authorityContext,nativeCurrent,g,preparation=null){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{
 const o=domains.get(domain);if(!o||o.faulted||o.readAcquisitionDecoding||readAcquisitionClassificationBytes>o.limits.maxHeldBytes-o.charge)return unavailable();
 const source=nativeSourceContextPoolAvailable(o,domain,handle),annotation=nativeAnnotationContextPoolAvailable(o,domain,handle),versionSet=nativeVersionSetContextPoolAvailable(o,domain,handle);if(!source&&!annotation&&!versionSet)return unavailable();
 let acquisitionCharge=readAcquisitionClassificationBytes;o.readAcquisitionDecoding=true;o.readAcquisitionDecodeCharge=acquisitionCharge;o.charge+=acquisitionCharge;
 try{
  const p=captureReadAcquisitionLimits(inputLimits);if(!p)return unavailable();
  // Eight logical frames: envelope, authority, canonical record, limits,
  // descriptor, current/facts packet, producer request, preparation binding.
  const maximum=Math.min(p.maxRequestBytes,o.limits.maxIntentBytes),acquisitionWorkspace=8*maximum;if(!natural(acquisitionWorkspace)||acquisitionWorkspace>o.limits.maxHeldBytes-o.charge)return unavailable();
  acquisitionCharge+=acquisitionWorkspace;o.readAcquisitionDecodeCharge=acquisitionCharge;o.charge+=acquisitionWorkspace;
  const normalized=normalizeReadAcquisitionArguments(envelope,authorityContext,p,maximum);if(!normalized||!current(o,normalized.record.context))return unavailable();
  const currentPacket=copySettingsData({current:nativeCurrent,facts:g},{...p,maxRequestBytes:maximum}),prepared=captureReadPreparationBinding(preparation);if(!currentPacket||!prepared)return unavailable();
  const request=readAcquisitionProducerRequest(normalized),retained=request.selection.kind==='retained';
  if(source){if(request.contract!=='read-context/1'||prepared.kind!=='none')return unavailable();return checkedReadAcquisitionReply(o,request,acquireNativeSourceContext(o,domain,handle,request,currentPacket.value.current,currentPacket.value.facts));}
  if(annotation){if(request.contract!=='read-context/1')return unavailable();if(retained){if(prepared.kind!=='none')return unavailable();return checkedReadAcquisitionReply(o,request,acquireNativeRetainedAnnotationContext(o,domain,handle,request,currentPacket.value.current,currentPacket.value.facts));}
   if(prepared.kind!=='prepared'||!nativeAnnotationCutMatchesRequest(o,domain,prepared.pool,prepared.cut,request))return unavailable();
   return checkedReadAcquisitionReply(o,request,installNativeAnnotationContext(o,domain,handle,prepared.pool,prepared.cut,currentPacket.value.current,currentPacket.value.facts));
  }
  if(request.contract!=='read-version-set/1')return unavailable();if(retained){if(prepared.kind!=='none')return unavailable();return checkedReadAcquisitionReply(o,request,acquireNativeRetainedVersionSetContext(o,domain,handle,request,currentPacket.value.current,currentPacket.value.facts));}
  if(prepared.kind!=='prepared'||!nativeVersionSetCutMatchesRequest(o,domain,prepared.pool,prepared.cut,request))return unavailable();
  return checkedReadAcquisitionReply(o,request,installNativeVersionSetContext(o,domain,handle,prepared.pool,prepared.cut,currentPacket.value.current,currentPacket.value.facts));
 }catch{o.faulted=true;return unavailable();}finally{o.charge-=acquisitionCharge;o.readAcquisitionDecodeCharge=0;o.readAcquisitionDecoding=false;}

 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function invocationReadAcquisitionDecodeUsage(domain){const o=domains.get(domain);return o?{busy:o.readAcquisitionDecoding===true,declaredWorkspaceBytes:o.readAcquisitionDecodeCharge??0}:null;}
export function stopInvocationSourceContext(domain,handle,r,c,g){const o=domains.get(domain);return o?stopNativeSourceContext(o,domain,handle,r,c,g):unavailable();}
export function holdInvocationSourceContext(domain,handle,r,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return null;try{const o=domains.get(domain);return o?holdNativeSourceContext(o,domain,handle,r,g):null;
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function releaseInvocationSourceContextHold(domain,handle,r,g){const o=domains.get(domain);return o?releaseNativeSourceContextHold(o,domain,handle,r,g):false;}
export function retireInvocationSourceContext(domain,handle,r,g){const o=domains.get(domain);return o?retireNativeSourceContext(o,domain,handle,r,g):false;}
export function retireInvocationSourceContexts(domain,handle,g){return retireAttachedNativePool(domain,handle,'source-contexts',g,(o,facts)=>retireNativeSourceContexts(o,domain,handle,facts));}
export function invocationSourceContextUsage(domain){const o=domains.get(domain);return o?nativeSourceContextUsage(o):null;}
export function invocationAuthorityContext(domain){const o=domains.get(domain);return o?freeze(copy(o.context)):null;}
export function installInvocationRetainedLifecycle(domain,token,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain,true);if(!admissionSection)return unavailable();try{
 const o=domains.get(domain);if(!o||o.faulted)return unavailable();const before=o.root,p=readNativeRetainedLifecycle(token,domain,before,g);if(!p||size(p.result.state)>o.limits.maxRootBytes||o.baselineReservations&&!baselineReservationsAllow(o.baselineReservations,p.result.state))return unavailable();
 if(o.admissionFence&&!['release-binding','release-annotations','release-provenance'].includes(p.kind))return unavailable();
 freeze(p.result.state);if(!consumeNativeRetainedLifecycle(token))return unavailable();if(p.result.state!==before){if(p.kind==='create-scratch'){carryScratchSourceJournal(before,p.result.state,p.result.document);}else {carryRetainedLifecycleSourceJournal(before,p.result.state);}}o.root=p.result.state;return {status:p.result.status,...(p.kind==='create-scratch'?{document:copy(p.result.document)}:{})};

 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function installInvocationSourceDestination(domain,token,g){
 const section=beginInvocationAdmissionMutation(domain,true);if(!section)return unavailable();try{
 const o=domains.get(domain);if(!o||o.faulted)return unavailable();const before=o.root,p=readSourceDestinationTransition(token,domain,before,g);if(!p||size(p.next)>o.limits.maxRootBytes||o.baselineReservations&&!baselineReservationsAllow(o.baselineReservations,p.next))return unavailable();if(o.admissionFence&&!['baseline','release','release-history'].includes(p.kind))return unavailable();freeze(p.next);const result=commitSourceDestinationTransition(token);if(!result)return unavailable();if(p.next!==before)carryRetainedLifecycleSourceJournal(before,p.next);o.root=p.next;return result;
 }finally{endInvocationAdmissionMutation(domain,section);}
}
export function installInvocationSourceRead(domain,token,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=domains.get(domain);if(!o||o.faulted)return unavailable();const before=o.root,p=readNativeSourceRead(token,domain,before,g);if(!p||size(p.result.state)>o.limits.maxRootBytes||o.baselineReservations&&!baselineReservationsAllow(o.baselineReservations,p.result.state))return unavailable();freeze(p.result.state);const result=commitNativeSourceRead(token);if(!result)return unavailable();if(p.result.state!==before)carryScratchSourceJournal(before,p.result.state,p.result.document);o.root=p.result.state;return result;
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
// Trusted host metadata controls, not new public/native execution jobs.
export function prepareInvocationResourceRegistration(domain,request,profile,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return null;try{const o=domains.get(domain);return o&&!o.faulted?prepareNativeResourceRegistration(o,domain,o.root,request,profile,g):null;
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function inspectInvocationResourceRegistrationPreparation(domain,token,g){const o=domains.get(domain);return o&&!o.faulted?readNativeResourceRegistration(token,o,domain,o.root,g):null;}
export function installInvocationResourceRegistration(domain,token,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=domains.get(domain);if(!o||o.faulted)return unavailable();const before=o.root,p=readNativeResourceRegistration(token,o,domain,before,g);if(!p||size(p.next)>o.limits.maxRootBytes||o.baselineReservations&&!baselineReservationsAllow(o.baselineReservations,p.next))return unavailable();freeze(p.next);const result=commitNativeResourceRegistration(token);if(!result)return unavailable();if(p.next!==before)carryRetainedLifecycleSourceJournal(before,p.next);o.root=p.next;return result;
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function publishInvocationResourceRegistration(domain,request,profile,g){const token=prepareInvocationResourceRegistration(domain,request,profile,g);if(!token)return unavailable();try{return installInvocationResourceRegistration(domain,token,g);}finally{discardNativeResourceRegistration(token);}}
export function prepareInvocationRelationLifecycle(domain,request,profile,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return null;try{const o=domains.get(domain);return o&&!o.faulted?prepareNativeRelationLifecycle(o,domain,o.root,request,profile,g):null;
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function inspectInvocationRelationLifecyclePreparation(domain,token,g){const o=domains.get(domain);return o&&!o.faulted?readNativeRelationLifecycle(token,o,domain,o.root,g):null;}
export function installInvocationRelationLifecycle(domain,token,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=domains.get(domain);if(!o||o.faulted)return unavailable();const before=o.root,p=readNativeRelationLifecycle(token,o,domain,before,g);if(!p||size(p.next)>o.limits.maxRootBytes||o.baselineReservations&&!baselineReservationsAllow(o.baselineReservations,p.next))return unavailable();freeze(p.next);const result=commitNativeRelationLifecycle(token);if(!result)return unavailable();carryRetainedLifecycleSourceJournal(before,p.next);o.root=p.next;return result;
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function publishInvocationRelationLifecycle(domain,request,profile,g){const token=prepareInvocationRelationLifecycle(domain,request,profile,g);if(!token)return unavailable();try{return installInvocationRelationLifecycle(domain,token,g);}finally{discardNativeRelationLifecycle(token);}}
export function publishInvocationAnnotationRestoration(domain,plan,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{
 const o=domains.get(domain);if(!o||o.faulted)return unavailable();const before=o.root,candidate=annotationRestorationCandidate(domain,before,o.context,plan,g);if(!candidate||size(candidate.state)>o.limits.maxRootBytes||o.baselineReservations&&!baselineReservationsAllow(o.baselineReservations,candidate.state))return unavailable();
 if(candidate.status==='unchanged'){if(candidate.state!==before)return unavailable();}
 else if(candidate.status==='restored'){
  const ds=before.documents.filter(d=>equal(d.document,candidate.document)),ns=candidate.state?.documents?.filter(d=>equal(d.document,candidate.document)),d=ds[0],n=ns?.[0];if(ds.length!==1||ns?.length!==1||d.records.length!==0||d.annotationLifecycle.entries.length!==0||!natural(candidate.state.publication)||candidate.state.publication!==before.publication+1||!natural(n.annotationSet)||n.annotationSet!==d.annotationSet+1)return unavailable();
  const frame=copy(candidate.state),f=frame.documents.find(d=>equal(d.document,candidate.document));frame.publication=before.publication;f.records=copy(d.records);f.annotationLifecycle=copy(d.annotationLifecycle);f.annotationSet=d.annotationSet;if(!equal(frame,before))return unavailable();
  freeze(candidate.state);carryRestoredAnnotationSourceJournal(before,candidate.state,candidate.document);o.root=candidate.state;
 }else return unavailable();discardAnnotationRestoration(plan);return {status:candidate.status};

 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
// Trusted coordinator facets. These do not expose roots or acknowledge writes.
export function openInvocationBaselineReservations(domain,limits,g){
 const o=domains.get(domain);if(!o||o.faulted||o.baselineReservations||!limits||typeof limits!=='object'||Array.isArray(limits))return null;
 // Snapshot the declared native ceiling once. The retained local pool keeps its
 // own accounting; its entire ceiling must also fit the actual parent domain.
 const captured={...limits},attachment=beginInvocationOwnerAttachment(domain,'baseline-reservations',captured?.maxHeldBytes,Object.freeze({kind:'annotation-baseline-reservation-pool/1'}));if(!attachment)return null;
 try{if(o.baselineReservations)return null;const pool=openBaselineReservationPool(o.context,o.root,o.limits.maxRootBytes,captured,g);if(!pool)return null;
  if(!commitInvocationOwnerAttachment(domain,attachment,pool))throw Error('native baseline reservation attachment invariant');o.baselineReservations=pool;return pool;
 }finally{abortInvocationOwnerAttachment(domain,attachment);}
}
export function reserveInvocationBaseline(domain,pool,document,envelope,g){
 const section=beginInvocationAdmissionMutation(domain);if(!section)return unavailable();try{const o=domains.get(domain);return !o||o.faulted||o.baselineReservations!==pool?unavailable():reserveBaselineCapacity(pool,o.root,document,envelope,g);}finally{endInvocationAdmissionMutation(domain,section);}
}
export function reserveInvocationWriteBaseline(domain,pool,claim,g){
 const o=domains.get(domain);if(!o||o.faulted||o.baselineReservations!==pool)return unavailable();
 const retained=reserveBaselineWriteCapacity(pool,o.root,domain,claim,g,false);if(retained.status==='reserved')return retained;
 const section=beginInvocationAdmissionMutation(domain);if(!section)return unavailable();
 try{return reserveBaselineWriteCapacity(pool,o.root,domain,claim,g);}finally{endInvocationAdmissionMutation(domain,section);}
}
export function prepareInvocationOrdinaryBaseline(domain,pool,claim,g,section){const o=domains.get(domain);return !section||!o||o.faulted||o.admissionMutation!==section||o.admissionFence||o.baselineReservations!==pool?null:prepareOrdinaryBaselineCapacity(pool,o.root,domain,claim,g);}
export function validateInvocationOrdinaryBaseline(domain,pool,claim,stage,g,section){const o=domains.get(domain);return !!section&&!!o&&!o.faulted&&o.admissionMutation===section&&!o.admissionFence&&o.baselineReservations===pool&&validateOrdinaryBaselineCapacity(pool,o.root,domain,claim,stage,g);}
export function commitInvocationOrdinaryBaseline(domain,pool,claim,stage,g,section){const o=domains.get(domain);if(!section||!o||o.faulted||o.admissionMutation!==section||o.admissionFence||o.baselineReservations!==pool)return null;const reservation=commitOrdinaryBaselineCapacity(pool,o.root,domain,claim,stage,g);if(!reservation)o.faulted=true;return reservation;}
export function discardInvocationOrdinaryBaseline(stage){return discardOrdinaryBaselineCapacity(stage);}
export function installInvocationAnnotationBaseline(domain,pool,claim,token,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain,true);if(!admissionSection)return unavailable();try{
 const o=domains.get(domain);if(!o||o.faulted||o.baselineReservations!==pool)return unavailable();const before=o.root,stage=stageAnnotationBaselineInstallation(pool,domain,before,claim,token,g);if(!stage)return unavailable();
 freeze(stage.next);if(stage.next!==before)carryBaselineSourceJournal(before,stage.next,stage.document);if(!commitAnnotationBaselineInstallation(pool,domain,before,claim,token,stage))return unavailable();o.root=stage.next;return {status:stage.status};

 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function installInvocationLoadedAnnotationBaseline(domain,token,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain,true);if(!admissionSection)return unavailable();try{
 const o=domains.get(domain),c=readLoadedAnnotationBaseline(token,domain);if(!o||o.faulted||!c||o.root!==c.before||!annotationLoadedBaselineGates.every(k=>g?.[k]===true)||size(c.next)>o.limits.maxRootBytes||o.baselineReservations&&!baselineReservationsAllow(o.baselineReservations,c.next))return unavailable();
 const ds=c.before.documents.filter(d=>equal(d.document,c.document)),ns=c.next.documents.filter(d=>equal(d.document,c.document)),a=ds[0],b=ns[0];if(ds.length!==1||ns.length!==1||a.live!==true||!equal(a.ordinaryBindings?.annotations,c.binding)||!equal(b.annotationBaseline,c.baseline)||!Object.hasOwn(a,'annotationBaseline')&&g.annotationBaselineAbsentAttested!==true)return unavailable();
 if(c.next===c.before){if(!equal(a.annotationBaseline,c.baseline))return unavailable();}
 else {if(g.versionsAvailable!==true||!natural(c.next.publication)||c.next.publication!==c.before.publication+1)return unavailable();const frame=copy(c.next),d=frame.documents.find(d=>equal(d.document,c.document));frame.publication=c.before.publication;if(Object.hasOwn(a,'annotationBaseline'))d.annotationBaseline=copy(a.annotationBaseline);else delete d.annotationBaseline;if(!equal(frame,c.before))return unavailable();}
 freeze(c.next);if(!completeLoadedAnnotationBaseline(token,domain))return unavailable();if(c.next!==c.before)carryBaselineSourceJournal(c.before,c.next,c.document);o.root=c.next;return {status:c.next===c.before?'current':'installed'};

 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function releaseInvocationBaseline(domain,pool,reservation,g){const o=domains.get(domain);return !o||o.baselineReservations!==pool?unavailable():releaseBaselineCapacity(pool,reservation,g);}
export function releaseInvocationIneligibleBaseline(domain,pool,claim,g){const o=domains.get(domain);return !o||o.baselineReservations!==pool?unavailable():releaseIneligibleBaselineCapacity(pool,domain,claim,g);}
export function stopInvocationBaselineReservations(domain,pool,g){const o=domains.get(domain);return !o||o.baselineReservations!==pool?unavailable():stopBaselineReservations(pool,g);}
export function invocationBaselineReservationUsage(domain,pool){const o=domains.get(domain);return !o||o.baselineReservations!==pool?null:baselineReservationUsage(pool);}
export function invocationExecutionState(domain,ticket,c,includeOutcome=true,maxPreparedBytes=Number.MAX_SAFE_INTEGER){const o=domains.get(domain),e=o?.slots.get(ticket);if(!entryCurrent(o,e,c)||!['pending','running','done'].includes(e.phase)||includeOutcome&&e.prepared&&(!natural(maxPreparedBytes)||size(e.prepared)>maxPreparedBytes))return null;return freeze({reference:e.reference,phase:e.phase==='done'?'terminal':'pending',policy:copy(e.policy),prepared:includeOutcome&&e.prepared?copy(e.prepared):null,maxResultBytes:o.limits.maxResultBytes});}
// Native composition wrappers; only these obtain the actual private owner record.
export function openInvocationSettingsReads(domain,p,g){return openAttachedNativePool(domain,'settings-reads','engine-settings-read-pool/1',o=>openNativeSettingsReads(o,domain,p,g));}
export function prepareInvocationSettingsRead(domain,handle,c,facts,basis,g){const o=domains.get(domain);return o?prepareNativeSettingsRead(o,domain,handle,c,facts,basis,g):null;}
export function acquireInvocationSettingsRead(domain,handle,c,packet,g){
 const admissionDomain=domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=domains.get(domain);return o?acquireNativeSettingsRead(o,domain,handle,c,packet,g):unavailable();
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function queryInvocationSettingsRead(domain,handle,envelope,c,packet,g){const o=domains.get(domain);return o?queryNativeSettingsRead(o,domain,handle,envelope,c,packet,g):unavailable();}
export function queryDecodedInvocationSettingsRead(domain,handle,envelope,inputLimits,c,packet,g){const o=domains.get(domain);return o?queryDecodedNativeSettingsRead(o,domain,handle,envelope,inputLimits,c,packet,g):unavailable();}
export function invocationSettingsReadDecodeUsage(domain,handle){const o=domains.get(domain);return o?nativeSettingsReadDecodeUsage(o,domain,handle):null;}
export function discardInvocationSettingsReadCut(domain,handle,packet,g){const o=domains.get(domain);return o?discardNativeSettingsReadCut(o,domain,handle,packet,g):false;}
export function stopInvocationSettingsRead(domain,handle,reference,c,g){const o=domains.get(domain);return o?stopNativeSettingsRead(o,domain,handle,reference,c,g):false;}
export function retireInvocationSettingsRead(domain,handle,reference,g){const o=domains.get(domain);return o?retireNativeSettingsRead(o,domain,handle,reference,g):false;}
export function retireInvocationSettingsReads(domain,handle,g){return retireAttachedNativePool(domain,handle,'settings-reads',g,(o,facts)=>retireNativeSettingsReads(o,domain,handle,facts));}
export function invocationSettingsReadUsage(domain){const o=domains.get(domain);return o?nativeSettingsReadUsage(o):null;}
export function invocationUsage(domain){const o=domains.get(domain);return o?{issued:o.issued,starts:o.starts,slots:o.slots.size,phases:[...o.slots.values()].map(e=>e.phase),faulted:o.faulted,declaredHeldBytes:o.charge,ordinaryArgumentBytes:[...o.slots.values()].reduce((n,e)=>n+(e.ordinaryCharge??0),0),retainedReplacementResultBytes:[...o.slots.values()].reduce((n,e)=>n+(e.replacementResultCharge??0),0),retainedAnnotationReceipts:[...o.slots.values()].filter(e=>e.annotationReceipt).length,annotationReceiptBytes:[...o.slots.values()].reduce((n,e)=>n+(e.annotationReceipt?size(e.annotationReceipt):0),0),retainedReversalReceipts:[...o.slots.values()].filter(e=>e.reversalReceipt).length,reversalReceiptBytes:[...o.slots.values()].reduce((n,e)=>n+(e.reversalReceipt?size(e.reversalReceipt):0),0)}:null;}

export function prepareInvocationConfiguredLine(domain,ticket,request,profile,c,g){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.settingsPrepared||entry.registrationPrepared||entry.lifecyclePrepared||entry.relationPrepared||!gates(g,catalog.admissionGates))return null;const plan=withAttachedNativeSettingsPool(domain,()=>prepareConfiguredLine(o,domain,ticket,entry.reference,o.root,request,profile,c,g,entry.configuredLinePrepared),!entry.configuredLinePrepared);if(plan)entry.configuredLinePrepared=plan;return plan;}

export function captureInvocationSettingsLifecycleBasis(domain,c,p,g){const o=domains.get(domain);return o?withAttachedNativeSettingsPool(domain,()=>captureNativeEngineSettingsLifecycleBasis(o,domain,c,p,g),true):null;}
export function openInvocationSerializationBasisPool(domain,p,g){return Boolean(openAttachedNativePool(domain,'serialization','source-serialization-basis-pool/1',o=>openSerializationBasisPool(o,domain,p,g)?nativeSerializationBasisPoolHandle(o,domain):null));}
export function prepareInvocationScopedSourceSerialization(domain,ticket,envelope,basis,profile,c,h){
 const admissionSection=beginInvocationAdmissionMutation(domain,Boolean(domains.get(domain)?.slots.get(ticket)?.settingsPrepared));if(!admissionSection)return null;try{const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.lifecyclePrepared||entry.relationPrepared||entry.registrationPrepared||entry.configuredLinePrepared||!gates(h,catalog.admissionGates))return null;const plan=prepareScopedSourceSerialization(o,domain,ticket,entry.reference,o.root,envelope,basis,c,profile,h,entry.settingsPrepared);if(plan)entry.settingsPrepared=plan;return plan;
 }finally{endInvocationAdmissionMutation(domain,admissionSection);}
}
export function captureInvocationSerializationBasis(domain,document,c,g){
 const admissionSection=beginInvocationAdmissionMutation(domain);if(!admissionSection)return null;try{const o=domains.get(domain);return o?captureSerializationBasis(o,domain,document,c,g):null;
 }finally{endInvocationAdmissionMutation(domain,admissionSection);}
}
export function inspectInvocationSerializationBasis(domain,basis,c,g){const o=domains.get(domain);return o?inspectSerializationBasis(o,domain,basis,c,g):unavailable();}
export function resolveInvocationSerializationBasis(domain,basis,c,g){const o=domains.get(domain);return o?resolveSerializationBasis(o,domain,basis,c,g):null;}
export function stopInvocationSerializationBasis(domain,basis,c,g){const o=domains.get(domain);return o?stopSerializationBasis(o,domain,basis,c,g):false;}
export function retireInvocationSerializationBasis(domain,basis,c,g){const o=domains.get(domain);return o?releaseSerializationBasis(o,domain,basis,c,g):false;}
export function retireInvocationSerializationBasisPool(domain,g){const o=domains.get(domain),handle=o?nativeSerializationBasisPoolHandle(o,domain):null;return retireAttachedNativePool(domain,handle,'serialization',g,(owner,facts)=>retireSerializationBasisPool(owner,domain,facts));}
export function invocationSerializationBasisUsage(domain){const o=domains.get(domain);return o?serializationBasisUsage(o):null;}
export function prepareInvocationScopedSettingsLifecycle(domain,ticket,envelope,basis,p,c,h){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.lifecyclePrepared||entry.relationPrepared||entry.registrationPrepared||entry.configuredLinePrepared||!gates(h,catalog.admissionGates))return null;const plan=withAttachedNativeSettingsPool(domain,()=>prepareScopedNativeEngineSettingsLifecycle(o,domain,ticket,entry.reference,o.root,envelope,basis,c,p,h,entry.settingsPrepared),!entry.settingsPrepared);if(plan)entry.settingsPrepared=plan;return plan;}
export function prepareInvocationBasisSettingsLifecycle(domain,ticket,envelope,lifecycleBasis,writeBases,p,c,h){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.lifecyclePrepared||entry.relationPrepared||entry.registrationPrepared||entry.configuredLinePrepared||!gates(h,catalog.admissionGates))return null;const plan=withAttachedNativeSettingsPool(domain,()=>prepareBasisNativeEngineSettingsLifecycle(o,domain,ticket,entry.reference,o.root,envelope,lifecycleBasis,writeBases,c,p,h,entry.settingsPrepared),!entry.settingsPrepared);if(plan)entry.settingsPrepared=plan;return plan;}
export function inspectInvocationSettingsLifecycleBasis(domain,basis,c,g){const o=domains.get(domain);return o?inspectNativeEngineSettingsLifecycleBasis(o,domain,basis,c,g):unavailable();}
export function resolveInvocationSettingsLifecycleBasis(domain,basis,c,g){const o=domains.get(domain);return o?resolveNativeEngineSettingsLifecycleBasis(o,domain,basis,c,g):null;}
export function stopInvocationSettingsLifecycleBasis(domain,basis,c,g){const o=domains.get(domain);return o?stopNativeEngineSettingsLifecycleBasis(o,domain,basis,c,g):false;}
export function retireInvocationSettingsLifecycleBasis(domain,basis,c,g){const o=domains.get(domain);return o?releaseNativeEngineSettingsLifecycleBasis(o,domain,basis,c,g):false;}
export function captureInvocationSettingsDefinitionBasis(domain,c,p,g){const o=domains.get(domain);return o?withAttachedNativeSettingsPool(domain,()=>captureNativeEngineSettingsDefinitionBasis(o,domain,c,p,g),true):null;}
export function prepareInvocationScopedSettingsDefinition(domain,ticket,envelope,basis,p,c,h){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.lifecyclePrepared||entry.relationPrepared||entry.registrationPrepared||entry.configuredLinePrepared||!gates(h,catalog.admissionGates))return null;const plan=withAttachedNativeSettingsPool(domain,()=>prepareScopedNativeEngineSettingsDefinition(o,domain,ticket,entry.reference,o.root,envelope,basis,c,p,h,entry.settingsPrepared),!entry.settingsPrepared);if(plan)entry.settingsPrepared=plan;return plan;}
export function inspectInvocationSettingsDefinitionBasis(domain,basis,c,g){const o=domains.get(domain);return o?inspectNativeEngineSettingsDefinitionBasis(o,domain,basis,c,g):unavailable();}
export function resolveInvocationSettingsDefinitionBasis(domain,basis,c,g){const o=domains.get(domain);return o?resolveNativeEngineSettingsDefinitionBasis(o,domain,basis,c,g):null;}
export function stopInvocationSettingsDefinitionBasis(domain,basis,c,g){const o=domains.get(domain);return o?stopNativeEngineSettingsDefinitionBasis(o,domain,basis,c,g):false;}
export function retireInvocationSettingsDefinitionBasis(domain,basis,c,g){const o=domains.get(domain);return o?releaseNativeEngineSettingsDefinitionBasis(o,domain,basis,c,g):false;}
export function captureInvocationSettingsWriteBasis(domain,layer,c,p,g){const o=domains.get(domain);return o?withAttachedNativeSettingsPool(domain,()=>captureNativeEngineSettingsWriteBasis(o,domain,layer,c,p,g),true):null;}
export function inspectInvocationSettingsWriteBasis(domain,basis,c,g){const o=domains.get(domain);return o?inspectNativeEngineSettingsWriteBasis(o,domain,basis,c,g):unavailable();}
// Trusted first-party lowering only, not a recipient disclosure endpoint.
export function resolveInvocationSettingsWriteBasis(domain,basis,c,g){const o=domains.get(domain);return o?resolveNativeEngineSettingsWriteBasis(o,domain,basis,c,g):null;}
export function stopInvocationSettingsWriteBasis(domain,basis,c,g){const o=domains.get(domain);return o?stopNativeEngineSettingsWriteBasis(o,domain,basis,c,g):false;}
export function retireInvocationSettingsWriteBasis(domain,basis,c,g){const o=domains.get(domain);return o?releaseNativeEngineSettingsWriteBasis(o,domain,basis,c,g):false;}
export function prepareInvocationScopedSemanticSettings(domain,ticket,envelope,bases,p,c,h){const o=domains.get(domain),entry=o?.slots.get(ticket);if(entry?.ordinaryPrepared||entry?.ordinaryPreparing)return null;if(!entryCurrent(o,entry,c)||o.faulted||entry.phase!=='issued'||entry.lifecyclePrepared||entry.relationPrepared||entry.registrationPrepared||entry.configuredLinePrepared||!gates(h,catalog.admissionGates))return null;const plan=withAttachedNativeSettingsPool(domain,()=>prepareScopedNativeEngineSettings(o,domain,ticket,entry.reference,o.root,envelope,bases,c,p,h,entry.settingsPrepared),!entry.settingsPrepared);if(plan)entry.settingsPrepared=plan;return plan;}

const ordinaryPlans=new WeakMap();
const ordinaryPermissionFields=Object.freeze(['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','sourceReadable','viewWritable','bounded','companionsReady','sourceWritable','allTargetsWritable']);
function ordinaryPermissions(g,kind){return gates(g,ordinaryPermissionFields)&&(kind!=='boundary'||gates(g,['boundaryLive','boundaryRetained']));}
function ordinaryPreparation(domain,ticket,envelope,inputLimits,c,g,semanticProfile){
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(!o||!entry||o.faulted||entry.ordinaryPreparing||!['issued','pending','done'].includes(entry.phase))return null;
 entry.ordinaryPreparing=true;
 try{
  const normalized=normalizeOrdinaryEditArguments(envelope,c,inputLimits,o.limits.maxIntentBytes);if(!normalized||!entryCurrent(o,entry,normalized.record.context)||normalized.record.request.invocation!==entry.reference||!gates(g,catalog.admissionGates)||!gates(g,['outsidePublicationGuard','preparationReserved']))return null;
  if(!normalized.configured&&semanticProfile!==null)return null;
  const cap={...normalized.record.limits,maxRequestBytes:Math.min(o.limits.maxIntentBytes,normalized.record.limits.maxRequestBytes)},capture=copySettingsData({arguments:normalized.record,semanticProfile:normalized.configured?semanticProfile:null},cap);if(!capture)return null;
  if(entry.ordinaryPrepared){const held=entry.ordinaryPrepared,original={kind:held.job.kind,configured:held.configured,record:held.capture.arguments,boundary:held.boundary};return sameOrdinaryEditArguments(original,normalized)&&equal(held.capture,capture.value)?{job:held.job,route:normalized.route}:null;}
  if(entry.phase!=='issued'||entry.settingsPrepared||entry.registrationPrepared||entry.lifecyclePrepared||entry.relationPrepared||entry.configuredLinePrepared||!ordinaryPermissions(g,normalized.kind))return null;
  if(!natural(capture.bytes)||capture.bytes>o.limits.maxHeldBytes-o.charge)return null;
  entry.ordinaryCharge=capture.bytes;o.charge+=capture.bytes;
  const request=ordinaryEditProducerRequest(normalized),before=o.root,context=normalized.record.context;
  const plan=normalized.configured?withAttachedNativeSettingsPool(domain,()=>prepareConfiguredLine(o,domain,ticket,entry.reference,before,{route:normalized.route,request:{...(normalized.route==='typed'?{family:10}:{name:'change-source'}),variant:'ordinary-edit',version:1,arguments:request}},capture.value.semanticProfile,context,g),true):normalized.kind==='selection'?prepareSelectionEdit(before,request,g,g.viewLimits):normalized.kind==='line'?prepareLineEdit(before,request,g,g.viewLimits,g.lineWorkLimits):prepareBoundaryCommand(before,request,g,g.viewLimits,g.boundaryWorkLimits);
  if(!plan)return null;if(normalized.configured)entry.configuredLinePrepared=plan;
  const native=captureEditingIntent(normalized.kind,plan,o.limits.maxIntentBytes,o,domain,ticket);if(!native||size({kind:normalized.kind,plan:native.value})>o.limits.maxIntentBytes){if(normalized.configured)o.faulted=true;return null;}
  const job=Object.freeze({kind:normalized.kind,plan});entry.ordinaryPrepared={job,capture:capture.value,boundary:normalized.boundary,configured:normalized.configured};ordinaryPlans.set(plan,{domain,ticket});return {job,route:normalized.route};
 }catch{o.faulted=true;return null;}finally{if(entry.ordinaryCharge&&!entry.ordinaryPrepared&&!entry.configuredLinePrepared){o.charge-=entry.ordinaryCharge;entry.ordinaryCharge=0;}entry.ordinaryPreparing=false;}
}
export function prepareInvocationOrdinaryEdit(domain,ticket,envelope,inputLimits,c,g,semanticProfile=null){return ordinaryPreparation(domain,ticket,envelope,inputLimits,c,g,semanticProfile)?.job??null;}
export function submitInvocationOrdinaryEdit(domain,ticket,envelope,inputLimits,c,p,g,semanticProfile=null){const prepared=ordinaryPreparation(domain,ticket,envelope,inputLimits,c,g,semanticProfile);return prepared?submitInvocation(domain,ticket,prepared.job,prepared.route,c,p,g):unavailable();}
function compositionPreparation(domain,ticket,envelope,inputLimits,c,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(!o||!entry||o.faulted||entry.ordinaryPreparing||!['issued','pending','done'].includes(entry.phase))return null;
 entry.ordinaryPreparing=true;
 try{
  const n=normalizeCompositionArguments(envelope,c,inputLimits,o.limits.maxIntentBytes);if(!n||!entryCurrent(o,entry,n.record.context)||n.record.request.invocation!==entry.reference||!gates(g,catalog.admissionGates)||!gates(g,['outsidePublicationGuard','preparationReserved']))return null;
  const cap={...n.record.limits,maxRequestBytes:Math.min(o.limits.maxIntentBytes,n.record.limits.maxRequestBytes)},capture=copySettingsData({arguments:n.record,semanticProfile:null},cap);if(!capture)return null;
  if(entry.ordinaryPrepared){const held=entry.ordinaryPrepared;return held.inputFamily==='F11-composition'&&sameCompositionArguments({record:held.capture.arguments},n)&&equal(held.capture,capture.value)?{job:held.job,route:n.route}:null;}
  if(entry.phase!=='issued'||entry.settingsPrepared||entry.registrationPrepared||entry.lifecyclePrepared||entry.relationPrepared||entry.configuredLinePrepared||!ordinaryPermissions(g,'selection')||!natural(capture.bytes)||capture.bytes>o.limits.maxHeldBytes-o.charge)return null;
  entry.ordinaryCharge=capture.bytes;o.charge+=capture.bytes;
  const plan=prepareCompositionCommit(o.root,n.record.request,g,g.viewLimits);if(!plan)return null;
  const native=captureEditingIntent('selection',plan,o.limits.maxIntentBytes,o,domain,ticket);if(!native||size({kind:'selection',plan:native.value})>o.limits.maxIntentBytes){discardSelectionEdit(plan,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true});return null;}
  const job=Object.freeze({kind:'selection',plan});entry.ordinaryPrepared={job,capture:capture.value,boundary:null,configured:false,inputFamily:'F11-composition'};ordinaryPlans.set(plan,{domain,ticket});return {job,route:n.route};
 }catch{o.faulted=true;return null;}finally{if(entry.ordinaryCharge&&!entry.ordinaryPrepared){o.charge-=entry.ordinaryCharge;entry.ordinaryCharge=0;}entry.ordinaryPreparing=false;}
}
export function prepareInvocationComposition(domain,ticket,envelope,inputLimits,c,g){return compositionPreparation(domain,ticket,envelope,inputLimits,c,g)?.job??null;}
export function submitInvocationComposition(domain,ticket,envelope,inputLimits,c,p,g){const prepared=compositionPreparation(domain,ticket,envelope,inputLimits,c,g);return prepared?submitInvocation(domain,ticket,prepared.job,prepared.route,c,p,g):unavailable();}
const exactBatchPermissionFields=Object.freeze(['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','sourceReadable','sourceWritable','allTargetsWritable','bounded','companionsReady','historyReady']);
function exactBatchPermissions(g){return gates(g,exactBatchPermissionFields);}
function exactBatchPreparation(domain,ticket,envelope,inputLimits,c,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(!o||!entry||o.faulted||entry.ordinaryPreparing||!['issued','pending','done'].includes(entry.phase))return null;
 entry.ordinaryPreparing=true;
 try{
  const n=normalizeExactBatchArguments(envelope,c,inputLimits,o.limits.maxIntentBytes);if(!n||!entryCurrent(o,entry,n.record.context)||n.record.request.invocation!==entry.reference||!gates(g,catalog.admissionGates)||!gates(g,['outsidePublicationGuard','preparationReserved']))return null;
  // A null shadow value is only a normalization placeholder, never current evidence.
  const r=n.record.request,shadow=normalizeCompoundAdmission({contract:'owned-annotation-plan/5',invocation:r.invocation,descriptor:r.invocation,sources:r.sources.map(s=>({...s,expectedAnnotationSet:null})),annotations:[],reads:r.reads,scopes:r.scopes},o.limits.maxIntentBytes);if(!shadow)return null;
  const record={...n.record,request:{...r,sources:shadow.sources.map(({expectedAnnotationSet,...s})=>s),reads:shadow.reads,scopes:shadow.scopes}};
  const cap={...record.limits,maxRequestBytes:Math.min(o.limits.maxIntentBytes,record.limits.maxRequestBytes)},capture=copySettingsData({arguments:record},cap);if(!capture)return null;
  if(entry.ordinaryPrepared){const held=entry.ordinaryPrepared;return held.inputFamily==='F11-exact-batch'&&sameExactBatchArguments({record:held.capture.arguments},{record})&&equal(held.capture,capture.value)?{job:held.job,route:n.route}:null;}
  if(entry.phase!=='issued'||entry.ordinaryPrepared||entry.settingsPrepared||entry.registrationPrepared||entry.lifecyclePrepared||entry.relationPrepared||entry.configuredLinePrepared||!exactBatchPermissions(g)||!natural(capture.bytes)||capture.bytes>o.limits.maxHeldBytes-o.charge)return null;
  entry.ordinaryCharge=capture.bytes;o.charge+=capture.bytes;
  const before=o.root,plan=copy(shadow);
  for(const source of plan.sources){
   const matches=before.documents.filter(d=>equal(d.document,source.document));if(matches.length!==1)return null;const d=matches[0];
   if(d.live===false||d.revision!==source.expectedSource||!natural(d.annotationSet))return null;
   try{validateCanonicalBatch(d.source,source.edits);}catch{return null;}
   if(source.edits.some(e=>!equal(e.expected,d.source.slice(e.lo,e.hi))))return null;
   source.expectedAnnotationSet=d.annotationSet;
  }
  const owned=freeze(copy(plan));if(size({kind:'compound',plan:owned})>o.limits.maxIntentBytes)return null;
  const job=Object.freeze({kind:'compound',plan:owned});entry.ordinaryPrepared={job,capture:capture.value,boundary:null,configured:false,inputFamily:'F11-exact-batch'};ordinaryPlans.set(owned,{domain,ticket});return {job,route:n.route};
 }catch{o.faulted=true;return null;}finally{if(entry.ordinaryCharge&&!entry.ordinaryPrepared){o.charge-=entry.ordinaryCharge;entry.ordinaryCharge=0;}entry.ordinaryPreparing=false;}
}
// Preparation remains private: callers never receive engine annotation-set stamps.
export function submitInvocationExactBatch(domain,ticket,envelope,inputLimits,c,p,g){const prepared=exactBatchPreparation(domain,ticket,envelope,inputLimits,c,g);return prepared?submitInvocation(domain,ticket,prepared.job,prepared.route,c,p,g):unavailable();}

function nativeLocalCommandHost(g,family){return family==='F13'?g?.annotationHost:g?.historyHost;}
function nativeLocalCommandPermissions(g,family,c){const h=nativeLocalCommandHost(g,family);return gates(g,['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady'])&&h&&h.owner&&typeof h.owner==='object'&&h.recipient===c.recipient&&h.profile===c.profile&&gates(h,family==='F13'?['authorized','bounded','notStopped','interpretationCurrent','companionsReady','historyReady','annotationWritable']:['authorized','bounded','notStopped','interpretationCurrent','companionsReady','versionsAvailable','historyReady','invocationFresh']);}
function nativeLocalCommandLimitBinding(family,h){return family==='F13'?{annotation:h.annotationLimits,membership:h.membershipLimits??null}:{history:h.limits,views:h.viewLimits};}
function nativeLocalCommandLimitsCurrent(held,g){return equal(held.nativeLimits,nativeLocalCommandLimitBinding(held.inputFamily,nativeLocalCommandHost(g,held.inputFamily)));}
function nativeLocalCommandPreparation(domain,ticket,envelope,inputLimits,c,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(!o||!entry||o.faulted||entry.ordinaryPreparing||!['issued','pending','done'].includes(entry.phase))return null;
 entry.ordinaryPreparing=true;
 try{
  const n=normalizeNativeLocalCommandArguments(envelope,c,inputLimits,o.limits.maxIntentBytes);if(!n||!entryCurrent(o,entry,n.record.context)||n.record.request.invocation!==entry.reference||!gates(g,catalog.admissionGates)||!gates(g,['outsidePublicationGuard','preparationReserved']))return null;
  const cap={...n.record.limits,maxRequestBytes:Math.min(o.limits.maxIntentBytes,n.record.limits.maxRequestBytes)},capture=copySettingsData({arguments:n.record},cap);if(!capture)return null;
  if(entry.ordinaryPrepared){const held=entry.ordinaryPrepared;return held.inputFamily===n.family&&equal(held.capture,capture.value)?{job:held.job,route:n.route}:null;}
  const charge=3*capture.bytes+2*o.limits.maxIntentBytes+2*o.limits.maxRootBytes,h=nativeLocalCommandHost(g,n.family);
  if(entry.phase!=='issued'||entry.settingsPrepared||entry.registrationPrepared||entry.lifecyclePrepared||entry.relationPrepared||entry.configuredLinePrepared||!nativeLocalCommandPermissions(g,n.family,c)||!natural(charge)||charge>o.limits.maxHeldBytes-o.charge)return null;
  const nativeLimits=nativeLocalCommandLimitBinding(n.family,h);
  entry.ordinaryCharge=charge;o.charge+=charge;
  const boundedLimits=captureNativeLocalCommandLimits(n.family,nativeLimits,cap);
  if(!boundedLimits)return null;
  // A larger native ceiling is not an oversized actual intent. Fund construction
  // before calling native builders; retain the original profile for currentness.
  const b=boundedLimits.value,nativeCeiling=n.family==='F13'?Math.max(b.annotation.maxPlanBytes,b.membership?.maxPlanBytes??0):Math.max(b.history.maxPlanBytes,b.history.maxViewPlanBytes),
   extra=2*Math.max(0,nativeCeiling-o.limits.maxIntentBytes);
  if(!natural(extra)||extra>o.limits.maxHeldBytes-o.charge)return null;
  entry.ordinaryCharge+=extra;o.charge+=extra;
  const plan=n.family==='F13'?prepareAnnotationEdit(o.root,n.record.request,h,h.annotationLimits):prepareReversal(o.root,n.record.request,h);if(!plan)return null;
  const captured=n.kind==='annotation'?captureEditingIntent('annotation',plan,o.limits.maxIntentBytes,o,domain,ticket):{value:plan};if(!captured||size({kind:n.kind,plan:captured.value})>o.limits.maxIntentBytes)return null;
  const job=Object.freeze({kind:n.kind,plan});entry.ordinaryPrepared={job,capture:capture.value,boundary:null,configured:false,inputFamily:n.family,nativeLimits:boundedLimits.value};ordinaryPlans.set(plan,{domain,ticket});return {job,route:n.route};
 }catch{o.faulted=true;return null;}finally{if(entry.ordinaryCharge&&!entry.ordinaryPrepared){o.charge-=entry.ordinaryCharge;entry.ordinaryCharge=0;}entry.ordinaryPreparing=false;}
}
// The existing F13/F14 argument/receipt meanings are unchanged. No raw plan is returned.
export function submitInvocationNativeLocalCommand(domain,ticket,envelope,inputLimits,c,p,g){const prepared=nativeLocalCommandPreparation(domain,ticket,envelope,inputLimits,c,g);return prepared?submitInvocation(domain,ticket,prepared.job,prepared.route,c,p,g):unavailable();}
function atomicAuthoringPermissions(g,sources){return gates(g,['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','bounded','companionsReady','historyReady'])&&(!sources.length||gates(g,['sourceReadable','sourceWritable','allTargetsWritable']));}
function atomicAuthoringPreparation(domain,ticket,envelope,inputLimits,c,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(!o||!entry||o.faulted||entry.ordinaryPreparing||!['issued','pending','done'].includes(entry.phase))return null;
 entry.ordinaryPreparing=true;
 try{
  const n=normalizeAtomicAuthoringArguments(envelope,c,inputLimits,o.limits.maxIntentBytes);if(!n||!entryCurrent(o,entry,n.record.context)||n.record.request.invocation!==entry.reference||!gates(g,catalog.admissionGates)||!gates(g,['outsidePublicationGuard','preparationReserved']))return null;
  const r=n.record.request,shadow=normalizeCompoundAdmission({contract:'owned-annotation-plan/5',invocation:r.invocation,descriptor:r.invocation,sources:r.sources.map(s=>({...s,expectedAnnotationSet:null})),annotations:[],reads:r.reads,scopes:r.scopes},o.limits.maxIntentBytes);if(!shadow)return null;
  const record={...n.record,request:{...r,sources:shadow.sources.map(({expectedAnnotationSet,...s})=>s),reads:shadow.reads,scopes:shadow.scopes}},cap={...record.limits,maxRequestBytes:Math.min(o.limits.maxIntentBytes,record.limits.maxRequestBytes)},capture=copySettingsData({arguments:record},cap);if(!capture)return null;
  if(entry.ordinaryPrepared){const held=entry.ordinaryPrepared;return held.inputFamily==='F11-F13-mixed'&&equal(held.capture,capture.value)?{job:held.job,route:n.route}:null;}
  const baseCharge=3*capture.bytes+2*o.limits.maxIntentBytes+2*o.limits.maxRootBytes;
  if(entry.phase!=='issued'||entry.settingsPrepared||entry.registrationPrepared||entry.lifecyclePrepared||entry.relationPrepared||entry.configuredLinePrepared||!atomicAuthoringPermissions(g,shadow.sources)||!natural(baseCharge)||baseCharge>o.limits.maxHeldBytes-o.charge)return null;
  entry.ordinaryCharge=baseCharge;o.charge+=baseCharge;
  const sourcePlan=copy(shadow);
  for(const source of sourcePlan.sources){
   const matches=o.root.documents.filter(d=>equal(d.document,source.document));if(matches.length!==1)return null;const d=matches[0];if(d.live===false||d.revision!==source.expectedSource||!natural(d.annotationSet))return null;
   try{validateCanonicalBatch(d.source,source.edits);}catch{return null;}if(source.edits.some(e=>!equal(e.expected,d.source.slice(e.lo,e.hi))))return null;source.expectedAnnotationSet=d.annotationSet;
  }
  const bundle=prepareAtomicAuthoringIntent(o,domain,ticket,o.root,sourcePlan,r.annotations,c,g,o.limits.maxIntentBytes,o.limits.maxRootBytes);if(!bundle)return null;
  const owned=freeze(bundle.plan),job=Object.freeze({kind:'compound',plan:owned});if(size(job)>o.limits.maxIntentBytes)return null;
  const receipts=prepareAtomicAnnotationResultHold(o,domain,ticket,owned,c,g,o.limits.maxIntentBytes,o.limits.maxResultBytes,o.limits.maxHeldBytes-o.charge);if(!receipts)return null;
  entry.ordinaryCharge+=receipts.charge;o.charge+=receipts.charge;
  entry.ordinaryPrepared={job,capture:capture.value,boundary:null,configured:false,inputFamily:'F11-F13-mixed',mixedBundle:bundle.token,atomicReceiptHold:receipts.hold};ordinaryPlans.set(owned,{domain,ticket});return {job,route:n.route};
 }catch{o.faulted=true;return null;}finally{if(entry.ordinaryCharge&&!entry.ordinaryPrepared){o.charge-=entry.ordinaryCharge;entry.ordinaryCharge=0;}entry.ordinaryPreparing=false;}
}
// No private plan, captured companion stamp or create allocation is returned.
export function submitInvocationAtomicAuthoring(domain,ticket,envelope,inputLimits,c,p,g){const prepared=atomicAuthoringPreparation(domain,ticket,envelope,inputLimits,c,g);return prepared?submitInvocation(domain,ticket,prepared.job,prepared.route,c,p,g):unavailable();}
// First-party preparation port for a validated private compound, not public ingress.
// The public mixed facade separately validates every authored participant.
export function prepareInvocationAtomicAnnotationReceipts(domain,ticket,plan,c,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket);
 if(!entryCurrent(o,entry,c)||o.faulted||entry.ordinaryPreparing||!['issued','pending','done'].includes(entry.phase)||!gates(g,catalog.admissionGates)||!gates(g,['outsidePublicationGuard','preparationReserved']))return null;
 entry.ordinaryPreparing=true;
 try{
  const normalized=normalizeCompoundAdmission(plan,o.limits.maxIntentBytes);if(!normalized||normalized.invocation!==entry.reference)return null;
  if(entry.ordinaryPrepared){const held=entry.ordinaryPrepared;return held.atomicReceiptHold&&atomicAnnotationResultPlanMatches(held.atomicReceiptHold,o,domain,ticket,normalized)?held.job:null;}
  if(entry.phase!=='issued'||entry.settingsPrepared||entry.registrationPrepared||entry.lifecyclePrepared||entry.relationPrepared||entry.configuredLinePrepared)return null;
  const retained=prepareAtomicAnnotationResultHold(o,domain,ticket,normalized,c,g,o.limits.maxIntentBytes,o.limits.maxResultBytes,o.limits.maxHeldBytes-o.charge);if(!retained)return null;
  const owned=freeze(normalized),job=Object.freeze({kind:'compound',plan:owned});
  entry.ordinaryCharge=retained.charge;o.charge+=retained.charge;
  entry.ordinaryPrepared={job,capture:{arguments:{request:owned,context:copy(c),limits:{}}},boundary:null,configured:false,inputFamily:'F11-F13-private-receipts',atomicReceiptHold:retained.hold};
  ordinaryPlans.set(owned,{domain,ticket});return job;
 }catch{o.faulted=true;return null;}finally{entry.ordinaryPreparing=false;}
}
// Trusted engine inspection, not recipient discovery or a raw content API.
export function invocationAtomicAnnotationResultUsage(domain,ticket,c){const o=domains.get(domain),entry=o?.slots.get(ticket),held=entry?.ordinaryPrepared;if(!entryCurrent(o,entry,c)||!held?.atomicReceiptHold)return null;const usage=atomicAnnotationResultHoldUsage(held.atomicReceiptHold,o,domain,ticket);return usage?{...usage,retainedReceipts:entry.atomicAnnotationReceipts?.length??0}:null;}
// Native internal result port; it does not select U01's public retrieval route.
export function observeAtomicAnnotationResult(domain,ticket,index,c,p,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket),held=entry?.ordinaryPrepared;
 if(!entryCurrent(o,entry,c)||!held?.atomicReceiptHold||entry.phase!=='done'||!policyShape(p)||!gates(g,catalog.releaseGates))return unavailable();
 const summary=releaseOutcome(entry.prepared,p);if(summary.observation!=='available'||!['applied','unchanged','notApplied'].includes(summary.publication))return unavailable();
 return releaseAtomicAnnotationResult(held.atomicReceiptHold,o,domain,ticket,entry.atomicAnnotationReceipts,index,g);
}
const selectionPermissionFields=Object.freeze(['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','sourceReadable','viewWritable','bounded','companionsReady']);
// A prepared plan is separately charged, not a new read-context lease. These
// selected managed consumers still depend on their original result's currentness.
// Failure settles not-applied; it neither cancels the invocation nor releases it.
function nativeSearchConsumerCurrent(o,domain,held,g){
 if(held.inputFamily==='F11')return nativeSourceSearchAllowsPreparation(o,domain,held.result,g.searchCurrent,g);
 if(held.inputFamily==='F12'&&held.capture.arguments.request.operation==='select-search-match')return nativeSourceSearchAllowsSelectionPreparation(o,domain,held.opaque,g.searchCurrent,g);
 return true;
}
function selectionPermissions(g,kind,request){return gates(g,selectionPermissionFields)&&(kind!=='boundary'||gates(g,['boundaryLive','boundaryRetained']))&&(request.operation!=='select-search-match'||gates(g.searchCurrent,['grant','scope','continuity','capacity','retained','notStopped']));}
function selectionPreparation(domain,ticket,envelope,inputLimits,c,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(!o||!entry||o.faulted||entry.ordinaryPreparing||!['issued','pending','done'].includes(entry.phase))return null;
 entry.ordinaryPreparing=true;
 try{
  const n=normalizeSelectionArguments(envelope,c,inputLimits,o.limits.maxIntentBytes);if(!n||!entryCurrent(o,entry,n.record.context)||n.record.request.invocation!==entry.reference||!gates(g,catalog.admissionGates)||!gates(g,['outsidePublicationGuard','preparationReserved']))return null;
  const cap={...n.record.limits,maxRequestBytes:Math.min(o.limits.maxIntentBytes,n.record.limits.maxRequestBytes)},capture=copySettingsData({arguments:n.record,semanticProfile:null},cap);if(!capture)return null;
  if(entry.ordinaryPrepared){const held=entry.ordinaryPrepared,original={kind:held.job.kind,record:held.capture.arguments,opaqueField:held.opaqueField,opaque:held.opaque};return held.inputFamily==='F12'&&sameSelectionArguments(original,n)&&equal(held.capture,capture.value)?{job:held.job,route:n.route}:null;}
  if(entry.phase!=='issued'||entry.settingsPrepared||entry.registrationPrepared||entry.lifecyclePrepared||entry.relationPrepared||entry.configuredLinePrepared||!selectionPermissions(g,n.kind,n.record.request)||!natural(capture.bytes)||capture.bytes>o.limits.maxHeldBytes-o.charge)return null;
  if(n.record.request.operation==='select-search-match'&&!nativeSourceSearchAllowsSelectionPreparation(o,domain,n.opaque,g.searchCurrent,g))return null;
  entry.ordinaryCharge=capture.bytes;o.charge+=capture.bytes;
  const request=selectionProducerRequest(n),before=o.root,plan=n.kind==='selection'?prepareSelectionEdit(before,request,g,g.viewLimits):prepareBoundaryCommand(before,request,g,g.viewLimits,g.boundaryWorkLimits);
  if(!plan)return null;
  const native=captureEditingIntent(n.kind,plan,o.limits.maxIntentBytes,o,domain,ticket);if(!native||size({kind:n.kind,plan:native.value})>o.limits.maxIntentBytes)return null;
  const job=Object.freeze({kind:n.kind,plan});entry.ordinaryPrepared={job,capture:capture.value,configured:false,inputFamily:'F12',opaqueField:n.opaqueField,opaque:n.opaque};ordinaryPlans.set(plan,{domain,ticket});return {job,route:n.route};
 }catch{o.faulted=true;return null;}finally{if(entry.ordinaryCharge&&!entry.ordinaryPrepared&&!entry.configuredLinePrepared){o.charge-=entry.ordinaryCharge;entry.ordinaryCharge=0;}entry.ordinaryPreparing=false;}
}
export function prepareInvocationSelection(domain,ticket,envelope,inputLimits,c,g){return selectionPreparation(domain,ticket,envelope,inputLimits,c,g)?.job??null;}
export function submitInvocationSelection(domain,ticket,envelope,inputLimits,c,p,g){const prepared=selectionPreparation(domain,ticket,envelope,inputLimits,c,g);return prepared?submitInvocation(domain,ticket,prepared.job,prepared.route,c,p,g):unavailable();}

const replacementPermissionFields=Object.freeze(['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','sourceReadable','sourceWritable','bounded','companionsReady','allMatchesWritable']);
function replacementPermissions(g){return gates(g,replacementPermissionFields)&&gates(g.searchCurrent,['grant','scope','continuity','capacity','retained','notStopped']);}
function replacementPreparation(domain,ticket,envelope,result,inputLimits,c,g){
 const o=domains.get(domain),entry=o?.slots.get(ticket);if(!o||!entry||o.faulted||entry.ordinaryPreparing||!['issued','pending','done'].includes(entry.phase))return null;
 entry.ordinaryPreparing=true;
 try{
  const n=normalizeReplacementArguments(envelope,c,inputLimits,o.limits.maxIntentBytes,result);if(!n||!entryCurrent(o,entry,n.record.context)||n.record.request.invocation!==entry.reference||!gates(g,catalog.admissionGates)||!gates(g,['outsidePublicationGuard','preparationReserved']))return null;
  const cap={...n.record.limits,maxRequestBytes:Math.min(o.limits.maxIntentBytes,n.record.limits.maxRequestBytes)},capture=copySettingsData({arguments:n.record,semanticProfile:null},cap);if(!capture)return null;
  if(entry.ordinaryPrepared){const held=entry.ordinaryPrepared,original={kind:held.job.kind,record:held.capture.arguments,result:held.result};return held.job.kind==='replacement'&&held.inputFamily==='F11'&&sameReplacementArguments(original,n)&&equal(held.capture,capture.value)?{job:held.job,route:n.route}:null;}
  if(entry.phase!=='issued'||entry.settingsPrepared||entry.registrationPrepared||entry.lifecyclePrepared||entry.relationPrepared||entry.configuredLinePrepared||!replacementPermissions(g))return null;
  if(!nativeSourceSearchAllowsPreparation(o,domain,n.result,g.searchCurrent,g))return null;
  const footprint=replacementResultFootprint(n.result);if(!footprint||!natural(footprint.replyBytes)||!natural(capture.bytes))return null;
  const charge=capture.bytes+footprint.replyBytes;if(!natural(charge)||charge>o.limits.maxHeldBytes-o.charge)return null;
  entry.ordinaryCharge=capture.bytes;entry.replacementResultCharge=footprint.replyBytes;o.charge+=charge;
  const before=o.root,plan=prepareReplacement(n.result,replacementProducerRequest(n),g.searchCurrent,before,g);if(!plan)return null;
  const native=captureEditingIntent('replacement',plan,o.limits.maxIntentBytes,o,domain,ticket);if(!native||size({kind:'replacement',plan:native.value})>o.limits.maxIntentBytes)return null;
  const job=Object.freeze({kind:'replacement',plan});entry.ordinaryPrepared={job,capture:capture.value,configured:false,inputFamily:'F11',result:n.result};ordinaryPlans.set(plan,{domain,ticket});return {job,route:n.route};
 }catch{o.faulted=true;return null;}finally{if(!entry.ordinaryPrepared){if(entry.ordinaryCharge){o.charge-=entry.ordinaryCharge;entry.ordinaryCharge=0;}if(entry.replacementResultCharge){o.charge-=entry.replacementResultCharge;entry.replacementResultCharge=0;}}entry.ordinaryPreparing=false;}
}
export function prepareInvocationReplacement(domain,ticket,envelope,result,inputLimits,c,g){return replacementPreparation(domain,ticket,envelope,result,inputLimits,c,g)?.job??null;}
export function submitInvocationReplacement(domain,ticket,envelope,result,inputLimits,c,p,g){const prepared=replacementPreparation(domain,ticket,envelope,result,inputLimits,c,g);return prepared?submitInvocation(domain,ticket,prepared.job,prepared.route,c,p,g):unavailable();}
