// Remaining selected F17 activation/rebind/container ordinary-data capture.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {lifecycleActivationInputForms,lifecycleActivationContextFields} from './lifecycle-activation-declarations.mjs';
const forms=lifecycleActivationInputForms(),limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0;
function fields(v){if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return null;const d=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k)),matches=(v,m)=>Object.entries(m).every(([k,value])=>v[k]===value);
export {lifecycleActivationInputForms};
export function normalizeLifecycleActivationArguments(envelope,context,inputLimits,maximum){
 const limits=fields(inputLimits);if(!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1||!natural(maximum))return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F17')return null;
 const a=fields(raw);if(!a||!text(a.invocation))return null;
 const form=forms.find(f=>f.variant===decoded.variant&&a.contract===f.contract&&a.operation===f.operation&&a.profile===f.profile&&exact(a,f.fields)&&matches(a,f.matches)&&(!f.nested||(()=>{const r=fields(a.request);return exact(r,f.nested.fields)&&matches(r,f.nested.matches);})()));if(!form)return null;
 const whole=copySettingsData({route:e.route,request:{...tuple,arguments:a}},cap),scope=copySettingsData(context,cap);if(!whole||!scope||!exact(scope.value,lifecycleActivationContextFields)||!Object.values(scope.value).every(text))return null;
 const captured=copySettingsData({form:form.id,request:whole.value.request.arguments,context:scope.value,limits},cap);if(!captured)return null;
 return Object.freeze({kind:'lifecycle-activation-arguments',variant:form.variant,form:form.id,record:captured.value,route:e.route});
}
export function sameLifecycleActivationArguments(a,b){return Boolean(a&&b&&a.kind==='lifecycle-activation-arguments'&&b.kind==='lifecycle-activation-arguments'&&a.variant===b.variant&&equal(a.record,b.record));}
export function lifecycleActivationProducerRequest(n){return n?n.record.request:null;}
export function lifecycleActivationProducerEnvelope(n){return n?{route:n.route,request:{...(n.route==='typed'?{family:16}:{name:'change-document-lifecycle'}),variant:n.variant,version:1,arguments:n.record.request}}:null;}
