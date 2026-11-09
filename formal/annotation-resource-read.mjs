// Storage boundary reference. No source decoder, native publication or baseline adoption.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {ordinarySourceBindingReady,pairIdentity} from './binding-pair-profile.mjs';
import {decodeSnapshot} from './annotation-snapshot.mjs';
import {revokeAnnotationLoadActivationAuthority} from './annotation-load-activation-state.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-resource-read.json',import.meta.url))),transport=JSON.parse(fs.readFileSync(new URL('./resource-read.json',import.meta.url))),owners=new WeakMap(),copy=structuredClone;
const size=v=>Buffer.byteLength(JSON.stringify(v)),natural=n=>Number.isSafeInteger(n)&&n>=0,exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const label=s=>typeof s==='string'&&s.length>0&&Buffer.from(s).toString('utf8')===s,allowed=(g,ks)=>ks.every(k=>g?.[k]===true),unavailable=()=>({status:'unavailable'});
const origin=r=>exact(r,transport.originFields)&&Object.values(r).every(label),originKey=r=>JSON.stringify(transport.originFields.map(k=>r[k]));
const version=(v,l)=>exact(v,['epoch','token'])&&label(v.epoch)&&typeof v.token==='string'&&Buffer.from(v.token).toString('utf8')===v.token&&size(v)<=l.maxTagBytes;
const held=t=>t.phase==='ready'&&!t.stopped||t.phase==='submitted'&&!t.quiescent;
const basis=d=>({document:d.document,sourceRevision:d.revision,annotationSet:d.annotationSet,sourceBinding:d.binding,inventory:d.ordinaryBindings});
export function annotationResourceReadRequestValid(r,authority,limits){
 return exact(r,catalog.requestFields)&&r.contract===catalog.contract&&r.operation==='annotation-load-candidate'&&r.profile==='vellum.annotations/1'&&origin(r.origin)&&pairIdentity(r.document)&&r.document.authority===authority&&pairIdentity(r.provider)&&pairIdentity(r.resource)&&['expectedSource','expectedAnnotationSet','expectedPair','expectedBinding','expectedRegistration'].every(k=>natural(r[k]))&&transport.readGuarantees.includes(r.requiredRead)&&size(r)<=limits.maxCaptureBytes;
}
function current(o,state,r,g){
 if(state?.authority!==o.authority||!annotationResourceReadRequestValid(r,o.authority,o.limits)||!Array.isArray(state.documents)||!Array.isArray(state.resourceRegistrations)||state.resourceRegistrations.length>o.limits.maxRegistryRows)return null;
 const ds=state.documents.filter(d=>equal(d.document,r.document)),d=ds[0],a=d?.ordinaryBindings?.annotations;if(ds.length!==1||d.live!==true||!a||d.revision!==r.expectedSource||d.annotationSet!==r.expectedAnnotationSet||d.ordinaryBindings.generation!==r.expectedPair||a.generation!==r.expectedBinding||!equal(a.provider,r.provider)||!equal(a.resource,r.resource)||!ordinarySourceBindingReady(state,d,d.binding,g))return null;
 const rows=state.resourceRegistrations.filter(x=>equal(x.provider,r.provider)&&equal(x.resource,r.resource)),row=rows[0];
 if(rows.length!==1||!exact(row,catalog.registrationFields)||!pairIdentity(row.provider)||!pairIdentity(row.resource)||!natural(row.generation)||row.generation!==r.expectedRegistration||row.identityKnown!==true||typeof row.atomicRead!=='boolean'||row.profile!==r.profile||!['identityDomain','identityKey','locator'].every(k=>label(row[k])&&Buffer.byteLength(row[k])<=o.limits.maxTagBytes&&row[k]===a[k])||r.requiredRead==='atomic-resource-snapshot'&&!row.atomicRead)return null;
 return {basis:copy(basis(d)),registration:copy(row)};
}
export function newAnnotationResourceReadRegistry(authority,limits,snapshotLimits,companionBytes=0){
 if(!label(authority)||!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||Buffer.byteLength(authority)>limits.maxTagBytes||decodeSnapshot(new Uint8Array(),snapshotLimits).status==='invalidProfile')return null;
 // Trusted composition may reserve one additional bounded participant per task.
 const reservation=limits.maxTranscriptBytes+2*limits.maxEnvelopeBytes+limits.maxCandidateBytes+companionBytes;if(!natural(companionBytes)||!natural(reservation)||!natural(reservation+limits.maxCaptureBytes))return null;
 const owner=Object.freeze({profile:catalog.profile});owners.set(owner,{authority,limits:copy(limits),snapshotLimits:copy(snapshotLimits),reservation,tasks:new Map(),used:new Set(),relations:new Map()});return owner;
}
export function admitAnnotationResourceRead(owner,state,r,g,parentContext=null){
 const o=owners.get(owner);if(!o||!allowed(g,catalog.launchGates))return unavailable();const c=current(o,state,r,g);if(!c)return unavailable();const captured={request:copy(r),...c},charge=size(captured)+o.reservation,u=annotationResourceReadUsage(owner),key=originKey(r.origin);
 if(o.parentOperations?.has(r.origin.operation))return unavailable();
 if(parentContext!==null&&(!o.parentOperations||!equal(parentContext,o.nativeParentContext)||
   !['authorized','scope','continuous','live','notStopped','readAllowed','annotationReadable','retained','replyReady','transferReady','resourcesReady'].every(k=>g?.[k]===true)||
   [...o.used].some(key=>JSON.parse(key)[0]===r.origin.operation)))return unavailable();
 const minimal={contract:catalog.evidenceContract,task:{kind:'annotation-resource-read-task/1'},origin:r.origin,provider:r.provider,resource:r.resource,kind:'failure',reason:'invalid-source'};
 if(size(captured)>o.limits.maxCaptureBytes||size(minimal)>o.limits.maxEnvelopeBytes||o.used.has(key)||u.tasks>=o.limits.maxTasks||u.outstanding>=o.limits.maxOutstanding||u.issued>=o.limits.maxIssued||charge>o.limits.maxHeldBytes-u.heldBytes)return unavailable();
 const task=Object.freeze({kind:'annotation-resource-read-task/1'});
 const parent=parentContext===null?null:{kind:'annotation-load-operation/1',context:copy(parentContext),request:captured.request,task,retired:false,retirement:{retired:true}};
 if(parent&&size([r.origin.operation,parent])>o.limits.maxCaptureBytes)return unavailable();
 o.used.add(key);o.tasks.set(task,{...captured,charge,phase:'ready',stopped:false,quiescent:false,contested:false,refused:false,rows:new Map(),witnesses:[],transcriptBytes:0,sequence:0,byteLength:0,candidate:null});
 if(parent)o.parentOperations.set(r.origin.operation,parent);
 return {status:'admitted',task};
}
const stillCurrent=(o,t,state,g)=>{const c=current(o,state,t.request,g);return c&&equal(c.basis,t.basis)&&equal(c.registration,t.registration);};
export function submitAnnotationResourceRead(owner,task,state,g){const o=owners.get(owner),t=o?.tasks.get(task);if(!t||t.phase!=='ready'||t.stopped||!allowed(g,catalog.launchGates)||!stillCurrent(o,t,state,g))return {status:'not-submitted'};t.phase='submitted';return {status:'submitted',capture:copy({request:t.request,basis:t.basis,registration:t.registration})};}
function conflict(t,w){if(t.witnesses.some(x=>equal(x,w)))return {status:'duplicate'};if(t.witnesses.length>=2)return {status:'evidence-full'};t.witnesses.push(copy(w));t.contested=true;t.candidate=null;return {status:'conflict'};}
function octets(xs,l){return Array.isArray(xs)&&xs.length<=l.maxChunkBytes&&Object.keys(xs).length===xs.length&&Object.keys(xs).every((k,i)=>k===String(i))&&xs.every(n=>Number.isInteger(n)&&n>=0&&n<=255);}
export function observeAnnotationResourceRead(owner,e,g){
 const o=owners.get(owner);if(!o||!allowed(g,['authentic','evidenceAllowed','ingressReady']))return unavailable();const t=o.tasks.get(e?.task),l=o.limits;
 if(!t||t.phase!=='submitted'||!Object.hasOwn(transport.events,e.kind)||!exact(e,[...transport.eventCommon,...transport.events[e.kind]])||e.contract!==catalog.evidenceContract||!equal(e.origin,t.request.origin)||!equal(e.provider,t.request.provider)||!equal(e.resource,t.request.resource)||size(e)>l.maxEnvelopeBytes)return unavailable();
 if(e.kind==='quiescence'){if(g.quiescenceAttested!==true)return unavailable();if(t.quiescent)return {status:'duplicate'};t.quiescent=true;return {status:'quiescent'};}
 if(['start','end'].includes(e.kind)&&(!allowed(g,['readAttested'])||!version(e.resourceVersion,l)||!(e.coherence==='captured-read'?t.request.requiredRead==='captured-read':e.coherence==='atomic-resource-snapshot'&&t.registration.atomicRead)))return unavailable();
 if(e.kind==='chunk'&&(!natural(e.sequence)||e.sequence>=l.maxChunks||!natural(e.offset)||e.offset>l.maxInputBytes||!octets(e.bytes,l)))return unavailable();
 if(e.kind==='end'&&(!natural(e.chunks)||e.chunks>l.maxChunks||!natural(e.byteLength)||e.byteLength>l.maxInputBytes))return unavailable();
 if(e.kind==='failure'&&!transport.failures.includes(e.reason))return unavailable();
 const w={kind:e.kind,...Object.fromEntries(transport.events[e.kind].map(k=>[k,e[k]]))};
 if(e.kind==='failure'){if(t.witnesses.some(x=>equal(x,w)))return {status:'duplicate'};if(t.witnesses.length)return conflict(t,w);t.witnesses.push(copy(w));t.candidate=null;return {status:'recorded'};}
 const key=e.kind==='chunk'?`chunk:${e.sequence}`:e.kind,prior=t.rows.get(key);if(prior)return equal(prior,w)?{status:'duplicate'}:conflict(t,w);
 if(t.contested||t.refused)return unavailable();if(e.kind==='chunk'&&t.stopped&&!t.witnesses.some(x=>x.kind==='end'))return {status:'discarded'};if(t.witnesses.length)return conflict(t,w);
 const start=t.rows.get('start');if(e.kind==='chunk'&&(!start||e.sequence!==t.sequence||e.offset!==t.byteLength))return unavailable();
 if(e.kind==='end'&&(!start||!equal(e.resourceVersion,start.resourceVersion)||e.coherence!==start.coherence||!t.stopped&&(e.chunks!==t.sequence||e.byteLength!==t.byteLength)))return unavailable();
 const charge=size(w);if(charge>l.maxTranscriptBytes-t.transcriptBytes){t.refused=true;t.candidate=null;return {status:'evidence-full'};}
 if(e.kind==='chunk'){if(e.bytes.length>l.maxInputBytes-t.byteLength){t.refused=true;t.candidate=null;return {status:'candidate-refused'};}t.sequence++;t.byteLength+=e.bytes.length;}
 t.rows.set(key,copy(w));t.transcriptBytes+=charge;
 if(e.kind==='end'){
  t.witnesses.push(copy(w));if(t.stopped)return {status:'recorded'};
  const bytes=[...t.rows.values()].filter(x=>x.kind==='chunk').flatMap(x=>x.bytes),decoded=decodeSnapshot(Uint8Array.from(bytes),o.snapshotLimits);
  if(decoded.status!=='decoded'){t.refused=true;return {status:'candidate-refused'};}
  const candidate={capture:copy({request:t.request,basis:t.basis,registration:t.registration}),resourceVersion:copy(e.resourceVersion),coherence:e.coherence,bytes,snapshot:copy(decoded.value)};
  if(size(candidate)>l.maxCandidateBytes){t.refused=true;return {status:'candidate-refused'};}t.candidate=candidate;return {status:'candidate'};
 }
 return {status:'recorded'};
}
export function inspectAnnotationResourceCandidate(owner,task,state,g){return inspectAnnotationResourceCandidateFromLedger(owners.get(owner),task,state,g);}
// Fixed first-party actual-ledger seam, not a supplied reader or root API.
export function inspectAnnotationResourceCandidateFromLedger(o,task,state,g){const t=o?.tasks.get(task);if(!t||!allowed(g,catalog.releaseGates)||t.stopped||t.contested||t.refused||!t.candidate||!stillCurrent(o,t,state,g))return unavailable();const out={status:'annotation-resource-candidate',...copy(t.candidate)};return size(out)<=o.limits.maxReplyBytes?out:unavailable();}
export function controlAnnotationResourceRead(owner,task,operation,g){const t=owners.get(owner)?.tasks.get(task);if(!t||g?.controlAllowed!==true||!['cancel','supersede'].includes(operation))return unavailable();if(t.stopped)return {status:'unchanged'};t.stopped=true;t.candidate=null;return {status:'stopped'};}
export function retireAnnotationResourceRead(owner,task,g){const o=owners.get(owner),t=o?.tasks.get(task);if(!t||!allowed(g,['retirementAllowed','retentionReleased','deliveryDrained'])||held(t))return unavailable();revokeAnnotationLoadActivationAuthority(task);
 const parent=o.parentOperations?.get(t.request.origin.operation);if(parent?.task===task)o.parentOperations.set(t.request.origin.operation,parent.retirement);
 o.tasks.delete(task);return {status:'retired'};}
export function annotationResourceReadUsage(owner){const o=owners.get(owner);return o?{tasks:o.tasks.size,outstanding:[...o.tasks.values()].filter(held).length,issued:o.used.size,heldBytes:o.relations.size*o.limits.maxCaptureBytes+[...o.tasks.values()].reduce((n,t)=>n+t.charge,0)}:null;}
export function annotationResourceReadStatus(owner,task){const t=owners.get(owner)?.tasks.get(task);return t?{phase:t.phase,stopped:t.stopped,quiescent:t.quiescent,contested:t.contested,refused:t.refused,candidate:!!t.candidate,sequence:t.sequence,byteLength:t.byteLength,witnesses:t.witnesses.length}:null;}
// Confined factory seam for format-neutral provenance, not a recipient API.
export function annotationResourceReadRecords(owner){return owners.get(owner)?.tasks??null;}
export function annotationResourceReadLedger(owner){return owners.get(owner)??null;}
