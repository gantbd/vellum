// Existing F07 resource-catalog argument meaning; not native authority or dispatch.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {registrationReadInputForm} from './registration-reads.mjs';
// Lazy access avoids a reader/normalizer initialization cycle. The native
// declaration remains the one source of truth; this does not issue a context.
function declaration(){const form=registrationReadInputForm();
if(form.family!=='F07'||form.contract!=='resource-catalog-query/1'||form.operation!=='capabilities'||form.variant!=='capabilities'||form.profile!=='scoped-resource-catalog/1'||!equal(form.fields,['contract','operation','context','profile','request'])||!equal(form.requests,{list:['operation','limit','continuation'],inspect:['operation','entry']}))throw Error('unsupported native resource-catalog query declaration');return form;}
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v),contextFields=['authority','engine','recipient','purpose','destination','policy','continuity','profile'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const d=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
export const registrationQueryArgumentClassificationBytes=Buffer.byteLength(JSON.stringify(Object.fromEntries(limitFields.map(k=>[k,Number.MAX_SAFE_INTEGER]))));
export function registrationQueryArgumentInputForm(){return declaration();}
export function captureRegistrationQueryArgumentLimits(input){const p=fields(input);return exact(p,limitFields)&&Object.values(p).every(natural)&&p.maxInputNodes>=1?Object.freeze(p):null;}
export function normalizeRegistrationQueryArguments(envelope,authorityContext,inputLimits,maximum){
 const form=declaration();
 const limits=fields(inputLimits);if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1)return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F07'||decoded.variant!==form.variant)return null;
 const args=fields(raw);if(!exact(args,form.fields)||args.contract!==form.contract||args.operation!==form.operation||args.profile!==form.profile||!object(args.context))return null;
 const r=fields(args.request);if(!r||typeof r.operation!=='string'||!Object.hasOwn(form.requests,r.operation)||!exact(r,form.requests[r.operation]))return null;
 const selectorRole=r.operation==='list'?'continuation':'entry',selector=r[selectorRole];if(r.operation==='list'?(!Number.isSafeInteger(r.limit)||r.limit<1||selector!==null&&!object(selector)):!object(selector))return null;
 const context=args.context,shadow={...args,context:null,request:{...r,[selectorRole]:null}},whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(authorityContext,cap);
 if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const captured=copySettingsData({request:whole.value.request.arguments,context:bound.value,limits,opaqueBindings:['registration-read-context',...(selector===null?[]:['registration-'+selectorRole])]},cap);if(!captured)return null;
 return Object.freeze({kind:'registration-query-arguments',record:captured.value,context,selector,selectorRole,route:e.route});
}
export function sameRegistrationQueryArguments(a,b){return Boolean(a&&b&&a.kind==='registration-query-arguments'&&b.kind==='registration-query-arguments'&&a.context===b.context&&a.selector===b.selector&&a.selectorRole===b.selectorRole&&equal(a.record,b.record));}
export function registrationQueryProducerRequest(n){if(!n)return null;const r=n.record.request;return {...r,context:n.context,request:{...r.request,[n.selectorRole]:n.selector}};}
export function registrationQueryProducerEnvelope(n){const request=registrationQueryProducerRequest(n);return request?{route:n.route,request:{...(n.route==='typed'?{family:6}:{name:'inspect-context'}),variant:'capabilities',version:1,arguments:request}}:null;}
