// Explicit source-only proposal data; no private companion versions or callbacks.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
const fields=['contract','operation','invocation','profile','sources','reads','scopes'];
const contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=s=>typeof s==='string'&&s.length>0;
const exact=(v,ks)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
export function exactBatchInputForm(){return Object.freeze({family:'F11',contract:'exact-source-batch/1',operation:'exact-batch',variant:'exact-batch',profileConstant:'bounded-explicit-source-batch/1',fields:Object.freeze([...fields])});}
export function normalizeExactBatchArguments(envelope,context,limits,maximum){
  if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1)return null;
  const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},owned=copySettingsData({envelope,context,limits},cap);if(!owned)return null;
  const {envelope:e,context:c}=owned.value;
  if(!exact(c,contextFields)||!Object.values(c).every(text)||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route)||!exact(e.request,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
  const {arguments:r,...tuple}=e.request,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple),form=exactBatchInputForm();
  if(decoded?.descriptor.id!=='F11'||decoded.variant!==form.variant||!exact(r,fields)||r.contract!==form.contract||r.operation!==form.operation||r.profile!==form.profileConstant||!text(r.invocation)||![r.sources,r.reads,r.scopes].every(Array.isArray))return null;
  for(const s of r.sources){
    if(!exact(s,['document','expectedSource','edits'])||!exact(s.document,['authority','id','incarnation'])||s.document.authority!==c.authority||!text(s.document.id)||!natural(s.document.incarnation)||!natural(s.expectedSource)||!Array.isArray(s.edits))return null;
    for(const e of s.edits)if(!exact(e,['lo','hi','expected','ins'])||!natural(e.lo)||!natural(e.hi)||e.lo>e.hi||![e.expected,e.ins].every(a=>Array.isArray(a)&&a.every(b=>Number.isInteger(b)&&b>=0&&b<=255)))return null;
  }
  return Object.freeze({route:e.route,record:Object.freeze({request:r,context:c,limits:owned.value.limits})});
}
export function sameExactBatchArguments(a,b){return Boolean(a&&b&&equal(a.record,b.record));}
