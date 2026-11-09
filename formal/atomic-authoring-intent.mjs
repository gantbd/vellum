// Owned mixed authoring preparation; public parsing and actual allocation/policy truth are separate.
import {isDeepStrictEqual as equal} from 'node:util';
import {prepareAnnotationEdit,annotationInvocationIntent,annotationEditPreparationCurrent} from './annotation-editing.mjs';
import {publishAnnotationPlan} from './annotation-compounds.mjs';
const bundles=new WeakMap(),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const natural=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const dense=a=>Array.isArray(a)&&Object.keys(a).length===a.length&&Object.keys(a).every((k,i)=>k===String(i));
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
function hostFor(g,index,request,invocation,count){
 const entries=g?.atomicAnnotationHosts;if(!dense(entries)||entries.length!==count)return null;const e=entries[index];
 return exact(e,['component','invocation','document','operation','host'])&&e.component===index&&e.invocation===invocation&&equal(e.document,request.document)&&e.operation===request.operation&&e.host&&typeof e.host==='object'?e.host:null;
}
function sourceCandidate(before,sourcePlan,g,c,maximumRoot){
 const result=sourcePlan.sources.length?publishAnnotationPlan(before,sourcePlan,g,c):{status:'unchanged',state:before};
 return ['applied','unchanged'].includes(result.status)&&size(result.state)<=maximumRoot?result.state:null;
}
function nativeRequest(a,invocation,c,state){
 const request={contract:'annotation-authoring/1',invocation,profile:c.profile,...copy(a)};
 if(['create','reattach'].includes(a.operation)){
  const d=state.documents.filter(d=>equal(d.document,a.document));if(d.length!==1)return null;
  request.target={document:copy(a.document),sourceRevision:a.target.basis==='before'?a.target.sourceRevision:d[0].revision,target:copy(a.target.target)};
 }
 return request;
}
function lower(a,value,descriptor){
 const entity=a.operation==='create'?value.allocation:a.annotation,base={document:copy(a.document),id:entity.id,incarnation:entity.incarnation};
 if(a.operation==='update-body')return {...base,kind:'body',expectedBody:a.expectedContent,body:a.body};
 if(a.operation==='delete')return {...base,kind:'delete',expectedBody:a.expectedContent,expectedTarget:a.expectedTarget};
 const target=a.target.basis==='candidate'?{basis:'candidate',descriptor,target:copy(a.target.target)}:copy(a.target);
 return a.operation==='create'?{...base,kind:'create',body:a.body,target}:{...base,kind:'reattach',expectedTarget:a.expectedTarget,target};
}
function distinctFields(annotations){
 const seen=new Map();for(const a of annotations){const key=JSON.stringify([a.document.authority,a.document.id,a.document.incarnation,a.id,a.incarnation]),fields=seen.get(key)??new Set();if(fields.has(a.kind)||fields.size&&['create','delete'].includes(a.kind)||fields.has('create')||fields.has('delete'))return false;fields.add(a.kind);seen.set(key,fields);}return true;
}
function recordCapacity(state,components){
 const count=state.documents.reduce((n,d)=>n+d.records.length,0),creates=components.filter(x=>x.request.operation==='create').length;
 return components.every(x=>state.documents.length<=x.limits.maxDocuments&&count<=x.limits.maxRecords&&(!creates||count+creates<=x.limits.maxRecords)&&size(state)<=x.limits.maxRootBytes);
}
export function prepareAtomicAuthoringIntent(parent,domain,ticket,before,sourcePlan,annotations,c,g,maximumIntent,maximumRoot){
 if(!dense(annotations)||annotations.length<1||!natural(g.maxParticipants)||sourcePlan.sources.length+annotations.length>g.maxParticipants)return null;
 const candidate=sourceCandidate(before,sourcePlan,g,c,maximumRoot);if(!candidate)return null;
 const components=[],lowered=[],values=[];
 for(let index=0;index<annotations.length;index++){
  const a=annotations[index],h=hostFor(g,index,a,sourcePlan.invocation,annotations.length);if(!h||h.recipient!==c.recipient||h.profile!==c.profile)return null;
  if(['create','delete'].includes(a.operation)&&!equal(h.membershipLimits,g.membershipLimits))return null;
  const basis=a.target?.basis==='candidate'?'candidate':'before',request=nativeRequest(a,sourcePlan.invocation,c,basis==='candidate'?candidate:before);if(!request)return null;
  const native=prepareAnnotationEdit(basis==='candidate'?candidate:before,request,h,h.annotationLimits);if(!native)return null;
  const captured=annotationInvocationIntent(native,maximumIntent);if(!captured||captured.invocation!==sourcePlan.invocation)return null;
  const loweredAnnotation=lower(a,captured.value,sourcePlan.descriptor);lowered.push(loweredAnnotation);values.push(captured.value);
  components.push({native,basis,request:copy(a),limits:copy(h.annotationLimits)});
 }
 if(!distinctFields(lowered)||!recordCapacity(before,components))return null;
 const plan={...copy(sourcePlan),annotations:lowered};
 if(size({sourcePlan,annotations,values,plan})>maximumIntent)return null;
 const token=Object.freeze({kind:'owned-atomic-authoring-intent/1'});
 bundles.set(token,{parent,domain,ticket,sourcePlan:freeze(copy(sourcePlan)),annotations:freeze(copy(annotations)),components,c:freeze(copy(c)),maximumRoot,plan:freeze(plan)});
 return {token,plan:freeze(copy(plan))};
}
export function atomicAuthoringIntentCurrent(token,parent,domain,ticket,before,g,c){
 const p=bundles.get(token);if(!p||p.parent!==parent||p.domain!==domain||p.ticket!==ticket||!equal(p.c,c))return false;
 const candidate=sourceCandidate(before,p.sourcePlan,g,c,p.maximumRoot);if(!candidate||!recordCapacity(before,p.components))return false;
 for(let index=0;index<p.components.length;index++){
  const x=p.components[index],h=hostFor(g,index,x.request,p.sourcePlan.invocation,p.components.length);
  if(!h||['create','delete'].includes(x.request.operation)&&!equal(h.membershipLimits,g.membershipLimits)||!annotationEditPreparationCurrent(x.basis==='candidate'?candidate:before,x.native,h))return false;
 }
 return true;
}
export function atomicAuthoringCandidateFits(token,parent,domain,ticket,state){
 const p=bundles.get(token);if(!p||p.parent!==parent||p.domain!==domain||p.ticket!==ticket)return false;
 const count=state.documents.reduce((n,d)=>n+d.records.length,0);return p.components.every(x=>state.documents.length<=x.limits.maxDocuments&&count<=x.limits.maxRecords&&size(state)<=x.limits.maxRootBytes);
}
export function atomicAuthoringMembershipChecks(token,parent,domain,ticket,g){
 const p=bundles.get(token);if(!p||p.parent!==parent||p.domain!==domain||p.ticket!==ticket)return null;
 return {...g,membershipAllocationPlan:copy(p.plan),membershipAllocations:p.plan.annotations.filter(a=>a.kind==='create').map(({document,id,incarnation})=>({document:copy(document),id,incarnation}))};
}
