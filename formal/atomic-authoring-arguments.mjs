// Explicit mixed data profile. Component identity and authorization evidence are not caller data.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {normalizeExactBatchArguments} from './exact-batch-arguments.mjs';
const native=JSON.parse(fs.readFileSync(new URL('./annotation-operations.json',import.meta.url)));
const fields=['contract','operation','invocation','profile','sources','annotations','reads','scopes'];
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const natural=n=>Number.isSafeInteger(n)&&n>=0,version=n=>natural(n)&&n>0,label=s=>typeof s==='string'&&s.length>0;
const document=d=>exact(d,['authority','id','incarnation'])&&label(d.authority)&&label(d.id)&&version(d.incarnation);
function targetShape(t){
 if(t?.kind==='document')return exact(t,['kind']);
 if(t?.kind==='point')return exact(t,['kind','offset','affinity','profile'])&&natural(t.offset)&&['before','after'].includes(t.affinity)&&['position','content'].includes(t.profile);
 return t?.kind==='range'&&exact(t,['kind','start','end'])&&natural(t.start)&&natural(t.end)&&t.start<t.end;
}
export function atomicAuthoringInputForm(){return Object.freeze({family:'F11',variant:'exact-batch',contract:'atomic-source-annotation/1',operation:'exact-batch',profileConstant:'bounded-atomic-authoring/1',fields:Object.freeze([...fields])});}
export function normalizeAtomicAuthoringArguments(envelope,context,limits,maximum){
 if(!natural(maximum)||!exact(limits,['maxRequestBytes','maxInputNodes','maxInputDepth'])||!Object.values(limits).every(natural))return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},owned=copySettingsData({envelope,context,limits},cap);if(!owned)return null;
 const {envelope:e,context:c}=owned.value,r=e?.request?.arguments,form=atomicAuthoringInputForm();
 if(!exact(e,['route','request'])||!exact(r,fields)||r.contract!==form.contract||r.operation!==form.operation||r.profile!==form.profileConstant||!Array.isArray(r.annotations)||r.annotations.length<1)return null;
 // Reuse the existing F11 tuple, document/source/edit and context guards.
 const source=normalizeExactBatchArguments({...e,request:{...e.request,arguments:{contract:'exact-source-batch/1',operation:'exact-batch',invocation:r.invocation,profile:'bounded-explicit-source-batch/1',sources:r.sources,reads:r.reads,scopes:r.scopes}}},c,owned.value.limits,maximum);if(!source)return null;
 for(const a of r.annotations){
  if(!Object.hasOwn(native.variants,a?.operation))return null;const variant=native.variants[a.operation];if(!exact(a,['operation','document','dependencies',...Object.keys(variant)])||!document(a.document)||a.document.authority!==c.authority||!Array.isArray(a.dependencies))return null;
  if(!a.dependencies.every(d=>exact(d,native.dependencyFields)&&label(d.domain)&&label(d.identity)&&version(d.generation)))return null;
  if(a.operation!=='create'&&(!exact(a.annotation,['document','id','incarnation'])||!equal(a.annotation.document,a.document)||!label(a.annotation.id)||!version(a.annotation.incarnation)))return null;
  if(Object.hasOwn(a,'expectedContent')&&!version(a.expectedContent)||Object.hasOwn(a,'expectedTarget')&&!version(a.expectedTarget)||Object.hasOwn(a,'body')&&typeof a.body!=='string')return null;
  if(['create','reattach'].includes(a.operation)){
   const t=a.target;if(!t||!['before','candidate'].includes(t.basis)||!exact(t,t.basis==='before'?['basis','sourceRevision','target']:['basis','target'])||t.basis==='before'&&!version(t.sourceRevision)||!targetShape(t.target))return null;
  }
 }
 return Object.freeze({route:e.route,record:Object.freeze({request:r,context:c,limits:owned.value.limits})});
}
