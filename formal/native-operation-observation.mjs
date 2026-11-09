// Versioned F06 native-effect profile. No publication summary, provider dispatch,
// arbitrary callbacks, new producer, OS service or transport is introduced.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {nativeSourceLoadOperationState} from './resource-read.mjs';
import {nativeAnnotationLoadOperationState} from './annotation-native-load.mjs';
import {nativeOrdinarySaveOperationState} from './source-save.mjs';
import {beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment,releaseInvocationOwnerAttachment} from './invocation-publication.mjs';
import {invocationObservationCharge,invocationObservationDecoderDeclarations} from './invocation-observation.mjs';
import {decodeTyped,decodeDynamic,planRoute} from './operation-catalog.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./invocation-observation.json',import.meta.url))),families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url)));
const owners=new WeakMap(),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v)),natural=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(v,keys)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort()),gates=(g,ks)=>ks.every(k=>g?.[k]===true),no=()=>({status:'unavailable'});
const kinds=Object.freeze(['source-load','annotation-load','source-save','annotation-save']),operations=Object.freeze(['status','retained-outcome','minimal-control-status','bounded-observation-wait']);
export function nativeOperationKinds(){return kinds;}
export function nativeOperationObservationInputForms(){return operations.map(operation=>({family:'F06',variant:operation,operation,contract:operation==='retained-outcome'?'native-operation-receipt/1':'native-operation-observation/1',fields:['contract','operation','task','context','profile',...(operation==='retained-outcome'?['facet']:operation==='bounded-observation-wait'?['after','budget']:[])]}));}
const flags=['publication','control','effect','reasons','details','recovery'],policyFields=invocationObservationDecoderDeclarations().policyFields;
const policy=p=>exact(p,policyFields)&&[...flags,'grant'].every(k=>typeof p[k]==='boolean')&&natural(p.epoch)&&typeof p.recipient==='string';
const version=v=>v===null||exact(v,['epoch','token'])&&typeof v.epoch==='string'&&typeof v.token==='string';
const failures=JSON.parse(fs.readFileSync(new URL('./resource-read.json',import.meta.url))).failures;
export function nativeOperationReceiptFailureReasons(){return [...failures];}
export function nativeOperationReceiptValid(r,kind){
 const common=['kind','phase','stopped','quiescent','contested'];
 if(!kinds.includes(kind)||r?.kind!==kind||!['stopped','quiescent','contested'].every(k=>typeof r[k]==='boolean'))return false;
 if(kind.endsWith('load'))return exact(r,[...common,'decodeRefused','receivedChunks','receivedBytes','witnesses'])&&['ready','submitted'].includes(r.phase)&&typeof r.decodeRefused==='boolean'&&natural(r.receivedChunks)&&natural(r.receivedBytes)&&Array.isArray(r.witnesses)&&r.witnesses.length<=2&&r.witnesses.every(w=>w?.kind==='failure'?exact(w,['kind','reason'])&&failures.includes(w.reason):w?.kind==='end'&&exact(w,['kind','resourceVersion','coherence','chunks','byteLength'])&&w.resourceVersion!==null&&version(w.resourceVersion)&&['captured-read','atomic-resource-snapshot'].includes(w.coherence)&&natural(w.chunks)&&natural(w.byteLength));
 const k=r.knowledge;
 return exact(r,[...common,'safeDisposition','destinationFaulted','knowledge','requiredAcknowledgment','requestedStrengthSatisfied'])&&['queued','dispatched'].includes(r.phase)&&['safeDisposition','destinationFaulted','requestedStrengthSatisfied'].every(x=>typeof r[x]==='boolean')&&exact(k,['application','observedBytes','synchronization','resourceVersion'])&&['unknown','written','not-written'].includes(k.application)&&natural(k.observedBytes)&&['unknown','succeeded','failed','not-requested'].includes(k.synchronization)&&version(k.resourceVersion)&&['written','synchronized'].includes(r.requiredAcknowledgment)&&r.requestedStrengthSatisfied===(k.application==='written'&&(r.requiredAcknowledgment==='written'||k.synchronization==='succeeded')&&!r.contested&&!r.destinationFaulted);
}
// Fixed dispatch to actual private parent/task relations, never a supplied getter.
function state(kind,owner,task,c){switch(kind){case 'source-load':return nativeSourceLoadOperationState(owner,task,c);case 'annotation-load':return nativeAnnotationLoadOperationState(owner,task,c);case 'source-save':case 'annotation-save':return nativeOrdinarySaveOperationState(owner,task,c,kind);default:return null;}}
// Trusted decoder/admission bridges, not discovery advice or public authority.
export function nativeOperationObservationParent(kind,owner,task,c){return state(kind,owner,task,c)?.domain??null;}
export function nativeOperationObservationDecoderReady(h){return owners.get(h)?.live===true;}
export function nativeOperationObservationStopTargetOwned(h){return owners.has(h);}
export function nativeOperationObservationDecoderContextMatches(h,c){const e=owners.get(h);return !!e&&equal(e.context,c);}
const reference=()=>Object.freeze({kind:'native-operation-progress/1'});
const receiptGates=kind=>['operationReceiptAllowed','replyReady',kind.startsWith('annotation')?'annotationReadable':'sourceReadable',kind.endsWith('load')?'readAllowed':'dispositionReadable'];
export function nativeOperationObservationCharge(limits){const base=invocationObservationCharge(limits);if(base===null)return null;const n=base+2*limits.maxReferenceBytes+2*limits.maxContextBytes+3*limits.maxReplyBytes+limits.maxControlReplyBytes;return natural(n)&&n<=limits.maxHeldBytes?n:null;}
export function openNativeOperationObservation(kind,owner,task,c,ceiling,limits,g){
 if(!kinds.includes(kind)||!policy(ceiling)||size({context:c,ceiling,limits})>limits?.maxContextBytes)return null;
 const context=copy(c),captured=copy(ceiling),bounds=copy(limits),charge=nativeOperationObservationCharge(bounds);
 if(charge===null||!captured.grant||captured.publication||captured.recipient!==context.recipient)return null;
 const m=state(kind,owner,task,context);if(!m||!nativeOperationReceiptValid(m.receipt,kind)||!natural(m.maxReplyBytes)||size(m.receipt)>m.maxReplyBytes||bounds.maxReplyBytes<m.maxReplyBytes+512||bounds.maxReplyBytes<bounds.maxContextBytes||bounds.maxControlReplyBytes<256||bounds.maxReferenceBytes<size({reference:reference(),facet:'ordinary'}))return null;
 const h=Object.freeze({kind:'native-operation-observation/1'}),record={kind,owner,task,context,ceiling:captured,limits:bounds,domain:m.domain,live:true,busy:false,receiptFaulted:false,charge,issued:0,controlIssued:0,refs:new Map(),last:{ordinary:null,control:null},waiters:new Map()};
 if(size({kind,owner,task,context,ceiling:captured,limits:bounds,domain:m.domain,parentAttachment:{kind:'native-owner-attachment/1'},live:true,busy:false,receiptFaulted:false,charge:Number.MAX_SAFE_INTEGER,issued:Number.MAX_SAFE_INTEGER,controlIssued:4,last:{ordinary:null,control:null}})>bounds.maxContextBytes)return null;
 const attachment=beginInvocationOwnerAttachment(m.domain,'observation',bounds.maxHeldBytes,h);if(!attachment)return null;
 try{if(!gates(g,catalog.admissionGates)||captured.control&&g?.controlAllowed!==true||captured.effect&&(!gates(g,receiptGates(kind))||g?.operationReceiptAdmissionAllowed!==true))return null;const still=state(kind,owner,task,context);if(!still||still.domain!==m.domain)return null;record.parentAttachment=attachment;owners.set(h,record);if(!commitInvocationOwnerAttachment(m.domain,attachment,h)){owners.delete(h);throw Error('native observation attachment invariant');}return h;}finally{abortInvocationOwnerAttachment(m.domain,attachment);}
}
function close(e){e.live=false;e.refs.clear();e.last={ordinary:null,control:null};for(const w of e.waiters.values())w.phase='ended';}
function current(e,c,p,g,facet){
 if(!e?.live||!equal(e.context,c)||!policy(p)||p.recipient!==e.ceiling.recipient||p.epoch!==e.ceiling.epoch)return null;
 if(p.grant===false||['authorized','scope','continuous','live','retained'].some(k=>g?.[k]===false)){close(e);return null;}
 if(!p.grant||!gates(g,catalog.releaseGates)||!(facet==='control'?g?.controlReady===true:g?.ordinaryReady===true&&g?.resourcesReady===true))return null;
 const selected={};for(const k of flags)selected[k]=p[k]&&e.ceiling[k];
 const evidence=facet==='ordinary'&&selected.effect&&!e.receiptFaulted&&gates(g,receiptGates(e.kind));
 const m=state(e.kind,e.owner,e.task,e.context);if(!m||m.domain!==e.domain){close(e);return null;}
 if(!nativeOperationReceiptValid(m.receipt,e.kind)||size(m.receipt)>m.maxReplyBytes){e.receiptFaulted=true;return facet==='control'?{m,selected,evidence:false}:null;}
 return {m,selected,evidence};
}
function validRequest(e,h,r){
 if(!operations.includes(r?.operation))return false;
 const extra=r.operation==='retained-outcome'?['facet']:r.operation==='bounded-observation-wait'?['after','budget']:[];
 return exact(r,['contract','operation','task','context','profile',...extra])&&r.contract===(r.operation==='retained-outcome'?'native-operation-receipt/1':'native-operation-observation/1')&&r.task===e.task&&r.context===h&&r.profile===e.context.profile&&(r.operation!=='retained-outcome'||exact(r.facet,['kind'])&&r.facet.kind===e.kind);
}
function stage(e,operation,c,p,g){
 const facet=operation==='minimal-control-status'?'control':'ordinary',s=current(e,c,p,g,facet);if(!s)return null;
 const r=s.m.receipt,phase=r.stopped||r.quiescent?'terminal':'pending';let body;
 if(operation==='retained-outcome'){if(!s.evidence)return null;body={status:'native-operation-receipt',facet:{kind:e.kind},receipt:copy(r)};}
 else{if(!s.selected.control)return null;body={status:'native-execution',phase,stopped:r.stopped,quiescent:r.quiescent,...(operation==='status'&&s.evidence?{receipt:copy(r)}:{})};}
 const signature=JSON.stringify(facet==='control'?{phase,stopped:r.stopped,quiescent:r.quiescent}:{control:s.selected.control?{phase,stopped:r.stopped,quiescent:r.quiescent}:null,receipt:s.evidence?r:null}),last=e.last[facet],fresh=last?.signature!==signature;
 if(fresh&&(facet==='control'?e.controlIssued>=4:e.issued>=e.limits.maxIssued))return null;
 const token=fresh?reference():last.token,frame={...body,observation:token};
 if(size(frame)>(facet==='control'?e.limits.maxControlReplyBytes:e.limits.maxReplyBytes))return null;
 return {frame,facet,signature,token,fresh};
}
function install(e,s){if(s.fresh){e.refs.set(s.token,{facet:s.facet});e.last[s.facet]={signature:s.signature,token:s.token};if(s.facet==='control')e.controlIssued++;else e.issued++;}return s.frame;}
function capturedInput(e,r,c,p){const shadow={...r,task:null,context:null,...(Object.hasOwn(r,'after')?{after:null}:{})};if(!policy(p)||!exact(c,Object.keys(e.context))||!Object.values(c).every(v=>typeof v==='string')||size({request:shadow,context:c,policy:p})>e.limits.maxContextBytes)return null;return {request:{...r,...(r.facet?{facet:copy(r.facet)}:{})},context:copy(c),policy:copy(p)};}
export function queryNativeOperationObservation(h,r,c,p,g){
 const e=owners.get(h);if(!e||e.busy)return no();e.busy=true;
 try{if(!validRequest(e,h,r))return no();const i=capturedInput(e,r,c,p);if(!i||i.request.operation==='bounded-observation-wait')return no();const s=stage(e,i.request.operation,i.context,i.policy,g);return s?install(e,s):no();}catch{e.receiptFaulted=true;return no();}finally{e.busy=false;}
}
export function startNativeOperationWait(h,r,c,p,g){
 const e=owners.get(h);if(!e||e.busy)return no();e.busy=true;
 try{if(!validRequest(e,h,r))return no();const i=capturedInput(e,r,c,p);if(!i||i.request.operation!=='bounded-observation-wait')return no();r=i.request;
 if(e.refs.get(r.after)?.facet!=='ordinary'||!natural(r.budget)||r.budget<1||r.budget>e.limits.maxWaitSteps||!gates(g,['waitCapacity','waitClockValid','retentionReady'])||e.waiters.size>=e.limits.maxWaiters||!current(e,i.context,i.policy,g,'ordinary')?.selected.control)return no();
 const entry={after:r.after,remaining:r.budget,phase:'waiting'};if(size(entry)>e.limits.maxWaitBytes)return no();const wait=Object.freeze({kind:'native-operation-wait/1'});e.waiters.set(wait,entry);return {status:'waiting',wait};
 }catch{e.receiptFaulted=true;return no();}finally{e.busy=false;}
}
export function advanceNativeOperationWait(h,wait,event,c,p,g){
 const e=owners.get(h),w=e?.waiters.get(wait);if(!e||e.busy||w?.phase!=='waiting'||!['poll','tick','expire','stop'].includes(event))return no();e.busy=true;
 try{if(size({context:c,policy:p})>e.limits.maxContextBytes)return no();c=copy(c);p=copy(p);if(!equal(e.context,c))return no();
 const end=r=>{w.phase='ended';return r;};
 if(event==='stop'||event==='expire')return g?.ownerControlAllowed===true?end({status:'observation-ended'}):no();
 if(event==='tick'){if(g?.waitClockValid!==true)return end(no());if(w.remaining<=1)return end({status:'observation-ended'});w.remaining--;return {status:'waiting',wait};}
 const s=stage(e,'status',c,p,g);if(!s)return end(no());if(s.token===w.after&&s.frame.phase!=='terminal')return {status:'waiting',wait};
 const result={status:'observed',result:s.frame};if(size(result)>e.limits.maxReplyBytes)return end(no());install(e,s);return end(result);
 }catch{e.receiptFaulted=true;w.phase='ended';return no();}finally{e.busy=false;}
}
export function closeNativeOperationObservation(h,c,g){const e=owners.get(h);if(!e||e.busy||!equal(e.context,c)||g?.ownerControlAllowed!==true)return no();close(e);return {status:'stopped'};}
export function retireNativeOperationWait(h,wait,g){const e=owners.get(h);if(!e||e.busy||!e.waiters.has(wait)||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return no();e.waiters.delete(wait);return {status:'retired'};}
export function retireNativeOperationObservation(h,g){const e=owners.get(h);if(!e||e.busy||e.waiters.size||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])||!releaseInvocationOwnerAttachment(e.domain,e.parentAttachment,g))return no();owners.delete(h);return {status:'retired'};}
export function nativeOperationObservationUsage(h){const e=owners.get(h);return e?{live:e.live,receiptFaulted:e.receiptFaulted,ordinaryIssued:e.issued,controlIssued:e.controlIssued,references:e.refs.size,waiters:e.waiters.size,remaining:[...e.waiters.values()].map(w=>w.remaining),declaredHeldBytes:e.charge}:null;}
export function invokeNativeOperationObservation(h,envelope,route,c,p,g){
 const identity=route==='typed'?'family':route==='dynamic'?'name':null;if(!identity||!exact(envelope,[identity,'variant','version','arguments']))return no();
 const d=(route==='typed'?decodeTyped:decodeDynamic)(families,Object.fromEntries([identity,'variant','version'].map(k=>[k,envelope[k]])));
 if(d?.descriptor.id!=='F06'||!operations.includes(d.variant)||envelope.arguments?.operation!==d.variant||planRoute(d,{schemaValid:true,enabled:true,grant:true,current:true,ordinaryCapacity:true,protectedCapacity:true,releaseAllowed:true}).sink!=='read')return no();
 return d.variant==='bounded-observation-wait'?startNativeOperationWait(h,envelope.arguments,c,p,g):queryNativeOperationObservation(h,envelope.arguments,c,p,g);
}
