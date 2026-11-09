// Owned F07 metadata reads, not F16 activation or a public stamp/token codec.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {beginInvocationAdmissionMutation,endInvocationAdmissionMutation,invocationAuthorityRoot,invocationAuthorityContext,invocationUsage,reserveInvocationRegistrationReadCharge,releaseInvocationRegistrationReadCharge,beginInvocationRegistrationReadDecode,extendInvocationRegistrationReadDecode,finishInvocationRegistrationReadDecode,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment,releaseInvocationOwnerAttachment} from './invocation-publication.mjs';
import {captureRegistrationQueryArgumentLimits,normalizeRegistrationQueryArguments,registrationQueryProducerRequest,registrationQueryArgumentClassificationBytes} from './registration-query-arguments.mjs';
import {copySettingsData} from './semantic-settings.mjs';
import {nativeResourceCatalogValid} from './resource-registration-state.mjs';
import {registrationQueryResultDeclarations,registrationQueryResultValid,registrationQueryPageBindingsValid} from './registration-query-results.mjs';
import {registrationAcquisitionResultValid} from './registration-acquisition-results.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./registration-reads.json',import.meta.url))),owners=new WeakMap(),domains=new WeakMap(),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v)),natural=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort()),gates=(g,ks)=>ks.every(k=>g?.[k]===true),no=()=>({status:'unavailable'}),freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const inputForm=freeze({family:catalog.family,contract:catalog.contract,operation:catalog.operation,variant:catalog.operation,profile:catalog.profile,fields:catalog.fields,requests:catalog.requests});
export function registrationReadInputForm(){return inputForm;}
const label=(s,max)=>typeof s==='string'&&s.length>0&&Buffer.byteLength(s)<=max&&Buffer.from(s).toString('utf8')===s;
const identity=(v,max)=>exact(v,['authority','id','incarnation'])&&label(v.authority,max)&&label(v.id,max)&&natural(v.incarnation);
const token=kind=>Object.freeze({kind}),key=r=>JSON.stringify([r.table,r.provider.authority,r.provider.id,r.provider.incarnation,r.resource.authority,r.resource.id,r.resource.incarnation]);
const rows=root=>[...root.resourceRegistrations.map(row=>({table:'read',row})),...root.writeRegistrations.map(row=>({table:'write',row}))];
const healthy=m=>m.acquisitionFaulted!==true&&invocationUsage(m.domain)?.faulted===false;
function scopeValid(m,c){return exact(c,catalog.contextFields)&&Object.values(c).every(v=>label(v,m.limits.maxContextBytes))&&c.profile===catalog.profile&&size(c)<=m.limits.maxContextBytes&&c.authority===m.authority&&c.engine===m.engine;}
function descriptor(table,row){const d=registrationQueryResultDeclarations(),fields=table==='write'?d.writeFields:d.readFields[row.profile];return Object.fromEntries(fields.map(k=>[k,copy(row[k])]));}
export function registrationReadCharge(l){if(!exact(l,catalog.limits)||!Object.values(l).every(natural))return null;const charge=l.maxContextBytes+l.maxPreparations*l.maxPacketBytes+l.maxContexts*(l.maxContextBytes+2*l.maxCaptureBytes+2*l.maxReplyBytes)+2*l.maxSelectors*l.maxSelectorBytes;return natural(charge)&&charge<=l.maxHeldBytes?charge:null;}
export function openRegistrationReadRegistry(domain,limits,g){
 const attachment=beginInvocationOwnerAttachment(domain,'registration-reads',0,token('registration-read-registry/1'));if(!attachment)return null;
 try{const c=invocationAuthorityContext(domain),charge=registrationReadCharge(limits);if(!c||invocationUsage(domain)?.faulted!==false||domains.has(domain)||charge===null||!gates(g,['ownerControlAllowed','resourcesReady','preparationReserved']))return null;
 const owner=token('registration-read-registry/1'),state={domain,authority:c.authority,engine:c.engine,limits:freeze(copy(limits)),charge,contexts:new Map(),packets:new Map(),selectors:new Map(),issued:0,acquisitionFaulted:false};
 const parentLease=reserveInvocationRegistrationReadCharge(domain,charge,g);if(!parentLease)return null;
 state.parentLease=parentLease;state.parentAttachment=attachment;owners.set(owner,state);domains.set(domain,owner);if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native registration read attachment invariant');return owner;
 }finally{abortInvocationOwnerAttachment(domain,attachment);}
}
export function prepareRegistrationVisibility(owner,c,facts,nativeProfile,g){const m=owners.get(owner),root=m&&invocationAuthorityRoot(m.domain);if(!m||!healthy(m)||m.decoderFaulted===true||m.decoding===true||!scopeValid(m,c)||!gates(g,catalog.preparationGates)||m.packets.size>=m.limits.maxPreparations||!nativeResourceCatalogValid(root,nativeProfile)||size(root)>m.limits.maxRootBytes||!Array.isArray(facts))return null;const all=rows(root);if(all.length>m.limits.maxRows||facts.length!==all.length||size({root,scope:c,facts})>m.limits.maxPacketBytes||facts.some(f=>!exact(f,['table','provider','resource','visible','descriptor','order'])||!identity(f.provider,m.limits.maxContextBytes)||!identity(f.resource,m.limits.maxContextBytes)||typeof f.visible!=='boolean'||typeof f.descriptor!=='boolean'||!f.visible&&(f.descriptor||f.order!==null)||f.visible&&!natural(f.order)))return null;const keys=facts.map(key),actual=all.map(({table,row})=>key({table,...row}));if(new Set(keys).size!==facts.length||!equal([...keys].sort(),[...actual].sort()))return null;const order=facts.filter(f=>f.visible).map(f=>f.order).sort((a,b)=>a-b);if(!equal(order,order.map((_,i)=>i)))return null;const packet=token('registration-visibility/1');m.packets.set(packet,{root,scope:freeze(copy(c)),facts:freeze(copy(facts))});return packet;}
function takePacket(m,packet,c){const p=m.packets.get(packet);if(!p||p.phase==='failed-acquisition'||!healthy(m)||p.root!==invocationAuthorityRoot(m.domain)||!equal(p.scope,c))return null;m.packets.delete(packet);return p;}
function project(p,ceiling=null){const all=rows(p.root),facts=new Map(p.facts.map(f=>[key(f),f])),selected=ceiling??p.facts.filter(f=>f.visible).sort((a,b)=>a.order-b.order).map(f=>({key:key(f),descriptor:f.descriptor}));const out=[];for(const entry of selected){const f=facts.get(entry.key),actual=all.find(({table,row})=>key({table,...row})===entry.key);if(!f?.visible||!actual)return null;const value={table:actual.table,...(entry.descriptor&&f.descriptor?{descriptor:descriptor(actual.table,actual.row)}:{})};out.push({key:entry.key,descriptor:entry.descriptor,value});}return out;}
function closeContext(m,context,ctx,invalidated=false){ctx.stopped=true;ctx.rows=[];if(invalidated)ctx.invalidated=true;ctx.entries.clear();ctx.continuations.clear();for(const [ref,s]of m.selectors)if(s.context===context)m.selectors.delete(ref);}
const captureFits=(m,rs)=>rs!==null&&rs.length<=m.limits.maxRows&&rs.every(r=>size(r.value)<=m.limits.maxRowBytes)&&size(rs)<=m.limits.maxCaptureBytes;
export function acquireRegistrationRead(owner,c,packet,g){
 const admissionDomain=owners.get(owner)?.domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return no();try{const m=owners.get(owner);if(!m||!healthy(m)||m.decoderFaulted===true||m.decoding===true||!scopeValid(m,c)||!gates(g,catalog.acquisitionGates))return no();const attempt={context:null,scope:null,rows:null};try{const reply=installRegistrationRead(m,c,packet,attempt);
 // Released registration acquisition checkpoint, separate from validator mutation targets.
 if(!registrationAcquisitionResultValid(reply,attempt.context)||(reply.status==='context')!==m.contexts.has(attempt.context)||reply.status==='context'&&!acquisitionStateBound(m.contexts.get(attempt.context),attempt))throw Error('REGISTRATION_ACQUISITION_RESULT_INVARIANT');return reply;
 }catch{m.acquisitionFaulted=true;if(attempt.context&&m.contexts.has(attempt.context)){closeContext(m,attempt.context,m.contexts.get(attempt.context),true);m.packets.set(packet,freeze({phase:'failed-acquisition',context:attempt.context,scope:attempt.scope}));}return no();}
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
function acquisitionStateBound(state,attempt){return exact(state,['scope','rows','entries','continuations','stopped','invalidated'])&&equal(state.scope,attempt.scope)&&equal(state.rows,attempt.rows)&&state.entries instanceof Map&&state.entries.size===0&&state.continuations instanceof Map&&state.continuations.size===0&&state.stopped===false&&state.invalidated===false;}
function installRegistrationRead(m,c,packet,attempt){const p=takePacket(m,packet,c);if(!p)return no();const rs=project(p);if(!captureFits(m,rs)||m.contexts.size>=m.limits.maxContexts||m.issued>=m.limits.maxIssued)return no();const context=token('registration-read-context/1'),reply={status:'context',context,coverage:'captured-permitted'};attempt.context=context;
 // Registration acquisition frame checkpoint, separate from validator mutation targets.
 if(reply.status!=='context'||!registrationAcquisitionResultValid(reply,context))throw Error('REGISTRATION_ACQUISITION_RESULT_INVARIANT');if(size(reply)>m.limits.maxReplyBytes)return no();attempt.scope=freeze(copy(c));attempt.rows=freeze(rs);if(size({phase:'failed-acquisition',context,scope:attempt.scope})>m.limits.maxPacketBytes)return no();const state={scope:attempt.scope,rows:attempt.rows,entries:new Map(),continuations:new Map(),stopped:false,invalidated:false};
 // Registration acquisition candidate checkpoint, separate from validator mutation targets.
 if(!acquisitionStateBound(state,attempt))throw Error('REGISTRATION_ACQUISITION_STATE_INVARIANT');m.contexts.set(context,state);m.issued++;return reply;}
export function queryRegistrationRead(owner,request,c,packet,g){const m=owners.get(owner);if(!m||m.decoding||m.decoderFaulted)return no();return queryOwnedRegistrationRead(m,request,c,packet,g);}
function queryOwnedRegistrationRead(m,request,c,packet,g){const ctx=m?.contexts.get(request?.context);if(!m||!ctx||ctx.stopped||!scopeValid(m,c)||!equal(ctx.scope,c)||!exact(request,catalog.fields)||request.contract!==catalog.contract||request.operation!==catalog.operation||request.profile!==catalog.profile||!Object.hasOwn(catalog.requests,request.request?.operation)||!exact(request.request,catalog.requests[request.request.operation]))return no();if(["authorized","scope","continuous","live","retained"].some(k=>g?.[k]===false)){closeContext(m,request.context,ctx,true);return no();}if(!gates(g,catalog.releaseGates))return no();try{const reply=releaseRegistrationQuery(m,ctx,request,c,packet);
 // Released registration query checkpoint, separate from validator mutation targets.
 if(!queryResultBound(m,ctx,request,reply))throw Error('REGISTRATION_QUERY_RESULT_INVARIANT');return reply;
 }catch{closeContext(m,request.context,ctx,true);return no();}}
function queryResultBound(m,ctx,request,reply,entries=ctx.entries,continuations=ctx.continuations,selectors=m.selectors){
 const r=request.request;if(!registrationQueryResultValid(reply,r.operation,request.context,r))return false;if(reply.status==='unavailable')return true;
 if(r.operation==='inspect'){const s=selectors.get(r.entry);if(!s||s.kind!=='entry'||s.context!==request.context)return false;const {reference,...value}=reply.entry;return equal(value,ctx.rows[s.index]?.value);}
 const prior=r.continuation===null?null:selectors.get(r.continuation);if(r.continuation!==null&&(!prior||prior.kind!=='continuation'||prior.context!==request.context||prior.limit!==r.limit))return false;const start=prior?.start??0,end=Math.min(start+r.limit,ctx.rows.length),continuation=end<ctx.rows.length?continuations.get(JSON.stringify([r.limit,end])):null;return registrationQueryPageBindingsValid(reply,request.context,ctx.rows,start,end,entries,continuation,selectors,r.limit);
}
function releaseRegistrationQuery(m,ctx,request,c,packet){const p=takePacket(m,packet,c);if(!p)return no();const current=project(p,ctx.rows);if(!captureFits(m,current)||!equal(current,ctx.rows)){closeContext(m,request.context,ctx,true);return no();}const r=request.request;
 if(r.operation==='inspect'){const s=m.selectors.get(r.entry);if(!s||s.kind!=='entry'||s.context!==request.context)return no();const reply={status:'registration',context:request.context,coverage:'captured-permitted',entry:{reference:r.entry,...copy(ctx.rows[s.index].value)}};
 // Registration inspect frame checkpoint, separate from validator mutation targets.
 if(!queryResultBound(m,ctx,request,reply))throw Error('REGISTRATION_QUERY_RESULT_INVARIANT');return size(reply)<=m.limits.maxReplyBytes?reply:no();}
 if(!Number.isSafeInteger(r.limit)||r.limit<1||r.limit>m.limits.maxPageRows)return no();let start=0;if(r.continuation!==null){const s=m.selectors.get(r.continuation);if(!s||s.kind!=='continuation'||s.context!==request.context||s.limit!==r.limit)return no();start=s.start;}
 const end=Math.min(start+r.limit,ctx.rows.length),entries=new Map(ctx.entries),continuations=new Map(ctx.continuations),selectors=new Map(m.selectors);let issued=m.issued;const mint=(kind,data)=>{if(selectors.size>=m.limits.maxSelectors||issued>=m.limits.maxIssued||size({kind,...data})>m.limits.maxSelectorBytes)return null;const ref=token('registration-'+kind+'/1');selectors.set(ref,{kind,...data});issued++;return ref;},out=[];
 for(let index=start;index<end;index++){let ref=entries.get(index);if(!ref){ref=mint('entry',{context:request.context,index});if(!ref)return no();entries.set(index,ref);}out.push({reference:ref,table:ctx.rows[index].value.table});}
 let continuation=null;if(end<ctx.rows.length){const k=JSON.stringify([r.limit,end]);continuation=continuations.get(k);if(!continuation){continuation=mint('continuation',{context:request.context,limit:r.limit,start:end});if(!continuation)return no();continuations.set(k,continuation);}}
 const reply={status:'page',context:request.context,coverage:'captured-permitted',entries:out,continuation};
 // Registration page frame checkpoint, separate from validator mutation targets.
 if(!queryResultBound(m,ctx,request,reply,entries,continuations,selectors))throw Error('REGISTRATION_QUERY_RESULT_INVARIANT');if(size(reply)>m.limits.maxReplyBytes)return no();ctx.entries=entries;ctx.continuations=continuations;m.selectors=selectors;m.issued=issued;return reply;
}
export function retireRegistrationVisibility(owner,packet,g){const m=owners.get(owner),p=m?.packets.get(packet);if(!p||m.decoding===true||!gates(g,['ownerControlAllowed','retentionReleased']))return no();if(p.phase==='failed-acquisition'){const ctx=m.contexts.get(p.context);if(!ctx?.stopped||!ctx.invalidated||!gates(g,['deliveryDrained']))return no();m.contexts.delete(p.context);}m.packets.delete(packet);return {status:'retired'};}
export function stopRegistrationRead(owner,context,c,g){const m=owners.get(owner),ctx=m?.contexts.get(context);if(!ctx||!equal(ctx.scope,c)||g?.ownerControlAllowed!==true)return no();closeContext(m,context,ctx);return {status:'stopped'};}
export function registrationReadStopTargetOwned(owner,reference){return owners.get(owner)?.contexts.has(reference)===true;}
export function retireRegistrationRead(owner,context,g){const m=owners.get(owner),ctx=m?.contexts.get(context);if(!ctx?.stopped||m.decoding===true||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])||[...m.packets.values()].some(p=>p.phase==='failed-acquisition'&&p.context===context))return no();m.contexts.delete(context);return {status:'retired'};}
export function retireRegistrationReadRegistry(owner,g){const m=owners.get(owner);if(!m||m.decoding===true||m.contexts.size||m.packets.size)return no();g=Object.freeze(Object.fromEntries(['ownerControlAllowed','retentionReleased','deliveryDrained'].map(k=>[k,g?.[k]===true])));if(!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])||owners.get(owner)!==m||m.decoding===true||m.contexts.size||m.packets.size)return no();if(!releaseInvocationRegistrationReadCharge(m.domain,m.parentLease,g))return no();if(!releaseInvocationOwnerAttachment(m.domain,m.parentAttachment,g))throw Error('native registration read discharge invariant');owners.delete(owner);return {status:'retired'};}
// The prepared visibility packet is an original native port, not JSON authority.
export function queryDecodedRegistrationRead(owner,envelope,inputLimits,c,packet,g){
 const m=owners.get(owner);if(!m||!healthy(m)||m.decoding||m.decoderFaulted)return no();
 const maximum=beginInvocationRegistrationReadDecode(m.domain,m.parentLease,registrationQueryArgumentClassificationBytes);
 if(maximum===null)return no();m.decoding=true;m.decodeCharge=registrationQueryArgumentClassificationBytes;
 try{
  // Registration decoder classification checkpoint.
  const p=captureRegistrationQueryArgumentLimits(inputLimits);if(!p)return no();
  const bound=Math.min(maximum,p.maxRequestBytes),workspace=7*bound;
  if(!natural(workspace)||!extendInvocationRegistrationReadDecode(m.domain,m.parentLease,workspace))return no();m.decodeCharge+=workspace;
  // Registration decoder argument checkpoint.
  const n=normalizeRegistrationQueryArguments(envelope,c,p,bound);if(!n||!scopeValid(m,n.record.context))return no();
  // Read only the fixed current release facts. Unknown/absent facts never grant;
  // unrelated native bookkeeping and opaque handles are not traversed.
  if(!g||typeof g!=='object'||Array.isArray(g))return no();const facts={};
  for(const key of catalog.releaseGates){const d=Object.getOwnPropertyDescriptor(g,key);if(!d)continue;if(!Object.hasOwn(d,'value')||d.enumerable!==true||d.value!==undefined&&typeof d.value!=='boolean')return no();if(d.value!==undefined)facts[key]=d.value;}
  const captured=copySettingsData(facts,{...p,maxRequestBytes:bound});if(!captured)return no();
  // Registration decoder native dispatch checkpoint.
  const request=registrationQueryProducerRequest(n),reply=queryOwnedRegistrationRead(m,request,n.record.context,packet,captured.value);
  // Registration decoder release checkpoint. Native installation is not rolled back.
  if(!registrationQueryResultValid(reply,request.request.operation,request.context,request.request))throw Error('REGISTRATION_DECODED_RESULT_INVARIANT');return reply;
 }catch{m.decoderFaulted=true;return no();}
 finally{const released=finishInvocationRegistrationReadDecode(m.domain,m.parentLease);m.decodeCharge=0;m.decoding=false;if(!released){m.decoderFaulted=true;throw Error('REGISTRATION_DECODED_REFUND_INVARIANT');}}
}
export function registrationReadDecodeUsage(owner){const m=owners.get(owner);return m?{busy:m.decoding===true,declaredWorkspaceBytes:m.decodeCharge??0,faulted:m.decoderFaulted===true}:null;}
// Trusted supervision only; opaque selectors are never derived from these counts.
export function registrationReadUsage(owner){const m=owners.get(owner);return m?{contexts:m.contexts.size,invalidated:[...m.contexts.values()].filter(c=>c.invalidated).length,preparations:m.packets.size,selectors:m.selectors.size,issued:m.issued,declaredHeldBytes:m.charge,acquisitionFaulted:m.acquisitionFaulted,failedAcquisitions:[...m.packets.values()].filter(p=>p.phase==='failed-acquisition').length}:null;}
