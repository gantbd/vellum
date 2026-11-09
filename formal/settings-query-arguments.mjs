// Existing F07 argument meaning. Native owner retains context/target authority.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {settingsQueryInputForms} from './settings-query-declarations.mjs';
const forms=settingsQueryInputForms(),operations=['effective-configuration','permitted-origin','capabilities'],common=['contract','operation','context','target','profile'];
if(forms.length!==3||!equal(forms.map(f=>f.operation),operations)||forms.some(f=>f.family!=='F07'||f.contract!=='configuration-query/1'||f.variant!==f.operation||!equal(f.fields,[...common,...(f.operation==='capabilities'?[]:['key'])])))throw Error('unsupported native settings query declaration');
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v),contextFields=['authority','engine','recipient','purpose','destination','policy','continuity','profile'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const d=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
export const settingsQueryArgumentClassificationBytes=Buffer.byteLength(JSON.stringify(Object.fromEntries(limitFields.map(k=>[k,Number.MAX_SAFE_INTEGER]))));
export function settingsQueryArgumentInputForms(){return forms;}
export function captureSettingsQueryArgumentLimits(input){const p=fields(input);return exact(p,limitFields)&&Object.values(p).every(natural)&&p.maxInputNodes>=1?Object.freeze(p):null;}
export function normalizeSettingsQueryArguments(envelope,authorityContext,inputLimits,maximum){
 const limits=fields(inputLimits);if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1)return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F07')return null;
 const args=fields(raw),form=forms.find(f=>f.operation===args?.operation);if(!form||!exact(args,form.fields)||args.contract!==form.contract||decoded.variant!==args.operation||!object(args.context)||!object(args.target))return null;
 const context=args.context,target=args.target,shadow=Object.fromEntries(form.fields.map(k=>[k,k==='context'||k==='target'?null:args[k]])),whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(authorityContext,cap);
 if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const r=whole.value.request.arguments;if(!text(r.profile)||r.operation!=='capabilities'&&!text(r.key))return null;
 const captured=copySettingsData({request:r,context:bound.value,limits,opaqueBindings:['settings-read-context','settings-read-target']},cap);if(!captured)return null;
 return Object.freeze({kind:'settings-query-arguments',record:captured.value,context,target,route:e.route});
}
export function sameSettingsQueryArguments(a,b){return Boolean(a&&b&&a.kind==='settings-query-arguments'&&b.kind==='settings-query-arguments'&&a.context===b.context&&a.target===b.target&&equal(a.record,b.record));}
export function settingsQueryProducerEnvelope(n){if(!n)return null;const r=n.record.request;return {route:n.route,request:{...(n.route==='typed'?{family:6}:{name:'inspect-context'}),variant:r.operation,version:1,arguments:{...r,context:n.context,target:n.target}}};}
