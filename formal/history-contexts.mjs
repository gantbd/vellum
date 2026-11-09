// Trusted serialized native reference. Opaque object handles are not a wire encoding.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {beginInvocationAdmissionMutation,endInvocationAdmissionMutation,invocationAuthorityRoot,invocationAuthorityContext,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment} from './invocation-publication.mjs';
import {captureHistory,readHistory,historyQueryDirections} from './history-queries.mjs';
import {historyQueryResultValid,historyQueryResultAliasesValid} from './history-query-results.mjs';
import {normalizeHistoryQueryArguments,historyQueryProducerRequest,captureHistoryQueryArgumentLimits,historyQueryArgumentClassificationBytes} from './history-query-arguments.mjs';
import {copySettingsData} from './semantic-settings.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./history-contexts.json',import.meta.url))),owners=new WeakMap(),copy=structuredClone;
const queries=JSON.parse(fs.readFileSync(new URL('./history-queries.json',import.meta.url)));
const size=v=>Buffer.byteLength(JSON.stringify(v)),natural=n=>Number.isSafeInteger(n)&&n>=0,label=s=>typeof s==='string'&&s.length>0&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Buffer.from(s))===s;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const gates=(g,keys)=>keys.every(k=>g?.[k]===true),unavailable=()=>({status:'unavailable'});
const contextValid=c=>exact(c,catalog.contextFields)&&Object.values(c).every(label);
function rootFor(o){const c=invocationAuthorityContext(o.domain),root=invocationAuthorityRoot(o.domain);return c&&c.authority===o.context.authority&&c.engine===o.context.engine&&root?.authority===c.authority?root:null;}
function validProjection(p,state,l){
 if(!exact(p,['details','scopes','groups'])||typeof p.details!=='boolean'||!Array.isArray(p.scopes)||!Array.isArray(p.groups)||p.scopes.length>l.maxScopes||p.groups.length>l.maxGroups)return false;
 if(!p.scopes.every(s=>exact(s,['internal','reference','complete'])&&label(s.internal)&&label(s.reference)&&exact(s.complete,['undo','redo'])&&Object.values(s.complete).every(b=>typeof b==='boolean'))||
    !p.groups.every(g=>exact(g,['internal','reference','evidenceReady'])&&label(g.internal)&&label(g.reference)&&typeof g.evidenceReady==='boolean'&&state.groups.filter(x=>x.id===g.internal).length===1))return false;
 for(const rows of [p.scopes,p.groups])if(new Set(rows.map(r=>r.internal)).size!==rows.length||new Set(rows.map(r=>r.reference)).size!==rows.length)return false;
 return true;
}
// Relevant complete participant frontiers, not the global publication/root identity.
function historyBasis(state,p,l){
 if(!state||state.groups.length>l.maxGroups||Object.keys(state.frontiers).length>l.maxScopes||state.groups.some(g=>g.scopes.length>l.maxParticipants))return null;
 const groups=[];
 for(const selected of p.groups){const found=state.groups.filter(g=>g.id===selected.internal);if(found.length!==1)return null;const g=found[0];groups.push({id:g.id,valid:g.valid,scopes:g.scopes.toSorted()});}
 groups.sort((a,b)=>a.id<b.id?-1:a.id>b.id?1:0);
 const scopes=[...new Set([...p.scopes.map(s=>s.internal),...groups.flatMap(g=>g.scopes)])].sort();if(scopes.length>l.maxScopes)return null;
 const basis={groups,frontiers:scopes.map(scope=>({scope,present:Object.hasOwn(state.frontiers,scope),value:copy(state.frontiers[scope]??null)}))};
 return size(basis)<=l.maxBasisBytes?freeze(basis):null;
}
export function openHistoryInspection(domain,context,limits,g){
 const attachment=beginInvocationOwnerAttachment(domain,'history',limits?.maxHeldBytes,Object.freeze({kind:'history-inspection-owner/1'}));if(!attachment)return null;
 try{
 if(!contextValid(context)||!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||!gates(g,catalog.acquisitionGates))return null;
 const o={domain,context:freeze(copy(context)),limits:freeze(copy(limits)),issued:0,entries:new Map(),stopped:false,faulted:false};if(!rootFor(o))return null;
 const charge=(limits.maxContexts+1)*(limits.maxCaptureBytes+limits.maxBasisBytes)+2*limits.maxReplyBytes+size(context);
 if(!natural(charge)||charge>limits.maxHeldBytes||size(unavailable())>limits.maxReplyBytes)return null;o.charge=charge;
 const owner=Object.freeze({kind:'history-inspection-owner/1'});o.parentAttachment=attachment;owners.set(owner,o);if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native history attachment invariant');return owner;
 }finally{abortInvocationOwnerAttachment(domain,attachment);}
}
export function acquireHistoryInspection(owner,projection,c,g){
 const admissionDomain=owners.get(owner)?.domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return unavailable();try{
 const o=owners.get(owner);if(!o||o.stopped||o.faulted||!equal(o.context,c)||!gates(g,catalog.acquisitionGates)||o.entries.size>=o.limits.maxContexts||o.issued>=o.limits.maxIssued)return unavailable();
 const root=rootFor(o);if(!root||!validProjection(projection,root,o.limits))return unavailable();
 const basis=historyBasis(root,projection,o.limits);if(!basis)return unavailable();
 const binding={context:'owned-history-private',...copy(c),...copy(projection)},capture=captureHistory(root,binding,o.limits);if(!capture)return unavailable();
 const context=Object.freeze({kind:'history-context/1'}),result={status:'history-context',context};if(size(result)>o.limits.maxReplyBytes)return unavailable();
 o.entries.set(context,{capture,basis,live:true});o.issued++;return result;

 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
function currentEntry(o,token,c,g){
 const e=o?.entries.get(token);if(!o||o.stopped||o.faulted||!e?.live||!equal(o.context,c))return null;if(['granted','retained','live','continuous','historyCurrent'].some(k=>g?.[k]===false)){e.live=false;return null;}if(!gates(g,catalog.releaseGates))return null;
 const basis=historyBasis(rootFor(o),e.capture.binding,o.limits);if(!basis||!equal(basis,e.basis)){e.live=false;return null;}return e;
}
export function queryHistoryInspection(owner,request,c,g){
 const o=owners.get(owner);if(!o||!request||!Object.hasOwn(queries.variants,request.operation)||!exact(request,[...queries.common,...queries.variants[request.operation]])||request.contract!==queries.contract||request.profile!==o.context.profile||!historyQueryDirections.includes(request.direction))return unavailable();const e=currentEntry(o,request.context,c,g);if(!e)return unavailable();
 try{
  const privateRequest={...request,context:e.capture.binding.context},out=readHistory(e.capture,privateRequest,{binding:e.capture.binding,granted:true,retained:true,live:true,continuous:true,transferReady:true,historyCurrent:true});
  if(!historyQueryResultValid(out,privateRequest,o.limits.maxDetails)||!historyQueryResultAliasesValid(out,e.capture.binding))throw Error('native history result invariant');
  if(out.status==='unavailable')return out;const result={...out,context:request.context};
  if(!historyQueryResultValid(result,request,o.limits.maxDetails)||!historyQueryResultAliasesValid(result,e.capture.binding))throw Error('native history rebound result invariant');
  if(!(size(result)<=o.limits.maxReplyBytes))return unavailable();
  const copied=copyResult(result);if(!historyQueryResultValid(copied,request,o.limits.maxDetails)||!historyQueryResultAliasesValid(copied,e.capture.binding)||!equal(copied,result))throw Error('native history copied result invariant');
  return copied;
 }catch{o.faulted=true;return unavailable();}
}
// The actual history owner parents decoding workspace within its declared child
// ceiling, now reserved at the invocation domain with an attachment frame.
// Neither logical reservation is a physical-memory measurement.
export function queryDecodedHistoryInspection(owner,envelope,inputLimits,c,g){
 const o=owners.get(owner);if(!o||o.stopped||o.faulted||o.historyDecoding||historyQueryArgumentClassificationBytes>o.limits.maxHeldBytes-o.charge)return unavailable();
 let charged=historyQueryArgumentClassificationBytes;o.historyDecoding=true;o.historyDecodeCharge=charged;o.charge+=charged;
 try{
  const p=captureHistoryQueryArgumentLimits(inputLimits);if(!p)return unavailable();
  // Seven logical frames follow the existing F02/F03 decoder pattern: envelope,
  // authority, canonical capture, limits, descriptor, facts and producer request.
  const maximum=p.maxRequestBytes,workspace=7*maximum;if(!natural(workspace)||workspace>o.limits.maxHeldBytes-o.charge)return unavailable();
  charged+=workspace;o.historyDecodeCharge=charged;o.charge+=workspace;
  const n=normalizeHistoryQueryArguments(envelope,c,p,maximum);if(!n||!equal(n.record.context,o.context))return unavailable();
  const packet=copySettingsData({facts:g},{...p,maxRequestBytes:maximum});if(!packet)return unavailable();
  return queryHistoryInspection(owner,historyQueryProducerRequest(n),n.record.context,packet.value.facts);
 }catch{o.faulted=true;return unavailable();}finally{o.charge-=charged;o.historyDecodeCharge=0;o.historyDecoding=false;}
}
export function historyInspectionDecodeUsage(owner){const o=owners.get(owner);return o?{busy:o.historyDecoding===true,declaredWorkspaceBytes:o.historyDecodeCharge??0}:null;}
// Preserve the issued native handle identity while copying every disclosed value.
function copyResult(result){const {context,...rest}=result;return {...copy(rest),context};}
// Trusted preparation bridge only. These private internals are never an F04 product.
export function historyInspectionReversalBindings(owner,context,c,g){
 const o=owners.get(owner),e=currentEntry(o,context,c,g);if(!e)return null;
 return {groups:e.capture.binding.groups.map(r=>({reference:r.reference,internal:r.internal,authority:c.authority})),scopes:e.capture.binding.scopes.map(r=>({reference:r.reference,internal:r.internal,authority:c.authority}))};
}
export function historyInspectionClaimMatches(owner,c){const o=owners.get(owner);return Boolean(o&&equal(o.context,c));}
export function historyInspectionStopTargetOwned(owner,reference){return owners.get(owner)?.entries.has(reference)===true;}
export function closeHistoryInspection(owner,context,recipient,g){const o=owners.get(owner),e=o?.entries.get(context);if(!e||recipient!==o.context.recipient||g?.ownerControlAllowed!==true)return unavailable();e.live=false;return {status:'stopped'};}
export function retireHistoryInspection(owner,context,g){const o=owners.get(owner);if(!o?.entries.has(context)||g?.ownerControlAllowed!==true||g.retentionReleased!==true||g.deliveryDrained!==true)return unavailable();o.entries.delete(context);return {status:'retired'};}
export function stopHistoryInspection(owner,recipient,g){const o=owners.get(owner);if(!o||recipient!==o.context.recipient||g?.ownerControlAllowed!==true)return unavailable();o.stopped=true;for(const e of o.entries.values())e.live=false;return {status:'stopped'};}
export function historyInspectionUsage(owner){const o=owners.get(owner);return o?{issued:o.issued,contexts:o.entries.size,live:[...o.entries.values()].filter(e=>e.live).length,stopped:o.stopped,faulted:o.faulted,declaredHeldBytes:o.charge}:null;}
