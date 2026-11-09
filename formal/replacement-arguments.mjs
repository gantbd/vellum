// Isolated existing request normalization; no wire codec or execution authority.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {replacementInputForms} from './literal-search.mjs';
const forms=replacementInputForms(),known=['replace-current','replace-all'];
if(forms.length!==2||Object.keys(forms).length!==2||new Set(forms.map(f=>f.operation)).size!==2||forms.some(f=>f.family!=='F11'||!known.includes(f.operation)||f.variant!==f.operation))throw Error('unsupported replacement declaration source');
const contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const d=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
export function normalizeReplacementArguments(envelope,context,inputLimits,maximum,result){
 const limits=fields(inputLimits);if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1||!object(result))return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F11')return null;
 const args=fields(raw),form=args&&forms.find(f=>f.operation===args.operation);if(!form||!exact(args,form.fields)||decoded.variant!==form.variant)return null;
 const whole=copySettingsData({route:e.route,request:{...tuple,arguments:args}},cap),bound=copySettingsData(context,cap);if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const r=whole.value.request.arguments;if(!text(r.invocation)||typeof r.replacement!=='string'||(r.operation==='replace-all'?r.index!==null:!natural(r.index)))return null;
 const capture=copySettingsData({request:r,context:bound.value,limits,opaqueBindings:['literal-search-result']},cap);if(!capture)return null;
 return Object.freeze({kind:'replacement',record:capture.value,result,route:e.route});
}
export function sameReplacementArguments(a,b){return Boolean(a&&b&&a.kind===b.kind&&a.result===b.result&&equal(a.record,b.record));}
export function replacementProducerRequest(n){return n?{...n.record.request}:null;}
