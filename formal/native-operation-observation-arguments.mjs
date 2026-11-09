// Explicit native F06 profile. Opaque task/context/progress roles are not copied
// or traversed; normalized ordinary values use the established bounded copier.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {captureInvocationObservationArgumentLimits} from './invocation-observation-arguments.mjs';
import {nativeOperationKinds,nativeOperationObservationInputForms,nativeOperationReceiptFailureReasons} from './native-operation-observation.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v),natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0;
const contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'],forms=()=>nativeOperationObservationInputForms();
function fields(v){if(!object(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return null;const ds=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(ds);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(ds[k],'value')||ds[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,ds[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
export function normalizeNativeOperationObservationArguments(envelope,c,inputLimits,maximum){
 const limits=captureInvocationObservationArgumentLimits(inputLimits);if(!limits||!natural(maximum))return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const header=fields(e.request);if(!exact(header,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=header,d=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(d?.descriptor.id!=='F06')return null;
 const a=fields(raw),f=forms().find(f=>f.operation===a?.operation);if(!f||!exact(a,f.fields)||a.contract!==f.contract||d.variant!==a.operation||!object(a.task)||!object(a.context))return null;
 const waiting=a.operation==='bounded-observation-wait';if(waiting&&(!object(a.after)||!natural(a.budget)||a.budget<1))return null;
 const shadow=Object.fromEntries(f.fields.map(k=>[k,['task','context','after'].includes(k)?null:a[k]])),whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),scope=copySettingsData(c,cap);
 if(!whole||!scope||!exact(scope.value,contextFields)||!Object.values(scope.value).every(text))return null;
 const r=whole.value.request.arguments;if(!text(r.profile)||r.operation==='retained-outcome'&&(!exact(r.facet,['kind'])||!nativeOperationKinds().includes(r.facet.kind)))return null;
 const captured=copySettingsData({request:r,context:scope.value,limits,opaqueBindings:waiting?['native-task','observation-context','ordinary-progress']:['native-task','observation-context']},cap);if(!captured)return null;
 return Object.freeze({kind:'native-operation-observation-arguments',record:captured.value,task:a.task,context:a.context,after:waiting?a.after:null,route:e.route});
}
export function sameNativeOperationObservationArguments(a,b){return Boolean(a&&b&&a.kind==='native-operation-observation-arguments'&&b.kind===a.kind&&a.task===b.task&&a.context===b.context&&a.after===b.after&&equal(a.record,b.record));}
export function nativeOperationObservationProducerRequest(n){return n?{...n.record.request,task:n.task,context:n.context,...(n.record.request.operation==='bounded-observation-wait'?{after:n.after}:{})}:null;}
// Structural metadata only. Native ownership, permission and evidence relations
// remain runtime guards, not facts established by JSON Schema validation.
export function nativeOperationObservationResultSchema(operation,continuation=false){
 const opaque={type:'object'},boolean={type:'boolean'},n={type:'integer',minimum:0,maximum:Number.MAX_SAFE_INTEGER},text={type:'string'},closed=properties=>({type:'object',properties,required:Object.keys(properties),additionalProperties:false}),version=closed({epoch:{type:'string',minLength:1},token:text}),unavailable=closed({status:{const:'unavailable'}});
 const receipt=kind=>{const common={kind:{const:kind},phase:{enum:kind.endsWith('load')?['ready','submitted']:['queued','dispatched']},stopped:boolean,quiescent:boolean,contested:boolean};return kind.endsWith('load')?closed({...common,decodeRefused:boolean,receivedChunks:n,receivedBytes:n,witnesses:{type:'array',maxItems:2,items:{oneOf:[closed({kind:{const:'end'},resourceVersion:version,coherence:{enum:['captured-read','atomic-resource-snapshot']},chunks:n,byteLength:n}),closed({kind:{const:'failure'},reason:{enum:nativeOperationReceiptFailureReasons()}})]}}}):closed({...common,safeDisposition:boolean,destinationFaulted:boolean,knowledge:closed({application:{enum:['unknown','written','not-written']},observedBytes:n,synchronization:{enum:['unknown','succeeded','failed','not-requested']},resourceVersion:{oneOf:[{type:'null'},version]}}),requiredAcknowledgment:{enum:['written','synchronized']},requestedStrengthSatisfied:boolean});};
 const execution=rich=>{const s=closed({status:{const:'native-execution'},phase:{enum:['pending','terminal']},stopped:boolean,quiescent:boolean,observation:opaque,...(rich?{receipt:{oneOf:nativeOperationKinds().map(receipt)}}:{})});if(rich)s.required=s.required.filter(k=>k!=='receipt');return s;};
 const waiting=closed({status:{const:'waiting'},wait:opaque}),ended=closed({status:{const:'observation-ended'}});
 const variants=operation==='retained-outcome'?nativeOperationKinds().map(kind=>closed({status:{const:'native-operation-receipt'},facet:closed({kind:{const:kind}}),receipt:receipt(kind),observation:opaque})):operation==='bounded-observation-wait'?(continuation?[waiting,ended,closed({status:{const:'observed'},result:execution(true)})]:[waiting]):['status','minimal-control-status'].includes(operation)?[execution(operation==='status')]:null;
 return variants?{$schema:inputSchemaDialect,oneOf:[unavailable,...variants]}:null;
}
export function nativeOperationObservationEngineDescriptors(){
 const opaque={type:'object'},text={type:'string',minLength:1},variants=forms().map(f=>{
  const properties={contract:{const:f.contract},operation:{const:f.operation},task:opaque,context:opaque,profile:text,...(f.operation==='retained-outcome'?{facet:{type:'object',properties:{kind:{enum:[...nativeOperationKinds()]}},required:['kind'],additionalProperties:false}}:f.operation==='bounded-observation-wait'?{after:opaque,budget:{type:'integer',minimum:1,maximum:Number.MAX_SAFE_INTEGER}}:{})};
  return {id:f.variant,input:{contract:f.contract,variant:f.variant,fields:properties},outcomes:{statusVocabulary:f.operation==='retained-outcome'?['native-operation-receipt','unavailable']:f.operation==='bounded-observation-wait'?['waiting','unavailable']:['native-execution','unavailable'],result:nativeOperationObservationResultSchema(f.operation),...(f.operation==='bounded-observation-wait'?{privateContinuation:nativeOperationObservationResultSchema(f.operation,true)}:{}),execution:'query-only-no-provider-dispatch-or-publication',events:'private-bounded-wait-continuations-not-a-public-event-transport'},requirements:{inputCoverage:inputSchemaCoverage,schema:{$schema:inputSchemaDialect,type:'object',properties,required:[...f.fields],additionalProperties:false},runtimeBindings:['actual-owned-parent-task-and-original-context','captured-current-recipient-epoch-disclosure','explicit-native-profile-and-prepaid-decoder-workspace','protected-control-and-producer-observer-separation'],notEstablished:['authority-from-schema-or-advice','hostile-handle-decoding-or-provider-truth','complete-F19-F18-or-production-proof']},limits:{names:['maxRequestBytes','maxInputNodes','maxInputDepth','maxContextBytes','maxReplyBytes','maxControlReplyBytes','maxHeldBytes'],values:'actual-owner-and-explicit-profile'},cancellation:'wait-stop-expiry-do-not-stop-producer; protected-discharge-is-independent'};
 });
 const ds=['native-operation-observation/1','native-operation-receipt/1'].map(contract=>({family:'F06',contract,version:1,kind:'query',variants:variants.filter(v=>v.input.contract===contract),presentation:{}}));
 const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};return freeze(ds);
}
