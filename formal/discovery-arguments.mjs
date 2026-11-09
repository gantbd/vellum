// Existing F01 argument capture only. Native selectors remain original handles.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {nativeDiscoveryQueryInputForms} from './native-discovery.mjs';
const operations=['list','inspect','advisory-enablement'],common=['contract','operation','context','profile'],contextFields=['authority','engine','recipient','purpose','destination','policy','continuity','profile'],extras={list:['limit','continuation'],inspect:['entry'],'advisory-enablement':['entry','variant','target']},roleFields={list:['context','continuation'],inspect:['context','entry'],'advisory-enablement':['context','entry','variant','target']},limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const d=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
// Lazy declaration lookup preserves clean initialization of the owner graph.
export function discoveryArgumentInputForms(){
 const forms=nativeDiscoveryQueryInputForms();return forms.length===3&&equal(forms.map(f=>f.operation),operations)&&forms.every(f=>f.family==='F01'&&f.contract==='contract-discovery/1'&&f.variant===f.operation&&equal([...f.fields].sort(),[...common,...extras[f.operation]].sort())&&equal([...f.contextFields].sort(),[...contextFields].sort()))?forms:null;
}
export const discoveryArgumentClassificationBytes=Buffer.byteLength(JSON.stringify(Object.fromEntries(limitFields.map(k=>[k,Number.MAX_SAFE_INTEGER]))));
export function captureDiscoveryArgumentLimits(input){const p=fields(input);return exact(p,limitFields)&&Object.values(p).every(natural)&&p.maxInputNodes>=1?Object.freeze(p):null;}
export function normalizeDiscoveryArguments(envelope,authorityContext,inputLimits,maximum){
 const limits=captureDiscoveryArgumentLimits(inputLimits);if(!natural(maximum)||!limits)return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F01')return null;
 const args=fields(raw),forms=discoveryArgumentInputForms(),form=forms?.find(f=>f.operation===args?.operation);if(!form||!exact(args,form.fields)||args.contract!==form.contract||decoded.variant!==args.operation)return null;
 const roles=roleFields[form.operation];if(!roles.every(k=>k==='continuation'&&args[k]===null||object(args[k])))return null;
 const opaque=Object.freeze(Object.fromEntries(roles.map(k=>[k,args[k]]))),shadow=Object.fromEntries(form.fields.map(k=>[k,roles.includes(k)?null:args[k]])),whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(authorityContext,cap);
 if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const r=whole.value.request.arguments;if(!text(r.profile)||r.operation==='list'&&(!natural(r.limit)||r.limit===0))return null;
 const capture=copySettingsData({request:r,context:bound.value,limits,opaqueBindings:[...roles]},cap);if(!capture)return null;
 return Object.freeze({kind:'discovery-arguments',record:capture.value,opaque,route:e.route});
}
export function sameDiscoveryArguments(a,b){return Boolean(a&&b&&a.kind==='discovery-arguments'&&b.kind==='discovery-arguments'&&equal(a.record,b.record)&&Object.keys(a.opaque).length===Object.keys(b.opaque).length&&Object.keys(a.opaque).every(k=>a.opaque[k]===b.opaque[k]));}
export function discoveryProducerRequest(n){return n?{...n.record.request,...n.opaque}:null;}
