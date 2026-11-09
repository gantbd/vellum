// Normalized source writes and historical baseline attribution; no filesystem adapter.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {sealRetainedLifecycleWriterGuard,sealSourceRebindWriterGuard} from './retained-lifecycle-state.mjs';
import {sealLifecycleWriterInspection,readLifecycleWriterInspection,readLifecycleWriterInspectionBinding} from './lifecycle-writer-inspection.mjs';
import {sealSourceSaveAsActivationAuthority,readSourceSaveAsActivationAuthority} from './source-saveas-authority.mjs';
import {sealSourceDestinationTransition,discardSourceDestinationArtifact,sourceDestinationGates,forgetNativeSourceTaskCodec,nativeSourceSaveAsBaseline} from './source-destination-state.mjs';
import {prepareSourceDestinationArtifact,prepareAnnotationDestinationArtifact,newSourceCodecRegistry,captureSerializedSource,readSerializedSource,retireSerializedSource,captureSerializedAnnotation,readSerializedAnnotation,retireSerializedAnnotation,prepareSerializedAnnotation,inspectSerializedAnnotationPreparation,commitSerializedAnnotationPreparation,discardSerializedAnnotationPreparation,sourceCodecUsage,annotationPreparationUsage} from './source-codec.mjs';
import {resourceReadBaselineEvidence,nativeSourceReadBaselineWitness,nativeSourceReadDomain} from './resource-read.mjs';
import {invocationAuthorityRoot,invocationAuthorityContext,nativeInvocationRootOrigin,invocationUsage,invocationOrdinaryAdmissionOpen,beginInvocationAdmissionMutation,endInvocationAdmissionMutation,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment,installInvocationAnnotationBaseline,prepareInvocationOrdinaryBaseline,validateInvocationOrdinaryBaseline,commitInvocationOrdinaryBaseline,discardInvocationOrdinaryBaseline} from './invocation-publication.mjs';
import {ordinarySourceBindingReady,ordinaryBindingIdentityReady,ordinaryAnnotationOwner} from './binding-pair-profile.mjs';
import {captureNativeAnnotationArtifact} from './annotation-artifact.mjs';
import {prepareAnnotationBaselineEnvelope} from './annotation-baseline-capacity.mjs';
import {annotationLoadProvenanceInfo,annotationLoadedBaselineShape,annotationDocumentProvenanceReleased,annotationLoadedBaselineEvidence,sealAnnotationLoadWriterGuard,prepareAnnotationReadAttachment,completeAnnotationReadAttachment,discardAnnotationReadAttachment,sealAnnotationReadCheckpoint,detachAnnotationReadAttachment,annotationReadDischargeGates} from './annotation-loaded-baseline-state.mjs';
import {annotationClaimGates,createAnnotationWriteClaim,annotationWriteClaimDispatchReady,annotationWriteClaimBound,retireAnnotationWriteClaim,readAnnotationWriteClaim,annotationInstallationGates,sealAnnotationBaselineInstallation,discardAnnotationBaselineInstallation} from './annotation-write-claims.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./source-save.json',import.meta.url),'utf8'));
const continuity=JSON.parse(fs.readFileSync(new URL('./source-save-continuity.json',import.meta.url),'utf8'));
const exportsCatalog=JSON.parse(fs.readFileSync(new URL('./source-export.json',import.meta.url),'utf8'));
const annotationCatalog=JSON.parse(fs.readFileSync(new URL('./annotation-write.json',import.meta.url),'utf8'));
const annotationBaselines=JSON.parse(fs.readFileSync(new URL('./annotation-baselines.json',import.meta.url),'utf8'));
const owners=new WeakMap(),copy=structuredClone;
const natural=n=>Number.isSafeInteger(n)&&n>=0,size=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const scalar=s=>typeof s==='string'&&Buffer.from(s,'utf8').toString('utf8')===s;
const identity=r=>exact(r,['authority','id','incarnation'])&&scalar(r.authority)&&r.authority.length>0&&scalar(r.id)&&r.id.length>0&&natural(r.incarnation);
const version=v=>exact(v,['epoch','token'])&&scalar(v.epoch)&&v.epoch.length>0&&scalar(v.token);
const allowed=(g,ks)=>ks.every(k=>g?.[k]===true),unavailable=()=>({status:'unavailable'});
const launchGates=['authorized','scope','continuous','live','notStopped','sourceReadable','writeAllowed','bindingCoverageComplete','writerCoverageComplete','providerAvailable','resourcesReady','launchAllowed'];
const releaseGates=['authorized','scope','continuous','live','notStopped','retained','replyReady'];
const codecGates={authorized:true,scope:true,continuous:true,live:true,notStopped:true,sourceReadable:true,resourcesReady:true,retained:true,replyReady:true};
const originKey=r=>JSON.stringify(catalog.originFields.map(k=>r[k]));
export const sourceSaveDestinationKey=row=>JSON.stringify([row.identityDomain,row.identityKey]);
// Trusted composition facet, not recipient reflection or possession-based authority.
export const sourceSaveRegistryAuthority=owner=>(owners.get(owner)??readLifecycleWriterInspection(owner))?.authority??null;
export function borrowSourceSaveLifecycleInspection(owner,g){const o=owners.get(owner);return o&&['ownerControlAllowed','preparationReserved','resourcesReady'].every(k=>g?.[k]===true)?sealLifecycleWriterInspection(owner,o):null;}
const queued=t=>t.phase==='queued'&&!t.stopped;
const held=t=>t.phase==='dispatched'&&!t.quiescent;
const resolved=t=>t.disposition&&t.knowledge.application!=='unknown'&&!t.contested;
const acknowledgment=t=>t.capture.request.acknowledgment==='synchronized'||t.capture.request.acknowledgment==='ordinary'&&t.capture.registration.syncSupported?'synchronized':'written';
const qualified=(o,t)=>t.phase==='dispatched'&&t.knowledge.application==='written'&&(acknowledgment(t)==='written'||t.knowledge.synchronization==='succeeded')&&!t.contested&&!o.faults.has(t.key);
function rootDocument(state,document,l){if(!Array.isArray(state?.documents)||state.documents.length>l.maxRegistryRows||size(state)>l.maxRootBytes)return null;const ds=state.documents.filter(d=>equal(d.document,document));return ds.length===1&&ds[0].live===true?ds[0]:null;}
function registration(state,d,r,l){
  if(!d.binding||!Array.isArray(state.writeRegistrations)||!Array.isArray(state.resourceOwners)||state.writeRegistrations.length>l.maxRegistryRows||state.resourceOwners.length>l.maxRegistryRows)return null;
  const b=d.binding,rows=state.writeRegistrations.filter(x=>equal(x.provider,b.provider)&&equal(x.resource,b.resource));if(rows.length!==1)return null;const row=rows[0];
  const bindings=state.resourceOwners.filter(x=>x.identityDomain===row.identityDomain&&x.identityKey===row.identityKey);
  if(!exact(row,catalog.registrationFields)||!identity(row.provider)||!identity(row.resource)||row.generation!==r.expectedRegistration||!natural(row.generation)||b.registrationGeneration!==row.generation||b.generation!==r.expectedBinding||!natural(b.generation)||row.identityKnown!==true||row.logicalContinuity!==true||typeof row.syncSupported!=='boolean'||typeof row.conditionalWrite!=='boolean'||[row.identityDomain,row.identityKey,row.locator,row.procedure].some(s=>!scalar(s)||!s||Buffer.byteLength(s)>l.maxTagBytes)||b.identityDomain!==row.identityDomain||b.identityKey!==row.identityKey||b.locator!==row.locator||bindings.length!==1||!equal(bindings[0].document,d.document)||bindings[0].bindingGeneration!==b.generation)return null;
  if(typeof row.causalReceipts!=='boolean'||typeof row.nonReuseTokens!=='boolean'||typeof row.exclusiveCreate!=='boolean'||r.protection==='conditional'&&!row.conditionalWrite||r.acknowledgment==='synchronized'&&!row.syncSupported)return null;return row;
}
function validRequest(r,l){return exact(r,catalog.requestFields)&&r.contract===catalog.contract&&r.operation==='captured-source-save'&&exact(r.origin,catalog.originFields)&&Object.values(r.origin).every(s=>scalar(s)&&s.length>0)&&identity(r.document)&&[r.expectedSource,r.expectedSerialization,r.expectedBinding,r.expectedRegistration].every(natural)&&['best-effort','conditional'].includes(r.protection)&&['ordinary','write-only','synchronized'].includes(r.acknowledgment)&&exact(r.condition,['policy','resourceVersion'])&&['pinned','own-chain'].includes(r.condition.policy)&&version(r.condition.resourceVersion)&&size(r.condition.resourceVersion)<=l.maxTagBytes&&size(r)<=l.maxCaptureBytes;}
function artifactBytes(o,t){
  const out=[];for(let lo=0;lo<t.capture.payloadByteLength;lo+=o.codecLimits.maxChunkBytes){const r=readSerializedSource(o.codec,t.artifact,[lo,Math.min(t.capture.payloadByteLength,lo+o.codecLimits.maxChunkBytes)],codecGates);if(r.status!=='serialized-bytes')return null;out.push(...r.bytes);}return out;
}
function writtenBaseline(o,t){
  if(t.annotation||!qualified(o,t))return null;const c=t.capture,bytes=artifactBytes(o,t);if(bytes===null)return null;
  const b={kind:'acknowledged-write',bindingGeneration:c.binding.generation,sourceRevision:c.sourceRevision,serializationGeneration:c.serialization.generation,profile:c.serialization.profile,bom:c.serialization.bom,origin:copy(c.request.origin),provider:copy(c.registration.provider),resourceVersion:copy(t.knowledge.resourceVersion),procedure:c.registration.procedure,acknowledgment:acknowledgment(t),requiredAcknowledgment:copy(c.request.acknowledgment),dispatchSerial:t.serial,bytes};return size(b)<=o.limits.maxBaselineBytes?b:null;
}
const annotationReadGates={authorized:true,scope:true,continuous:true,live:true,notStopped:true,annotationReadable:true,completeDisclosure:true,resourcesReady:true,retained:true,transferReady:true,replyReady:true};
function annotationArtifactBytes(o,t){
 if(!t.annotation)return null;const bytes=[];for(let lo=0;lo<t.capture.payloadByteLength;lo+=o.codecLimits.maxChunkBytes){const r=readSerializedAnnotation(o.codec,t.artifact,[lo,Math.min(t.capture.payloadByteLength,lo+o.codecLimits.maxChunkBytes)],t.annotation.basis,annotationReadGates);if(r.status!=='serialized-annotation-bytes')return null;bytes.push(...r.bytes);}return bytes;
}
function writtenAnnotationBaseline(o,t){
 const c=t.capture,b=c.exportState?.originalAnnotationBinding;if(!t.annotation||!b||!qualified(o,t))return null;const bytes=annotationArtifactBytes(o,t);if(bytes===null)return null;
 const out={kind:'acknowledged-annotation-write',annotationBindingGeneration:b.generation,receiptBindingGeneration:c.binding.generation,sourceRevision:c.annotation.sourceRevision,annotationSet:c.annotation.annotationSet,profile:c.annotation.profile,context:copy(t.annotation.basis.context),origin:copy(c.request.origin),provider:copy(c.registration.provider),resourceVersion:copy(t.knowledge.resourceVersion),procedure:c.registration.procedure,acknowledgment:acknowledgment(t),requiredAcknowledgment:copy(c.request.acknowledgment),dispatchSerial:t.serial,bytes};return size(out)<=o.limits.maxBaselineBytes?out:null;
}
function annotationBaselineAnchor(o,document,binding,b,domain=null){
 if(annotationLoadedBaselineShape(b)){const matches=[...o.annotationReadOwners.entries()].filter(([d,owner])=>(domain===null||d===domain)&&annotationLoadedBaselineEvidence(owner,document,binding,b));return matches.length===1?{serial:0}:null;}
 if(!b||b.kind!=='acknowledged-annotation-write'||!exact(b,annotationBaselines.baselineFields))return null;const t=[...o.tasks.values()].find(t=>equal(t.capture.request.origin,b.origin));if(!t?.annotation||!equal(t.capture.request.document,document)||!equal(t.capture.exportState?.originalAnnotationBinding,binding))return null;const actual=writtenAnnotationBaseline(o,t);if(actual&&b.resourceVersion===null)actual.resourceVersion=null;return actual&&equal(actual,b)?{serial:t.serial}:null;
}
function annotationBindingEntries(state,document,o,minimumGeneration){
 const validBinding=b=>exact(b,['generation','provider','resource','registrationGeneration','identityDomain','identityKey','locator'])&&natural(b.generation)&&b.generation>0&&identity(b.provider)&&identity(b.resource)&&natural(b.registrationGeneration)&&[b.identityDomain,b.identityKey,b.locator].every(s=>scalar(s)&&s.length>0);
 const entries=[];if(!Array.isArray(state.documents)||state.documents.length>o.limits.maxRegistryRows)return null;for(const d of state.documents.filter(d=>equal(d.document,document))){const p=d.ordinaryBindings,history=Object.hasOwn(d,'annotationBindingHistory')?d.annotationBindingHistory:[];if(!exact(p,['generation','annotations'])||!natural(p.generation)||p.generation<minimumGeneration||p.annotations!==null&&!validBinding(p.annotations)||!Array.isArray(history)||history.length>o.limits.maxRegistryRows||history.some(h=>!exact(h,['binding','annotationBaseline'])||!validBinding(h.binding)||h.annotationBaseline!==null&&!exact(h.annotationBaseline,annotationBaselines.baselineFields)&&!annotationLoadedBaselineShape(h.annotationBaseline)))return null;if(d.live===false&&d.lifecycle?.phase==='provenance-released'){if(!annotationDocumentProvenanceReleased(d))return null;continue;}if(p.annotations)entries.push({binding:p.annotations,baseline:d.annotationBaseline});for(const h of history)entries.push({binding:h.binding,baseline:h.annotationBaseline});}return entries;
}
function baselineAnchor(o,document,binding,b){
  if(resourceReadBaselineEvidence(o.readOwner,document,binding,b,codecGates))return {serial:0};
  if(!b||b.kind!=='acknowledged-write'||!exact(b,continuity.baselineFields))return null;
  const t=[...o.tasks.values()].find(t=>equal(t.capture.request.origin,b.origin));if(!t||!equal(document,t.capture.request.document)||!equal(binding,t.capture.binding))return null;
  const actual=writtenBaseline(o,t);if(actual&&b.resourceVersion===null)actual.resourceVersion=null;return actual&&equal(actual,b)?{serial:t.serial}:null;
}
export function newSourceSaveRegistry(authority,readOwner,limits,codecLimits){
  if(!scalar(authority)||!authority||!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||Buffer.byteLength(authority)>limits.maxTagBytes)return null;
  const codec=newSourceCodecRegistry(codecLimits),charge=limits.maxCaptureBytes+5*limits.maxEnvelopeBytes+limits.maxDerivationBytes+codecLimits?.maxArtifactBytes;
  if(!codec||!natural(charge)||!natural(charge*limits.maxTasks)||!natural(codecLimits.maxArtifactBytes*limits.maxTasks)||codecLimits.maxArtifacts<limits.maxTasks||codecLimits.maxIssued<limits.maxIssued||codecLimits.maxHeldBytes<codecLimits.maxArtifactBytes*limits.maxTasks||!natural(codecLimits.maxArtifactBytes+256)||codecLimits.maxReplyBytes<codecLimits.maxArtifactBytes+256||codecLimits.maxChunkBytes===0)return null;
  const owner=Object.freeze({profile:catalog.profile});owners.set(owner,{authority,readOwner,limits:copy(limits),codecLimits:copy(codecLimits),codec,charge,tasks:new Map(),used:new Set(),faults:new Set(),parentOperations:new Map(),annotationReadOwners:new Map(),nextDispatch:1,nativeWriterDomains:new Map(),nativeWriterSponsor:null,nativeWriterBindingPending:false});
  const domain=nativeSourceReadDomain(readOwner);if(domain&&!bindInvocationSourceWriter(owner,domain,{ownerControlAllowed:true,resourcesReady:true,preparationReserved:true,writerCoverageComplete:true})){owners.delete(owner);return null;}return owner;
}
// Internal composition only. Native ledgers are conservatively reserved together;
// this is not a unique-physical-allocation or total transient/RSS bound.
function nativeWriterFunding(o){
 const maximum=Number.MAX_SAFE_INTEGER,domain={kind:'invocation-domain/1'},lease={kind:'native-owner-attachment/1'},marker={profile:catalog.profile};
 const edge=size({domain,lease,owner:marker,charge:maximum}),frame=size({sponsor:domain,lease,limits:o.limits,codecLimits:o.codecLimits,charge:maximum});
 // A spent origin is a JSON subvalue of a bounded request. A destination key
 // contains two maxTagBytes strings; JSON escaping costs at most six bytes each.
 const faultKey=12*o.limits.maxTagBytes+7,spent=o.limits.maxIssued*(2*o.limits.maxCaptureBytes+faultKey),node=o.limits.maxHeldBytes+o.codecLimits.maxHeldBytes+spent+frame;
 return [edge,faultKey,spent,node,node+edge].every(natural)?{edge,node}:null;
}
export function bindInvocationSourceWriter(owner,domain,g){
 const o=owners.get(owner);if(!o||o.nativeWriterBindingPending||invocationAuthorityRoot(domain)?.authority!==o.authority||invocationUsage(domain)?.faulted!==false)return false;
 if(o.nativeWriterDomains.has(domain))return invocationOrdinaryAdmissionOpen(domain);
 o.nativeWriterBindingPending=true;let attachment=null;
 try{
  if(!allowed(g,['ownerControlAllowed','resourcesReady','preparationReserved','writerCoverageComplete']))return false;
  // Legacy work cannot be retroactively relabelled as funded native work.
  if(!o.nativeWriterSponsor&&(o.tasks.size||o.used.size||o.annotationReadOwners.size||sourceCodecUsage(o.codec)?.issued))return false;
  const funding=nativeWriterFunding(o);if(!funding)return false;const units=funding.edge+(o.nativeWriterSponsor?0:funding.node);
  attachment=beginInvocationOwnerAttachment(domain,'source-writer',units,owner);if(!attachment)return false;
  if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('shared source writer attachment invariant');
  o.nativeWriterDomains.set(domain,{attachment,units});o.nativeWriterSponsor??={domain,attachment,nodeBytes:funding.node};return true;
 }finally{if(attachment)abortInvocationOwnerAttachment(domain,attachment);o.nativeWriterBindingPending=false;}
}
function nativeWriterRootAdmission(o,state){return !o.nativeWriterDomains.size||[...o.nativeWriterDomains.keys()].some(domain=>(invocationAuthorityRoot(domain)===state||nativeInvocationRootOrigin(state)===domain)&&invocationOrdinaryAdmissionOpen(domain));}
// Trusted supervision, not a recipient-visible ownership or reflection surface.
export function sourceWriterAttachmentUsage(owner){const o=owners.get(owner);return o?{sponsor:o.nativeWriterSponsor?.domain??null,nodeBytes:o.nativeWriterSponsor?.nodeBytes??0,domains:o.nativeWriterDomains.size,pending:o.nativeWriterBindingPending,wholeWriterRetirementSupported:false}:null;}
export function attachAnnotationReadProvenance(owner,readOwner,g){
 const o=owners.get(owner),p=annotationLoadProvenanceInfo(readOwner),keys=['ownerControlAllowed','preparationReserved','resourcesReady','writerCoverageComplete'];if(!o||!p||p.context.authority!==o.authority||!allowed(g,keys))return unavailable();
 const stage=prepareAnnotationReadAttachment(owner,o,readOwner);if(!stage)return unavailable();let section=null;
 try{if(!bindInvocationSourceWriter(owner,p.domain,Object.fromEntries(keys.map(k=>[k,true]))))return unavailable();section=beginInvocationAdmissionMutation(p.domain);if(!section)throw Error('funded writer provenance section invariant');const status=completeAnnotationReadAttachment(stage);if(!status)throw Error('funded writer provenance commit invariant');return {status};}
 finally{discardAnnotationReadAttachment(stage);if(section)endInvocationAdmissionMutation(p.domain,section);}
}
export function detachAnnotationReadProvenance(owner,readOwner,g){const o=owners.get(owner),p=annotationLoadProvenanceInfo(readOwner),root=p&&invocationAuthorityRoot(p.domain);return o&&p&&root&&detachAnnotationReadAttachment(owner,o,readOwner,root,g)?{status:'detached'}:unavailable();}
export function prepareAnnotationReadRetirementCheckpoint(owner,readOwner,state,document,g){const o=owners.get(owner),p=annotationLoadProvenanceInfo(readOwner);if(!o||!p||invocationAuthorityRoot(p.domain)!==state||!allowed(g,annotationReadDischargeGates)||o.annotationReadOwners.get(p.domain)!==readOwner)return null;const d=rootDocument(state,document,o.limits),b=d?.annotationBaseline;if(!d||b?.kind!=='acknowledged-annotation-write'||!annotationBaselineAnchor(o,document,d.ordinaryBindings?.annotations,b,p.domain))return null;const rows=[...o.tasks.entries()].filter(([,t])=>t.annotation?.domain===p.domain&&equal(t.capture.request.document,document)&&equal(t.capture.request.origin,b.origin));return rows.length===1?sealAnnotationReadCheckpoint(owner,o,readOwner,rows[0][0],rows[0][1],b):null;}
export function prepareAnnotationLoadedReadGuard(owner,readOwner,document,binding,g){const pair=readLifecycleWriterInspectionBinding(owner),o=owners.get(owner)??pair?.writer,p=annotationLoadProvenanceInfo(readOwner),actualOwner=owners.has(owner)?owner:pair?.owner;return o&&p&&allowed(g,['authorized','scope','continuous','bindingCoverageComplete','writerCoverageComplete','dispositionReadable','preparationReserved','resourcesReady'])?sealAnnotationLoadWriterGuard(actualOwner,o,readOwner,p.domain,document,binding):null;}
export function admitSourceSave(owner,state,r,g){
  const o=owners.get(owner),plan=ordinarySourceAdmission(o,owner,state,r,g);return plan?captureAdmission(o,owner,state,r,plan.capture.registration,plan.capture.binding,plan.capture.baseline,plan.capture.anchorSerial):unavailable();
}
function ordinarySourceAdmission(o,owner,state,r,g){
  if(!o||state?.authority!==o.authority||!nativeWriterRootAdmission(o,state)||!allowed(g,launchGates)||!validRequest(r,o.limits)||r.document.authority!==o.authority)return null;
  const d=rootDocument(state,r.document,o.limits),row=d&&registration(state,d,r,o.limits),anchor=d&&baselineAnchor(o,d.document,d.binding,d.sourceBaseline);if(!row||!anchor||d.revision!==r.expectedSource||d.serialization?.generation!==r.expectedSerialization||!equal(r.condition.resourceVersion,d.sourceBaseline?.resourceVersion)||!version(d.sourceBaseline?.resourceVersion))return null;
  if(!ordinarySourceBindingReady(state,d,d.binding,g))return null;
  return sourceAdmissionPlan(o,owner,state,r,row,d.binding,d.sourceBaseline,anchor.serial);
}
function captureAdmission(o,owner,state,r,row,binding,baseline,anchorSerial,exportState=null){
  const plan=sourceAdmissionPlan(o,owner,state,r,row,binding,baseline,anchorSerial,exportState);if(!plan)return unavailable();const {capture,receiptCapture,key,used}=plan;
  const encoded=captureSerializedSource(o.codec,state,{contract:'serialized-source/1',operation:'capture-source',document:r.document,expectedSource:r.expectedSource,expectedSerialization:r.expectedSerialization},codecGates);if(encoded.status!=='captured')return unavailable();
  const task=Object.freeze({kind:'source-save-task/1'});o.used.add(used);o.tasks.set(task,{capture,receiptCapture,key,artifact:encoded.artifact,phase:'queued',serial:null,dispatchCondition:null,causal:null,stopped:false,quiescent:false,disposition:false,contested:false,reservationReleased:!exportState,bindingApplied:false,knowledge:{application:'unknown',observedBytes:0,synchronization:'unknown',resourceVersion:null},conflicts:[]});return {status:'admitted',task};
}
function sourceAdmissionPlan(o,owner,state,r,row,binding,baseline,anchorSerial,exportState=null){
  const d=rootDocument(state,r.document,o.limits);if(!d||o.parentOperations.has(r.origin.operation))return null;
  const key=sourceSaveDestinationKey(row),capture={request:copy(r),registration:copy(row),binding:copy(binding),baseline:copy(baseline),anchorSerial,sourceRevision:d.revision,serialization:copy(d.serialization),payloadByteLength:d.source.length+(d.serialization.bom?3:0),...(exportState?{exportState:copy(exportState)}:{})},used=originKey(r.origin),u=sourceSaveUsage(owner);
  const receiptCapture={document:copy(r.document),sourceRevision:d.revision,serializationGeneration:d.serialization.generation,bindingGeneration:binding.generation,profile:d.serialization.profile,bom:d.serialization.bom,payloadByteLength:capture.payloadByteLength,procedure:row.procedure,condition:copy(r.condition),...(exportState?{export:{purpose:r.purpose,sourceBindingGeneration:r.expectedBinding,proposedBindingGeneration:binding.generation}}:{})};
  const initialCondition={resourceVersion:copy(r.condition.resourceVersion),chain:[],...(exportState?{presence:r.condition.presence}:{})};if(r.condition.policy==='own-chain'&&size(initialCondition)>o.limits.maxDerivationBytes)return null;
  const receiptFrame={...receiptCapture,dispatchSerial:Number.MAX_SAFE_INTEGER,dispatchCondition:initialCondition};
  const minimal={contract:catalog.evidenceContract,task:{kind:'source-save-task/1'},origin:r.origin,provider:row.provider,resource:row.resource,capture:receiptFrame,kind:'result',application:'not-written',observedBytes:0,synchronization:'not-requested',resourceVersion:null};
  if(size(capture)>o.limits.maxCaptureBytes||size(minimal)>o.limits.maxEnvelopeBytes||o.used.has(used)||o.faults.has(key)||u.tasks>=o.limits.maxTasks||u.issued>=o.limits.maxIssued||[...o.tasks.values()].filter(t=>t.key===key&&queued(t)).length>=o.limits.maxPerDestination||o.charge>o.limits.maxHeldBytes-u.heldBytes)return null;
  return nativeWriterRootAdmission(o,state)?{capture,receiptCapture,key,used}:null;
}
function derive(o,t,g){
  const c=t.capture,out={resourceVersion:copy(c.request.condition.resourceVersion),chain:[]};if(c.request.condition.policy==='pinned')return out;
  const earlier=[...o.tasks.values()].filter(x=>x.key===t.key&&x.phase==='dispatched'&&x.serial>c.anchorSerial).sort((a,b)=>a.serial-b.serial);
  for(const x of earlier){
    if(!resolved(x))return null;if(x.knowledge.application==='not-written')continue;
    if(!g.causalProgressionAllowed||t.nativeSourceDomain&&g.causalProgressionAllowed!==true||!c.registration.causalReceipts||!c.registration.nonReuseTokens||!equal(x.capture.binding,c.binding)||!equal(x.capture.registration,c.registration)||!equal(x.capture.request.document,c.request.document)||!x.causal||!version(x.knowledge.resourceVersion)||!equal(x.causal.beforeVersion,out.resourceVersion)||!equal(x.causal.beforeVersion,x.dispatchCondition.resourceVersion)||!equal(x.causal.afterVersion,x.knowledge.resourceVersion)||x.causal.beforeVersion.epoch!==x.causal.afterVersion.epoch||equal(x.causal.beforeVersion,x.causal.afterVersion))return null;
    out.chain.push({origin:copy(x.capture.request.origin),dispatchSerial:x.serial,beforeVersion:copy(x.causal.beforeVersion),afterVersion:copy(x.causal.afterVersion)});out.resourceVersion=copy(x.causal.afterVersion);if(size(out)>o.limits.maxDerivationBytes)return null;
  }
  return size(out)<=o.limits.maxDerivationBytes?out:null;
}
export function dispatchSourceSave(owner,task,state,probe,g){
  const o=owners.get(owner),t=o?.tasks.get(task);if(!t||!queued(t)||state?.authority!==o.authority||(t.annotation?!annotationCatalog.launchGates.every(k=>g?.[k]===true):!allowed(g,launchGates))||!g.probeAttested||(t.annotation||t.nativeSourceDomain)&&g.probeAttested!==true||o.faults.has(t.key))return {status:'not-dispatched'};
  if(t.nativeSourceDomain&&(state!==invocationAuthorityRoot(t.nativeSourceDomain)||invocationUsage(t.nativeSourceDomain)?.faulted!==false))return {status:'not-dispatched'};
  if(t.annotation?.reservationClaim&&!annotationWriteClaimDispatchReady(t.annotation.reservationClaim,owner,task))return {status:'not-dispatched'};
  if(t.annotation?.reservationClaim&&invocationUsage(t.annotation.domain)?.faulted!==false)return {status:'not-dispatched'};
  const rows=[...o.tasks.values()],head=rows.find(x=>x.key===t.key&&queued(x));if(head!==t||rows.some(x=>x.key===t.key&&x.phase==='dispatched'&&!resolved(x))||sourceSaveUsage(owner).outstanding>=o.limits.maxOutstanding)return {status:'not-dispatched'};
  const c=t.capture,d=rootDocument(state,c.request.document,o.limits),row=c.exportState?targetRegistration(state,c.request,o.limits):d&&registration(state,d,c.request,o.limits);
  if(t.nativeSourceDomain&&t.annotation&&!ordinaryBindingIdentityReady(state,d,g))return {status:'not-dispatched'};
  if(d&&(!c.exportState||c.request.purpose==='save-as')&&!ordinarySourceBindingReady(state,d,c.exportState?c.binding:d.binding,g))return {status:'not-dispatched'};
  if(c.exportState){if(!d||!exportCurrent(state,t,o,g)||!t.annotation&&!allowed(g,exportsCatalog.gates))return {status:'not-dispatched'};}
  else if(!row||!equal(row,c.registration)||!equal(d.binding,c.binding)||!baselineAnchor(o,c.request.document,c.binding,c.baseline))return {status:'not-dispatched'};
  if(t.annotation?.ordinary&&((d?.annotationBaseline?.kind==='loaded-annotation-capture')&&!annotationBaselineAnchor(o,d.document,d.ordinaryBindings?.annotations,d.annotationBaseline,t.annotation.domain)||(c.baseline?.kind==='loaded-annotation-capture')&&!annotationBaselineAnchor(o,c.request.document,c.exportState.originalAnnotationBinding,c.baseline,t.annotation.domain)))return {status:'not-dispatched'};
  const dispatchCondition=c.exportState?{resourceVersion:copy(c.request.condition.resourceVersion),chain:[],presence:c.request.condition.presence}:derive(o,t,g);if(!dispatchCondition||!natural(o.nextDispatch)||o.nextDispatch===Number.MAX_SAFE_INTEGER)return {status:'not-dispatched'};
  if(!exact(probe,['provider','resource','registrationGeneration','identityDomain','identityKey','resourceVersion',...(c.exportState?['presence']:[])])||!equal(probe.provider,row.provider)||!equal(probe.resource,row.resource)||probe.registrationGeneration!==row.generation||probe.identityDomain!==row.identityDomain||probe.identityKey!==row.identityKey||!(probe.resourceVersion===null&&c.exportState&&dispatchCondition.presence==='absent')&&!version(probe.resourceVersion)||size(probe)>o.limits.maxCaptureBytes||!equal(probe.resourceVersion,dispatchCondition.resourceVersion)||c.exportState&&probe.presence!==dispatchCondition.presence)return {status:'not-dispatched'};
  const receiptCapture={...t.receiptCapture,dispatchSerial:o.nextDispatch,dispatchCondition:copy(dispatchCondition)},minimal={contract:catalog.evidenceContract,task,origin:c.request.origin,provider:row.provider,resource:row.resource,capture:receiptCapture,kind:'result',application:'unknown',observedBytes:0,synchronization:'unknown',resourceVersion:null};if(size(minimal)>o.limits.maxEnvelopeBytes)return {status:'not-dispatched'};
  t.phase='dispatched';t.serial=o.nextDispatch++;t.dispatchCondition=dispatchCondition;t.receiptCapture=receiptCapture;const {baseline,exportState,...handoff}=c;return {status:'dispatched',task,capture:copy(handoff),receiptCapture:copy(t.receiptCapture),requiredAcknowledgment:acknowledgment(t)};
}
export function readSourceSavePayload(owner,task,range,g){
  const o=owners.get(owner),t=o?.tasks.get(task);if(!t||t.annotation||t.phase!=='dispatched'||t.stopped||!allowed(g,releaseGates)||!g.sourceReadable||t.nativeSourceDomain&&g.sourceReadable!==true||!equal(g.provider,t.capture.registration.provider)||!equal(g.resource,t.capture.registration.resource))return unavailable();return readSerializedSource(o.codec,t.artifact,range,codecGates);
}
function resultShape(e,t,l){return catalog.applications.includes(e.application)&&catalog.synchronizations.includes(e.synchronization)&&natural(e.observedBytes)&&e.observedBytes<=t.receiptCapture.payloadByteLength&&(e.resourceVersion===null||version(e.resourceVersion)&&size(e.resourceVersion)<=l.maxTagBytes)&&
  (e.application==='written'?e.observedBytes===t.receiptCapture.payloadByteLength:e.application==='not-written'?e.observedBytes===0&&e.synchronization==='not-requested'&&e.resourceVersion===null:e.synchronization==='unknown'&&e.resourceVersion===null);}
function join(a,b){
  if(a.application!=='unknown'&&b.application!=='unknown'&&a.application!==b.application||a.synchronization!=='unknown'&&b.synchronization!=='unknown'&&a.synchronization!==b.synchronization||a.resourceVersion!==null&&b.resourceVersion!==null&&!equal(a.resourceVersion,b.resourceVersion)||a.application==='not-written'&&b.observedBytes>0||b.application==='not-written'&&a.observedBytes>0)return null;
  return {application:a.application==='unknown'?b.application:a.application,observedBytes:Math.max(a.observedBytes,b.observedBytes),synchronization:a.synchronization==='unknown'?b.synchronization:a.synchronization,resourceVersion:copy(a.resourceVersion??b.resourceVersion)};
}
function recordConflict(o,t,incoming){if(t.conflicts.some(x=>equal(x,incoming)))return {status:'duplicate'};if(t.conflicts.length===2)return {status:'evidence-full'};t.conflicts.push(copy(incoming));t.contested=true;o.faults.add(t.key);return {status:'conflict'};}
export function observeSourceSave(owner,e,g){
  const o=owners.get(owner),t=o?.tasks.get(e?.task);if(!t||t.phase!=='dispatched'||!g?.authentic||!g.evidenceAllowed||!g.ingressReady||!Object.hasOwn(catalog.events,e.kind)||!exact(e,[...catalog.eventCommon,...catalog.events[e.kind]])||e.contract!==catalog.evidenceContract||!equal(e.origin,t.capture.request.origin)||!equal(e.provider,t.capture.registration.provider)||!equal(e.resource,t.capture.registration.resource)||!equal(e.capture,t.receiptCapture)||size(e)>o.limits.maxEnvelopeBytes)return unavailable();
  if((t.annotation||t.nativeSourceDomain)&&(!['authentic','evidenceAllowed','ingressReady'].every(k=>g?.[k]===true)||g[e.kind==='result'?'resultAttested':e.kind==='quiescence'?'quiescenceAttested':e.kind==='disposition'?'dispositionAttested':'causalityAttested']!==true))return unavailable();
  if(e.kind==='quiescence'||e.kind==='disposition'){const field=e.kind==='quiescence'?'quiescent':'disposition';if(!g[e.kind==='quiescence'?'quiescenceAttested':'dispositionAttested'])return unavailable();if(t[field])return {status:'duplicate'};t[field]=true;return {status:'recorded'};}
  if(e.kind==='causal'){
    if(t.annotation)return unavailable();
    if(!g.causalityAttested||!t.capture.registration.causalReceipts||![e.beforeVersion,e.afterVersion].every(v=>version(v)&&size(v)<=o.limits.maxTagBytes))return unavailable();const fact={beforeVersion:copy(e.beforeVersion),afterVersion:copy(e.afterVersion)};
    if(t.knowledge.application==='not-written'||!equal(fact.beforeVersion,t.dispatchCondition.resourceVersion)||t.knowledge.resourceVersion!==null&&!equal(fact.afterVersion,t.knowledge.resourceVersion)||t.causal&&!equal(fact,t.causal))return recordConflict(o,t,{kind:'causal',...fact});
    if(t.causal)return {status:'duplicate'};t.causal=fact;return {status:'recorded'};
  }
  if(!g.resultAttested||!resultShape(e,t,o.limits))return unavailable();
  const incoming=Object.fromEntries(catalog.events.result.map(k=>[k,copy(e[k])])),merged=join(t.knowledge,incoming);
  if(!merged||incoming.application==='not-written'&&t.causal||incoming.resourceVersion!==null&&t.causal&&!equal(incoming.resourceVersion,t.causal.afterVersion))return recordConflict(o,t,incoming);
  if(equal(merged,t.knowledge))return {status:'duplicate'};t.knowledge=merged;return {status:'recorded'};
}
export function controlSourceSave(owner,task,operation,g){const t=owners.get(owner)?.tasks.get(task);if(!t||!g?.controlAllowed||(t.annotation||t.nativeSourceDomain)&&g.controlAllowed!==true||!['cancel','supersede'].includes(operation))return unavailable();if(t.stopped)return {status:'unchanged'};t.stopped=true;return {status:'stopped'};}
function annotationPeerProducerReferencesClear(state,t,o){
 if(!Array.isArray(state?.documents)||state.documents.length>o.limits.maxRegistryRows)return false;
 for(const d of state.documents){if(equal(d.document,t.capture.request.document))continue;const history=Object.hasOwn(d,'annotationBindingHistory')?d.annotationBindingHistory:[];if(!Array.isArray(history)||history.length>o.limits.maxRegistryRows||history.some(h=>!exact(h,['binding','annotationBaseline'])))return false;if(d.lifecycle?.phase==='provenance-released'){if(!annotationDocumentProvenanceReleased(d))return false;continue;}for(const b of [d.annotationBaseline,...history.map(h=>h.annotationBaseline)]){if(b===null||b===undefined)continue;if(!annotationLoadedBaselineShape(b)&&!(b.kind==='acknowledged-annotation-write'&&exact(b,annotationBaselines.baselineFields)))return false;if(b.kind==='acknowledged-annotation-write'&&equal(b.origin,t.capture.request.origin)&&equal(b.context,t.annotation.basis.context))return false;}}
 return true;
}
export function inspectSourceSave(owner,task,g){
  const o=owners.get(owner),t=o?.tasks.get(task);if(!t||!allowed(g,releaseGates))return unavailable();const required=t.capture.request.acknowledgment==='synchronized'||t.capture.request.acknowledgment==='ordinary'&&t.capture.registration.syncSupported?'synchronized':'written';
  return {status:'save-evidence',phase:t.phase,stopped:t.stopped,quiescent:t.quiescent,safeDisposition:t.disposition,contested:t.contested,destinationFaulted:o.faults.has(t.key),knowledge:copy(t.knowledge),requiredAcknowledgment:required,requestedStrengthSatisfied:t.knowledge.application==='written'&&(required==='written'||t.knowledge.synchronization==='succeeded')&&!t.contested&&!o.faults.has(t.key),...(t.capture.exportState?{export:{purpose:t.capture.request.purpose,reservationReleased:t.reservationReleased,bindingApplied:t.bindingApplied}}:{})};
}
// First-party seam for the separately versioned native observation profile.
// Never expose export workflow state or infer baseline adoption from a write.
export function nativeOrdinarySaveOperationState(owner,task,c,kind){
 const o=owners.get(owner),t=o?.tasks.get(task),p=t&&o.parentOperations.get(t.capture.request.origin.operation);
 const expected=kind==='source-save'?'ordinary-source-save':kind==='annotation-save'?'ordinary-annotation-save':null;
 if(!expected||!p||p.retired||p.kind!==expected||p.task!==task||!o.nativeWriterDomains.has(p.domain)||!equal(p.context,c)||!equal(c,invocationAuthorityContext(p.domain)))return null;
 const required=t.capture.request.acknowledgment==='synchronized'||t.capture.request.acknowledgment==='ordinary'&&t.capture.registration.syncSupported?'synchronized':'written';
 const receipt={kind,phase:t.phase,stopped:t.stopped,quiescent:t.quiescent,safeDisposition:t.disposition,contested:t.contested,destinationFaulted:o.faults.has(t.key),knowledge:copy(t.knowledge),requiredAcknowledgment:required,requestedStrengthSatisfied:t.knowledge.application==='written'&&(required==='written'||t.knowledge.synchronization==='succeeded')&&!t.contested&&!o.faults.has(t.key)};
 return {domain:p.domain,maxReplyBytes:o.limits.maxEnvelopeBytes+512,receipt};
}
export function retireSourceSave(owner,task,state,g){
  const o=owners.get(owner),t=o?.tasks.get(task);if(!t||t.annotation&&!['retirementAllowed','retentionReleased','deliveryDrained'].every(k=>g?.[k]===true)||!t.reservationReleased||state?.authority!==o.authority||size(state)>o.limits.maxRootBytes||!g?.retirementAllowed||!g.retentionReleased||!g.deliveryDrained||queued(t)||t.phase==='dispatched'&&(!t.quiescent||!resolved(t)))return unavailable();
  if(t.nativeSourceDomain&&(state!==invocationAuthorityRoot(t.nativeSourceDomain)||!['retirementAllowed','retentionReleased','deliveryDrained'].every(k=>g?.[k]===true)))return unavailable();
  if(t.nativeSourceDomain&&!t.annotation&&!nativeSourceProducerReferencesClear(state,o,t))return unavailable();
  if(t.annotation?.reservationClaim&&annotationWriteClaimBound(t.annotation.reservationClaim))return unavailable();
  if(!t.annotation&&t.phase==='dispatched'&&t.knowledge.application==='written'){
    const ds=state.documents.filter(d=>(d.live||d.lifecycle?.phase==='disposed-retained')&&equal(d.document,t.capture.request.document)).flatMap(d=>[d,...(d.bindingHistory??[]).map(h=>({...d,binding:h.binding,sourceBaseline:h.sourceBaseline}))]).filter(d=>equal(d.binding??d.lifecycle?.retainedBinding,t.capture.binding)),dependents=[...o.tasks.values()].filter(x=>x!==t&&queued(x)&&x.key===t.key);
    if(ds.some(d=>{const a=baselineAnchor(o,d.document,d.binding??d.lifecycle?.retainedBinding,d.sourceBaseline);return !a||a.serial<=t.serial;})||dependents.some(x=>x.capture.anchorSerial<=t.serial))return unavailable();
  }
  if(t.annotation&&t.capture.exportState?.originalAnnotationBinding&&t.phase==='dispatched'&&t.knowledge.application==='written'){
    if(!annotationPeerProducerReferencesClear(state,t,o)||t.annotation.reservationClaim&&!annotationPeerProducerReferencesClear(invocationAuthorityRoot(t.annotation.domain),t,o))return unavailable();
    const entries=annotationBindingEntries(state,t.capture.request.document,o,t.capture.exportState.originalAnnotationBinding.generation);if(!entries||entries.filter(e=>equal(e.binding,t.capture.exportState.originalAnnotationBinding)).some(e=>{const a=annotationBaselineAnchor(o,t.capture.request.document,e.binding,e.baseline);return !a||a.serial<=t.serial;}))return unavailable();
    if(t.annotation.reservationClaim){const actual=invocationAuthorityRoot(t.annotation.domain),nativeEntries=actual?.authority===o.authority&&annotationBindingEntries(actual,t.capture.request.document,o,t.capture.exportState.originalAnnotationBinding.generation);if(!nativeEntries||nativeEntries.filter(e=>equal(e.binding,t.capture.exportState.originalAnnotationBinding)).some(e=>{const a=annotationBaselineAnchor(o,t.capture.request.document,e.binding,e.baseline);return !a||a.serial<=t.serial;}))return unavailable();}
  }
  const retired=t.annotation?retireSerializedAnnotation(o.codec,t.artifact,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true}):retireSerializedSource(o.codec,t.artifact,{retentionReleased:true,deliveryDrained:true});if(retired.status!=='retired')return unavailable();if(t.annotation?.reservationClaim)retireAnnotationWriteClaim(t.annotation.reservationClaim);forgetNativeSourceTaskCodec(task);
  const parent=o.parentOperations.get(t.capture.request.origin.operation);if(parent?.task===task)o.parentOperations.set(t.capture.request.origin.operation,parent.retirement);
  o.tasks.delete(task);return {status:'retired'};
}
const sourceRetentionOrigin=r=>exact(r,catalog.originFields)&&Object.values(r).every(s=>scalar(s)&&s.length>0);
function sourceBindingRetentionShape(b){const fields=['generation','provider','resource','registrationGeneration','identityDomain','identityKey','locator'];return exact(b,[...fields,...(Object.hasOwn(b??{},'readOrigin')?['readOrigin']:[]),...(Object.hasOwn(b??{},'writeOrigin')?['writeOrigin']:[])])&&natural(b.generation)&&b.generation>0&&identity(b.provider)&&identity(b.resource)&&natural(b.registrationGeneration)&&[b.identityDomain,b.identityKey,b.locator].every(s=>scalar(s)&&s.length>0)&&(!Object.hasOwn(b,'readOrigin')||sourceRetentionOrigin(b.readOrigin))&&(!Object.hasOwn(b,'writeOrigin')||sourceRetentionOrigin(b.writeOrigin));}
function sourceBaselineRetentionShape(b){if(b===null)return true;const fields=b?.kind==='acknowledged-write'?continuity.baselineFields:b?.kind==='loaded-capture'?['kind','bindingGeneration','sourceRevision','serializationGeneration','profile','origin','provider','resourceVersion','coherence','bom','bytes']:null;return fields&&exact(b,fields)&&sourceRetentionOrigin(b.origin)&&identity(b.provider)&&[b.bindingGeneration,b.sourceRevision,b.serializationGeneration].every(n=>natural(n)&&n>0)&&scalar(b.profile)&&b.profile.length>0&&typeof b.bom==='boolean'&&(b.resourceVersion===null||version(b.resourceVersion))&&Array.isArray(b.bytes)&&b.bytes.every(v=>Number.isInteger(v)&&v>=0&&v<=255);}
function sourceHistoryRetentionShape(history,l){return Array.isArray(history)&&history.length<=l.maxRegistryRows&&history.every(h=>exact(h,['binding','sourceBaseline'])&&sourceBindingRetentionShape(h.binding)&&sourceBaselineRetentionShape(h.sourceBaseline));}
function nativeSourceProducerReferencesClear(state,o,t){
 if(!Array.isArray(state?.documents)||state.documents.length>o.limits.maxRegistryRows)return false;const wanted=t.capture.request.origin;
 for(const d of state.documents){const life=d.lifecycle;if(d.live===false&&life?.phase==='provenance-released'){if(d.binding!==null||life.retainedBinding!==null)return false;continue;}if(d.live!==true&&!(d.live===false&&life?.phase==='disposed-retained'))return false;const history=Object.hasOwn(d,'bindingHistory')?d.bindingHistory:[];if(!sourceHistoryRetentionShape(history,o.limits)||!Object.hasOwn(d,'sourceBaseline')||!sourceBaselineRetentionShape(d.sourceBaseline))return false;const binding=d.binding??life?.retainedBinding??null,entries=[{binding,sourceBaseline:d.sourceBaseline},...history];if(binding!==null&&!sourceBindingRetentionShape(binding))return false;
  for(const e of entries){if(e.sourceBaseline?.kind==='acknowledged-write'&&equal(e.sourceBaseline.origin,wanted))return false;if(!equal(d.document,t.capture.request.document)&&equal(e.binding?.writeOrigin,wanted))return false;}
 }
 for(const peer of o.tasks.values()){if(peer===t||peer.phase!=='queued'||peer.stopped)continue;const b=peer.capture.baseline;if(b?.kind==='acknowledged-write'&&equal(b.origin,wanted))return false;}
 return true;
}
// Current lifecycle supervision, not public status or a physical-release witness.
export function sourceSaveDestinationResolved(owner,document,binding,g){
  const o=owners.get(owner)??readLifecycleWriterInspection(owner);if(!o||document?.authority!==o.authority||!binding||!allowed(g,['authorized','scope','continuous','bindingCoverageComplete','writerCoverageComplete','dispositionReadable']))return false;const key=sourceSaveDestinationKey(binding);
  return !o.faults.has(key)&&[...o.tasks.values()].every(t=>t.key!==key||(t.phase==='queued'?t.stopped:resolved(t)));
}
export function prepareRetainedLifecycleWriterGuard(owner,document,binding,g){const o=owners.get(owner)??readLifecycleWriterInspection(owner);if(!o||!identity(document)||document.authority!==o.authority||!['ownerControlAllowed','preparationReserved','resourcesReady'].every(k=>g?.[k]===true)||binding&&!sourceSaveDestinationResolved(owner,document,binding,g))return null;return sealRetainedLifecycleWriterGuard(owner,o,document,binding);}
export function prepareSourceRebindWriterGuard(owner,state,request,g){const o=owners.get(owner)??readLifecycleWriterInspection(owner);if(!o||state?.authority!==o.authority||!['ownerControlAllowed','preparationReserved','resourcesReady'].every(k=>g?.[k]===true))return null;const d=rootDocument(state,request.document,o.limits),row=targetRegistration(state,request,o.limits);return d&&row?sealSourceRebindWriterGuard(owner,o,d.document,d.binding,row):null;}
export function publishSourceSaveBaseline(owner,state,document,g){
  const o=owners.get(owner),fail=()=>({status:'unavailable',state});if(!o||state?.authority!==o.authority||!allowed(g,continuity.publicationGates))return fail();const d=rootDocument(state,document,o.limits);if(!d)return fail();
  const candidates=[...o.tasks.values()].filter(t=>!t.annotation&&qualified(o,t)&&equal(t.capture.request.document,document)&&equal(t.capture.binding,d.binding)&&baselineRegistrationCurrent(state,d,t,o)).sort((a,b)=>b.serial-a.serial);if(!candidates.length)return fail();const b=writtenBaseline(o,candidates[0]);if(!b)return fail();if(equal(d.sourceBaseline,b))return {status:'unchanged',state};
  if(!natural(state.publication)||state.publication===Number.MAX_SAFE_INTEGER)return fail();const next=copy(state);next.documents.find(x=>equal(x.document,document)).sourceBaseline=b;next.publication++;return size(next)<=o.limits.maxRootBytes?{status:'published',state:next}:fail();
}
export function inspectSourceOutput(owner,state,document,g){
  const o=owners.get(owner);if(!o||state?.authority!==o.authority||!allowed(g,continuity.comparisonGates))return unavailable();const d=rootDocument(state,document,o.limits);if(!d||!Array.isArray(state.resourceOwners)||state.resourceOwners.length>o.limits.maxRegistryRows||!d.binding||!baselineAnchor(o,document,d.binding,d.sourceBaseline))return unavailable();const rows=state.resourceOwners.filter(r=>r.identityDomain===d.binding.identityDomain&&r.identityKey===d.binding.identityKey);if(rows.length!==1||!equal(rows[0].document,document)||rows[0].bindingGeneration!==d.binding.generation)return unavailable();
  const temp=newSourceCodecRegistry(o.codecLimits),c=captureSerializedSource(temp,state,{contract:'serialized-source/1',operation:'capture-source',document,expectedSource:d.revision,expectedSerialization:d.serialization?.generation},codecGates);if(c.status!=='captured')return unavailable();
  const bytes=artifactBytes({...o,codec:temp},{artifact:c.artifact,capture:{payloadByteLength:d.source.length+(d.serialization.bom?3:0)}});retireSerializedSource(temp,c.artifact,{retentionReleased:true,deliveryDrained:true});if(bytes===null||bytes.length>o.limits.maxComparisonBytes-d.sourceBaseline.bytes.length)return unavailable();
  const out={status:'source-output',comparison:equal(bytes,d.sourceBaseline.bytes)?'equal':'different',current:{sourceRevision:d.revision,serializationGeneration:d.serialization.generation},baseline:{kind:d.sourceBaseline.kind,sourceRevision:d.sourceBaseline.sourceRevision,serializationGeneration:d.sourceBaseline.serializationGeneration,origin:copy(d.sourceBaseline.origin)},externalFreshness:'not-established'};return size(out)<=o.limits.maxEnvelopeBytes?out:unavailable();
}
function annotationBaselineCurrent(state,d,t,o){
 const b=d.ordinaryBindings?.annotations;if(!b||!equal(b,t.capture.exportState?.originalAnnotationBinding))return false;const rows=state.writeRegistrations.filter(r=>equal(r.provider,b.provider)&&equal(r.resource,b.resource));return rows.length===1&&equal(rows[0],t.capture.registration)&&ordinaryAnnotationOwner(d,state.resourceOwners.find(r=>bindingKey(r)===bindingKey(b)));
}
export function publishAnnotationSaveBaseline(owner,state,document,c,g,domain=null){
 const o=owners.get(owner),fail=()=>({status:'unavailable',state});if(!o||state?.authority!==o.authority||!allowed(g,annotationBaselines.publicationGates))return fail();const d=rootDocument(state,document,o.limits),binding=d?.ordinaryBindings?.annotations;if(!d||!binding||!ordinarySourceBindingReady(state,d,d.binding,g))return fail();
 if(!Object.hasOwn(d,'annotationBaseline')&&g.annotationBaselineAbsentAttested!==true||Object.hasOwn(d,'annotationBaseline')&&d.annotationBaseline!==null&&!annotationBaselineAnchor(o,document,binding,d.annotationBaseline,domain))return fail();
 const candidates=[...o.tasks.values()].filter(t=>t.annotation&&qualified(o,t)&&equal(t.capture.request.document,document)&&annotationBaselineCurrent(state,d,t,o)).sort((a,b)=>b.serial-a.serial);if(!candidates.length||!equal(c,candidates[0].annotation.basis.context))return fail();const b=writtenAnnotationBaseline(o,candidates[0]);if(!b)return fail();if(equal(b,d.annotationBaseline))return {status:'unchanged',state};
 if(g.versionsAvailable!==true||!natural(state.publication)||state.publication===Number.MAX_SAFE_INTEGER)return fail();const next=copy(state);next.documents.find(d=>equal(d.document,document)).annotationBaseline=b;next.publication++;return size(next)<=o.limits.maxRootBytes?{status:'published',state:next}:fail();
}
// Synchronous trusted coordination: certificate creation and native consumption
// share one serialized call; no prepared root/certificate escapes to a caller.
export function installAnnotationWriteBaseline(owner,task,domain,pool,g){
 const o=owners.get(owner),t=o?.tasks.get(task),claim=t?.annotation?.reservationClaim;if(!o||!claim||t.annotation.domain!==domain||!allowed(g,annotationInstallationGates)||invocationUsage(domain)?.faulted!==false)return unavailable();
 const s=readAnnotationWriteClaim(claim),before=invocationAuthorityRoot(domain),d=rootDocument(before,t.capture.request.document,o.limits);if(!s||s.owner!==owner||s.task!==task||!d||!ordinarySourceBindingReady(before,d,d.binding,g)||!annotationBaselineCurrent(before,d,t,o))return unavailable();
 const b=writtenAnnotationBaseline(o,t);if(!b)return unavailable();const result=publishAnnotationSaveBaseline(owner,before,d.document,t.annotation.basis.context,g,domain);let next,baseline,producerTask=task,mode;
 const bound=s.link?.domain===domain&&s.link.pool===pool;
 if(result.status==='published'&&equal(result.state.documents.find(x=>equal(x.document,d.document)).annotationBaseline,b)){
  if(bound)mode='publish';else if(s.released&&['baseline-installed','baseline-current'].includes(s.transfer)&&annotationBaselineAnchor(o,d.document,d.ordinaryBindings.annotations,d.annotationBaseline)&&equal(d.annotationBaseline.origin,b.origin))mode='refresh';else return unavailable();next=result.state;baseline=b;
 }else{
  const anchor=annotationBaselineAnchor(o,d.document,d.ordinaryBindings.annotations,d.annotationBaseline),entry=anchor&&[...o.tasks.entries()].find(([,p])=>p.annotation&&equal(p.capture.request.origin,d.annotationBaseline.origin));
  if(!anchor||anchor.serial<t.serial||!entry||!entry[1].annotation.reservationClaim||!annotationBaselineCurrent(before,d,entry[1],o)||!equal(d.annotationBaseline.context,t.annotation.basis.context))return unavailable();
  if(!bound)return s.released&&entry[0]===task&&equal(d.annotationBaseline,b)?{status:'current'}:unavailable();producerTask=entry[0];mode=producerTask===task?'current':'superseded';next=before;baseline=d.annotationBaseline;
 }
 const token=sealAnnotationBaselineInstallation(owner,task,claim,domain,pool,before,next,baseline,producerTask,mode);if(!token)return unavailable();try{return installInvocationAnnotationBaseline(domain,pool,claim,token,g);}finally{discardAnnotationBaselineInstallation(token);}
}
export function inspectAnnotationOutput(owner,state,document,domain,c,binding,profile,g){
 const o=owners.get(owner);if(!o||state?.authority!==o.authority||!allowed(g,annotationBaselines.comparisonGates))return unavailable();const d=rootDocument(state,document,o.limits),b=d?.annotationBaseline;if(!d||!d.ordinaryBindings?.annotations||!ordinarySourceBindingReady(state,d,d.binding,g)||!annotationBaselineAnchor(o,document,d.ordinaryBindings.annotations,b,domain)||!equal(c,b.context))return unavailable();
 const actual=invocationAuthorityRoot(domain)?.documents.filter(x=>equal(x.document,document)),fields=['document','live','source','revision','annotationSet','records','annotationLifecycle'];if(actual?.length!==1||!equal(fields.map(k=>d[k]),fields.map(k=>actual[0][k]))||!profile?.native||b.bytes.length>o.limits.maxComparisonBytes)return unavailable();
 const p=copy(profile),remaining=o.limits.maxComparisonBytes-b.bytes.length;p.native.maxArtifactBytes=Math.min(p.native.maxArtifactBytes,o.codecLimits.maxOutputBytes,remaining);const current=captureNativeAnnotationArtifact(domain,document,c,binding,p,g);if(current.status!=='annotation-artifact'||current.bytes.length>remaining)return unavailable();
 const out={status:'annotation-output',comparison:equal(current.bytes,b.bytes)?'equal':'different',current:{sourceRevision:d.revision,annotationSet:d.annotationSet},baseline:{sourceRevision:b.sourceRevision,annotationSet:b.annotationSet,annotationBindingGeneration:b.annotationBindingGeneration,origin:copy(b.origin)},externalFreshness:'not-established'};return size(out)<=o.limits.maxEnvelopeBytes?out:unavailable();
}
// Trusted supervision: declared logical charges, not public reflection or physical costs.
export function sourceSaveUsage(owner){const o=owners.get(owner);return o?{tasks:o.tasks.size,queued:[...o.tasks.values()].filter(queued).length,outstanding:[...o.tasks.values()].filter(held).length,issued:o.used.size,heldBytes:o.annotationReadOwners.size*o.limits.maxCaptureBytes+o.tasks.size*o.charge+[...o.tasks.values()].reduce((n,t)=>n+(t.annotation?.claimCharge??0),0),faultedDestinations:o.faults.size}:null;}
export function sourceSaveCodecUsage(owner){const o=owners.get(owner);return o?{codec:sourceCodecUsage(o.codec),preparation:annotationPreparationUsage(o.codec)}:null;}

// Explicitly select guarded prelaunch capacity binding for an actual queued
// annotation ticket. A failed pool bind cannot silently fall back to launch.
export function claimAnnotationWriteReservation(owner,task,g){
 const o=owners.get(owner),t=o?.tasks.get(task);if(!t?.annotation||!allowed(g,annotationClaimGates))return unavailable();if(t.annotation.reservationClaim)return {status:'claim',claim:t.annotation.reservationClaim};
 const c=t.capture,b=c.exportState?.originalAnnotationBinding;if(t.phase!=='queued'||t.stopped||!b||!t.annotation.domain)return unavailable();const bytes=annotationArtifactBytes(o,t);if(bytes===null)return unavailable();
 const draft={kind:'acknowledged-annotation-write',annotationBindingGeneration:b.generation,receiptBindingGeneration:c.binding.generation,sourceRevision:c.annotation.sourceRevision,annotationSet:c.annotation.annotationSet,profile:c.annotation.profile,context:copy(t.annotation.basis.context),origin:copy(c.request.origin),provider:copy(c.registration.provider),resourceVersion:null,procedure:c.registration.procedure,acknowledgment:acknowledgment(t),requiredAcknowledgment:copy(c.request.acknowledgment),dispatchSerial:0,bytes},envelope=prepareAnnotationBaselineEnvelope(draft,{maxRootBytes:o.limits.maxRootBytes,maxBaselineBytes:o.limits.maxBaselineBytes,maxTagBytes:o.limits.maxTagBytes}),charge=size({draft,claim:{kind:'annotation-write-reservation-claim/1'},owner,task,domain:{kind:'invocation-domain/1'},link:{domain:{kind:'invocation-domain/1'},pool:{kind:'annotation-baseline-reservation-pool/1'},reservation:{kind:'annotation-baseline-reservation/1'}},released:false,transfer:'baseline-superseded'});
 if(!envelope||!natural(charge)||charge>o.limits.maxBaselineBytes||charge>o.limits.maxHeldBytes-sourceSaveUsage(owner).heldBytes)return unavailable();const claim=createAnnotationWriteClaim(owner,task,o,t,draft,t.annotation.domain);t.annotation.reservationClaim=claim;t.annotation.claimCharge=charge;return {status:'claim',claim};
}

// Explicit annotation export joins this exact writer/codec owner, not a parallel lane.
// The standalone admitAnnotationExport helper below retains its legacy supplied-
// root composition. Native export enters through prepareNativeSourceDestination,
// which requires an actual funded writer/domain binding before this preparation.
function annotationWriteCandidate(owner,state,r,domain,c,g,ordinary=false){
  const o=owners.get(owner),fail=()=>null;
  if(!o||state?.authority!==o.authority||!annotationCatalog.launchGates.every(k=>g?.[k]===true)||!exact(r,annotationCatalog.requestFields)||r.contract!==annotationCatalog.contract||r.operation!==annotationCatalog.operation||!identity(r.document)||r.document.authority!==o.authority||!identity(r.provider)||!identity(r.resource)||!exact(r.origin,catalog.originFields)||!Object.values(r.origin).every(s=>scalar(s)&&s.length>0)||![r.expectedSource,r.expectedAnnotationSet,r.expectedBinding,r.expectedRegistration].every(natural)||!['best-effort','conditional'].includes(r.protection)||!['ordinary','write-only','synchronized'].includes(r.acknowledgment)||!exact(r.condition,['policy','presence','resourceVersion'])||r.condition.policy!=='pinned'||!['absent','present'].includes(r.condition.presence)||!(r.condition.presence==='absent'?r.condition.resourceVersion===null:version(r.condition.resourceVersion))||size(r)>o.limits.maxCaptureBytes)return fail();
  if(o.parentOperations.has(r.origin.operation))return fail();
  const d=rootDocument(state,r.document,o.limits),actual=invocationAuthorityRoot(domain)?.documents.filter(x=>equal(x.document,r.document)),row=targetRegistration(state,r,o.limits);
  if(!d||actual?.length!==1||!equal([d.document,d.live,d.source,d.revision,d.annotationSet],[actual[0].document,actual[0].live,actual[0].source,actual[0].revision,actual[0].annotationSet])||!row||d.revision!==r.expectedSource||d.annotationSet!==r.expectedAnnotationSet||(d.binding?.generation??0)!==r.expectedBinding||r.condition.resourceVersion!==null&&size(r.condition.resourceVersion)>o.limits.maxTagBytes||r.condition.presence==='absent'&&(g.createDestinationAllowed!==true||!row.exclusiveCreate)||r.condition.presence==='present'&&g.overwriteDestinationAllowed!==true)return fail();
  const found=ownedDestination(state,o,row,r.document);if(!found)return fail();const old=d.binding??null;
  if(found.row){const active=(old&&bindingKey(old)===bindingKey(row)&&found.row.bindingGeneration===old.generation)||ordinaryAnnotationOwner(d,found.row),pending=Array.isArray(found.row.exportOrigins)&&found.row.exportOrigins.length>0&&found.row.exportOrigins.length<o.limits.maxTasks&&found.row.exportOrigins.every(origin=>[...o.tasks.values()].some(t=>t.capture.exportState&&!t.reservationReleased&&t.key===bindingKey(row)&&equal(t.capture.request.document,r.document)&&equal(t.capture.request.origin,origin)&&equal(t.capture.exportState.originalBinding,old)));if(!active&&!pending||Object.hasOwn(found.row,'exportOrigins')&&!pending)return fail();}
  const originalAnnotationBinding=ordinaryAnnotationOwner(d,found.row)&&sameBindingContext(d.ordinaryBindings.annotations,row)&&ordinarySourceBindingReady(state,d,d.binding,g)?copy(d.ordinaryBindings.annotations):null;
  if(ordinary&&(!originalAnnotationBinding||r.condition.presence!=='present'))return fail();
  if(ordinary&&(d.annotationBaseline?.kind==='loaded-annotation-capture')&&(!annotationBaselineAnchor(o,d.document,originalAnnotationBinding,d.annotationBaseline,domain)||!equal(r.condition.resourceVersion,d.annotationBaseline.resourceVersion)))return fail();
  const request={...copy(r),purpose:'annotation-export'},destination=ordinary?copy(originalAnnotationBinding):{generation:sameBindingContext(old,row)?old.generation:0,...bindingContext(row)},annotation={sourceRevision:d.revision,annotationSet:d.annotationSet,profile:'vellum.annotations/1'},capture={request,registration:copy(row),binding:destination,baseline:null,anchorSerial:0,annotation,payloadByteLength:o.codecLimits.maxOutputBytes,exportState:{originalBinding:copy(old),originalAnnotationBinding}};
  const receiptCapture={document:copy(r.document),annotation:copy(annotation),bindingGeneration:destination.generation,payloadByteLength:capture.payloadByteLength,procedure:row.procedure,condition:copy(r.condition),export:{purpose:'annotation-export',sourceBindingGeneration:r.expectedBinding,proposedBindingGeneration:destination.generation}},initialCondition={resourceVersion:copy(r.condition.resourceVersion),chain:[],presence:r.condition.presence},minimal={contract:catalog.evidenceContract,task:{kind:'source-save-task/1'},origin:r.origin,provider:row.provider,resource:row.resource,capture:{...receiptCapture,dispatchSerial:Number.MAX_SAFE_INTEGER,dispatchCondition:initialCondition},kind:'result',application:'unknown',observedBytes:0,synchronization:'unknown',resourceVersion:null},key=bindingKey(row),used=originKey(r.origin),u=sourceSaveUsage(owner);
  const annotationBasis={context:copy(c),document:copy(r.document),sourceRevision:d.revision,annotationSet:d.annotationSet};
  if(ordinary&&d.annotationBaseline?.kind==='loaded-annotation-capture')capture.baseline=copy(d.annotationBaseline);
  if(size({capture,annotationBasis})>o.limits.maxCaptureBytes||size(minimal)>o.limits.maxEnvelopeBytes||o.used.has(used)||o.faults.has(key)||u.tasks>=o.limits.maxTasks||u.issued>=o.limits.maxIssued||[...o.tasks.values()].filter(t=>t.key===key&&queued(t)).length>=o.limits.maxPerDestination||o.charge>o.limits.maxHeldBytes-u.heldBytes)return fail();
  let root=state;if(!ordinary){const next=copy(state);let target=next.resourceOwners.find(x=>bindingKey(x)===key);if(!target){target={identityDomain:row.identityDomain,identityKey:row.identityKey,document:copy(r.document),bindingGeneration:destination.generation};next.resourceOwners.push(target);}target.exportOrigins=[...(target.exportOrigins??[]),copy(r.origin)];root=finishDestinationRoot(state,next,o);if(!root)return fail();}
  if(ordinary){request.contract='annotation-save/1';request.purpose='annotation-save';request.operation='captured-annotation-save';delete receiptCapture.export;}
  return {o,root,capture,receiptCapture,key,used};
}
export function admitAnnotationExport(owner,state,r,domain,c,binding,profile,g){
  const candidate=annotationWriteCandidate(owner,state,r,domain,c,g);if(!candidate)return {status:'unavailable',state};const {o,root,capture,receiptCapture,key,used}=candidate,fail=()=>({status:'unavailable',state});
  const encoded=captureSerializedAnnotation(o.codec,domain,r.document,c,binding,profile,g);if(encoded.status!=='captured')return fail();
  capture.payloadByteLength=encoded.byteLength;receiptCapture.payloadByteLength=encoded.byteLength;
  const task=Object.freeze({kind:'source-save-task/1'});o.used.add(used);o.tasks.set(task,{capture,receiptCapture,key,artifact:encoded.artifact,annotation:{basis:copy(encoded.basis),domain},phase:'queued',serial:null,dispatchCondition:null,causal:null,stopped:false,quiescent:false,disposition:false,contested:false,reservationReleased:false,bindingApplied:false,knowledge:{application:'unknown',observedBytes:0,synchronization:'unknown',resourceVersion:null},conflicts:[]});return {status:'admitted',task,state:root};
}
export function readAnnotationWritePayload(owner,task,range,basis,g){
  const o=owners.get(owner),t=o?.tasks.get(task);if(!t?.annotation||t.phase!=='dispatched'||t.stopped||!equal(basis,t.annotation.basis)||!annotationCatalog.releaseGates.every(k=>g?.[k]===true)||!equal(g.provider,t.capture.registration.provider)||!equal(g.resource,t.capture.registration.resource))return unavailable();
  return readSerializedAnnotation(o.codec,t.artifact,range,basis,g);
}

// Ordinary active-binding save, not a copy export or a public wire protocol.
// All preparation and final assignment run synchronously under native serialization.
function saveOperationRelease(o,domain,kind,intent,c,g){
 const request=intent?.request;
 return !!o&&o.nativeWriterDomains.has(domain)&&equal(c,invocationAuthorityContext(domain))&&
  ['authorized','scope','continuous','live','notStopped','writeAllowed','dispositionReadable','retained','replyReady','transferReady','resourcesReady',kind==='ordinary-source-save'?'sourceReadable':'annotationReadable'].every(k=>g?.[k]===true)&&
  request&&exact(request.origin,catalog.originFields)&&Object.values(request.origin).every(s=>scalar(s)&&s.length>0)&&size(intent)<=o.limits.maxCaptureBytes&&
  size({status:'observe-existing',task:{kind:'source-save-task/1'}})<=o.limits.maxEnvelopeBytes;
}
function prepareSaveOperation(o,domain,kind,intent,c,g){
 if(!saveOperationRelease(o,domain,kind,intent,c,g)||o.parentOperations.has(intent.request.origin.operation)||
    [...o.used].some(key=>JSON.parse(key)[0]===intent.request.origin.operation))return null;
 const parent={kind,context:copy(c),intent:copy(intent),retirement:{retired:true}};
 return size([intent.request.origin.operation,{...parent,domain:{kind:'invocation-domain/1'},task:{kind:'source-save-task/1'}}])<=o.limits.maxCaptureBytes?parent:null;
}
// Internal closed lookup only. An unconsumed result is not a public status.
function priorSaveOperation(o,domain,kind,intent,c,g){
 if(!saveOperationRelease(o,domain,kind,intent,c,g))return unavailable();
 const parent=o.parentOperations.get(intent.request.origin.operation);
 if(!parent)return {status:'unconsumed'};
 return !parent.retired&&parent.domain===domain&&parent.kind===kind&&equal(parent.context,c)&&equal(parent.intent,intent)&&o.tasks.has(parent.task)?
  {status:'observe-existing',task:parent.task}:unavailable();
}
export function ordinarySourceSaveOperationPrior(owner,domain,request,c,g){
 const o=owners.get(owner);if(!o||size({request,context:c})>o.limits.maxCaptureBytes)return unavailable();
 const intent=copy({request}),context=copy(c),prior=priorSaveOperation(o,domain,'ordinary-source-save',intent,context,g);
 return prior.status==='unconsumed'?{...prior,request:intent.request,context}:prior;
}
export function requestInvocationOrdinaryAnnotationSave(owner,domain,pool,r,c,binding,profile,operationContext,g){
 const o=owners.get(owner),input={request:r,context:c,binding};if(!o||size({intent:input,scope:operationContext})>o.limits.maxCaptureBytes)return unavailable();
 const intent=copy(input),scope=copy(operationContext),prior=priorSaveOperation(o,domain,'ordinary-annotation-save',intent,scope,g);
 return prior.status==='unconsumed'?admitOrdinaryAnnotationSave(owner,domain,pool,intent.request,intent.context,intent.binding,profile,g,scope):prior;
}
export function admitOrdinaryAnnotationSave(owner,domain,pool,r,c,binding,profile,g,operationContext=null){
 if(!owners.get(owner)?.nativeWriterDomains.has(domain))return unavailable();
 const section=beginInvocationAdmissionMutation(domain);if(!section)return unavailable();try{
 if(!invocationOrdinaryAdmissionOpen(domain))return unavailable();
 if(r?.contract!=='annotation-save/1'||r.operation!=='captured-annotation-save')return unavailable();
 const state=invocationAuthorityRoot(domain),candidate=annotationWriteCandidate(owner,state,{...r,contract:annotationCatalog.contract,operation:annotationCatalog.operation},domain,c,g,true);if(!candidate||!allowed(g,annotationClaimGates)||invocationUsage(domain)?.faulted!==false)return unavailable();
 const {o,capture,receiptCapture,key,used}=candidate,prepared=prepareSerializedAnnotation(o.codec,domain,r.document,c,binding,profile,g);if(prepared.status!=='prepared')return unavailable();
 let claim=null,stage=null,committed=false,issued=false;
 try{
  const value=inspectSerializedAnnotationPreparation(o.codec,prepared.preparation,domain,g);if(value.status!=='prepared-annotation')return unavailable();capture.payloadByteLength=value.byteLength;receiptCapture.payloadByteLength=value.byteLength;
  const task=Object.freeze({kind:'source-save-task/1'}),b=capture.exportState.originalAnnotationBinding,ticket={capture,receiptCapture,key,artifact:null,annotation:{basis:copy(value.basis),domain,ordinary:true},phase:'queued',serial:null,dispatchCondition:null,causal:null,stopped:false,quiescent:false,disposition:false,contested:false,reservationReleased:true,bindingApplied:false,knowledge:{application:'unknown',observedBytes:0,synchronization:'unknown',resourceVersion:null},conflicts:[]};
  const parent=operationContext===null?null:prepareSaveOperation(o,domain,'ordinary-annotation-save',{request:r,context:c,binding},operationContext,g);
  if(operationContext!==null&&!parent)return unavailable();
  const draft={kind:'acknowledged-annotation-write',annotationBindingGeneration:b.generation,receiptBindingGeneration:capture.binding.generation,sourceRevision:capture.annotation.sourceRevision,annotationSet:capture.annotation.annotationSet,profile:capture.annotation.profile,context:copy(value.basis.context),origin:copy(capture.request.origin),provider:copy(capture.registration.provider),resourceVersion:null,procedure:capture.registration.procedure,acknowledgment:acknowledgment(ticket),requiredAcknowledgment:copy(capture.request.acknowledgment),dispatchSerial:0,bytes:value.bytes};
  const envelope=prepareAnnotationBaselineEnvelope(draft,{maxRootBytes:o.limits.maxRootBytes,maxBaselineBytes:o.limits.maxBaselineBytes,maxTagBytes:o.limits.maxTagBytes}),charge=size({draft,claim:{kind:'annotation-write-reservation-claim/1'},owner,task,domain:{kind:'invocation-domain/1'},link:{domain:{kind:'invocation-domain/1'},pool:{kind:'annotation-baseline-reservation-pool/1'},reservation:{kind:'annotation-baseline-reservation/1'}},released:false,transfer:'baseline-superseded'});
  if(!envelope||!natural(charge)||charge>o.limits.maxBaselineBytes||o.charge+charge>o.limits.maxHeldBytes-sourceSaveUsage(owner).heldBytes)return unavailable();
  claim=createAnnotationWriteClaim(owner,task,o,ticket,draft,domain);ticket.annotation.reservationClaim=claim;ticket.annotation.claimCharge=charge;
  stage=prepareInvocationOrdinaryBaseline(domain,pool,claim,g,section);if(!stage||!validateInvocationOrdinaryBaseline(domain,pool,claim,stage,g,section)||invocationAuthorityRoot(domain)!==state)return unavailable();
  // No host/user callback, await or externally visible handle intervenes here.
  const encoded=commitSerializedAnnotationPreparation(o.codec,prepared.preparation,domain,g);if(encoded.status!=='captured')return unavailable();issued=true;ticket.artifact=encoded.artifact;o.used.add(used);o.tasks.set(task,ticket);
  if(parent)o.parentOperations.set(r.origin.operation,{...parent,domain,task});
  const reservation=commitInvocationOrdinaryBaseline(domain,pool,claim,stage,g,section);if(!reservation)throw Error('Trusted serialized ordinary annotation admission commit invalidated');
  committed=true;return {status:'admitted',task};
 }finally{
  if(stage)discardInvocationOrdinaryBaseline(stage);if(!committed&&!issued){if(claim)retireAnnotationWriteClaim(claim);discardSerializedAnnotationPreparation(o.codec,prepared.preparation,{ownerControlAllowed:true});}
 }
 }finally{endInvocationAdmissionMutation(domain,section);}
}

function targetRegistration(state,r,l){
  if(!Array.isArray(state?.writeRegistrations)||state.writeRegistrations.length>l.maxRegistryRows)return null;const rows=state.writeRegistrations.filter(x=>equal(x.provider,r.provider)&&equal(x.resource,r.resource));if(rows.length!==1)return null;const row=rows[0];
  return exact(row,catalog.registrationFields)&&identity(row.provider)&&identity(row.resource)&&natural(row.generation)&&row.generation===r.expectedRegistration&&row.identityKnown===true&&row.logicalContinuity===true&&['syncSupported','conditionalWrite','causalReceipts','nonReuseTokens','exclusiveCreate'].every(k=>typeof row[k]==='boolean')&&[row.identityDomain,row.identityKey,row.locator,row.procedure].every(s=>scalar(s)&&s.length>0&&Buffer.byteLength(s)<=l.maxTagBytes)&&!(r.protection==='conditional'&&r.condition?.presence!=='absent'&&!row.conditionalWrite)&&!(r.acknowledgment==='synchronized'&&!row.syncSupported)?row:null;
}
function baselineRegistrationCurrent(state,d,t,o){
  if(!t.capture.exportState)return equal(registration(state,d,t.capture.request,o.limits),t.capture.registration);const row=targetRegistration(state,t.capture.request,o.limits);if(!row||!equal(row,t.capture.registration)||!Array.isArray(state.resourceOwners)||state.resourceOwners.length>o.limits.maxRegistryRows)return false;const matches=state.resourceOwners.filter(x=>bindingKey(x)===bindingKey(row));return matches.length===1&&equal(matches[0].document,d.document)&&matches[0].bindingGeneration===d.binding.generation;
}
const bindingKey=b=>b?sourceSaveDestinationKey(b):null;
const bindingContext=row=>({provider:copy(row.provider),resource:copy(row.resource),registrationGeneration:row.generation,identityDomain:row.identityDomain,identityKey:row.identityKey,locator:row.locator});
const sameBindingContext=(b,row)=>b&&equal(Object.fromEntries(Object.keys(bindingContext(row)).map(k=>[k,b[k]])),bindingContext(row));
const containsOrigin=(row,origin)=>Array.isArray(row?.exportOrigins)&&row.exportOrigins.some(x=>equal(x,origin));
function ownedDestination(state,o,row,document){
  if(!Array.isArray(state.resourceOwners)||state.resourceOwners.length>o.limits.maxRegistryRows)return null;const rows=state.resourceOwners.filter(x=>bindingKey(x)===bindingKey(row));if(rows.length>1||rows.length===1&&!equal(rows[0].document,document))return null;return {row:rows[0]??null};
}
function reservationCurrent(state,t,o){const c=t.capture,row=targetRegistration(state,c.request,o.limits),found=row&&ownedDestination(state,o,row,c.request.document);return row&&equal(row,c.registration)&&found&&containsOrigin(found.row,c.request.origin);}
function exportCurrent(state,t,o,g){
  if(t.annotation?.ordinary){const d=rootDocument(state,t.capture.request.document,o.limits);return state===invocationAuthorityRoot(t.annotation.domain)&&d&&equal(d.binding,t.capture.exportState.originalBinding)&&ordinarySourceBindingReady(state,d,d.binding,g)&&annotationBaselineCurrent(state,d,t,o)&&g.overwriteDestinationAllowed===true;}
  const c=t.capture,d=rootDocument(state,c.request.document,o.limits);if(t.annotation&&(c.request.condition.presence==='absent'?g.createDestinationAllowed!==true:g.overwriteDestinationAllowed!==true))return false;return d&&equal(d.binding??null,c.exportState.originalBinding)&&!t.reservationReleased&&reservationCurrent(state,t,o)&&
    (c.request.condition.presence==='absent'?g.createDestinationAllowed&&c.registration.exclusiveCreate:g.overwriteDestinationAllowed);
}
function finishDestinationRoot(state,next,o){if(!natural(state.publication)||state.publication===Number.MAX_SAFE_INTEGER||next.resourceOwners.length>o.limits.maxRegistryRows)return null;next.publication++;return size(next)<=o.limits.maxRootBytes?next:null;}
function retainOldBinding(d,n,o){if(!d.binding||equal(d.binding,n.binding))return true;const history=Object.hasOwn(d,'bindingHistory')?d.bindingHistory:[];if(!Array.isArray(history)||history.length>=o.limits.maxRegistryRows||history.some(h=>!exact(h,['binding','sourceBaseline'])))return false;n.bindingHistory=[...copy(history),{binding:copy(d.binding),sourceBaseline:copy(d.sourceBaseline)}];return true;}
export function admitSourceExport(owner,state,r,g){return sourceExportCandidate(owner,state,r,g,false);}
function sourceExportCandidate(owner,state,r,g,prepare){
  const o=owners.get(owner),fail=()=>({status:'unavailable',state});if(!o||state?.authority!==o.authority||!allowed(g,exportsCatalog.gates)||!exact(r,exportsCatalog.exportFields)||r.contract!==exportsCatalog.exportContract||r.operation!=='captured-source-export'||!identity(r.document)||r.document.authority!==o.authority||!identity(r.provider)||!identity(r.resource)||!exact(r.origin,catalog.originFields)||!Object.values(r.origin).every(s=>scalar(s)&&s.length>0)||![r.expectedSource,r.expectedSerialization,r.expectedBinding,r.expectedRegistration].every(natural)||!['copy-export','save-as'].includes(r.purpose)||!['best-effort','conditional'].includes(r.protection)||!['ordinary','write-only','synchronized'].includes(r.acknowledgment)||!exact(r.condition,['policy','presence','resourceVersion'])||r.condition.policy!=='pinned'||!['absent','present'].includes(r.condition.presence)||!(r.condition.presence==='absent'?r.condition.resourceVersion===null:version(r.condition.resourceVersion))||size(r)>o.limits.maxCaptureBytes)return fail();
  const d=rootDocument(state,r.document,o.limits),row=targetRegistration(state,r,o.limits);if(!d||!row||d.revision!==r.expectedSource||d.serialization?.generation!==r.expectedSerialization||(d.binding?.generation??0)!==r.expectedBinding||r.condition.resourceVersion!==null&&size(r.condition.resourceVersion)>o.limits.maxTagBytes||r.condition.presence==='absent'&&(!g.createDestinationAllowed||!row.exclusiveCreate)||r.condition.presence==='present'&&!g.overwriteDestinationAllowed)return fail();
  const found=ownedDestination(state,o,row,r.document);if(!found)return fail();const old=d.binding??null,same=sameBindingContext(old,row),generation=same?old.generation:r.purpose==='copy-export'?0:r.expectedBinding+1;if(!natural(generation)||!same&&r.purpose==='save-as'&&r.expectedBinding===Number.MAX_SAFE_INTEGER)return fail();
  if(found.row){const active=(old&&bindingKey(old)===bindingKey(row)&&found.row.bindingGeneration===old.generation)||ordinaryAnnotationOwner(d,found.row);const pending=Array.isArray(found.row.exportOrigins)&&found.row.exportOrigins.length>0&&found.row.exportOrigins.length<o.limits.maxTasks&&found.row.exportOrigins.every(origin=>[...o.tasks.values()].some(t=>t.capture.exportState&&!t.reservationReleased&&t.key===bindingKey(row)&&equal(t.capture.request.document,r.document)&&equal(t.capture.request.origin,origin)&&equal(t.capture.exportState.originalBinding,old)));if(!active&&!pending||Object.hasOwn(found.row,'exportOrigins')&&!pending)return fail();}
  const binding=same?copy(old):{generation,...bindingContext(row),writeOrigin:copy(r.origin)};if(r.purpose==="save-as"&&!ordinarySourceBindingReady(state,d,binding,g))return fail();const exportState={originalBinding:copy(old)},next=copy(state);let target=next.resourceOwners.find(x=>bindingKey(x)===bindingKey(row));if(!target){target={identityDomain:row.identityDomain,identityKey:row.identityKey,document:copy(r.document),bindingGeneration:generation};next.resourceOwners.push(target);}target.exportOrigins=[...(target.exportOrigins??[]),copy(r.origin)];const root=finishDestinationRoot(state,next,o);if(!root)return fail();
  if(prepare){const plan=sourceAdmissionPlan(o,owner,state,r,row,binding,null,0,exportState);return plan?{status:'prepared',state:root,plan:{status:'admitted',capture:plan.capture,receiptCapture:plan.receiptCapture}}:fail();}const admitted=captureAdmission(o,owner,state,r,row,binding,null,0,exportState);return admitted.status==='admitted'?{...admitted,state:root}:fail();
}
function removeReservation(state,t){
  const next=copy(state),c=t.capture,row=next.resourceOwners.find(x=>bindingKey(x)===t.key&&equal(x.document,c.request.document));if(!containsOrigin(row,c.request.origin))return null;row.exportOrigins=row.exportOrigins.filter(x=>!equal(x,c.request.origin));
  if(!row.exportOrigins.length){delete row.exportOrigins;const d=next.documents.find(d=>equal(d.document,c.request.document));if((!d?.binding||bindingKey(d.binding)!==t.key||d.binding.generation!==row.bindingGeneration)&&!ordinaryAnnotationOwner(d,row))next.resourceOwners=next.resourceOwners.filter(x=>x!==row);}return next;
}
export function releaseSourceExport(owner,task,state,g){return sourceExportRelease(owner,task,state,g,true);}
function sourceExportRelease(owner,task,state,g,commit){
  const o=owners.get(owner),t=o?.tasks.get(task),fail=()=>({status:'unavailable',state});if(commit&&t?.nativeSourceDomain)return fail();if(!t?.capture.exportState||t.annotation&&(!exportsCatalog.activationGates.every(k=>g?.[k]===true)||g.reservationReleaseAllowed!==true)||state?.authority!==o.authority||!allowed(g,exportsCatalog.activationGates)||!g.reservationReleaseAllowed||size(state)>o.limits.maxRootBytes)return fail();if(t.reservationReleased)return {status:'unchanged',state};if(t.phase==='queued'?!t.stopped:!resolved(t)||o.faults.has(t.key))return fail();const next=removeReservation(state,t),root=next&&finishDestinationRoot(state,next,o);if(!root)return fail();if(commit)t.reservationReleased=true;return {status:'released',state:root};
}
export function activateSourceSaveAs(owner,task,state,probe,g){return sourceSaveAsCandidate(owner,task,state,probe,g,true);}
// Local activation only: no inspection fallback, new ticket, dispatch or I/O.
export function borrowSourceSaveAsActivationAuthority(owner,task,domain,g){const o=owners.get(owner);return o&&['ownerControlAllowed','preparationReserved','resourcesReady'].every(k=>g?.[k]===true)&&invocationUsage(domain)?.faulted===false?sealSourceSaveAsActivationAuthority(owner,o,task,domain):null;}
function sourceSaveAsCandidate(owner,task,state,probe,g,commit,actualWriter=null,nativeBaseline=null){
  const o=owners.get(owner)??actualWriter,t=o?.tasks.get(task),fail=()=>({status:'unavailable',state});if(commit&&t?.nativeSourceDomain)return fail();if(!t?.capture.exportState||t.capture.request.purpose!=='save-as'||state?.authority!==o.authority||!allowed(g,exportsCatalog.activationGates)||!g.probeAttested||!g.resourceContinuityAttested||t.stopped||!qualified(o,t)||!resolved(t))return fail();const c=t.capture,d=rootDocument(state,c.request.document,o.limits);
  if(t.bindingApplied)return d&&equal(d.binding,c.binding)?{status:'unchanged',state}:fail();if(!d||!exportCurrent(state,t,o,g)||!exact(probe,['provider','resource','registrationGeneration','identityDomain','identityKey','resourceVersion','presence'])||probe.presence!=='present'||!equal(probe.provider,c.registration.provider)||!equal(probe.resource,c.registration.resource)||probe.registrationGeneration!==c.registration.generation||probe.identityDomain!==c.registration.identityDomain||probe.identityKey!==c.registration.identityKey||!equal(probe.resourceVersion,t.knowledge.resourceVersion))return fail();
  if(!ordinarySourceBindingReady(state,d,c.binding,g))return fail();const old=d.binding;if(old&&!sourceSaveDestinationResolved(owner,d.document,old,g))return fail();if(!sourceSaveDestinationResolved(owner,d.document,c.binding,g))return fail();
  const oldRows=old?state.resourceOwners.filter(x=>bindingKey(x)===bindingKey(old)):[];if(old&&(oldRows.length!==1||!equal(oldRows[0].document,d.document)||oldRows[0].bindingGeneration!==old.generation||bindingKey(old)!==t.key&&oldRows[0].exportOrigins?.length))return fail();
  const b=nativeBaseline??writtenBaseline(o,t);if(!b)return fail();let next=copy(state);if(old&&bindingKey(old)!==t.key)next.resourceOwners=next.resourceOwners.filter(x=>bindingKey(x)!==bindingKey(old));const n=next.documents.find(x=>equal(x.document,d.document)),row=next.resourceOwners.find(x=>bindingKey(x)===t.key);n.binding=copy(c.binding);n.sourceBaseline=b;if(!retainOldBinding(d,n,o))return fail();row.bindingGeneration=c.binding.generation;next=removeReservation(next,t);const root=next&&finishDestinationRoot(state,next,o);if(!root)return fail();if(commit){t.reservationReleased=true;t.bindingApplied=true;}return {status:'activated',state:root};
}
export function rebindSourceDocument(owner,state,r,g){
  const o=owners.get(owner)??readLifecycleWriterInspection(owner),fail=()=>({status:'unavailable',state});if(!o||state?.authority!==o.authority||!allowed(g,exportsCatalog.activationGates)||!g.destinationAllowed||!g.rebindAllowed||!exact(r,exportsCatalog.rebindFields)||r.contract!==exportsCatalog.rebindContract||r.operation!=='activate-source-binding'||!identity(r.document)||r.document.authority!==o.authority||!identity(r.provider)||!identity(r.resource)||!natural(r.expectedBinding)||!natural(r.expectedRegistration)||size(r)>o.limits.maxCaptureBytes)return fail();const d=rootDocument(state,r.document,o.limits),row=targetRegistration(state,r,o.limits);if(!d||!row||(d.binding?.generation??0)!==r.expectedBinding||[...o.tasks.values()].some(t=>t.capture.exportState&&!t.reservationReleased&&equal(t.capture.request.document,r.document)))return fail();const found=ownedDestination(state,o,row,r.document);if(!found||d.binding&&!sourceSaveDestinationResolved(owner,d.document,d.binding,g)||!sourceSaveDestinationResolved(owner,d.document,row,g))return fail();
  if(found.row&&(!d.binding||bindingKey(d.binding)!==bindingKey(row)||found.row.bindingGeneration!==d.binding.generation||found.row.exportOrigins?.length))return fail();if(sameBindingContext(d.binding,row))return {status:'unchanged',state};if(r.expectedBinding===Number.MAX_SAFE_INTEGER)return fail();
  const next=copy(state),n=next.documents.find(x=>equal(x.document,r.document));if(d.binding){const oldRows=next.resourceOwners.filter(x=>bindingKey(x)===bindingKey(d.binding));if(oldRows.length!==1||!equal(oldRows[0].document,d.document)||oldRows[0].bindingGeneration!==d.binding.generation||oldRows[0].exportOrigins?.length)return fail();next.resourceOwners=next.resourceOwners.filter(x=>x!==oldRows[0]);}
  n.binding={generation:r.expectedBinding+1,...bindingContext(row)};if(!ordinarySourceBindingReady(state,d,n.binding,g))return fail();n.sourceBaseline=null;if(!retainOldBinding(d,n,o))return fail();next.resourceOwners.push({identityDomain:row.identityDomain,identityKey:row.identityKey,document:copy(d.document),bindingGeneration:n.binding.generation});const root=finishDestinationRoot(state,next,o);return root?{status:'rebound',state:root}:fail();
}
// Fixed first-party owned-invocation staging on its actual current root. This is
// not a public root replacement or writer API; only one task-bound authority works.
export function prepareNativeSourceSaveAsActivation(authority,domain,before,profile,probe,g){
 const p=readSourceSaveAsActivationAuthority(authority,domain);if(!p||before?.authority!==p.writer.authority||!sourceDestinationGates('activate').every(k=>g?.[k]===true))return null;
 const h={...g,ordinaryBindingLimits:profile?.bindings},inspection=sealLifecycleWriterInspection(p.owner,p.writer),baseline=nativeSourceSaveAsBaseline(p.writer,p.task,domain);if(!baseline)return null;const result=sourceSaveAsCandidate(inspection,p.task,before,probe,h,false,p.writer,baseline);if(!['activated','unchanged'].includes(result.status))return null;
 const plan={status:result.status,capture:copy(p.record.capture),receiptCapture:copy(p.record.receiptCapture),baseline:copy(result.state.documents.find(d=>equal(d.document,p.record.capture.request.document)).sourceBaseline)};
 return sealSourceDestinationTransition(p.owner,p.writer,domain,before,result.state,profile,'activate',plan,p.task);
}
// Closed native factory. Preparation is not a writer ticket or an external effect.
export function prepareNativeSourceDestination(owner,domain,profile,kind,input,g,operationContext=null){
 const protectedKind=['baseline','release','release-history'].includes(kind),writer=owners.get(owner);
 const required=sourceDestinationGates(kind);if(!writer||!required)return null;
 // Validate caller facts before selecting/funding a first writer attachment,
 // but do not let those fact getters install the fence during this admission.
 const preflightSection=beginInvocationAdmissionMutation(domain,protectedKind);if(!preflightSection)return null;
 try{if(!required.every(k=>g?.[k]===true))return null;}finally{endInvocationAdmissionMutation(domain,preflightSection);}
 if(protectedKind?!writer.nativeWriterDomains.has(domain):!bindInvocationSourceWriter(owner,domain,{ownerControlAllowed:true,resourcesReady:true,preparationReserved:true,writerCoverageComplete:true}))return null;
 const section=beginInvocationAdmissionMutation(domain,['baseline','release','release-history'].includes(kind));if(!section)return null;try{
 if(!['baseline','release','release-history'].includes(kind)&&!invocationOrdinaryAdmissionOpen(domain))return null;
 const o=owners.get(owner),before=invocationAuthorityRoot(domain);if(!o||invocationUsage(domain)?.faulted!==false||!sourceDestinationGates(kind)?.every(k=>g?.[k]===true)||before?.authority!==o.authority)return null;const h={...g,ordinaryBindingLimits:profile?.bindings};let result,plan,task=null,artifact=null,readWitness=null;
 if(kind==='annotation-admit'){if(!exact(input,['request','context','binding','profile']))return null;const candidate=annotationWriteCandidate(owner,before,input.request,domain,input.context,h);if(!candidate)return null;const prepared=prepareAnnotationDestinationArtifact(o.codec,domain,input.request.document,input.context,input.binding,input.profile,h);if(prepared.status!=='prepared')return null;artifact=prepared.preparation;candidate.capture.payloadByteLength=prepared.byteLength;candidate.receiptCapture.payloadByteLength=prepared.byteLength;result={state:candidate.root};plan={status:'admitted',capture:candidate.capture,receiptCapture:candidate.receiptCapture,annotationBasis:copy(prepared.basis)};}
 else if(kind==='ordinary-admit'){const candidate=ordinarySourceAdmission(o,owner,before,input,h);if(!candidate)return null;const c=candidate.capture;readWitness=c.baseline.kind==='loaded-capture'?nativeSourceReadBaselineWitness(o.readOwner,domain,o,c.request.document,c.binding,c.baseline):null;if(c.baseline.kind==='loaded-capture'&&!readWitness||c.baseline.kind==='acknowledged-write'&&![...o.tasks.values()].some(t=>t.nativeSourceDomain===domain&&equal(t.capture.request.origin,c.baseline.origin)))return null;result={state:before};plan={status:'admitted',capture:c,receiptCapture:candidate.receiptCapture};const prepared=prepareSourceDestinationArtifact(o.codec,before,{contract:'serialized-source/1',operation:'capture-source',document:input.document,expectedSource:input.expectedSource,expectedSerialization:input.expectedSerialization},codecGates);if(prepared.status!=='prepared')return null;artifact=prepared.preparation;}
 else if(kind==='baseline'){if(!exact(input,['document']))return null;const d=rootDocument(before,input.document,o.limits);if(!d||!sourceBaselineRetentionShape(d.sourceBaseline)||!ordinarySourceBindingReady(before,d,d.binding,h))return null;if(d.sourceBaseline?.kind==='acknowledged-write'&&![...o.tasks.values()].some(t=>t.nativeSourceDomain===domain&&equal(t.capture.request.origin,d.sourceBaseline.origin)&&baselineAnchor(o,d.document,d.binding,d.sourceBaseline)))return null;result=publishSourceSaveBaseline(owner,before,input.document,h);if(!['published','unchanged'].includes(result.status))return null;const b=result.state.documents.find(d=>equal(d.document,input.document)).sourceBaseline,entry=[...o.tasks.entries()].find(([,t])=>!t.annotation&&t.nativeSourceDomain===domain&&equal(t.capture.request.origin,b.origin));if(!entry)return null;task=entry[0];plan={status:result.status,request:copy(input),capture:copy(entry[1].capture),baseline:copy(b)};}
 else if(kind==='admit'){result=sourceExportCandidate(owner,before,input,h,true);if(result.status!=='prepared')return null;plan=result.plan;const prepared=prepareSourceDestinationArtifact(o.codec,before,{contract:'serialized-source/1',operation:'capture-source',document:input.document,expectedSource:input.expectedSource,expectedSerialization:input.expectedSerialization},codecGates);if(prepared.status!=='prepared')return null;artifact=prepared.preparation;}
 else if(kind==='release-history'){if(!exact(input,['document','expectedBinding']))return null;result=releaseSourceBindingHistory(owner,before,input.document,input.expectedBinding,h);if(!['released','unchanged'].includes(result.status))return null;plan={status:result.status,request:copy(input)};}
 else {if(!exact(input,kind==='release'?['task']:['task','probe']))return null;task=input.task;const t=o.tasks.get(task);if(!t||t.nativeSourceDomain!==domain||t.annotation&&(kind!=='release'||t.annotation.domain!==domain))return null;result=kind==='release'?sourceExportRelease(owner,task,before,h,false):sourceSaveAsCandidate(owner,task,before,input.probe,h,false);if(!['released','activated','unchanged'].includes(result.status))return null;plan={status:result.status,capture:copy(t.capture),receiptCapture:copy(t.receiptCapture),...(kind==='activate'?{baseline:copy(result.state.documents.find(d=>equal(d.document,t.capture.request.document)).sourceBaseline)}:{})};}
 if(operationContext!==null){
  const parent=kind==='ordinary-admit'?prepareSaveOperation(o,domain,'ordinary-source-save',{request:input},operationContext,g):null;
  if(!parent){if(artifact)discardSourceDestinationArtifact(artifact);return null;}plan.parentOperation=parent;
 }
 const token=sealSourceDestinationTransition(owner,o,domain,before,result.state,profile,kind,plan,task,artifact,readWitness);if(!token&&artifact)discardSourceDestinationArtifact(artifact);return token;
 }finally{endInvocationAdmissionMutation(domain,section);}
}
export function releaseSourceBindingHistory(owner,state,document,expectedBinding,g){
  const o=owners.get(owner),fail=()=>({status:'unavailable',state});if(!o||state?.authority!==o.authority||!allowed(g,exportsCatalog.activationGates)||!allowed(g,['bindingHistoryReleaseAllowed','historyEvidenceReleased','snapshotEvidenceReleased','readEvidenceReleased','writeEvidenceReleased','providerEvidenceReleased']))return fail();const d=rootDocument(state,document,o.limits);if(!d||!natural(expectedBinding)||(d.binding?.generation??0)!==expectedBinding||!sourceHistoryRetentionShape(d.bindingHistory,o.limits))return fail();if(!d.bindingHistory.length)return {status:'unchanged',state};const next=copy(state);next.documents.find(x=>equal(x.document,document)).bindingHistory=[];const root=finishDestinationRoot(state,next,o);return root?{status:'released',state:root}:fail();
}
