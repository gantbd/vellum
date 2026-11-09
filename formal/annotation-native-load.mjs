// Storage boundary composition. No raw reader owner or replacement root escapes.
import {isDeepStrictEqual as equal} from 'node:util';
import {newAnnotationResourceReadRegistry,admitAnnotationResourceRead,submitAnnotationResourceRead,observeAnnotationResourceRead,inspectAnnotationResourceCandidate,controlAnnotationResourceRead,retireAnnotationResourceRead,annotationResourceReadUsage,annotationResourceReadStatus} from './annotation-resource-read.mjs';
import {beginInvocationAdmissionMutation,endInvocationAdmissionMutation,invocationAuthorityRoot,invocationAuthorityContext,invocationUsage,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment} from './invocation-publication.mjs';
import {restoreInvocationAnnotations} from './annotation-restoration.mjs';
import {annotationResourceReadRecords,annotationResourceReadLedger,annotationResourceReadRequestValid} from './annotation-resource-read.mjs';
import {ordinarySourceBindingReady} from './binding-pair-profile.mjs';
import {registerAnnotationLoadProvenance,annotationLoadedBaselineGates,prepareLoadedAnnotationBaseline,annotationLoadedBaselineEvidence,annotationLoadWasAdopted,sealLoadedAnnotationBaseline,discardLoadedAnnotationBaseline,discardAnnotationLoadWriterGuard,annotationReadDischargeReady,annotationReadDisposedDischargeReady,annotationReadProvenanceWriters,discardAnnotationReadCheckpoint,forgetAnnotationReadAdoption} from './annotation-loaded-baseline-state.mjs';
import {prepareAnnotationLoadedReadGuard,prepareAnnotationReadRetirementCheckpoint} from './source-save.mjs';
import {installInvocationLoadedAnnotationBaseline} from './invocation-publication.mjs';
import {sealAnnotationLoadActivationAuthority} from './annotation-load-activation-state.mjs';
const owners=new WeakMap(),domains=new WeakMap(),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const allowed=(g,keys)=>keys.every(k=>g?.[k]===true),unavailable=()=>({status:'unavailable'}),healthy=o=>o&&invocationUsage(o.domain)?.faulted===false;
const releaseGates=['authorized','scope','continuous','live','notStopped','annotationReadable','retained','replyReady'];
export function newInvocationAnnotationReadRegistry(domain,limits,snapshotLimits,g){
 const natural=n=>Number.isSafeInteger(n)&&n>=0;
 if(!natural(limits?.maxIssued)||!natural(limits?.maxCaptureBytes)||!natural(limits?.maxHeldBytes))return null;
 const replayBytes=limits.maxIssued*limits.maxCaptureBytes,lifetimeBytes=2*replayBytes,units=limits.maxHeldBytes+lifetimeBytes;if(!natural(replayBytes)||!natural(lifetimeBytes)||!natural(units))return null;
 const attachment=beginInvocationOwnerAttachment(domain,'annotation-read',units,Object.freeze({kind:'native-annotation-read-owner/1'}));if(!attachment)return null;
 try{
 if(invocationUsage(domain)?.faulted!==false||domains.has(domain)||!allowed(g,['ownerControlAllowed','resourcesReady','preparationReserved']))return null;
 const c=invocationAuthorityContext(domain),reader=newAnnotationResourceReadRegistry(c.authority,limits,snapshotLimits,limits?.maxCandidateBytes);if(!reader)return null;
 const ledger=annotationResourceReadLedger(reader);ledger.parentOperations=new Map();ledger.nativeParentContext=copy(c);
 const owner=Object.freeze({kind:'native-annotation-read-owner/1'}),state={domain,reader,parentAttachment:attachment,limits:copy(limits),snapshotLimits:copy(snapshotLimits),restorations:new Map(),consumed:new WeakSet()};if(!registerAnnotationLoadProvenance(owner,domain,c,annotationResourceReadRecords(reader),state.restorations,limits.maxCandidateBytes,annotationResourceReadLedger(reader)))return null;owners.set(owner,state);domains.set(domain,owner);if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native annotation reader attachment invariant');return owner;
 }finally{abortInvocationOwnerAttachment(domain,attachment);}
}
export function admitInvocationAnnotationRead(owner,request,g){
 const admissionDomain=owners.get(owner)?.domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{const o=owners.get(owner);return healthy(o)?admitAnnotationResourceRead(o.reader,invocationAuthorityRoot(o.domain),request,g):unavailable();
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
// Original request/task observation is not candidate applicability or restoration.
export function requestInvocationAnnotationRead(owner,request,c,g){
 const o=owners.get(owner),ledger=o&&annotationResourceReadLedger(o.reader);
 if(!ledger||size({request,context:c})>o.limits.maxCaptureBytes)return unavailable();
 // Lookup and child admission use the same bounded private semantic input.
 request=copy(request);c=copy(c);
 if(!ledger||!equal(c,invocationAuthorityContext(o.domain))||
    !allowed(g,[...releaseGates,'readAllowed','transferReady','resourcesReady'])||
    !annotationResourceReadRequestValid(request,ledger.authority,ledger.limits)||
    size({status:'observe-existing',task:{kind:'annotation-resource-read-task/1'}})>o.limits.maxEnvelopeBytes)return unavailable();
 if(ledger.parentOperations.has(request.origin.operation)){
  const t=ledger.parentOperations.get(request.origin.operation),task=t.task;
  if(t.retired||!equal(t.context,c)||!ledger.tasks.has(task))return unavailable();
  return equal(t.request,request)?{status:'observe-existing',task}:unavailable();
 }
 const section=beginInvocationAdmissionMutation(o.domain);if(!section)return unavailable();
 try{return healthy(o)?admitAnnotationResourceRead(o.reader,invocationAuthorityRoot(o.domain),request,g,c):unavailable();}
 finally{endInvocationAdmissionMutation(o.domain,section);}
}
export function submitInvocationAnnotationRead(owner,task,g){const o=owners.get(owner);return healthy(o)?submitAnnotationResourceRead(o.reader,task,invocationAuthorityRoot(o.domain),g):{status:'not-submitted'};}
// Receipts and safe control remain available after native execution faults.
export function observeInvocationAnnotationRead(owner,event,g){const o=owners.get(owner);return o?observeAnnotationResourceRead(o.reader,event,g):unavailable();}
export function inspectInvocationAnnotationCandidate(owner,task,g){const o=owners.get(owner);return healthy(o)?inspectAnnotationResourceCandidate(o.reader,task,invocationAuthorityRoot(o.domain),g):unavailable();}
export function borrowAnnotationLoadActivationAuthority(owner,task,domain,g){const o=owners.get(owner);return healthy(o)&&o.domain===domain&&allowed(g,['ownerControlAllowed','preparationReserved','resourcesReady'])?sealAnnotationLoadActivationAuthority(owner,o,annotationResourceReadLedger(o.reader),task,domain):null;}
export function restoreInvocationAnnotationRead(owner,task,context,binding,profile,g){
 const o=owners.get(owner);if(!healthy(o)||o.consumed.has(task)||!equal(profile?.snapshot,o.snapshotLimits))return unavailable();
 const before=invocationAuthorityRoot(o.domain),candidate=inspectAnnotationResourceCandidate(o.reader,task,before,g);if(candidate.status!=='annotation-resource-candidate')return unavailable();
 // Preallocate and bound provenance before native publication. The reader has
 // already charged a maxCandidateBytes companion reservation for this task.
 const retained={candidate:copy(candidate),context:copy(context),binding:copy(binding),profile:copy(profile),beforePublication:before.publication};
 if(size(retained)>o.limits.maxCandidateBytes)return unavailable();
 const r=restoreInvocationAnnotations(o.domain,candidate.capture.request.document,Uint8Array.from(candidate.bytes),context,binding,profile,g);
 if(!['restored','unchanged'].includes(r.status))return r;
 // These final assignments are one serialized reference transition with native
 // installation, not a claim of production crash/concurrent atomicity.
 o.restorations.set(task,retained);o.consumed.add(task);return r;
}
export function inspectInvocationAnnotationRestoration(owner,task,g){
 const o=owners.get(owner),r=o?.restorations.get(task),s=o&&annotationResourceReadStatus(o.reader,task);if(!r||!s||!allowed(g,releaseGates))return unavailable();
 const out={status:'annotation-read-restoration',publication:r.candidate.snapshot.records.length?'restored':'unchanged',provenance:s.contested||s.refused?'contested':'uncontested',baseline:annotationLoadWasAdopted(owner,task)?'adopted':'not-adopted'};return size(out)<=o.limits.maxReplyBytes?out:unavailable();
}
// Same pure native candidate for direct and owned baseline publication.
export function annotationBaselineCandidate(owner,o,task,before,g){
 const r=o.restorations.get(task),b=prepareLoadedAnnotationBaseline(owner,task);if(!r||!b)return null;
 const ds=before.documents.filter(d=>equal(d.document,r.candidate.capture.request.document)),d=ds[0],binding=r.candidate.capture.basis.inventory.annotations,rows=before.resourceRegistrations.filter(row=>equal(row.provider,b.provider)&&equal(row.resource,b.resource));if(ds.length!==1||d.live!==true||!equal(d.ordinaryBindings?.annotations,binding)||rows.length!==1||!equal(rows[0],r.candidate.capture.registration)||!ordinarySourceBindingReady(before,d,d.binding,g))return null;
 const prior=d.annotationBaseline,current=prior&&annotationLoadedBaselineEvidence(owner,d.document,binding,prior)&&equal(prior,b);if(prior&&!current||!Object.hasOwn(d,'annotationBaseline')&&g.annotationBaselineAbsentAttested!==true)return null;
 let next=before;if(!current){if(g.versionsAvailable!==true||before.publication===Number.MAX_SAFE_INTEGER)return null;next=copy(before);next.documents.find(x=>equal(x.document,d.document)).annotationBaseline=b;next.publication++;if(size(next)>r.profile.native.maxRootBytes)return null;}return {next,document:copy(d.document),binding:copy(binding)};
}
export function adoptInvocationAnnotationReadBaseline(owner,task,writer,g){
 const o=owners.get(owner);if(!healthy(o)||!annotationLoadedBaselineGates.every(k=>g?.[k]===true))return unavailable();const p=prepareNativeAnnotationReadBaseline(owner,o,task,invocationAuthorityRoot(o.domain),writer,g);if(!p)return unavailable();
 try{return installInvocationLoadedAnnotationBaseline(o.domain,p.token,g);}finally{discardLoadedAnnotationBaseline(p.token);discardAnnotationLoadWriterGuard(p.guard);}
}
// Preserve the confined native installation factory; this never installs a root.
export function prepareNativeAnnotationReadBaseline(owner,o,task,before,writer,g){const c=annotationBaselineCandidate(owner,o,task,before,g);if(!c)return null;const guard=prepareAnnotationLoadedReadGuard(writer,owner,c.document,c.binding,g);if(!guard)return null;const token=sealLoadedAnnotationBaseline(owner,task,before,c.next,guard);if(!token){discardAnnotationLoadWriterGuard(guard);return null;}return {token,guard,result:{status:c.next===before?'current':'installed',state:c.next,document:copy(c.document)},document:copy(c.document)};}
export function controlInvocationAnnotationRead(owner,task,operation,g){const o=owners.get(owner);return o?controlAnnotationResourceRead(o.reader,task,operation,g):unavailable();}
export function retireInvocationAnnotationRead(owner,task,g){const o=owners.get(owner);if(!o)return unavailable();const r=o.restorations.get(task);if(!r)return retireAnnotationResourceRead(o.reader,task,g);const root=invocationAuthorityRoot(o.domain);if(annotationReadDisposedDischargeReady(owner,task,root,g)&&invocationAuthorityRoot(o.domain)===root){const result=retireAnnotationResourceRead(o.reader,task,g);if(result.status!=='retired')return result;forgetAnnotationReadAdoption(owner,task);o.restorations.delete(task);return result;}for(const writer of annotationReadProvenanceWriters(owner)){const checkpoint=prepareAnnotationReadRetirementCheckpoint(writer,owner,root,r.candidate.capture.request.document,g);if(!checkpoint)continue;try{if(!annotationReadDischargeReady(owner,task,root,checkpoint,g)||invocationAuthorityRoot(o.domain)!==root)continue;const result=retireAnnotationResourceRead(o.reader,task,g);if(result.status!=='retired')return result;forgetAnnotationReadAdoption(owner,task);o.restorations.delete(task);return result;}finally{discardAnnotationReadCheckpoint(checkpoint);}}return unavailable();}
// Trusted supervision, not physical reclamation or recipient reflection.
export function invocationAnnotationReadUsage(owner){const o=owners.get(owner),u=o&&annotationResourceReadUsage(o.reader);return u?{...u,restorations:o.restorations.size,pinnedProducers:o.restorations.size}:null;}
export function invocationAnnotationReadStatus(owner,task){const o=owners.get(owner),s=o&&annotationResourceReadStatus(o.reader,task);return s?{...s,restoration:o.consumed.has(task)}:null;}
// Fixed actual-parent observation seam; candidate bytes/restoration/baseline
// interpretation are separate products, not read-completion evidence.
export function nativeAnnotationLoadOperationState(owner,task,c){
 const o=owners.get(owner),ledger=o&&annotationResourceReadLedger(o.reader),t=ledger?.tasks.get(task),p=t&&ledger.parentOperations?.get(t.request.origin.operation);
 if(!p||p.retired||p.task!==task||!equal(p.context,c)||!equal(c,invocationAuthorityContext(o.domain)))return null;
 const receipt={kind:'annotation-load',phase:t.phase,stopped:t.stopped,quiescent:t.quiescent,contested:t.contested,decodeRefused:t.refused,receivedChunks:t.sequence,receivedBytes:t.byteLength,witnesses:copy(t.witnesses.filter(w=>['end','failure'].includes(w.kind)))};
 return {domain:o.domain,maxReplyBytes:2*o.limits.maxEnvelopeBytes+512,receipt};
}
