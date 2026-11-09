// Existing F04 argument meaning. Native lookup retains handle/alias authority.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {historyQueryInputForms} from './history-queries.mjs';
const forms=historyQueryInputForms(),operations=['frontier','availability','permitted-blocking-detail'],common=['contract','operation','context','scope','direction','profile'];
if(forms.length!==3||!equal(forms.map(f=>f.operation).toSorted(),operations.toSorted())||forms.some(f=>f.family!=='F04'||f.contract!=='history-query/1'||f.variant!==f.operation||!equal(f.directions,['undo','redo'])||!equal([...f.fields].sort(),[...common,...(f.operation==='frontier'?[]:['group'])].sort())))throw Error('unsupported native history query declaration');
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v),contextFields=['authority','engine','recipient','purpose','destination','profile','policy','continuity'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const d=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
export const historyQueryArgumentClassificationBytes=Buffer.byteLength(JSON.stringify(Object.fromEntries(limitFields.map(k=>[k,Number.MAX_SAFE_INTEGER]))));
export function historyQueryArgumentInputForms(){return Object.freeze([...forms]);}
export function captureHistoryQueryArgumentLimits(input){const p=fields(input);return exact(p,limitFields)&&Object.values(p).every(natural)&&p.maxInputNodes>=1?Object.freeze(p):null;}
export function normalizeHistoryQueryArguments(envelope,authorityContext,inputLimits,maximum){
 const limits=fields(inputLimits);if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1)return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F04')return null;
 const args=fields(raw),form=forms.find(f=>f.operation===args?.operation);if(!form||!exact(args,form.fields)||args.contract!==form.contract||decoded.variant!==args.operation||!object(args.context))return null;
 const opaque=args.context,shadow=Object.fromEntries(form.fields.map(k=>[k,k==='context'?null:args[k]])),whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(authorityContext,cap);if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const r=whole.value.request.arguments,c=bound.value;if(!text(r.scope)||!text(r.profile)||!form.directions.includes(r.direction)||(r.operation!=='frontier'&&r.group!==null&&!text(r.group)))return null;
 const capture=copySettingsData({request:r,context:c,limits,opaqueBindings:['history-context']},cap);if(!capture)return null;
 return Object.freeze({kind:'history-query-arguments',record:capture.value,opaque,route:e.route});
}
export function sameHistoryQueryArguments(a,b){return Boolean(a&&b&&a.kind==='history-query-arguments'&&b.kind==='history-query-arguments'&&a.opaque===b.opaque&&equal(a.record,b.record));}
export function historyQueryProducerRequest(n){return n?{...n.record.request,context:n.opaque}:null;}
