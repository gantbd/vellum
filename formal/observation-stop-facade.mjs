// Closed F19 stop composition. Target/domain/scope are first-party lowering
// inputs, never authority fields supplied by a public command. No retained
// registry, callback handler, issuer, producer cancellation or policy mutation.
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {copySettingsData} from './semantic-settings.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
import {nativeResourceMetadataVocabulary} from './resource-registration-state.mjs';
import {invocationOwnerAttachmentMatches,invocationOwnerAttachmentInventory,invocationReadStopTargetOwned,stopInvocationSourceContext,stopInvocationAnnotationContext,stopInvocationVersionSetContext,stopInvocationSettingsRead} from './invocation-publication.mjs';
import {historyInspectionClaimMatches,historyInspectionStopTargetOwned,closeHistoryInspection} from './history-contexts.mjs';
import {nativeDiscoveryStopTargetOwned,stopNativeDiscovery} from './native-discovery.mjs';
import {registrationReadStopTargetOwned,stopRegistrationRead} from './registration-reads.mjs';
import {registrationObservationStopTargetOwned,stopRegistrationObservation,registrationObservationCommandOwned,requestRegistrationObservationCommand,registrationObservationReceiptOwned,releaseRegistrationObservationReceipt,stopRegistrationObservationReceipt,validateRegistrationObservationReceiptReply} from './registration-events.mjs';
import {stopBoundInvocationPublicationObserver} from './native-publication-observation.mjs';
import {invocationObservationStopTargetOwned,closeInvocationObservation} from './invocation-observation.mjs';
import {nativeOperationObservationStopTargetOwned,closeNativeOperationObservation} from './native-operation-observation.mjs';
const no=()=>({status:'unavailable'}),object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
const kinds=Object.freeze(['source','annotation','version-set','history','settings','discovery','resource-catalog','registration-stream','publication-stream','execution','native-operation','registration-receipt']);
const attachmentKinds=Object.freeze(['source-contexts','annotation-contexts','version-set-contexts','history','settings-reads','discovery','registration-reads','registration-events','publication-observer','observation','observation','registration-events']);
function fields(v){if(!object(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return null;const ds=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(ds);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(ds[k],'value')||ds[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,ds[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
export function observationStopKinds(){return kinds;}
// inputLimits is the host's already funded bounded input frame. This stateless
// adapter neither reserves new capacity nor proves a hostile host/Proxy safe.
function captureStop(envelope,authorityContext,claimedScope,inputLimits,kind){
 const bounds=fields(inputLimits);if(!exact(bounds,['maxRequestBytes','maxInputNodes','maxInputDepth']))return null;
 const e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,d=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple),r=fields(raw);
 if(d?.descriptor.id!=='F19'||d.variant!=='stop-observation'||!exact(r,['contract','operation','context'])||r.contract!=='read-context/1'||r.operation!=='stop-observation'||!object(r.context))return null;
 // Never traverse an opaque owner/context token. Copy all ordinary caller and
 // lowering values before the first independently attributed current-fact read.
 const captured=copySettingsData({authorityContext,claimedScope,route:e.route,tuple,request:{...r,context:null},kind},bounds);return captured?{...captured,reference:r.context}:null;
}
export function dispatchBoundObservationStop(domain,authorityContext,target,envelope,claimedScope,g,inputLimits){
 const t=fields(target);if(!exact(t,['kind','owner'])||!kinds.includes(t.kind)||!object(t.owner))return no();
 const captured=captureStop(envelope,authorityContext,claimedScope,inputLimits,t.kind);if(!captured)return no();
 const a=captured.value.authorityContext,c=captured.value.claimedScope,request={...captured.value.request,context:captured.reference},owner=t.owner,kind=captured.value.kind;
 const allowed=g?.ownerControlAllowed===true;if(!allowed||!invocationOwnerAttachmentMatches(domain,a,owner,attachmentKinds[kinds.indexOf(kind)]))return no();
 return stopCaptured(domain,owner,kind,request,c);
}
function stopCaptured(domain,owner,kind,request,c){
 const r=request;
 const checks={ownerControlAllowed:true};let result;
 switch(kind){
  case 'source':result=stopInvocationSourceContext(domain,owner,request,c,checks);break;
  case 'annotation':result=stopInvocationAnnotationContext(domain,owner,request,c,checks);break;
  case 'version-set':result=stopInvocationVersionSetContext(domain,owner,request,c,checks);break;
  case 'history':result=historyInspectionClaimMatches(owner,c)?closeHistoryInspection(owner,r.context,c.recipient,checks):no();break;
  case 'settings':result=stopInvocationSettingsRead(domain,owner,r.context,c,checks)?{status:'stopped'}:no();break;
  case 'discovery':result=stopNativeDiscovery(owner,r.context,c,checks);break;
  case 'resource-catalog':result=stopRegistrationRead(owner,r.context,c,checks);break;
  case 'registration-stream':result=stopRegistrationObservation(owner,r.context,c,checks);break;
  case 'registration-receipt':result=stopRegistrationObservationReceipt(owner,r.context,c,checks);break;
  case 'publication-stream':result=r.context===owner?stopBoundInvocationPublicationObserver(owner,c,checks):no();break;
  case 'execution':result=r.context===owner?closeInvocationObservation(owner,c,checks):no();break;
  case 'native-operation':result=r.context===owner?closeNativeOperationObservation(owner,c,checks):no();break;
  default:return no();
 }
 return result?.status==='stopped'?{status:'stopped'}:no();
}
// One finite pass over actual, already funded attachment metadata. Each test is
// pure native membership; no mutable stop/query is tried to discover an issuer.
function stopKind(domain,entry,reference){
 const owner=entry.owner;
 switch(entry.kind){
  case 'source-contexts':return invocationReadStopTargetOwned(domain,owner,reference,'source')?'source':null;
  case 'annotation-contexts':return invocationReadStopTargetOwned(domain,owner,reference,'annotation')?'annotation':null;
  case 'version-set-contexts':return invocationReadStopTargetOwned(domain,owner,reference,'version-set')?'version-set':null;
  case 'settings-reads':return invocationReadStopTargetOwned(domain,owner,reference,'settings')?'settings':null;
  case 'history':return historyInspectionStopTargetOwned(owner,reference)?'history':null;
  case 'discovery':return nativeDiscoveryStopTargetOwned(owner,reference)?'discovery':null;
  case 'registration-reads':return registrationReadStopTargetOwned(owner,reference)?'resource-catalog':null;
  case 'registration-events':return registrationObservationStopTargetOwned(owner,reference)?'registration-stream':registrationObservationReceiptOwned(owner,reference)?'registration-receipt':null;
  case 'publication-observer':return reference===owner?'publication-stream':null;
  case 'observation':if(reference!==owner)return null;if(invocationObservationStopTargetOwned(owner))return 'execution';return nativeOperationObservationStopTargetOwned(owner)?'native-operation':null;
  default:return null;
 }
}
export function dispatchObservationStop(domain,authorityContext,envelope,claimedScope,g,inputLimits){
 const captured=captureStop(envelope,authorityContext,claimedScope,inputLimits,null);if(!captured)return no();
 const data=captured.value,scope=data.claimedScope,authority=data.authorityContext,reference=captured.reference;
 const permitted=g?.ownerControlAllowed===true;if(!permitted)return no();
 const inventory=invocationOwnerAttachmentInventory(domain,authority,{ownerControlAllowed:true});if(!inventory)return no();
 let selected=null;
 for(const entry of inventory.entries){const kind=stopKind(domain,entry,reference);if(kind!==null){if(selected!==null)return no();selected={owner:entry.owner,kind};}}
 if(selected===null)return no();
 return stopCaptured(domain,selected.owner,selected.kind,{...data.request,context:reference},scope);
}
// F19 command binding over the same local registry, not a baseline payload
// release or an implicit new preparation. Its bounded input frame is supplied
// by the host, as for the stop facade above.
export function dispatchRegistrationObservationCommand(domain,authorityContext,envelope,claimedScope,g,inputLimits){
 const bounds=fields(inputLimits),e=fields(envelope);if(!exact(bounds,['maxRequestBytes','maxInputNodes','maxInputDepth'])||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return no();
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return no();
 const {arguments:raw,...tuple}=h,d=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple),r=fields(raw);
 if(d?.descriptor.id!=='F19'||!['establish-bounded-stream','authorized-resynchronization'].includes(d.variant)||!exact(r,['contract','operation','command','stream','preparation'])||r.contract!=='registration-observation-command/1'||r.operation!==d.variant||!object(r.command)||r.preparation!==null&&!object(r.preparation)||r.operation==='establish-bounded-stream'&&r.stream!==null||r.operation==='authorized-resynchronization'&&!object(r.stream))return no();
 const captured=copySettingsData({authorityContext,claimedScope,route:e.route,tuple,request:{...r,command:null,stream:null,preparation:null}},bounds);if(!captured)return no();
 const data=captured.value,command=r.command,stream=r.stream,preparation=r.preparation,operation=data.request.operation;
 let permitted;try{permitted=g?.ownerControlAllowed===true;}catch(cause){throw new Error('registration-observation-command-protocol-fault',{cause});}if(!permitted)return no();
 const inventory=invocationOwnerAttachmentInventory(domain,data.authorityContext,{ownerControlAllowed:true});if(!inventory)return no();let owner=null;
 for(const entry of inventory.entries)if(entry.kind==='registration-events'&&registrationObservationCommandOwned(entry.owner,command)){if(owner!==null)return no();owner=entry.owner;}
 if(owner===null)return no();
 const result=requestRegistrationObservationCommand(owner,command,operation,stream,data.claimedScope,preparation,g);
 return ['command-settled','observe-existing'].includes(result?.status)&&result.command===command?{status:result.status,command}:no();
}
// F06 original-result delivery. The preparation is a first-party lowering input,
// not an authority-bearing field accepted from the public request. Actual native
// certificate identity/coverage/current-root checks remain mandatory at release.
export function dispatchRegistrationObservationReceipt(domain,authorityContext,envelope,claimedScope,preparation,g,inputLimits){
 const bounds=fields(inputLimits),e=fields(envelope);if(!exact(bounds,['maxRequestBytes','maxInputNodes','maxInputDepth'])||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return no();
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return no();
 const {arguments:raw,...tuple}=h,d=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple),r=fields(raw),facet=r&&fields(r.facet);
 if(d?.descriptor.id!=='F06'||d.variant!=='retained-outcome'||!exact(r,['contract','operation','command','context','profile','facet'])||r.contract!=='registration-observation-receipt/1'||r.operation!=='retained-outcome'||!exact(facet,['kind'])||facet.kind!=='registry-baseline'||!object(r.command)||!object(r.context)||!object(preparation))return no();
 const captured=copySettingsData({authorityContext,claimedScope,route:e.route,tuple,request:{...r,command:null,context:null}},bounds);if(!captured)return no();
 const data=captured.value,command=r.command,context=r.context,scope=data.claimedScope;if(data.request.profile!==scope?.profile)return no();
 // Internal pure identity lookup, never a recipient grant or returned inventory.
 // Native release independently checks all current disclosure facts afterwards.
 const inventory=invocationOwnerAttachmentInventory(domain,data.authorityContext,{ownerControlAllowed:true});if(!inventory)return no();let owner=null;
 for(const entry of inventory.entries)if(entry.kind==='registration-events'&&registrationObservationReceiptOwned(entry.owner,context)){if(owner!==null)return no();owner=entry.owner;}
 if(owner===null)return no();
 try{const result=releaseRegistrationObservationReceipt(owner,context,command,scope,preparation,g);if(exact(result,['status'])&&result.status==='unavailable')return no();return validateRegistrationObservationReceiptReply(owner,context,command,scope,result)?result:no();}
 catch(cause){stopRegistrationObservationReceipt(owner,context,scope,{ownerControlAllowed:true});throw new Error('registration-observation-receipt-protocol-fault',{cause});}
}
// Handler-owned accepted-registry metadata, not a builtin-profile widening.
export function registrationObservationEngineDescriptors(){
 const opaque={type:'object'},text={type:'string',minLength:1},boolean={type:'boolean'},closed=properties=>({type:'object',properties,required:Object.keys(properties),additionalProperties:false}),unavailable=closed({status:{const:'unavailable'}}),facet=closed({kind:{const:'registry-baseline'}});
 const source=closed({profile:{const:'strict-utf8-one-bom/1'},atomicRead:boolean,insertionLineBreak:{enum:[...nativeResourceMetadataVocabulary().insertionLineBreaks]}}),annotation=closed({profile:{const:'vellum.annotations/1'},atomicRead:boolean}),write=closed({procedure:text,identityKnown:boolean,logicalContinuity:boolean,syncSupported:boolean,conditionalWrite:boolean,causalReceipts:boolean,nonReuseTokens:boolean,exclusiveCreate:boolean});
 const entry={oneOf:[closed({reference:opaque,table:{enum:['read','write']}}),closed({reference:opaque,table:{const:'read'},descriptor:source}),closed({reference:opaque,table:{const:'read'},descriptor:annotation}),closed({reference:opaque,table:{const:'write'},descriptor:write})]},baseline={status:{const:'baseline'},coverage:{const:'captured-policy-current'},generation:opaque,entries:{type:'array',items:entry}},receiptResult={$schema:inputSchemaDialect,oneOf:[unavailable,closed({status:{const:'operation-receipt'},facet,receipt:{oneOf:[closed(baseline),closed({...baseline,stream:opaque})]}})]};
 const commandResult={$schema:inputSchemaDialect,oneOf:[unavailable,...['command-settled','observe-existing'].map(status=>closed({status:{const:status},command:opaque}))]};
 const variant=(contract,id,properties,result,statuses)=>({id,input:{contract,variant:id,fields:properties},outcomes:{statusVocabulary:statuses,result,execution:contract==='registration-observation-receipt/1'?'historical-query-only-no-command-execution':'one-original-native-command-or-retained-control-lookup',events:'no-public-event-transport-or-command-replay-payload'},requirements:{inputCoverage:inputSchemaCoverage,schema:{$schema:inputSchemaDialect,...closed(properties)},runtimeBindings:['actual-attached-registry-and-original-native-identities','complete-original-scope-and-captured-current-grants','exact-historical-slot-and-current-root-certificate','prepaid-frames-and-nonrefundable-issuance','protected-control-and-retained-command-claims'],notEstablished:['authority-from-schema-or-discovery','host-policy-truth-or-hostile-wire-decoding','whole-shutdown-or-independent-certification']},limits:{names:['maxRequestBytes','maxInputNodes','maxInputDepth','maxContextBytes','maxPacketBytes','maxReplyBytes','maxBindingBytes','maxPreparations','maxIssued','maxHeldBytes'],values:'actual-registry-and-explicit-profile'},cancellation:'receipt-stop-is-not-producer-cancellation; actual-command-claim-until-protected-context-retirement'});
 const commandContract='registration-observation-command/1',receiptContract='registration-observation-receipt/1';
 const commands=['establish-bounded-stream','authorized-resynchronization'].map(operation=>variant(commandContract,operation,{contract:{const:commandContract},operation:{const:operation},command:opaque,stream:operation==='establish-bounded-stream'?{type:'null'}:opaque,preparation:{oneOf:[opaque,{type:'null'}]}},commandResult,['command-settled','observe-existing','unavailable']));
 const receipt=variant(receiptContract,'retained-outcome',{contract:{const:receiptContract},operation:{const:'retained-outcome'},command:opaque,context:opaque,profile:text,facet},receiptResult,['operation-receipt','unavailable']);
 const values=[{family:'F19',contract:commandContract,version:1,kind:'localCommand',variants:commands,presentation:{}},{family:'F06',contract:receiptContract,version:1,kind:'query',variants:[receipt],presentation:{}}],freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};return freeze(values);
}
