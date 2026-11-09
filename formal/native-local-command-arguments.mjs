// Existing standalone F13/F14 products on the engine's shared typed/dynamic tuple.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
const annotation=JSON.parse(fs.readFileSync(new URL('./annotation-operations.json',import.meta.url))),history=JSON.parse(fs.readFileSync(new URL('./history-reversal.json',import.meta.url)));
const contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'];
const readCatalog=name=>JSON.parse(fs.readFileSync(new URL(name,import.meta.url)));
const annotationLimits=readCatalog('./annotation-editing.json').limits,membershipLimits=readCatalog('./annotation-membership.json').limits,
 viewLimits=readCatalog('./selection-products.json').limits,historyLimits=['maxDependencies','maxParticipants','maxChanges','maxPlanBytes',...readCatalog('./history-views.json').workLimits];
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort()),natural=n=>Number.isSafeInteger(n)&&n>=0,label=s=>typeof s==='string'&&s.length>0;
export function captureNativeLocalCommandLimits(family,binding,limits){
 const owned=copySettingsData(binding,limits);if(!owned)return null;
 const b=owned.value,valid=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&keys.every(k=>Object.hasOwn(v,k)&&natural(v[k]))&&Object.values(v).every(natural);
 return (family==='F13'?valid(b.annotation,annotationLimits)&&(b.membership===null||valid(b.membership,membershipLimits)):
  family==='F14'&&valid(b.history,historyLimits)&&valid(b.views,viewLimits))?owned:null;
}
export function nativeLocalCommandInputForms(){return Object.freeze([
 ...Object.keys(annotation.variants).map(variant=>Object.freeze({family:'F13',variant,contract:annotation.contract,fields:Object.freeze([...Object.keys(annotation.common),...Object.keys(annotation.variants[variant])])})),
 ...history.variants.map(variant=>Object.freeze({family:'F14',variant,contract:history.contract,fields:Object.freeze([...history.common])}))
]);}
export function normalizeNativeLocalCommandArguments(envelope,context,limits,maximum){
 if(!natural(maximum)||!exact(limits,['maxRequestBytes','maxInputNodes','maxInputDepth'])||!Object.values(limits).every(natural))return null;
 const owned=copySettingsData({envelope,context,limits},{...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)});if(!owned)return null;
 const {envelope:e,context:c}=owned.value;if(!exact(c,contextFields)||!Object.values(c).every(label)||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route)||!exact(e.request,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:r,...tuple}=e.request,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple),form=nativeLocalCommandInputForms().find(f=>f.family===decoded?.descriptor.id&&f.variant===decoded.variant);
 if(!form||!exact(r,form.fields)||r.contract!==form.contract||r.operation!==form.variant||!label(r.invocation)||r.profile!==c.profile||!Array.isArray(r.dependencies)||!(form.family==='F13'?r.document?.authority===c.authority:r.authority===c.authority))return null;
 return Object.freeze({route:e.route,family:form.family,kind:form.family==='F13'?'annotation':'reversal',record:Object.freeze({request:r,context:c,limits:owned.value.limits})});
}
