// Existing F06 request meaning; native owners retain identity, grants and wait authority.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {invocationObservationInputForms,invocationReceiptInputForm,invocationReceiptKinds,validInvocationReceiptSelector} from './invocation-observation.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
const forms=invocationObservationInputForms(),operations=['status','bounded-observation-wait','retained-outcome','minimal-control-status'],common=['contract','operation','invocation','context','profile'];
if(forms.length!==4||!equal(forms.map(f=>f.operation).toSorted(),operations.toSorted())||forms.some(f=>f.family!=='F06'||f.contract!=='execution-observation/1'||f.variant!==f.operation||!equal(f.fields.toSorted(),[...common,...(f.operation==='bounded-observation-wait'?['after','budget']:[])].toSorted())))throw Error('unsupported native observation declaration');
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v),contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const d=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
export const invocationObservationArgumentClassificationBytes=Buffer.byteLength(JSON.stringify(Object.fromEntries(limitFields.map(k=>[k,Number.MAX_SAFE_INTEGER]))));
export function captureInvocationObservationArgumentLimits(input){const p=fields(input);return exact(p,limitFields)&&Object.values(p).every(natural)&&p.maxInputNodes>=1?Object.freeze(p):null;}
export function invocationObservationArgumentInputForms(){return forms;}
export function normalizeInvocationObservationArguments(envelope,authorityContext,inputLimits,maximum,receipt=false){
 if(typeof receipt!=='boolean')return null;
 const limits=fields(inputLimits);if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1)return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F06')return null;
 const args=fields(raw),form=(receipt?[invocationReceiptInputForm()]:forms).find(f=>f.operation===args?.operation);if(!form||!exact(args,form.fields)||args.contract!==form.contract||decoded.variant!==args.operation||!object(args.context))return null;
 const waiting=args.operation==='bounded-observation-wait';if(waiting&&(!object(args.after)||!Number.isSafeInteger(args.budget)||args.budget<1))return null;
 const context=args.context,after=waiting?args.after:null,shadow=Object.fromEntries(form.fields.map(k=>[k,k==='context'||k==='after'?null:args[k]])),whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(authorityContext,cap);
 if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const r=whole.value.request.arguments;if(!text(r.invocation)||!text(r.profile)||receipt&&!validInvocationReceiptSelector(r.facet))return null;
 const captured=copySettingsData({request:r,context:bound.value,limits,opaqueBindings:waiting?['observation-context','ordinary-observation']:['observation-context']},cap);if(!captured)return null;
 return Object.freeze({kind:'invocation-observation-arguments',record:captured.value,context,after,route:e.route});
}
export function sameInvocationObservationArguments(a,b){return Boolean(a&&b&&a.kind==='invocation-observation-arguments'&&b.kind==='invocation-observation-arguments'&&a.context===b.context&&a.after===b.after&&equal(a.record,b.record));}
export function invocationObservationProducerRequest(n){return n?{...n.record.request,context:n.context,...(n.record.request.operation==='bounded-observation-wait'?{after:n.after}:{})}:null;}
// Handler-owned metadata uses accepted-registry discovery; older builtin
// profiles and the four execution-observation/1 forms remain unchanged.
export function invocationReceiptEngineDescriptor(){
 const f=invocationReceiptInputForm(),text={type:'string',minLength:1},branches=invocationReceiptKinds().map(kind=>({type:'object',properties:{kind:{const:kind},...(kind==='atomic-annotation'?{index:{type:'integer',minimum:0,maximum:Number.MAX_SAFE_INTEGER}}:{})},required:kind==='atomic-annotation'?['kind','index']:['kind'],additionalProperties:false}));
 const properties={contract:{const:f.contract},operation:{const:f.operation},invocation:text,context:{type:'object'},profile:text,facet:{oneOf:branches}};
 const schema={$schema:inputSchemaDialect,type:'object',properties,required:[...f.fields],additionalProperties:false};
 const d={family:f.family,contract:f.contract,version:1,kind:'query',variants:[{id:f.variant,input:{contract:f.contract,variant:f.variant,fields:properties},outcomes:{statusVocabulary:['operation-receipt','unavailable'],execution:'query-only-no-replay',events:'no-new-event-or-receipt-reference-issuance'},requirements:{inputCoverage:inputSchemaCoverage,schema,runtimeBindings:['original-observation-domain-ticket-and-caller-scope','explicit-captured-facet-ceiling-and-current-receipt-grant','original-native-command-receipt-and-native-disclosure-ceiling','prepaid-context-decoder-and-complete-reply-envelope'],notEstablished:['authority-from-schema-or-advice','provider-or-physical-truth','public-acquisition-transport-or-whole-facade']},limits:{names:['maxContextBytes','maxReplyBytes','maxHeldBytes','maxRequestBytes','maxInputNodes','maxInputDepth'],values:'actual-owner-and-explicit-admitted-profile'},cancellation:'observation-stop-does-not-cancel-producer; protected-control-and-discharge-remain-separate'}],presentation:{}};
 const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};return freeze(d);
}
