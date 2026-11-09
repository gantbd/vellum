// Normalized serialized source-load reference. Provider/identity truth is supplied.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {resolveOwner} from './identity-residency.mjs';
import {membershipInventoryActive,membershipInventoryValid} from './annotation-membership.mjs';
import {beginInvocationAdmissionMutation,endInvocationAdmissionMutation,invocationAuthorityRoot,invocationAuthorityContext,invocationUsage,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment} from './invocation-publication.mjs';
import {nativeSourceReadOpenGates,nativeSourceReadLaunchGates,nativeSourceReadActivationGates,nativeSourceReadProfileValid,sealNativeSourceRead,nativeSourceReadRetirementReady} from './source-read-state.mjs';
import {newSourceCodecRegistry,beginSourceImport,appendSourceChunk,finishSourceImport,inspectSourceCandidate,abortSourceImport,retireSourceImport,sourceCodecUsage,sourceCandidateActivationWitness} from './source-codec.mjs';
import {readLifecycleWriterInspectionBinding} from './lifecycle-writer-inspection.mjs';
import {sealSourceLoadActivationAuthority,readSourceLoadActivationAuthority,sourceLoadActivationDecoded,revokeSourceLoadActivationAuthority} from './source-load-authority.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./resource-read.json',import.meta.url),'utf8'));
const codecCatalog=JSON.parse(fs.readFileSync(new URL('./source-codec.json',import.meta.url),'utf8'));
const owners=new WeakMap(),copy=structuredClone;
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const natural=n=>Number.isSafeInteger(n)&&n>=0,size=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const scalar=s=>typeof s==='string'&&Buffer.from(s,'utf8').toString('utf8')===s;
const identity=r=>exact(r,['authority','id','incarnation'])&&scalar(r.authority)&&r.authority.length>0&&scalar(r.id)&&r.id.length>0&&natural(r.incarnation);
const origin=r=>exact(r,catalog.originFields)&&Object.values(r).every(s=>scalar(s)&&s.length>0);
const version=v=>exact(v,['epoch','token'])&&scalar(v.epoch)&&v.epoch.length>0&&scalar(v.token);
const chunkShape=(e,l)=>natural(e.sequence)&&e.sequence<l.maxChunks&&natural(e.offset)&&e.offset<=l.maxInputBytes&&Array.isArray(e.bytes)&&e.bytes.length<=l.maxChunkBytes&&Object.keys(e.bytes).length===e.bytes.length&&Object.keys(e.bytes).every((key,i)=>key===String(i))&&e.bytes.every(b=>Number.isInteger(b)&&b>=0&&b<=255);
const lookupGates=['authorized','scope','continuous','live','notStopped','identityAllowed','bindingCoverageComplete'];
const launchGates=[...lookupGates,'readAllowed','providerAvailable','resourcesReady','launchAllowed'];
const activationGates=[...lookupGates,'activationAllowed','companionsReady','versionsAvailable','replyReady'];
const releaseGates=['authorized','scope','continuous','live','notStopped','retained','replyReady'];
const allowed=(g,ks)=>ks.every(k=>g?.[k]);
const unavailable=()=>({status:'unavailable'}),originKey=r=>JSON.stringify(catalog.originFields.map(k=>r[k]));
const held=t=>!t.stopped&&t.phase==='ready'||t.phase==='submitted'&&!t.quiescent;
function registration(state,r,l){
  if(!Array.isArray(state?.resourceRegistrations)||state.resourceRegistrations.length>l.maxRegistryRows)return null;
  const rows=state.resourceRegistrations.filter(x=>equal(x.resource,r.resource)&&equal(x.provider,r.provider));if(rows.length!==1)return null;const row=rows[0];
  return exact(row,catalog.registrationFields)&&identity(row.provider)&&identity(row.resource)&&row.generation===r.expectedRegistration&&natural(row.generation)&&
    [row.identityDomain,row.identityKey,row.locator].every(s=>scalar(s)&&s.length>0&&Buffer.byteLength(s)<=l.maxTagBytes)&&typeof row.identityKnown==='boolean'&&typeof row.atomicRead==='boolean'&&row.profile===r.profile&&
    ['LF','CRLF','CR'].includes(row.insertionLineBreak)&&!(r.requiredRead==='atomic-resource-snapshot'&&!row.atomicRead)?row:null;
}
function requestValid(r,l){return exact(r,catalog.requestFields)&&r.contract===catalog.contract&&r.operation==='source-load-candidate'&&origin(r.origin)&&identity(r.provider)&&identity(r.resource)&&natural(r.expectedRegistration)&&r.profile===codecCatalog.profile&&catalog.readGuarantees.includes(r.requiredRead)&&size(r)<=l.maxCaptureBytes;}
function currentRegistration(state,t,l){if(state?.authority!==t.authority)return null;const row=registration(state,t.request,l);return row&&equal(row,t.registration)?row:null;}
function currentBinding(state,row,l){
  if(!Array.isArray(state?.documents)||!Array.isArray(state?.resourceOwners)||state.documents.length>l.maxRegistryRows||state.resourceOwners.length>l.maxRegistryRows)return {invalid:true};
  const rows=state.resourceOwners.filter(x=>x.identityDomain===row.identityDomain&&x.identityKey===row.identityKey);
  if(rows.length>1)return {invalid:true};if(!rows.length)return {invalid:false,document:null};
  const ds=state.documents.filter(d=>equal(d.document,rows[0].document));const d=ds[0];
  return ds.length===1&&d.live===true&&d.binding&&d.binding.identityDomain===row.identityDomain&&d.binding.identityKey===row.identityKey&&d.binding.generation===rows[0].bindingGeneration?{invalid:false,document:d}:{invalid:true};
}
function ownerDecision(state,row,l,g){
  const found=currentBinding(state,row,l);if(found.invalid)return {decision:'unsupportedEvidence',document:null};
  const decision=resolveOwner({authorized:g.authorized,comparable:row.identityKnown,relation:row.identityKnown?(found.document?'same':'distinct'):'unknown',continuityRequired:true,continuityValid:g.continuous,capacity:state.documents.length<l.maxDocuments&&state.resourceOwners.length<l.maxBindings});
  return {decision,document:found.document};
}
export function lookupLoadedResource(state,r,g,limits){
  if(!allowed(g,lookupGates)||!requestValid(r,limits))return unavailable();const row=registration(state,r,limits);if(!row)return unavailable();const found=ownerDecision(state,row,limits,g);
  if(found.decision==='joinExisting')return g.ownerAccessAllowed&&g.replyReady?{status:'joined',document:copy(found.document.document)}:unavailable();
  return found.decision==='newOwner'?{status:'not-loaded'}:unavailable();
}
export function newResourceReadRegistry(authority,limits,codecLimits){
  if(!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||!scalar(authority)||!authority||Buffer.byteLength(authority)>limits.maxTagBytes)return null;
  const codec=newSourceCodecRegistry(codecLimits),codecReservation=codecLimits?.maxContextBytes+codecLimits?.maxInputBytes+codecLimits?.maxCandidateBytes;
  if(!codec||!natural(codecReservation)||!natural(codecReservation*limits.maxTasks)||!natural(codecLimits.maxCandidateBytes+64)||codecLimits.maxReplyBytes<codecLimits.maxCandidateBytes+64||codecLimits.maxImports<limits.maxTasks||codecLimits.maxIssued<limits.maxIssued||codecLimits.maxHeldBytes<codecReservation*limits.maxTasks||!natural(limits.maxCaptureBytes+limits.maxTranscriptBytes+2*limits.maxEnvelopeBytes+codecReservation))return null;
  const ref=Object.freeze({profile:catalog.profile});owners.set(ref,{authority,limits:copy(limits),codecLimits:copy(codecLimits),codec,codecReservation,tasks:new Map(),used:new Set()});return ref;
}
export function admitResourceRead(owner,state,r,g,parentContext=null){
 const admissionDomain=owners.get(owner)?.nativeDomain??null;const admissionSection=admissionDomain?beginInvocationAdmissionMutation(admissionDomain):null;if(admissionDomain&&!admissionSection)return unavailable();try{
  const nativeOwner=owners.get(owner);if(nativeOwner?.nativeDomain&&(state!==invocationAuthorityRoot(nativeOwner.nativeDomain)||invocationUsage(nativeOwner.nativeDomain)?.faulted!==false||!nativeSourceReadLaunchGates.every(k=>g?.[k]===true)))return unavailable();
  const o=owners.get(owner);if(!o||state?.authority!==o.authority||!allowed(g,launchGates)||!requestValid(r,o.limits))return unavailable();const row=registration(state,r,o.limits);if(!row||!row.identityKnown)return unavailable();
  // The operation identity is admitted here, not inferred from an attempt label.
  // Trusted raw entry cannot add another attempt to an already owned operation.
  if(o.parentOperations?.has(r.origin.operation))return unavailable();
  if(parentContext!==null&&(!o.nativeDomain||!equal(parentContext,invocationAuthorityContext(o.nativeDomain))||
    !['authorized','scope','continuous','live','notStopped','readAllowed','retained','replyReady','transferReady','resourcesReady'].every(k=>g?.[k]===true)||
    [...o.used].some(key=>JSON.parse(key)[0]===r.origin.operation)))return unavailable();
  const contextReservation=size({origin:r.origin,provider:r.provider,resource:r.resource,bindingGeneration:r.expectedRegistration,resourceVersion:null,profile:r.profile})-4+o.limits.maxTagBytes;
  if(contextReservation>o.codecLimits.maxContextBytes)return unavailable();
  const u=resourceReadUsage(owner),used=originKey(r.origin),captured={authority:o.authority,request:copy(r),registration:copy(row)},charge=size(captured)+o.limits.maxTranscriptBytes+2*o.limits.maxEnvelopeBytes+o.codecReservation;
  const minimal={contract:catalog.evidenceContract,task:{kind:'resource-read-task/1'},origin:r.origin,provider:r.provider,resource:r.resource,kind:'failure',reason:'invalid-source'};
  if(size(captured)>o.limits.maxCaptureBytes||size(minimal)>o.limits.maxEnvelopeBytes||o.used.has(used)||u.tasks>=o.limits.maxTasks||u.outstanding>=o.limits.maxOutstanding||u.issued>=o.limits.maxIssued||charge>o.limits.maxHeldBytes-u.heldBytes)return unavailable();
  const task=Object.freeze({kind:'resource-read-task/1'});
  const parent=parentContext===null?null:{kind:'source-load-operation/1',context:copy(parentContext),request:captured.request,task,retired:false,retirement:{retired:true}};
  if(parent&&size([r.origin.operation,parent])>o.limits.maxCaptureBytes)return unavailable();
  o.used.add(used);o.tasks.set(task,{...captured,charge,phase:'ready',stopped:false,quiescent:false,context:null,stream:null,candidate:null,rows:new Map(),transcriptBytes:0,sequence:0,byteLength:0,witnesses:[],contested:false,codecRefused:false,activation:null});
  if(parent)o.parentOperations.set(r.origin.operation,parent);
  return {status:'admitted',task};

 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function submitResourceRead(owner,task,state,g){
  const nativeOwner=owners.get(owner);if(nativeOwner?.nativeDomain&&(state!==invocationAuthorityRoot(nativeOwner.nativeDomain)||invocationUsage(nativeOwner.nativeDomain)?.faulted!==false||!nativeSourceReadLaunchGates.every(k=>g?.[k]===true)))return {status:'not-submitted'};
  const o=owners.get(owner),t=o?.tasks.get(task);if(!t||t.phase!=='ready'||t.stopped||!allowed(g,launchGates)||!currentRegistration(state,t,o.limits))return {status:'not-submitted'};
  t.phase='submitted';return {status:'submitted',task,capture:copy({request:t.request,registration:t.registration})};
}
function invalidate(o,t){if(t.stream)abortSourceImport(o.codec,t.stream,{controlAllowed:true});t.candidate=null;}
function conflict(o,t,witness){
  if(t.witnesses.some(w=>equal(w,witness)))return {status:'duplicate'};
  if(t.witnesses.length>=2)return {status:'evidence-full'};
  t.witnesses.push(copy(witness));t.contested=true;invalidate(o,t);return {status:'conflict'};
}
const codecGates={authorized:true,sourceReadable:true,scope:true,continuous:true,live:true,notStopped:true,resourcesReady:true,contextCurrent:true,coherentInput:true};
const codecRelease={authorized:true,scope:true,continuous:true,live:true,notStopped:true,retained:true,replyReady:true};
function coherent(t,value){return value==='captured-read'?t.request.requiredRead==='captured-read':value==='atomic-resource-snapshot'&&t.registration.atomicRead;}
export function observeResourceRead(owner,event,g){
  if(owners.get(owner)?.nativeDomain&&(!['authentic','evidenceAllowed','ingressReady'].every(k=>g?.[k]===true)||['start','end'].includes(event?.kind)&&g.readAttested!==true||event?.kind==='quiescence'&&g.quiescenceAttested!==true))return unavailable();
  const o=owners.get(owner);if(!o||!g?.authentic||!g.evidenceAllowed||!g.ingressReady)return unavailable();const t=o.tasks.get(event?.task);
  if(!t||t.phase!=='submitted'||!Object.hasOwn(catalog.events,event.kind)||!exact(event,[...catalog.eventCommon,...catalog.events[event.kind]])||event.contract!==catalog.evidenceContract||!equal(event.origin,t.request.origin)||!equal(event.provider,t.request.provider)||!equal(event.resource,t.request.resource)||size(event)>o.limits.maxEnvelopeBytes)return unavailable();
  if(event.kind==='quiescence'){if(!g.quiescenceAttested)return unavailable();if(t.quiescent)return {status:'duplicate'};t.quiescent=true;return {status:'quiescent'};}
  if(['start','end'].includes(event.kind)&&(!g.readAttested||!version(event.resourceVersion)||size(event.resourceVersion)>o.limits.maxTagBytes||!coherent(t,event.coherence)))return unavailable();
  if(event.kind==='failure'&&!catalog.failures.includes(event.reason))return unavailable();
  if(event.kind==='chunk'&&!chunkShape(event,o.codecLimits))return unavailable();
  if(event.kind==='end'&&(!natural(event.chunks)||event.chunks>o.codecLimits.maxChunks||!natural(event.byteLength)||event.byteLength>o.codecLimits.maxInputBytes))return unavailable();
  const witness={kind:event.kind,...Object.fromEntries(catalog.events[event.kind].map(k=>[k,event[k]]))};
  if(event.kind==='failure'){
    if(t.witnesses.some(w=>equal(w,witness)))return {status:'duplicate'};
    if(t.witnesses.length)return conflict(o,t,witness);
    t.witnesses.push(copy(witness));invalidate(o,t);return {status:'recorded'};
  }
  const rowKey=event.kind==='chunk'?`chunk:${event.sequence}`:event.kind,prior=t.rows.get(rowKey);
  if(prior)return equal(prior,witness)?{status:'duplicate'}:conflict(o,t,witness);
  if(t.contested||t.codecRefused)return unavailable();
  if(event.kind==='chunk'&&t.stopped&&!t.witnesses.some(w=>w.kind==='end'))return {status:'discarded'};
  if(t.witnesses.length)return conflict(o,t,witness);
  if(event.kind==='chunk'&&(!t.context||!natural(event.sequence)||!natural(event.offset)||event.sequence!==t.sequence||event.offset!==t.byteLength))return unavailable();
  if(event.kind==='end'&&(!t.context||!natural(event.chunks)||!natural(event.byteLength)||!equal(event.resourceVersion,t.context.resourceVersion)||event.coherence!==t.rows.get('start')?.coherence||!t.stopped&&(event.chunks!==t.sequence||event.byteLength!==t.byteLength)))return unavailable();
  const charge=size(witness);if(charge>o.limits.maxTranscriptBytes-t.transcriptBytes){t.codecRefused=true;invalidate(o,t);return {status:'evidence-full'};}
  if(event.kind==='start'){
    t.context={origin:copy(t.request.origin),provider:copy(t.request.provider),resource:copy(t.request.resource),bindingGeneration:t.request.expectedRegistration,resourceVersion:copy(event.resourceVersion),profile:t.request.profile};
    if(!t.stopped){const r=beginSourceImport(o.codec,t.context,codecGates);if(r.status!=='open'){t.codecRefused=true;return {status:'candidate-refused',reason:'codec-refusal'};}t.stream=r.import;}
  }else if(event.kind==='chunk'){
    const r=appendSourceChunk(o.codec,t.stream,t.context,{sequence:event.sequence,offset:event.offset,bytes:event.bytes},codecGates);
    if(r.status!=='open'){t.codecRefused=true;invalidate(o,t);return {status:'candidate-refused',reason:'codec-refusal'};}t.sequence++;t.byteLength+=event.bytes.length;
  }else{
    if(!t.stopped){const r=finishSourceImport(o.codec,t.stream,t.context,codecGates);if(r.status!=='candidate'){t.codecRefused=true;return {status:'candidate-refused',reason:'codec-refusal'};}t.candidate=r.candidate;}
    t.witnesses.push(copy(witness));
  }
  t.rows.set(rowKey,copy(witness));t.transcriptBytes+=charge;return {status:event.kind==='end'&&!t.stopped?'candidate':'recorded'};
}
export function controlResourceRead(owner,task,operation,g){
  if(owners.get(owner)?.nativeDomain&&g?.controlAllowed!==true)return unavailable();
  const o=owners.get(owner),t=o?.tasks.get(task);if(!t||!g?.controlAllowed||!['cancel','supersede'].includes(operation))return unavailable();if(t.stopped)return {status:'unchanged'};t.stopped=true;invalidate(o,t);return {status:'stopped'};
}
export function inspectResourceCandidate(owner,task,g){
  if(owners.get(owner)?.nativeDomain&&!releaseGates.every(k=>g?.[k]===true))return unavailable();
  const o=owners.get(owner);if(!o||!allowed(g,releaseGates))return unavailable();const t=o.tasks.get(task);if(!t||t.stopped||t.contested||t.codecRefused||!t.candidate)return unavailable();return inspectSourceCandidate(o.codec,t.candidate,t.context,codecRelease);
}
export function activateResourceCandidate(owner,task,state,g){return resourceActivationCandidate(owner,task,state,g,true);}
function resourceActivationCandidate(owner,task,state,g,commit,actualReader=null,actualDecoded=null){
  const o=owners.get(owner)??actualReader,t=o?.tasks.get(task),fail=()=>({status:'unavailable',state});
  if(commit&&o?.nativeDomain)return fail();if(!t||!allowed(g,activationGates)||t.stopped||t.contested||t.codecRefused||!currentRegistration(state,t,o.limits)||!t.candidate||!membershipInventoryValid(state,g))return fail();
  const choice=ownerDecision(state,t.registration,o.limits,g);if(choice.decision==='unsupportedEvidence')return fail();
  if(t.activation){const d=state.documents.find(d=>equal(d.document,t.activation.document));return d?.live&&d.binding?.generation===t.activation.bindingGeneration&&g.ownerAccessAllowed&&choice.document&&equal(choice.document.document,d.document)?{status:'already-activated',state,document:copy(d.document)}:fail();}
  if(choice.decision==='joinExisting'){
    if(!g.ownerAccessAllowed)return fail();if(commit)t.activation={kind:'joined',document:copy(choice.document.document),bindingGeneration:choice.document.binding.generation};return {status:'joined',state,document:copy(choice.document.document)};
  }
  const a=state.identityAllocation;
  if(choice.decision!=='newOwner'||!g.creationAllowed||!natural(state.publication)||state.publication===Number.MAX_SAFE_INTEGER||!exact(a,['namespace','next','last'])||!scalar(a.namespace)||!a.namespace||!natural(a.next)||!natural(a.last)||a.next>a.last||a.next===Number.MAX_SAFE_INTEGER||!Array.isArray(state.retiredDocuments)||state.retiredDocuments.length>o.limits.maxRegistryRows||size(state)>o.limits.maxRootBytes)return fail();
  const doc={authority:state.authority,id:`${a.namespace}:${a.next}`,incarnation:1};if(!identity(doc)||state.documents.some(d=>equal(d.document,doc))||state.retiredDocuments.some(d=>equal(d,doc)))return fail();
  const decoded=actualDecoded??inspectSourceCandidate(o.codec,t.candidate,t.context,codecRelease);if(decoded.status!=='source-candidate')return fail();
  const raw=[...t.rows.entries()].filter(([key])=>key.startsWith('chunk:')).sort(([,a],[,b])=>a.sequence-b.sequence).flatMap(([,row])=>row.bytes);
  if(raw.length!==decoded.rawByteLength)return fail();
  const row=t.registration,binding={generation:1,provider:copy(row.provider),resource:copy(row.resource),registrationGeneration:row.generation,identityDomain:row.identityDomain,identityKey:row.identityKey,locator:row.locator,readOrigin:copy(t.request.origin)};
  const d={document:doc,live:true,revision:1,source:copy(decoded.source),serialization:{generation:1,profile:t.request.profile,bom:decoded.bom},insertionLineBreak:row.insertionLineBreak,annotationSet:1,records:[],...(membershipInventoryActive(state,g)?{annotationLifecycle:{entries:[]}}:{}),binding,
    sourceBaseline:{kind:'loaded-capture',bindingGeneration:1,sourceRevision:1,serializationGeneration:1,profile:t.request.profile,origin:copy(t.request.origin),provider:copy(row.provider),resourceVersion:copy(t.context.resourceVersion),coherence:t.rows.get('start').coherence,bom:decoded.bom,bytes:raw}};
  const ownerRow={identityDomain:row.identityDomain,identityKey:row.identityKey,document:copy(doc),bindingGeneration:1};
  if(size({document:d,binding:ownerRow})>o.limits.maxActivatedBytes)return fail();
  const next=copy(state);next.documents.push(d);next.resourceOwners.push(ownerRow);next.identityAllocation.next++;next.publication++;
  if(size(next)>o.limits.maxRootBytes||!membershipInventoryValid(next,g))return fail();
  if(commit)t.activation={kind:'created',document:copy(doc),bindingGeneration:1};return {status:'created',state:next,document:copy(doc)};
}
export function newInvocationSourceReadRegistry(domain,limits,codecLimits,profile,g){
 // Spent origin keys outlive their task payloads. A key is smaller than its
 // bounded request, so reserve the whole lifetime bound separately in the parent.
 if(!natural(limits?.maxIssued)||!natural(limits?.maxCaptureBytes)||!natural(limits?.maxHeldBytes))return null;
 const replayBytes=limits?.maxIssued*limits?.maxCaptureBytes,lifetimeBytes=2*replayBytes,units=limits?.maxHeldBytes+lifetimeBytes;
 if(!natural(replayBytes)||!natural(lifetimeBytes)||!natural(units))return null;
 const attachment=beginInvocationOwnerAttachment(domain,'source-read',units,Object.freeze({profile:catalog.profile}));if(!attachment)return null;
 try{const root=invocationAuthorityRoot(domain);if(invocationUsage(domain)?.faulted!==false||!nativeSourceReadOpenGates.every(k=>g?.[k]===true)||!nativeSourceReadProfileValid(root,profile))return null;const ref=newResourceReadRegistry(root.authority,limits,codecLimits),o=ref&&owners.get(ref);if(!o)return null;o.ref=ref;o.nativeDomain=domain;o.nativeProfile=freeze(copy(profile));o.parentOperations=new Map();o.parentAttachment=attachment;if(!commitInvocationOwnerAttachment(domain,attachment,ref))throw Error('native source reader attachment invariant');return ref;}
 finally{abortInvocationOwnerAttachment(domain,attachment);}
}
// First-party ownership witness, not a recipient result or authority grant.
export function nativeSourceReadDomain(owner){return owners.get(owner)?.nativeDomain??null;}
// Fixed first-party observation seam. No transcript/chunk bytes or candidate
// applicability escapes, and a raw task is not an owned command.
export function nativeSourceLoadOperationState(owner,task,c){
 const o=owners.get(owner),t=o?.tasks.get(task),p=t&&o.parentOperations?.get(t.request.origin.operation);
 if(!o?.nativeDomain||!p||p.retired||p.task!==task||!equal(p.context,c)||!equal(c,invocationAuthorityContext(o.nativeDomain)))return null;
 const receipt={kind:'source-load',phase:t.phase,stopped:t.stopped,quiescent:t.quiescent,contested:t.contested,decodeRefused:t.codecRefused,receivedChunks:t.sequence,receivedBytes:t.byteLength,witnesses:copy(t.witnesses.filter(w=>['end','failure'].includes(w.kind)))};
 return {domain:o.nativeDomain,maxReplyBytes:2*o.limits.maxEnvelopeBytes+512,receipt};
}
export function admitInvocationSourceRead(owner,request,g){const o=owners.get(owner);return o?.nativeDomain?admitResourceRead(owner,invocationAuthorityRoot(o.nativeDomain),request,g):unavailable();}
// Native admission/retry only: no provider dispatch or candidate release.
export function requestInvocationSourceRead(owner,request,c,g){
 const o=owners.get(owner);
 if(!o?.nativeDomain||size({request,context:c})>o.limits.maxCaptureBytes)return unavailable();
 // One private semantic input before any current-fact supplier can reenter.
 request=copy(request);c=copy(c);
 if(!o?.nativeDomain||!equal(c,invocationAuthorityContext(o.nativeDomain))||
    !['authorized','scope','continuous','live','notStopped','readAllowed','retained','replyReady','transferReady','resourcesReady'].every(k=>g?.[k]===true)||
    !requestValid(request,o.limits)||size({status:'observe-existing',task:{kind:'resource-read-task/1'}})>o.limits.maxEnvelopeBytes)return unavailable();
 if(o.parentOperations.has(request.origin.operation)){
  const t=o.parentOperations.get(request.origin.operation),task=t.task;
  if(t.retired||!equal(t.context,c)||!o.tasks.has(task))return unavailable();
  return equal(t.request,request)?{status:'observe-existing',task}:unavailable();
 }
 // Existing native admission owns duplicate/fence serialization and current root.
 return admitResourceRead(owner,invocationAuthorityRoot(o.nativeDomain),request,g,c);
}
export function submitInvocationSourceRead(owner,task,g){const o=owners.get(owner);return o?.nativeDomain?submitResourceRead(owner,task,invocationAuthorityRoot(o.nativeDomain),g):{status:'not-submitted'};}
export function prepareNativeSourceReadActivation(owner,task,g){const o=owners.get(owner),t=o?.tasks.get(task);if(!o?.nativeDomain||!t||invocationUsage(o.nativeDomain)?.faulted!==false||!nativeSourceReadActivationGates.every(k=>g?.[k]===true))return null;const before=invocationAuthorityRoot(o.nativeDomain),h={...g,membershipLimits:o.nativeProfile.membership},result=resourceActivationCandidate(owner,task,before,h,false);if(!['created','joined','already-activated'].includes(result.status)||(result.status==='created'?g.creationAllowed!==true:g.ownerAccessAllowed!==true))return null;if(result.status==='created'){const d=result.state.documents.find(d=>equal(d.document,result.document));d.ordinaryBindings={generation:1,annotations:null};d.annotationBaseline=null;d.lifecycle={generation:1,phase:'live',retainedBinding:null};}const decoded=inspectSourceCandidate(o.codec,t.candidate,t.context,codecRelease);return decoded.status==='source-candidate'?sealNativeSourceRead(owner,o,task,o.nativeDomain,before,result,decoded):null;}
export function borrowSourceLoadActivationAuthority(owner,task,inspection,domain,g){const o=owners.get(owner),t=o?.tasks.get(task);if(!o||o.nativeDomain!==domain||!t||invocationUsage(domain)?.faulted!==false||!nativeSourceReadOpenGates.every(k=>g?.[k]===true))return null;const pair=readLifecycleWriterInspectionBinding(inspection),witness=sourceCandidateActivationWitness(o.codec,t.candidate,t.context);return sealSourceLoadActivationAuthority(owner,o,task,domain,pair,witness);}
// Fixed owned activation on its actual current root; no public root/decoder API.
export function prepareNativeSourceLoadActivation(authority,domain,before,profile,g){return prepareNativeSourceLoadActivationFromRecord(readSourceLoadActivationAuthority(authority,domain),domain,before,profile,g);}
export function prepareNativeSourceLoadActivationFromRecord(p,domain,before,profile,g){if(!p||p.domain!==domain||before?.authority!==p.reader.authority||!equal(profile,p.reader.nativeProfile)||!nativeSourceReadActivationGates.every(k=>g?.[k]===true))return null;const decoded=sourceLoadActivationDecoded(p),result=resourceActivationCandidate(null,p.task,before,{...g,membershipLimits:profile.membership},false,p.reader,decoded);if(!['created','joined','already-activated'].includes(result.status)||(result.status==='created'?g.creationAllowed!==true:g.ownerAccessAllowed!==true))return null;if(result.status==='created'){const d=result.state.documents.find(d=>equal(d.document,result.document));d.ordinaryBindings={generation:1,annotations:null};d.annotationBaseline=null;d.lifecycle={generation:1,phase:'live',retainedBinding:null};}return sealNativeSourceRead(p.readerOwner,p.reader,p.task,domain,before,result,decoded);}
export function retireInvocationSourceRead(owner,task,g){const o=owners.get(owner);return o?.nativeDomain?retireResourceRead(owner,task,invocationAuthorityRoot(o.nativeDomain),g):unavailable();}
export function loadedBaselineUncontested(owner,state,document,g){
  const o=owners.get(owner);if(!o||!allowed(g,releaseGates))return false;const d=state.documents.find(d=>equal(d.document,document));if(!d?.live||!d.binding||!d.sourceBaseline)return false;
  return state.authority===o.authority&&resourceReadBaselineEvidence(owner,document,d.binding,d.sourceBaseline,g);
}
// Validate retained historical load provenance without replacing a current root baseline.
// Trusted composition witness: actual native producer records, never public data.
export function nativeSourceReadBaselineWitness(owner,domain,writer,document,binding,baseline){const o=owners.get(owner);if(!o||o.nativeDomain!==domain||o.nativeWriter!==writer||!resourceReadBaselineEvidence(owner,document,binding,baseline,Object.fromEntries(releaseGates.map(k=>[k,true]))))return null;const entry=[...o.tasks.entries()].find(([,t])=>equal(t.request.origin,binding.readOrigin));if(!entry)return null;const [taskRef,task]=entry;return {reader:o,taskRef,task,activation:copy(task.activation),context:copy(task.context),rows:copy([...task.rows]),witnesses:copy(task.witnesses)};}
export function resourceReadBaselineEvidence(owner,document,binding,baseline,g){
  if(owners.get(owner)?.nativeDomain&&!releaseGates.every(k=>g?.[k]===true))return false;
  const o=owners.get(owner);if(!o||!allowed(g,releaseGates)||!binding||!baseline||document?.authority!==o.authority)return false;const d={binding,sourceBaseline:baseline};
  const t=[...o.tasks.values()].find(t=>equal(t.request.origin,d.binding.readOrigin));if(!t)return false;
  const raw=[...t.rows.entries()].filter(([key])=>key.startsWith('chunk:')).sort(([,a],[,b])=>a.sequence-b.sequence).flatMap(([,row])=>row.bytes);
  return Boolean(t.activation?.kind==='created'&&equal(t.activation.document,document)&&!t.contested&&!t.codecRefused&&t.witnesses.length===1&&t.witnesses[0].kind==='end'&&d.binding.generation===t.activation.bindingGeneration&&d.sourceBaseline.kind==='loaded-capture'&&d.sourceBaseline.bindingGeneration===d.binding.generation&&d.sourceBaseline.sourceRevision===1&&d.sourceBaseline.serializationGeneration===1&&d.sourceBaseline.profile===t.request.profile&&equal(d.sourceBaseline.origin,t.request.origin)&&equal(d.binding.provider,t.request.provider)&&equal(d.binding.resource,t.request.resource)&&d.binding.registrationGeneration===t.registration.generation&&d.binding.identityDomain===t.registration.identityDomain&&d.binding.identityKey===t.registration.identityKey&&equal(d.sourceBaseline.provider,t.request.provider)&&equal(d.sourceBaseline.resourceVersion,t.context?.resourceVersion)&&d.sourceBaseline.coherence===t.rows.get('start')?.coherence&&d.sourceBaseline.bom===(raw[0]===239&&raw[1]===187&&raw[2]===191)&&equal(d.sourceBaseline.bytes,raw));
}
export function retireResourceRead(owner,task,state,g){
  const nativeOwner=owners.get(owner),nativeTask=nativeOwner?.tasks.get(task);if(nativeOwner?.nativeDomain&&(state!==invocationAuthorityRoot(nativeOwner.nativeDomain)||!nativeTask||!nativeSourceReadRetirementReady(nativeOwner,nativeTask,state,g)))return unavailable();
  const o=owners.get(owner),t=o?.tasks.get(task);if(!t||state?.authority!==o.authority||!g?.retirementAllowed||!g.retentionReleased||!g.deliveryDrained||held(t))return unavailable();
  if(t.activation?.kind==='created'&&state.documents.some(d=>(d.live||d.lifecycle?.phase==='disposed-retained')&&equal(d.document,t.activation.document)&&[d.binding??d.lifecycle?.retainedBinding,...(d.bindingHistory??[]).map(h=>h.binding)].some(b=>equal(b?.readOrigin,t.request.origin))))return unavailable();
  if(t.stream&&retireSourceImport(o.codec,t.stream,{retentionReleased:true,deliveryDrained:true}).status!=='retired')return unavailable();revokeSourceLoadActivationAuthority(task);
  const parent=o.parentOperations?.get(t.request.origin.operation);if(parent?.task===task)o.parentOperations.set(t.request.origin.operation,parent.retirement);
  o.tasks.delete(task);return {status:'retired'};
}
// Trusted supervision, not public F06 reflection or physical accounting.
export function resourceReadCodecUsage(owner){const o=owners.get(owner);return o?sourceCodecUsage(o.codec):null;}
export function resourceReadUsage(owner){const o=owners.get(owner);return o?{tasks:o.tasks.size,outstanding:[...o.tasks.values()].filter(held).length,issued:o.used.size,heldBytes:[...o.tasks.values()].reduce((sum,t)=>sum+t.charge,0)}:null;}
export function resourceReadStatus(owner,task){const t=owners.get(owner)?.tasks.get(task);return t?{phase:t.phase,stopped:t.stopped,quiescent:t.quiescent,contested:t.contested,codecRefused:t.codecRefused,candidate:!!t.candidate,activation:t.activation?.kind??null,witnesses:t.witnesses.length,sequence:t.sequence,byteLength:t.byteLength}:null;}
