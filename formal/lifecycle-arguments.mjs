// Selected existing F17 argument capture, not native authority or publication.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {documentLifecycleInputForms} from './lifecycle-products.mjs';
import {lifecycleInvocationInputForm} from './lifecycle-invocation-state.mjs';
// Native guards participate in the owner graph. Read their fixed declarations
// only after module evaluation, so every supported entry point can initialize.
function declarations(){const forms=documentLifecycleInputForms(),wrapper=lifecycleInvocationInputForm();
 if(wrapper.family!=='F17'||wrapper.contract!=='lifecycle-invocation/1'||wrapper.operation!=='change-document-lifecycle'||wrapper.profile!=='native-document-lifecycle/1'||!equal(wrapper.fields,['contract','operation','invocation','profile','request'])||forms.length!==5)throw Error('unsupported native lifecycle declaration');
 return {forms,wrapper};}
const natural=n=>Number.isSafeInteger(n)&&n>=0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v),text=v=>typeof v==='string'&&v.length>0;
const limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
function fields(v){if(!object(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return null;const d=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,keys)=>v&&Object.keys(v).length===keys.length&&keys.every(k=>Object.hasOwn(v,k));
export function lifecycleArgumentInputForms(){return declarations().forms;}
export function captureLifecycleArgumentLimits(input){const p=fields(input);return exact(p,limitFields)&&Object.values(p).every(natural)&&p.maxInputNodes>0?Object.freeze(p):null;}
export function normalizeLifecycleArguments(envelope,authorityContext,inputLimits,maximum){
 const {forms,wrapper}=declarations(),scopeFields=wrapper.contextFields;
 const limits=captureLifecycleArgumentLimits(inputLimits);if(!limits||!natural(maximum))return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F17')return null;
 const a=fields(raw);if(!exact(a,wrapper.fields)||a.contract!==wrapper.contract||a.operation!==wrapper.operation||a.profile!==wrapper.profile||!text(a.invocation))return null;
 const r=fields(a.request),form=forms.find(f=>f.operation===r?.operation);if(!form||form.variant!==decoded.variant||r.contract!==form.contract||!exact(r,form.fields))return null;
 const invocation=a.invocation,whole=copySettingsData({route:e.route,request:{...tuple,arguments:{...a,request:r}}},cap),scope=copySettingsData(authorityContext,cap);
 if(!whole||!scope||!exact(scope.value,scopeFields)||!Object.values(scope.value).every(text))return null;
 const captured=copySettingsData({request:whole.value.request.arguments,context:scope.value,limits},cap);if(!captured)return null;
 return Object.freeze({kind:'lifecycle-arguments',variant:form.variant,operation:form.operation,record:captured.value,invocation,route:e.route});
}
export function sameLifecycleArguments(a,b){return Boolean(a&&b&&a.kind==='lifecycle-arguments'&&b.kind==='lifecycle-arguments'&&a.invocation===b.invocation&&a.variant===b.variant&&equal(a.record,b.record));}
export function lifecycleProducerRequest(n){return n?{...n.record.request,invocation:n.invocation}:null;}
export function lifecycleProducerEnvelope(n){const request=lifecycleProducerRequest(n);return request?{route:n.route,request:{...(n.route==='typed'?{family:16}:{name:'change-document-lifecycle'}),variant:n.variant,version:1,arguments:request}}:null;}
