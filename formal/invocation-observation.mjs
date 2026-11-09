// Serialized local observation, never execution. Actual parent funds the child
// ceiling and attachment frame; host clocks and complete factory coverage remain separate.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {invocationExecutionState,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment,releaseInvocationOwnerAttachment,observeAnnotationResult,observeReversalResult,observeRelationResult,observeLifecycleResult,observeRegistrationResult,observeScopedRegistrationResult,observeSettingsCreatedResult,observeAtomicAnnotationResult} from './invocation-publication.mjs';
import {releaseOutcome} from './outcome-projection.mjs';
import {invocationObservationResultValid} from './invocation-observation-results.mjs';
import {projectedEditResultValid} from './projected-edit-results.mjs';
import {nativeLifecycleResultValid} from './lifecycle-results.mjs';
import {registrationResultValueValid} from './registration-result-values.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./invocation-observation.json',import.meta.url))),products=JSON.parse(fs.readFileSync(new URL('./execution-observation.json',import.meta.url))),owners=new WeakMap(),copy=structuredClone;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort()),natural=n=>Number.isSafeInteger(n)&&n>=0,size=v=>Buffer.byteLength(JSON.stringify(v));
const flags=['publication','control','effect','reasons','details','recovery'],policyFields=Object.freeze([...flags,'grant','epoch','recipient']),policy=p=>exact(p,policyFields)&&[...flags,'grant'].every(k=>typeof p[k]==='boolean')&&natural(p.epoch)&&typeof p.recipient==='string';
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;},gates=(g,ks)=>ks.every(k=>g?.[k]===true),unavailable=()=>({status:'unavailable'});
const observation=()=>Object.freeze({kind:'execution-observation/1'});
const inputForms=freeze(Object.entries(products.variants).map(([operation,extra])=>({family:products.family,contract:products.contract,variant:operation,operation,fields:[...products.common,...extra]})));
export function invocationObservationInputForms(){return inputForms;}
const receiptKinds=Object.freeze(['annotation','atomic-annotation','reversal','relation','lifecycle','host-registration','scoped-registration','created-settings']);
const receiptInputForm=freeze({family:'F06',variant:'retained-outcome',contract:'execution-receipt/1',operation:'retained-outcome',fields:['contract','operation','invocation','context','profile','facet']});
export function invocationReceiptInputForm(){return receiptInputForm;}
export function invocationReceiptKinds(){return receiptKinds;}
export function validInvocationReceiptSelector(facet){return Boolean(facet&&receiptKinds.includes(facet.kind)&&exact(facet,facet.kind==='atomic-annotation'?['kind','index']:['kind'])&&(facet.kind!=='atomic-annotation'||natural(facet.index)));}
const receiptOverhead=Math.max(...receiptKinds.map(kind=>size({status:'operation-receipt',facet:{kind,...(kind==='atomic-annotation'?{index:Number.MAX_SAFE_INTEGER}:{})},receipt:null})-4));
// Trusted first-party decoder bridge, not recipient inspection or new F06 verbs.
export function invocationObservationDecoderDeclarations(){return freeze({policyFields:[...policyFields],releaseGates:[...catalog.releaseGates]});}
export function invocationObservationDecoderReady(context){return owners.get(context)?.live===true;}
export function invocationObservationStopTargetOwned(context){return owners.has(context);}
export function invocationObservationDecoderContextMatches(context,c){const e=owners.get(context);return Boolean(e?.live&&equal(e.context,c));}
export function faultInvocationObservationDecoder(context){return resultFault(owners.get(context));}
export function invocationObservationCharge(limits){if(!exact(limits,catalog.limits)||!Object.values(limits).every(natural))return null;const charge=2*limits.maxReplyBytes+limits.maxControlReplyBytes+limits.maxContextBytes+(limits.maxWaiters+1)*limits.maxWaitBytes+(limits.maxIssued+2)*limits.maxReferenceBytes;return natural(charge)&&charge<=limits.maxHeldBytes?charge:null;}
export function openInvocationObservation(domain,ticket,c,ceiling,limits,g,receiptCeiling=null){
 const attachment=beginInvocationOwnerAttachment(domain,'observation',limits?.maxHeldBytes,Object.freeze({kind:'invocation-observation-context/1'}));if(!attachment)return null;
 try{
 const m=invocationExecutionState(domain,ticket,c,false);if(!m||!policy(ceiling)||!ceiling.grant||ceiling.recipient!==c.recipient||ceiling.epoch!==m.policy.epoch||!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||!gates(g,catalog.admissionGates)||size({context:c,ceiling,reference:m.reference})>limits.maxContextBytes)return null;
 const full={status:'observed',result:{status:'execution',phase:'terminal',observation:observation(),outcome:null}},minimal={status:'execution',phase:'terminal',observation:observation()};
 const charge=invocationObservationCharge(limits);
 // Opt-in ceiling is separately authorized; ordinary details permission is not
 // receipt permission. Its complete record fits the existing context envelope.
 let receipt=null;
 if(receiptCeiling!==null){
  if(!exact(receiptCeiling,receiptKinds)||!receiptKinds.every(k=>typeof receiptCeiling[k]==='boolean')||g.operationReceiptAdmissionAllowed!==true||size({context:c,ceiling,reference:m.reference,receipt:{ceiling:receiptCeiling,busy:false,faulted:false}})>limits.maxContextBytes||!natural(m.maxResultBytes+receiptOverhead)||limits.maxReplyBytes<m.maxResultBytes+receiptOverhead)return null;
  receipt={ceiling:freeze(copy(receiptCeiling)),busy:false,faulted:false};
 }
 if(limits.maxReplyBytes<m.maxResultBytes+size(full)||limits.maxControlReplyBytes<size(minimal)||limits.maxReferenceBytes<size({reference:observation(),facet:'ordinary'})||charge===null)return null;
 const context=Object.freeze({kind:'invocation-observation-context/1'});owners.set(context,{domain,ticket,parentAttachment:attachment,context:freeze(copy(c)),ceiling:freeze(copy(ceiling)),limits:freeze(copy(limits)),reference:m.reference,issued:0,controlIssued:0,refs:new Map(),last:{ordinary:null,control:null},waiters:new Map(),live:true,charge,...(receipt?{receipt}:{})});if(!commitInvocationOwnerAttachment(domain,attachment,context))throw Error('native observation attachment invariant');return context;
 }finally{abortInvocationOwnerAttachment(domain,attachment);}
}
function closeContext(e){e.live=false;e.refs.clear();e.last={ordinary:null,control:null};for(const w of e.waiters.values())w.phase='ended';}
function resultFault(e){if(e){e.faulted=true;closeContext(e);}return unavailable();}
function current(e,c,p,g,facet){
 if(!e?.live||!equal(e.context,c)||!policy(p)||!e.ceiling.grant||p.recipient!==e.ceiling.recipient||p.epoch!==e.ceiling.epoch)return null;
 if(p.grant===false||['authorized','scope','continuous','live','retained'].some(k=>g?.[k]===false)){closeContext(e);return null;}
 if(!gates(g,catalog.releaseGates)||!(facet==='control'?g?.controlReady===true:g?.ordinaryReady===true&&g?.resourcesReady===true))return null;
 const m=invocationExecutionState(e.domain,e.ticket,e.context,facet==='ordinary');if(!m||m.reference!==e.reference||m.policy.epoch!==p.epoch||m.policy.recipient!==p.recipient){closeContext(e);return null;}
 const selected={...p};for(const flag of flags)selected[flag]=p[flag]&&e.ceiling[flag];return {m,selected,control:selected.control&&m.policy.control};
}
function requestValid(e,context,r){const form=r&&inputForms.find(f=>f.operation===r.operation);return form&&exact(r,form.fields)&&r.contract===form.contract&&r.context===context&&r.invocation===e.reference&&r.profile===e.context.profile;}
function stage(e,operation,c,p,g){
 const facet=operation==='minimal-control-status'?'control':'ordinary',s=current(e,c,p,g,facet);if(!s)return null;
 const outcome=facet==='ordinary'&&s.m.phase==='terminal'&&s.m.prepared?releaseOutcome(s.m.prepared,s.selected):null;
 let body;if(operation==='retained-outcome'){if(!outcome||outcome.observation!=='available')return null;body={status:'outcome',outcome};}
 else{if(!s.control)return null;body={status:'execution',phase:s.m.phase,...(operation==='status'&&outcome?.observation==='available'?{outcome}:{})};}
 const signature=JSON.stringify(facet==='control'?{phase:s.m.phase}:{phase:s.control?s.m.phase:null,outcome:outcome?.observation==='available'?outcome:null}),last=e.last[facet],fresh=last?.signature!==signature;
 if(fresh&&(facet==='control'?e.controlIssued>=2:e.issued>=e.limits.maxIssued))return null;const token=fresh?observation():last.token,frame={...copy(body),observation:token};
 if(!invocationObservationResultValid(frame,operation,token))throw Error('native observation result invariant');
 if(size(frame)>(facet==='control'?e.limits.maxControlReplyBytes:e.limits.maxReplyBytes))return null;return {frame,facet,signature,token,fresh};
}
function install(e,s){if(s.fresh){e.refs.set(s.token,{facet:s.facet});e.last[s.facet]={signature:s.signature,token:s.token};if(s.facet==='control')e.controlIssued++;else e.issued++;}return s.frame;}
export function queryInvocationObservation(context,request,c,p,g){const e=owners.get(context);try{if(!e||e.receipt?.busy||!requestValid(e,context,request)||request.operation==='bounded-observation-wait')return unavailable();const s=stage(e,request.operation,c,p,g);return s?install(e,s):unavailable();}catch{return resultFault(e);}}
// A typed facet of retained outcome, never command replay or receipt issuance.
// Native release retains command-specific ceilings, grants and opaque roles.
export function queryInvocationReceipt(context,request,c,p,g){
 const e=owners.get(context),r=e?.receipt;if(!r||r.busy||r.faulted)return unavailable();r.busy=true;
 try{
  if(!exact(request,receiptInputForm.fields)||request.contract!==receiptInputForm.contract||request.operation!==receiptInputForm.operation||request.context!==context||request.invocation!==e.reference||request.profile!==e.context.profile||!validInvocationReceiptSelector(request.facet)||r.ceiling[request.facet.kind]!==true||g?.operationReceiptAllowed!==true)return unavailable();
  const s=current(e,c,p,g,'ordinary');if(!s||s.m.phase!=='terminal'||!s.m.prepared)return unavailable();
  const kind=request.facet.kind;let value,valid;
  switch(kind){
   case 'annotation':value=observeAnnotationResult(e.domain,e.ticket,c,s.selected,g);valid=projectedEditResultValid(value,'F13');break;
   case 'atomic-annotation':value=observeAtomicAnnotationResult(e.domain,e.ticket,request.facet.index,c,s.selected,g);valid=projectedEditResultValid(value,'F13');break;
   case 'reversal':value=observeReversalResult(e.domain,e.ticket,c,s.selected,g);valid=projectedEditResultValid(value,'F14');break;
   case 'relation':value=observeRelationResult(e.domain,e.ticket,c,s.selected,g);valid=nativeLifecycleResultValid(value,'relation');break;
   case 'lifecycle':value=observeLifecycleResult(e.domain,e.ticket,c,s.selected,g);valid=nativeLifecycleResultValid(value);break;
   case 'host-registration':value=observeRegistrationResult(e.domain,e.ticket,c,s.selected,g);valid=registrationResultValueValid(value,kind);break;
   case 'scoped-registration':value=observeScopedRegistrationResult(e.domain,e.ticket,c,s.selected,g);valid=registrationResultValueValid(value,kind);break;
   case 'created-settings':value=observeSettingsCreatedResult(e.domain,e.ticket,c,s.selected,g);valid=registrationResultValueValid(value,kind);break;
   default:return unavailable();
  }
  if(!valid||value.status!=='unavailable'&&value.result.invocation!==e.reference)throw Error('native receipt result invariant');
  if(value.status==='unavailable'||!e.live||owners.get(context)!==e||!invocationExecutionState(e.domain,e.ticket,e.context,false))return unavailable();
  // Native result <= the original parent's maxResultBytes; the complete fixed
  // wrapper was prepaid at open. Never serialize or clone opaque receipt roles.
  return {status:'operation-receipt',facet:copy(request.facet),receipt:value};
 }catch{r.faulted=true;return unavailable();}finally{r.busy=false;}
}
export function startInvocationWait(context,request,c,p,g){
 const e=owners.get(context);try{if(!e||e.receipt?.busy||!requestValid(e,context,request)||request.operation!=='bounded-observation-wait'||e.refs.get(request.after)?.facet!=='ordinary'||!Number.isSafeInteger(request.budget)||request.budget<1||request.budget>e.limits.maxWaitSteps||!current(e,c,p,g,'ordinary')?.control||g?.waitCapacity!==true||g?.waitClockValid!==true||g?.retentionReady!==true||e.waiters.size>=e.limits.maxWaiters)return unavailable();
 const entry={after:request.after,remaining:request.budget,phase:'waiting'};if(size(entry)>e.limits.maxWaitBytes)return unavailable();const wait=Object.freeze({kind:'invocation-observation-wait/1'}),result={status:'waiting',wait};if(!invocationObservationResultValid(result,'bounded-observation-wait',null,wait))throw Error('native observation wait invariant');e.waiters.set(wait,entry);return result;
 }catch{return resultFault(e);}
}
export function advanceInvocationWait(context,wait,event,c,p,g){
 const e=owners.get(context),w=e?.waiters.get(wait);try{if(e?.receipt?.busy||!w||w.phase!=='waiting'||!equal(e.context,c)||!['poll','tick','expire','stop'].includes(event))return unavailable();
 const end=result=>{if(!invocationObservationResultValid(result,'bounded-observation-wait',result.result?.observation,wait))throw Error('native observation completion invariant');w.phase='ended';return result;};
 const waiting=()=>{const result={status:'waiting',wait};if(!invocationObservationResultValid(result,'bounded-observation-wait',null,wait))throw Error('native observation continuing wait invariant');return result;};
 if(event==='stop'||event==='expire')return equal(e.context,c)&&g?.ownerControlAllowed===true?end({status:'observation-ended'}):unavailable();
 if(event==='tick'){if(g?.waitClockValid!==true)return end(unavailable());if(w.remaining<=1)return end({status:'observation-ended'});w.remaining--;return waiting();}
 const s=stage(e,'status',c,p,g);if(!s)return end(unavailable());if(s.token===w.after&&s.frame.phase!=='terminal')return waiting();
 const result={status:'observed',result:s.frame};if(size(result)>e.limits.maxReplyBytes)return end(unavailable());if(!invocationObservationResultValid(result,'bounded-observation-wait',s.token,wait))throw Error('native observation delivery invariant');install(e,s);return end(result);
 }catch{return resultFault(e);}
}
export function closeInvocationObservation(context,c,g){const e=owners.get(context);if(!e||!equal(e.context,c)||g?.ownerControlAllowed!==true)return unavailable();closeContext(e);return {status:'stopped'};}
export function retireInvocationWait(context,wait,g){const e=owners.get(context);if(!e?.waiters.has(wait)||g?.ownerControlAllowed!==true||g?.retentionReleased!==true||g?.deliveryDrained!==true)return unavailable();e.waiters.delete(wait);return {status:'retired'};}
export function retireInvocationObservation(context,g){const e=owners.get(context);if(!e||e.receipt?.busy||g?.ownerControlAllowed!==true||g?.retentionReleased!==true||g?.deliveryDrained!==true||e.waiters.size)return unavailable();if(!releaseInvocationOwnerAttachment(e.domain,e.parentAttachment,g))return unavailable();owners.delete(context);return {status:'retired'};}
export function invocationObservationUsage(context){const e=owners.get(context);return e?{live:e.live,issued:e.issued+e.controlIssued,ordinaryIssued:e.issued,controlIssued:e.controlIssued,references:e.refs.size,waiters:e.waiters.size,waiting:[...e.waiters.values()].filter(w=>w.phase==='waiting').length,remaining:[...e.waiters.values()].map(w=>w.remaining),declaredHeldBytes:e.charge}:null;}
