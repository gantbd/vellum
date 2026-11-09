// Owned native F13 semantic bridge. Issuer truth and a hostile wire decoder remain external.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {publishAnnotationPlan,validCompoundTarget} from './annotation-compounds.mjs';
import {mergeDependencies} from './annotation-operations.mjs';
import {membershipReady} from './annotation-membership.mjs';
import {prepareAnnotationResultAdmission,captureAnnotationResult,releaseAnnotationResult,validAnnotationResultReceipt} from './annotation-results.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-editing.json',import.meta.url))),native=JSON.parse(fs.readFileSync(new URL('./annotation-operations.json',import.meta.url))),owned=new WeakMap(),copy=structuredClone;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort()),size=v=>Buffer.byteLength(JSON.stringify(v));
const natural=n=>Number.isSafeInteger(n)&&n>=0,version=n=>natural(n)&&n>0,scalar=s=>typeof s==='string'&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Buffer.from(s))===s,label=s=>scalar(s)&&s.length>0;
const identity=x=>exact(x,['authority','id','incarnation'])&&label(x.authority)&&label(x.id)&&version(x.incarnation);
const dependency=x=>exact(x,native.dependencyFields)&&label(x.domain)&&label(x.identity)&&version(x.generation);
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const key=d=>JSON.stringify([d.domain,d.identity]),canonical=ds=>[...new Map(ds.map(d=>[key(d),d])).values()].sort((a,b)=>key(a)<key(b)?-1:key(a)>key(b)?1:0);
const membership=r=>['create','delete'].includes(r?.operation);
function allocation(r,h){const a=h.annotationAllocation;return h.allocationCurrent===true&&exact(a,['invocation','document','id','incarnation'])&&a.invocation===r.invocation&&equal(a.document,r.document)&&label(a.id)&&version(a.incarnation);}
function context(h,l){return h&&h.owner&&typeof h.owner==='object'&&label(h.recipient)&&label(h.profile)&&equal(h.annotationLimits,l)&&Array.isArray(h.contexts)&&h.contexts.length<=l.maxContexts&&h.contexts.every(dependency)&&Array.isArray(h.requiredDependencies)&&h.requiredDependencies.length<=l.maxDependencies&&h.requiredDependencies.every(dependency)&&size({contexts:h.contexts,required:h.requiredDependencies,recipient:h.recipient,profile:h.profile})<=l.maxRequestBytes;}
function current(state,r,h,l){
 if(!exact(l,catalog.limits)||!Object.values(l).every(natural)||!context(h,l)||!natural(h.maxParticipants)||h.maxParticipants<1||!catalog.gates.filter(k=>k!=='versionsAvailable').every(k=>h[k]===true)||size(state)>l.maxRootBytes||!natural(state?.publication)||!Array.isArray(state.documents)||state.documents.length>l.maxDocuments||!state.documents.every(d=>Array.isArray(d.records))||state.documents.reduce((n,d)=>n+d.records.length,0)>l.maxRecords)return null;
 if(!r||![...catalog.operations,'create','delete'].includes(r.operation)||!exact(r,[...Object.keys(native.common),...Object.keys(native.variants[r.operation])])||r.contract!==native.contract||!label(r.invocation)||!identity(r.document)||r.document.authority!==state.authority||r.profile!==h.profile||!Array.isArray(r.dependencies)||r.dependencies.length>l.maxDependencies||!r.dependencies.every(dependency)||size(r)>l.maxRequestBytes||!mergeDependencies(h.requiredDependencies,r.dependencies,h.contexts,l.maxDependencies))return null;
 if(membership(r)&&!membershipReady(state,h))return null;
 if(r.operation==='create'){
  const documents=state.documents.filter(d=>equal(d.document,r.document));if(documents.length!==1||documents[0].live===false||!allocation(r,h)||!scalar(r.body)||Buffer.byteLength(r.body)>l.maxBodyBytes||state.documents.reduce((n,d)=>n+d.records.length,0)>=l.maxRecords)return null;
  const d=documents[0],a=h.annotationAllocation;
  return !d.records.some(x=>x.id===a.id)&&!d.annotationLifecycle.entries.some(x=>x.id===a.id&&x.incarnation===a.incarnation)&&h.targetCurrent===true&&exact(r.target,catalog.targetFields)&&equal(r.target.document,r.document)&&r.target.sourceRevision===d.revision&&validCompoundTarget(d.source,r.target.target)?{d}:null;
 }
 if(!exact(r.annotation,['document','id','incarnation'])||!equal(r.annotation.document,r.document)||!label(r.annotation.id)||!version(r.annotation.incarnation))return null;
 const documents=state.documents.filter(d=>equal(d.document,r.document));if(documents.length!==1||documents[0].live===false)return null;const d=documents[0],rows=d.records.filter(x=>x.id===r.annotation.id);
 if(rows.length!==1||rows[0].incarnation!==r.annotation.incarnation||!version(rows[0].bodyVersion)||!version(rows[0].targetGeneration)||!version(d.revision)||!version(d.annotationSet))return null;const row=rows[0];
 if(r.operation==='update-body')return version(r.expectedContent)&&row.bodyVersion===r.expectedContent&&scalar(r.body)&&Buffer.byteLength(r.body)<=l.maxBodyBytes?{d,row}:null;
 if(r.operation==='delete')return version(r.expectedContent)&&version(r.expectedTarget)&&row.bodyVersion===r.expectedContent&&row.targetGeneration===r.expectedTarget?{d,row}:null;
 return version(r.expectedTarget)&&row.targetGeneration===r.expectedTarget&&h.targetCurrent===true&&exact(r.target,catalog.targetFields)&&equal(r.target.document,r.document)&&r.target.sourceRevision===d.revision&&validCompoundTarget(d.source,r.target.target)?{d,row}:null;
}
export function prepareAnnotationEdit(state,request,h,limits){
 if(!current(state,request,h,limits))return null;
 const value={request:copy(request),limits:copy(limits),requiredDependencies:copy(canonical(h.requiredDependencies)),recipient:h.recipient,profile:h.profile,...(membership(request)?{membershipLimits:copy(h.membershipLimits),allocation:request.operation==='create'?copy(h.annotationAllocation):null}:{})};if(size(value)>limits.maxPlanBytes)return null;
 freeze(value);const plan=Object.freeze({kind:'native-annotation-field-plan/1'});owned.set(plan,{value,owner:h.owner});return plan;
}
export function annotationInvocationIntent(plan,maximum){const p=owned.get(plan);return p&&natural(maximum)&&size(p.value)<=maximum?{invocation:p.value.request.invocation,value:copy(p.value)}:null;}
export function admitAnnotationResult(plan,c,g,maximum){const p=owned.get(plan);return p?prepareAnnotationResultAdmission(p.value,c,g,maximum):null;}
export function captureAnnotationEditResult(plan,result){const p=owned.get(plan);if(!p)throw Error('native annotation result owner unavailable');const receipt=captureAnnotationResult(p.value,result);if(!validAnnotationResultReceipt(receipt))throw Error('invalid annotation receipt vocabulary');return receipt;}
export {releaseAnnotationResult};
// Descriptive data from the same private declarations used by current().
// This is not live enablement, policy truth, a wire codec or handler registration.
export function annotationEngineDescriptor(){
 const supported=[...catalog.operations,'create','delete'];
 return freeze({family:native.family,contract:native.contract,version:1,kind:'localCommand',
  variants:Object.keys(native.variants).filter(id=>supported.includes(id)).map(id=>({id,
   input:{contract:native.contract,variant:id,fields:copy({...native.common,...native.variants[id]})},
   outcomes:copy(native.results),
   requirements:{admission:copy(catalog.gates.filter(k=>k!=='versionsAvailable')),versionCapacity:'required-for-changing-publication',currentPredicates:'operation-and-owner-specific'},
   limits:{names:copy(catalog.limits),values:'current-execution-profile'},
   cancellation:'owned-prepublication-stop; accepted-publication-is-not-retracted'})),presentation:{}});
}
export function annotationEditPreparationCurrent(state,plan,h){
 const p=owned.get(plan);if(!p||p.owner!==h?.owner||p.value.recipient!==h.recipient||p.value.profile!==h.profile)return false;const {request:r,limits:l}=p.value;
 if(!current(state,r,h,l)||!equal(p.value.requiredDependencies,canonical(h.requiredDependencies)))return false;
 if(membership(r)&&(!equal(p.value.membershipLimits,h.membershipLimits)||r.operation==='create'&&!equal(p.value.allocation,h.annotationAllocation)))return false;
 return true;
}
export function publishAnnotationEdit(state,plan,h){
 const p=owned.get(plan),fail=()=>({status:'not-published',state});if(!annotationEditPreparationCurrent(state,plan,h))return fail();const {request:r,limits:l}=p.value;
 const entity=r.operation==='create'?p.value.allocation:r.annotation,base={document:copy(r.document),id:entity.id,incarnation:entity.incarnation};
 let annotation;
 if(r.operation==='create')annotation={...base,kind:'create',body:r.body,target:{basis:'before',sourceRevision:r.target.sourceRevision,target:copy(r.target.target)}};
 else if(r.operation==='delete')annotation={...base,kind:'delete',expectedBody:r.expectedContent,expectedTarget:r.expectedTarget};
 else annotation=r.operation==='update-body'?{...base,kind:'body',expectedBody:r.expectedContent,body:r.body}:{...base,kind:'reattach',expectedTarget:r.expectedTarget,target:{basis:'before',sourceRevision:r.target.sourceRevision,target:copy(r.target.target)}};
 const input={contract:'owned-annotation-plan/1',invocation:r.invocation,descriptor:r.invocation,sources:[],annotations:[annotation]};if(size(input)>l.maxPlanBytes)return fail();
 // Only the original owned native request may mint this private whole-plan binding.
 if(r.operation==='create')h={...h,membershipAllocationPlan:copy(input),membershipAllocations:[{document:copy(r.document),id:entity.id,incarnation:entity.incarnation}]};
 const result=publishAnnotationPlan(state,input,h);return ['applied','unchanged'].includes(result.status)&&size(result.state)<=l.maxRootBytes?result:fail();
}
