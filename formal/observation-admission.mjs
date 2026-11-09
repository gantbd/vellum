// One selected fixed host/origin accounting domain per actual invocation authority.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {validateCapacityProfile,admitCapacity,activateCapacity} from './capacity-admission.mjs';
import {invocationAuthorityContext,invocationExecutionState,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment,releaseInvocationOwnerAttachment} from './invocation-publication.mjs';
import {invocationObservationCharge,openInvocationObservation,invocationObservationUsage,retireInvocationObservation,queryInvocationObservation,queryInvocationReceipt,startInvocationWait,invocationObservationDecoderDeclarations,invocationObservationDecoderReady,invocationObservationDecoderContextMatches,faultInvocationObservationDecoder} from './invocation-observation.mjs';
import {captureInvocationObservationArgumentLimits,normalizeInvocationObservationArguments,invocationObservationProducerRequest} from './invocation-observation-arguments.mjs';
import {copySettingsData} from './semantic-settings.mjs';
import {nativeOperationKinds,nativeOperationObservationParent,nativeOperationObservationCharge,openNativeOperationObservation,nativeOperationObservationUsage,retireNativeOperationObservation,queryNativeOperationObservation,startNativeOperationWait,nativeOperationObservationDecoderReady,nativeOperationObservationDecoderContextMatches} from './native-operation-observation.mjs';
import {normalizeNativeOperationObservationArguments,nativeOperationObservationProducerRequest} from './native-operation-observation-arguments.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./observation-admission.json',import.meta.url))),owners=new WeakMap(),domains=new WeakMap(),copy=structuredClone;
const size=v=>Buffer.byteLength(JSON.stringify(v)),natural=n=>Number.isSafeInteger(n)&&n>=0,exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;},unavailable=()=>({status:'unavailable'}),scopes=['host','origin'];
const demand=(bytes=0,slots=0,work=0,fanout=0,completions=0)=>({bytes,slots,work,fanout,completions});
const empty=()=>({used:Object.fromEntries(scopes.map(s=>[s,demand()]))});
export const invocationObservationDecoderFrames=Object.freeze(['envelope','authority','canonical','limits','descriptor','policy','facts','producer']);
function decoderProfile(input,reference,c,ceiling){
 const p=captureInvocationObservationArgumentLimits(input);if(!p)return null;
 const workspace=invocationObservationDecoderFrames.length*p.maxRequestBytes;if(!natural(workspace))return null;
 const captured=copySettingsData(ceiling,p);if(!captured)return null;const declarations=invocationObservationDecoderDeclarations(),policy=Object.fromEntries(declarations.policyFields.map(k=>[k,k==='epoch'?captured.value.epoch:k==='recipient'?captured.value.recipient:false])),facts=Object.fromEntries([...declarations.releaseGates,'controlReady'].map(k=>[k,true]));
 if(!copySettingsData({policy,facts},p))return null;
 // Structural minimum only: this placeholder context supplies no native authority.
 const request={contract:'execution-observation/1',operation:'minimal-control-status',invocation:reference,context:{},profile:c.profile};
 for(const route of ['typed','dynamic']){const envelope={route,request:{...(route==='typed'?{family:5}:{name:'inspect-execution'}),variant:request.operation,version:1,arguments:request}};if(!normalizeInvocationObservationArguments(envelope,c,p,p.maxRequestBytes))return null;}
 return {limits:p,workspace,busy:false,faulted:false};
}
function validProfile(p){try{validateCapacityProfile(p);return true;}catch{return false;}}
function baseCharge(profile,limits,decoderEnabled){const maximal=copy(profile),used=empty().used;maximal.deadlineTicks=Number.MAX_SAFE_INTEGER;for(const s of scopes){for(const d of ['bytes','slots','work','fanout'])maximal.scope[s].hard[d].ceiling=Number.MAX_SAFE_INTEGER;maximal.scope[s].protectedSlots=Number.MAX_SAFE_INTEGER;maximal.scope[s].softBytes=Number.MAX_SAFE_INTEGER;used[s]=demand(...Array(5).fill(Number.MAX_SAFE_INTEGER));}const record={parentAttachment:{kind:'native-owner-attachment/1'},reservation:{kind:'observation-reservation/1'},context:{kind:'invocation-observation-context/1'},bytes:Number.MAX_SAFE_INTEGER,phase:'unknown',...(decoderEnabled?{decoder:{limits:{maxRequestBytes:Number.MAX_SAFE_INTEGER,maxInputNodes:Number.MAX_SAFE_INTEGER,maxInputDepth:Number.MAX_SAFE_INTEGER},workspace:Number.MAX_SAFE_INTEGER,busy:false,faulted:false}}:{})};return (decoderEnabled?size({maxRequestBytes:Number.MAX_SAFE_INTEGER,maxInputNodes:Number.MAX_SAFE_INTEGER,maxInputDepth:Number.MAX_SAFE_INTEGER}):0)+2*size({profile:maximal,limits,used})+limits.maxReservations*size(record)+size(decoderEnabled?{parentAttachment:{kind:'native-owner-attachment/1'},issued:Number.MAX_SAFE_INTEGER,faulted:false,stopped:false,decoderEnabled:true}:{parentAttachment:{kind:'native-owner-attachment/1'},issued:Number.MAX_SAFE_INTEGER,faulted:true,stopped:true});}
export function openObservationAdmission(domain,profile,limits,g,decoderEnabled=false,nativeProfile=null){
 const nativeEnabled=nativeProfile==='native-operation-observation/1';if(nativeProfile!==null&&!nativeEnabled)return null;
 const attachment=beginInvocationOwnerAttachment(domain,'observation-admission',limits?.maxBookkeepingBytes,Object.freeze({kind:'observation-admission-owner/1'}));if(!attachment)return null;
 try{
 if(typeof decoderEnabled!=='boolean'||typeof nativeEnabled!=='boolean'||nativeEnabled&&!decoderEnabled||!invocationAuthorityContext(domain)||domains.has(domain)||!validProfile(profile)||!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||g?.ownerControlAllowed!==true||g.capacityActivationAuthorized!==true||g.preparationReserved!==true)return null;
 const bytes=baseCharge(profile,limits,decoderEnabled)+(nativeEnabled?limits.maxReservations*size({nativeKind:'annotation-load'})+size({nativeEnabled:true}):0);if(!natural(bytes)||bytes>limits.maxBookkeepingBytes)return null;const admitted=admitCapacity(profile,empty(),demand(bytes));if(admitted.status!=='admitted')return null;
 const pool=Object.freeze({kind:'observation-admission-owner/1'});owners.set(pool,{domain,parentAttachment:attachment,profile:freeze(copy(profile)),limits:freeze(copy(limits)),state:admitted.state,baseBytes:bytes,issued:0,leases:new Map(),stopped:false,faulted:false,...(decoderEnabled?{decoderEnabled:true}:{}),...(nativeEnabled?{nativeEnabled:true}:{})});domains.set(domain,pool);if(!commitInvocationOwnerAttachment(domain,attachment,pool))throw Error('native observation admission attachment invariant');return pool;
 }finally{abortInvocationOwnerAttachment(domain,attachment);}
}
function releaseOccupancy(o,r){const next=copy(o.state);for(const s of scopes){next.used[s].bytes-=r.bytes;next.used[s].slots--;next.used[s].completions--;}o.state=next;}
export function admitPooledObservation(pool,ticket,c,ceiling,limits,g,decoderLimits=null,receiptCeiling=null){
 const o=owners.get(pool);if(!o||o.stopped||o.faulted||g?.authorized!==true||g.scope!==true||g.preparationReserved!==true||o.leases.size>=o.limits.maxReservations||!invocationExecutionState(o.domain,ticket,c,false))return unavailable();
 const bytes=invocationObservationCharge(limits);if(bytes===null||decoderLimits!==null&&!o.decoderEnabled)return unavailable();
 // Fixed scalar classification fits the pool's bookkeeping scratch frame. Fund
 // variable decoder workspace before constructing its minimum native products.
 const capturedLimits=decoderLimits===null?null:captureInvocationObservationArgumentLimits(decoderLimits);if(decoderLimits!==null&&!capturedLimits)return unavailable();
 const reservedWorkspace=capturedLimits?invocationObservationDecoderFrames.length*capturedLimits.maxRequestBytes:0;if(!natural(reservedWorkspace))return unavailable();
 // Child contexts have their own parent reservation. This edge funds only this
 // reservation's prepaid decoder workspace and its separate metadata envelope.
 const reservation=Object.freeze({kind:'observation-reservation/1'}),attachment=beginInvocationOwnerAttachment(o.domain,'observation-admission',reservedWorkspace,reservation);if(!attachment)return unavailable();let record,decoder;
 try{
 // Observation decoder workspace reservation checkpoint.
 decoder=decoderLimits===null?null:decoderProfile(decoderLimits,invocationExecutionState(o.domain,ticket,c,false).reference,c,ceiling);if(decoderLimits!==null&&(!decoder||decoder.workspace!==reservedWorkspace))return unavailable();
 const total=bytes+(decoder?.workspace??0);if(!natural(total))return unavailable();const admitted=admitCapacity(o.profile,o.state,demand(total,1,1,1,1));if(admitted.status!=='admitted')return unavailable();
 record={context:null,parentAttachment:attachment,bytes:total,phase:'unknown',...(o.decoderEnabled?{decoder}:{})};o.state=admitted.state;o.leases.set(reservation,record);o.issued++;
 if(!commitInvocationOwnerAttachment(o.domain,attachment,reservation))throw Error('native observation reservation attachment invariant');
 }finally{abortInvocationOwnerAttachment(o.domain,attachment);}
 try{
  const context=openInvocationObservation(o.domain,ticket,c,ceiling,limits,{...g,registryReady:true},receiptCeiling);
  if(!context&&!releaseInvocationOwnerAttachment(o.domain,record.parentAttachment,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true}))throw Error('native refused observation reservation discharge invariant');
  if(!context){releaseOccupancy(o,record);o.leases.delete(reservation);return unavailable();}
  record.context=context;
  if(invocationObservationUsage(context)?.declaredHeldBytes!==bytes)throw Error('producer reservation mismatch');record.phase='active';return {status:'context',context,reservation};
 }catch{o.faulted=true;return {status:'unavailable',reservation};}
}
// Explicit native-profile admission through the same actual-domain pool. The
// existing eight decoder frames and capacity ledger are reused, not duplicated.
export function admitPooledNativeOperationObservation(pool,kind,owner,task,c,ceiling,limits,g,inputLimits){
 const o=owners.get(pool);if(!o?.nativeEnabled||o.stopped||o.faulted||!nativeOperationKinds().includes(kind)||o.leases.size>=o.limits.maxReservations||size({context:c,ceiling,limits})>limits?.maxContextBytes)return unavailable();
 c=copy(c);ceiling=copy(ceiling);limits=copy(limits);
 const bytes=nativeOperationObservationCharge(limits),capturedLimits=captureInvocationObservationArgumentLimits(inputLimits);if(bytes===null||!capturedLimits||nativeOperationObservationParent(kind,owner,task,c)!==o.domain)return unavailable();
 const workspace=invocationObservationDecoderFrames.length*capturedLimits.maxRequestBytes;if(!natural(workspace))return unavailable();
 const reservation=Object.freeze({kind:'observation-reservation/1'}),attachment=beginInvocationOwnerAttachment(o.domain,'observation-admission',workspace,reservation);if(!attachment)return unavailable();let record;
 try{
  if(g?.authorized!==true||g.scope!==true||g.preparationReserved!==true||nativeOperationObservationParent(kind,owner,task,c)!==o.domain)return unavailable();
  const declarations=invocationObservationDecoderDeclarations(),p=Object.fromEntries(declarations.policyFields.map(k=>[k,k==='epoch'?ceiling.epoch:k==='recipient'?ceiling.recipient:false])),facts=Object.fromEntries([...declarations.releaseGates,'controlReady','operationReceiptAllowed','sourceReadable','annotationReadable','readAllowed','dispositionReadable'].map(k=>[k,true]));
  if(!copySettingsData({policy:p,facts},capturedLimits))return unavailable();
  const request={contract:'native-operation-observation/1',operation:'minimal-control-status',task,context:{},profile:c.profile};
  for(const route of ['typed','dynamic'])if(!normalizeNativeOperationObservationArguments({route,request:{...(route==='typed'?{family:5}:{name:'inspect-execution'}),variant:request.operation,version:1,arguments:request}},c,capturedLimits,capturedLimits.maxRequestBytes))return unavailable();
  const total=bytes+workspace;if(!natural(total))return unavailable();const admitted=admitCapacity(o.profile,o.state,demand(total,1,1,1,1));if(admitted.status!=='admitted')return unavailable();
  record={context:null,parentAttachment:attachment,bytes:total,phase:'unknown',nativeKind:kind,decoder:{limits:capturedLimits,workspace,busy:false,faulted:false}};o.state=admitted.state;o.leases.set(reservation,record);o.issued++;
  if(!commitInvocationOwnerAttachment(o.domain,attachment,reservation))throw Error('native observation decoder reservation invariant');
 }catch{if(record){o.faulted=true;return {status:'unavailable',reservation};}return unavailable();}finally{abortInvocationOwnerAttachment(o.domain,attachment);}
 try{
  const context=openNativeOperationObservation(kind,owner,task,c,ceiling,limits,{...g,registryReady:true});
  if(!context){if(!releaseInvocationOwnerAttachment(o.domain,record.parentAttachment,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true}))throw Error('native refused decoder reservation discharge invariant');releaseOccupancy(o,record);o.leases.delete(reservation);return unavailable();}
  record.context=context;if(nativeOperationObservationUsage(context)?.declaredHeldBytes!==bytes)throw Error('native observation child reservation mismatch');record.phase='active';return {status:'context',context,reservation};
 }catch{o.faulted=true;return {status:'unavailable',reservation};}
}
export function retireObservationReservation(pool,reservation,g){
 const o=owners.get(pool),r=o?.leases.get(reservation);if(!r||r.decoder?.busy||r.phase!=='active'||!r.context)return unavailable();
 g=Object.freeze(Object.fromEntries(['ownerControlAllowed','retentionReleased','deliveryDrained'].map(k=>[k,g?.[k]===true])));
 if(o.leases.get(reservation)!==r||g?.ownerControlAllowed!==true||g.retentionReleased!==true||g.deliveryDrained!==true)return unavailable();
 const usage=r.nativeKind?nativeOperationObservationUsage:invocationObservationUsage,retire=r.nativeKind?retireNativeOperationObservation:retireInvocationObservation;
 if(usage(r.context)!==null&&retire(r.context,g).status!=='retired')return unavailable();
 if(usage(r.context)!==null)return unavailable();
 if(!releaseInvocationOwnerAttachment(o.domain,r.parentAttachment,g))throw Error('native observation reservation discharge invariant');
 releaseOccupancy(o,r);o.leases.delete(reservation);return {status:'retired'};
}
export function activateObservationAdmission(pool,candidate,g){const o=owners.get(pool);if(!o||g?.ownerControlAllowed!==true||g.capacityActivationAuthorized!==true||!validProfile(candidate)||candidate.deadlineTicks!==o.profile.deadlineTicks)return unavailable();const next=activateCapacity(o.profile,o.state,candidate,true);if(next.status!=='activated')return unavailable();o.profile=freeze(copy(next.profile));o.state=next.state;return {status:'activated'};}
export function stopObservationAdmission(pool,g){const o=owners.get(pool);if(!o||g?.ownerControlAllowed!==true)return unavailable();o.stopped=true;return {status:'stopped'};}
export function observationAdmissionUsage(pool){const o=owners.get(pool);return o?{issued:o.issued,reservations:o.leases.size,unknown:[...o.leases.values()].filter(r=>r.phase==='unknown').length,stopped:o.stopped,faulted:o.faulted,baseBytes:o.baseBytes,profile:copy(o.profile),state:copy(o.state)}:null;}
function queryDecodedNativeObservation(r,d,envelope,c,p,g){
 try{
  const native=normalizeNativeOperationObservationArguments(envelope,c,d.limits,d.limits.maxRequestBytes);if(!native||!nativeOperationObservationDecoderContextMatches(r.context,native.record.context))return unavailable();
  const nativePacket=copySettingsData({policy:p,facts:g},d.limits);if(!nativePacket)return unavailable();
  const request=nativeOperationObservationProducerRequest(native);return request.operation==='bounded-observation-wait'?startNativeOperationWait(r.context,request,native.record.context,nativePacket.value.policy,nativePacket.value.facts):queryNativeOperationObservation(r.context,request,native.record.context,nativePacket.value.policy,nativePacket.value.facts);
 }catch{d.faulted=true;return unavailable();}
}
export function queryDecodedPooledObservation(pool,reservation,envelope,c,p,g){
 const o=owners.get(pool),r=o?.leases.get(reservation),d=r?.decoder;if(!r||r.phase!=='active'||!d||d.busy||d.faulted||!(r.nativeKind?nativeOperationObservationDecoderReady(r.context):invocationObservationDecoderReady(r.context)))return unavailable();
 d.busy=true;
 try{
  if(r.nativeKind)return queryDecodedNativeObservation(r,d,envelope,c,p,g);
  const n=normalizeInvocationObservationArguments(envelope,c,d.limits,d.limits.maxRequestBytes)??normalizeInvocationObservationArguments(envelope,c,d.limits,d.limits.maxRequestBytes,true);if(!n||!invocationObservationDecoderContextMatches(r.context,n.record.context))return unavailable();
  const packet=copySettingsData({policy:p,facts:g},d.limits);if(!packet)return unavailable();
  const request=invocationObservationProducerRequest(n);return request.contract==='execution-receipt/1'?queryInvocationReceipt(r.context,request,n.record.context,packet.value.policy,packet.value.facts):request.operation==='bounded-observation-wait'?startInvocationWait(r.context,request,n.record.context,packet.value.policy,packet.value.facts):queryInvocationObservation(r.context,request,n.record.context,packet.value.policy,packet.value.facts);
 }catch{d.faulted=true;return faultInvocationObservationDecoder(r.context);}finally{d.busy=false;}
}
export function observationDecoderUsage(pool,reservation){const d=owners.get(pool)?.leases.get(reservation)?.decoder;return d?{busy:d.busy,faulted:d.faulted,declaredWorkspaceBytes:d.workspace,inputLimits:copy(d.limits)}:null;}
