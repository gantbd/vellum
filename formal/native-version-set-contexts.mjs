// Selected native atomic set installation and whole-parent-gated member queries.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {captureSourceProfile,sourceReadResultValid} from './source-products.mjs';
import {newVersionRegistry,acquireVersionSet,queryVersionMember,versionSetAcquisitionResultValid} from './version-set-contexts.mjs';
import {logicalLines} from './reference-profile.mjs';
import {annotationQueryResultValid} from './annotation-queries.mjs';
import {validateNativeVersionSetCapture,inspectNativeVersionSetCapture,consumeNativeVersionSetCapture,validateNativeVersionSetHeld} from './native-version-set-capture.mjs';
const pools=new WeakMap(),fields=['setLimits','maxGroups','maxSelectors','maxHolds','maxIssued','maxRegistryBytes','maxCaptureBytes','maxContextBytes','maxRequestBytes','maxReplyBytes','maxSelectorBytes','maxSelectorRecordBytes','maxInputNodes','maxInputDepth','maxHeldBytes'],limitFields=['groups','participants','inputRoots','retainedCuts','contexts','units','selectors','holds','issuance','sourceBytes','source','annotation'];
const sourceCatalog=JSON.parse(fs.readFileSync(new URL('./source-products.json',import.meta.url),'utf8')),annotationCatalog=JSON.parse(fs.readFileSync(new URL('./annotation-queries.json',import.meta.url),'utf8'));
const natural=n=>Number.isSafeInteger(n)&&n>=0,yes=(g,ks)=>ks.every(k=>g?.[k]===true),exact=(v,ks)=>{if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(d);return keys.length===ks.length&&ks.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable);},copy=(v,p,n)=>copySettingsData(v,{maxRequestBytes:n,maxInputNodes:p.maxInputNodes,maxInputDepth:p.maxInputDepth}),token=kind=>Object.freeze({kind}),no=()=>({status:'unavailable'}),discharge=['ownerControlAllowed','retentionReleased','deliveryDrained'];
function pool(owner,domain,handle){const m=pools.get(owner);return m&&m.domain===domain&&m.handle===handle&&!m.closed?m:null;}
export function openNativeVersionSetContexts(owner,domain,p,g){
 if(pools.has(owner)||owner.faulted||!exact(p,fields)||!fields.filter(k=>k!=='setLimits').every(k=>natural(p[k]))||!yes(g,['ownerControlAllowed','resourcesReady','preparationReserved']))return null;
 const captured=copy(p,p,p.maxRequestBytes),l=captured?.value.setLimits;if(!captured||!exact(l,limitFields)||!limitFields.filter(k=>!['source','annotation'].includes(k)).every(k=>natural(l[k]))||!captureSourceProfile(l.source)||!exact(l.annotation,['maxInputRows','maxCaptureBytes','maxRowBytes','maxPageRows'])||!Object.values(l.annotation).every(natural))return null;
 // setLimits are per retained set; group/alias/hold/issuance limits are pool-wide.
 const stage=p.maxRegistryBytes+p.maxCaptureBytes+l.participants*p.maxSelectorRecordBytes,charge=captured.bytes+2*p.maxRequestBytes+3*stage+2*p.maxContextBytes+3*p.maxReplyBytes+p.maxGroups*(p.maxRegistryBytes+p.maxCaptureBytes+p.maxContextBytes+p.maxReplyBytes)+(2*p.maxSelectors+p.maxHolds)*(p.maxSelectorBytes+p.maxSelectorRecordBytes);
 if(!natural(stage)||!natural(charge)||charge>p.maxHeldBytes||!natural(owner.charge)||charge>owner.limits.maxHeldBytes-owner.charge)return null;
 const handle=token('native-version-set-context-pool/1');pools.set(owner,{domain,handle,profile:captured.value,charge,parents:new Map(),selectors:new Map(),holds:new Map(),issued:0,closed:false});owner.charge+=charge;return handle;
}
// Ready flags here are private consequences of native admission/currentness,
// not additional caller-granted authority or public parent-validation tokens.
function loweredHost(host){return {...host,registryReady:true,retentionReady:true,replyReady:true,transferReady:true,members:host.members.map(m=>({...m,registryReady:true,retentionReady:true,replyReady:true,transferReady:true}))};}
export function installNativeVersionSetContext(owner,domain,handle,capturePool,cut,host,g){
 const m=pool(owner,domain,handle);if(!m)return no();const p=m.profile,packet=copy({host,facts:g},p,p.maxRequestBytes)?.value;if(!packet||packet.facts?.outsidePublicationGuard!==true)return no();
 if(!validateNativeVersionSetCapture(owner,domain,capturePool,cut,packet.host,packet.facts))return no();
 if(!yes(packet.facts,['preparationReserved','resourcesReady'])||m.parents.size>=p.maxGroups)return no();
 const value=inspectNativeVersionSetCapture(owner,domain,capturePool,cut,{ownerControlAllowed:true}),candidate=value&&copy(value,p,p.maxCaptureBytes)?.value,scope=candidate&&copy({scope:candidate.scope,invalidated:false},p,p.maxContextBytes)?.value;if(!candidate||!scope)return no();
 const count=1+candidate.members.length;if(count>p.maxSelectors-m.selectors.size||count>p.maxIssued-m.issued)return no();
 const context=token('native-version-set-context/1'),registry=newVersionRegistry('native-set-'+(m.issued+1),p.setLimits),acquired=acquireVersionSet(registry,candidate,loweredHost(packet.host));if(acquired.result.status!=='version-set'||!copy(acquired.state,p,p.maxRegistryBytes))return no();
 const bindings=new Map([[acquired.result.context,context]]),pending=[{selector:context,record:{kind:'parent',parent:context,internal:acquired.result.context}}],participants=[];
 for(const member of acquired.result.participants){const selector=token('native-version-set-member/1'),document=copy(member.document,p,p.maxSelectorRecordBytes)?.value;if(!document)return no();bindings.set(member.snapshot,selector);pending.push({selector,record:{kind:'member',parent:context,internal:member.snapshot,document}});participants.push({document:member.document,snapshot:selector,data:member.data});}
 const reply={status:'version-set',context,participants};
 if(!versionSetAcquisitionResultValid(reply,{contract:'read-version-set/1',participants:acquired.result.participants,selection:{kind:'capture-current'}},context)||reply.participants.length!==candidate.members.length||reply.participants.some((row,i)=>row.snapshot!==pending[i+1].selector||!equal(row.document,candidate.members[i].document)||!equal(row.data,candidate.members[i].candidate.data))){owner.faulted=true;return no();}
 if(pending.some(row=>!copy(row.selector,p,p.maxSelectorBytes)||!copy(row.record,p,p.maxSelectorRecordBytes))||!copy(reply,p,p.maxReplyBytes))return no();
 Object.freeze(reply);
 try{const parents=new Map(m.parents),selectors=new Map(m.selectors),held={...scope,value:candidate,registry:acquired.state,bindings,holds:0};parents.set(context,held);for(const row of pending)selectors.set(row.selector,row.record);
  if(!consumeNativeVersionSetCapture(owner,domain,capturePool,cut,packet.host,packet.facts))return no();
  m.parents=parents;m.selectors=selectors;m.issued+=count;return reply;
 }catch(error){owner.faulted=true;throw error;}
}
export function inspectNativeVersionSetContext(owner,domain,handle,reference,g){const m=pool(owner,domain,handle),alias=m?.selectors.get(reference),held=alias&&m.parents.get(alias.parent);return held&&g?.ownerControlAllowed===true?structuredClone(held.value):null;}
// Existing F02 retained selection over an original available coherent parent.
// This copies an owned projection; it is not arbitrary historical-root lookup.
export function acquireNativeRetainedVersionSetContext(owner,domain,handle,request,host,g){
 const m=pool(owner,domain,handle);if(!m||!exact(request,['contract','operation','authority','participants','selection','profile'])||!exact(request.selection,['kind','reference'])||request.selection.kind!=='retained')return no();
 const reference=request.selection.reference,p=m.profile,packet=copy({request:{...request,selection:{kind:'retained',reference:null}},host,facts:g},p,p.maxRequestBytes)?.value,alias=m.selectors.get(reference),original=alias?.kind==='parent'?m.parents.get(reference):null;
 if(!packet||packet.facts?.outsidePublicationGuard!==true||!original||original.invalidated)return no();const r=packet.request;
 if(r.contract!=='read-version-set/1'||r.operation!=='snapshot-handoff'||r.authority!==owner.context.authority||r.profile!==original.scope.profile||!Array.isArray(r.participants)||!r.participants.length||r.participants.length>p.setLimits.participants)return no();
 const selected=[],keys=new Set();for(const participant of r.participants){
  if(!exact(participant,['document','data'])||!exact(participant.document,['authority','id','incarnation'])||participant.document.authority!==r.authority||!natural(participant.document.incarnation)||participant.document.incarnation<1||!['authority','id'].every(k=>typeof participant.document[k]==='string'&&participant.document[k].length)||!Array.isArray(participant.data)||!participant.data.length||participant.data.length>2||new Set(participant.data).size!==participant.data.length)return no();
  const key=JSON.stringify([participant.document.authority,participant.document.id]),matches=original.value.members.filter(member=>equal(member.document,participant.document));if(keys.has(key)||matches.length!==1||participant.data.some(k=>!matches[0].candidate.data.includes(k)))return no();keys.add(key);selected.push({member:matches[0],data:participant.data});
 }
 // Even explicit subsets/narrowing require the original complete parent to be
 // currently available. A selector cannot bypass a gap in an excluded peer.
 if(!validateNativeVersionSetHeld(owner,original,p,packet.host,packet.facts))return no();
 if(!yes(packet.facts,['preparationReserved','resourcesReady'])||m.parents.size>=p.maxGroups)return no();
 const members=selected.map(({member,data})=>{const source=data.includes('source'),annotations=data.includes('annotations'),candidate={...member.candidate,data:[...data].sort(),root:{document:member.document,...(source?{source:member.candidate.root.source,sourceRevision:member.candidate.root.sourceRevision}:{})},sourceProfile:source?member.candidate.sourceProfile:null,annotations:annotations?member.candidate.annotations:null};candidate.units=(source?candidate.root.source.length:0)+(annotations?candidate.annotations.rows.length:0);return {...member,candidate};}),nativeFacets=original.value.nativeFacets.filter(row=>selected.some(({member,data})=>equal(member.document,row.document)&&data.includes('annotations'))),candidate=copy({...original.value,members,nativeFacets},p,p.maxCaptureBytes)?.value,scope=copy({scope:original.scope,invalidated:false},p,p.maxContextBytes)?.value;if(!candidate||!scope)return no();
 const count=1+candidate.members.length;if(count>p.maxSelectors-m.selectors.size||count>p.maxIssued-m.issued)return no();
 const current={...packet.host,members:candidate.members.map(member=>packet.host.members.find(h=>equal(h.document,member.document)))},context=token('native-version-set-context/1'),registry=newVersionRegistry('native-set-'+(m.issued+1),p.setLimits),acquired=acquireVersionSet(registry,candidate,loweredHost(current));if(acquired.result.status!=='version-set'||!copy(acquired.state,p,p.maxRegistryBytes))return no();
 const bindings=new Map([[acquired.result.context,context]]),pending=[{selector:context,record:{kind:'parent',parent:context,internal:acquired.result.context}}],participants=[];
 for(const member of acquired.result.participants){const selector=token('native-version-set-member/1'),document=copy(member.document,p,p.maxSelectorRecordBytes)?.value;if(!document)return no();bindings.set(member.snapshot,selector);pending.push({selector,record:{kind:'member',parent:context,internal:member.snapshot,document}});participants.push({document:member.document,snapshot:selector,data:member.data});}
 const reply={status:'version-set',context,participants};
 if(!versionSetAcquisitionResultValid(reply,{contract:'read-version-set/1',participants:acquired.result.participants,selection:request.selection},context)||reply.participants.length!==candidate.members.length||reply.participants.some((row,i)=>row.snapshot!==pending[i+1].selector||!equal(row.document,candidate.members[i].document)||!equal(row.data,candidate.members[i].candidate.data))){owner.faulted=true;return no();}
 if(pending.some(row=>!copy(row.selector,p,p.maxSelectorBytes)||!copy(row.record,p,p.maxSelectorRecordBytes))||!copy(reply,p,p.maxReplyBytes))return no();Object.freeze(reply);
 try{const retainedParents=new Map(m.parents),retainedSelectors=new Map(m.selectors),held={...scope,value:candidate,registry:acquired.state,bindings,holds:0};retainedParents.set(context,held);for(const row of pending)retainedSelectors.set(row.selector,row.record);m.parents=retainedParents;m.selectors=retainedSelectors;m.issued+=count;return reply;}catch(error){owner.faulted=true;throw error;}
}
function normalizedMemberQuery(m,request){
 if(!request||typeof request!=='object')return null;const op=Object.getOwnPropertyDescriptor(request,'operation'),contract=Object.getOwnPropertyDescriptor(request,'contract');if(!op||!Object.hasOwn(op,'value')||typeof op.value!=='string'||!contract||!Object.hasOwn(contract,'value'))return null;
 const source=contract.value===sourceCatalog.contract,annotation=contract.value===annotationCatalog.contract;if(!source&&!annotation)return null;const variants=source?sourceCatalog.variants:annotationCatalog.variants;if(!Object.hasOwn(variants,op.value))return null;
 const fields=source?[...sourceCatalog.common,...variants[op.value]]:[...Object.keys(annotationCatalog.common),...Object.keys(variants[op.value])];if(!exact(request,fields))return null;
 const opaque=source?null:op.value==='enumerate'?'continuation':'annotation',value=copy({...request,snapshot:null,...(opaque?{[opaque]:null}:{})},m.profile,m.profile.maxRequestBytes)?.value;return value?{request:{...value,snapshot:request.snapshot,...(opaque?{[opaque]:request[opaque]}:{})},source,opaque}:null;
}
export function queryNativeVersionSetMember(owner,domain,handle,request,host,g){
 const m=pool(owner,domain,handle);if(!m)return no();const parsed=normalizedMemberQuery(m,request);if(!parsed)return no();const r=parsed.request,alias=m.selectors.get(r.snapshot),held=alias?.kind==='member'?m.parents.get(alias.parent):null,member=held?.value.members.find(p=>equal(p.document,alias.document));
 if(!held||held.invalidated||!member||!equal(r.document,alias.document)||r.profile!==held.scope.profile||!member.candidate.data.includes(parsed.source?'source':'annotations'))return no();let opaque=null;
 if(parsed.source){const xs=member.candidate.root.source,boundary=n=>natural(n)&&n<=xs.length&&(n===xs.length||(xs[n]&0xc0)!==0x80);if(r.operation==='bounded-range'?!(boundary(r.lo)&&boundary(r.hi)&&r.lo<=r.hi):!(natural(r.line)&&r.line<logicalLines(xs).length))return no();}
 else if(r.operation==='enumerate'){if(!natural(r.limit)||r.limit<1||r.limit>m.profile.setLimits.annotation.maxPageRows)return no();if(r.continuation!==null){opaque=m.selectors.get(r.continuation);if(opaque?.kind!=='continuation'||opaque.parent!==alias.parent||opaque.member!==r.snapshot||opaque.limit!==r.limit)return no();}}
 else{opaque=m.selectors.get(r.annotation);if(opaque?.kind!=='record'||opaque.parent!==alias.parent||opaque.member!==r.snapshot)return no();}
 const packet=copy({host,facts:g},m.profile,m.profile.maxRequestBytes)?.value;if(!packet||!validateNativeVersionSetHeld(owner,held,m.profile,packet.host,packet.facts))return no();
 const lowered={...r,snapshot:alias.internal,...(parsed.opaque?{[parsed.opaque]:opaque?.internal??null}:{})};
 try{
  const out=queryVersionMember(held.registry,lowered,loweredHost(packet.host));if(out.result.status==='unavailable')return no();if(!copy(out.state,m.profile,m.profile.maxRegistryBytes))return no();
  const pending=[],bindings=new Map(held.bindings),staged=new Map();
  const bind=(internal,kind)=>{const raw=out.state.registry.selectors.find(p=>p.token===internal);if(!raw||raw.kind!==kind||raw.context!==alias.internal)throw Error('version-set query returned a foreign selector');let selector=bindings.get(internal);if(selector){const row=staged.get(selector)??m.selectors.get(selector);if(row?.kind!==kind||row.parent!==alias.parent||row.member!==r.snapshot||kind==='continuation'&&row.limit!==r.limit)throw Error('version-set selector changed meaning');return selector;}selector=token('native-version-set-'+kind+'/1');const record={kind,parent:alias.parent,member:r.snapshot,internal,...(kind==='continuation'?{limit:r.limit}:{})};pending.push({selector,record});staged.set(selector,record);bindings.set(internal,selector);return selector;};
  let reply;if(parsed.source){if(!['content','line'].includes(out.result.status))throw Error('unexpected source product');reply={...out.result,context:r.snapshot};}
  else if(out.result.status==='page')reply={status:'page',snapshot:r.snapshot,records:out.result.records.map(row=>({...row,reference:bind(row.reference,'record')})),continuation:out.result.continuation===null?null:bind(out.result.continuation,'continuation')};
  else if(out.result.status==='record')reply={status:'record',snapshot:r.snapshot,record:{...out.result.record,reference:r.annotation}};
  else if(out.result.status==='location')reply={status:'location',snapshot:r.snapshot,reference:r.annotation,location:out.result.location};
  else throw Error('unexpected annotation product');
  if(parsed.source&&!sourceReadResultValid(reply,r,{authority:alias.document.authority,document:alias.document.id,incarnation:alias.document.incarnation,revision:member.candidate.root.sourceRevision})){owner.faulted=true;return no();}
  if(!parsed.source&&(!annotationQueryResultValid(reply,r)||out.result.status==='page'&&(reply.continuation!==(out.result.continuation===null?null:bindings.get(out.result.continuation))||reply.records.some((row,i)=>row.reference!==bindings.get(out.result.records[i].reference))))){owner.faulted=true;return no();}
  if(pending.length>m.profile.maxSelectors-m.selectors.size||pending.length>m.profile.maxIssued-m.issued||pending.some(row=>!copy(row.selector,m.profile,m.profile.maxSelectorBytes)||!copy(row.record,m.profile,m.profile.maxSelectorRecordBytes))||!copy(reply,m.profile,m.profile.maxReplyBytes))return no();
  Object.freeze(reply);const selectors=new Map(m.selectors);for(const row of pending)selectors.set(row.selector,row.record);held.registry=out.state;held.bindings=bindings;m.selectors=selectors;m.issued+=pending.length;return reply;
 }catch(error){owner.faulted=true;throw error;}
}
export function validateNativeVersionSetContext(owner,domain,handle,reference,host,g){const m=pool(owner,domain,handle),alias=m?.selectors.get(reference),held=['parent','member'].includes(alias?.kind)?m.parents.get(alias.parent):null;return !!m&&validateNativeVersionSetHeld(owner,held,m.profile,host,g);}
export function stopNativeVersionSetContext(owner,domain,handle,r,scope,g){const m=pool(owner,domain,handle);if(!m||!exact(r,['contract','operation','context'])||r.contract!=='read-context/1'||r.operation!=='stop-observation'||g?.ownerControlAllowed!==true)return no();const alias=m.selectors.get(r.context),held=['parent','member'].includes(alias?.kind)?m.parents.get(alias.parent):null,c=copy(scope,m.profile,m.profile.maxContextBytes)?.value;if(!held||!c||!equal(c,held.scope))return no();held.invalidated=true;return {status:'stopped'};}
export function retireNativeVersionSetContext(owner,domain,handle,reference,g){const m=pool(owner,domain,handle),alias=m?.selectors.get(reference),held=['parent','member'].includes(alias?.kind)?m.parents.get(alias.parent):null;if(!held?.invalidated||held.holds!==0||!yes(g,discharge))return false;for(const [selector,row]of m.selectors)if(row.parent===alias.parent)m.selectors.delete(selector);return m.parents.delete(alias.parent);}
export function retireNativeVersionSetContexts(owner,domain,handle,g){const m=pool(owner,domain,handle);if(!m||m.parents.size||m.selectors.size||m.holds.size||!yes(g,discharge))return false;m.closed=true;m.profile=null;owner.charge-=m.charge;return true;}
export function nativeVersionSetContextUsage(owner){const m=pools.get(owner);return m?{closed:m.closed,groups:m.parents.size,selectors:m.selectors.size,holds:m.holds.size,invalidated:[...m.parents.values()].filter(c=>c.invalidated).length,issued:m.issued,declaredHeldBytes:m.closed?0:m.charge}:null;}
// Closed first-party F05 ports. A member hold retains its whole coherent parent;
// neither a parent selector nor an annotation-only member becomes source authority.
export function nativeVersionSetContextPoolAvailable(owner,domain,handle){return !!pool(owner,domain,handle);}
export function locateNativeVersionSetContextPool(owner,domain,reference){const m=pools.get(owner);return m&&m.domain===domain&&!m.closed&&m.selectors.get(reference)?.kind==='member'?m.handle:null;}
export function nativeVersionSetStopTargetOwned(owner,domain,handle,reference){const m=pool(owner,domain,handle),alias=m?.selectors.get(reference);return Boolean(alias&&['parent','member'].includes(alias.kind)&&m.parents.has(alias.parent));}
export function inspectNativeVersionSetSearchContext(owner,domain,handle,reference,c,g){
 const m=pool(owner,domain,handle),alias=m?.selectors.get(reference),held=alias?.kind==='member'?m.parents.get(alias.parent):null;if(!held)return null;
 const scope=copy(c,m.profile,m.profile.maxRequestBytes)?.value;if(!scope||!equal(scope.document,alias.document))return null;
 if(!['recipient','purpose','destination','policy','continuity','profile'].every(k=>equal(scope[k],held.scope[k])))return null;
 // Stable recipient scope is not current authority. The existing current-host
 // frame (including all members) travels on the private accepted-facts port.
 const member=held.value.members.find(p=>equal(p.document,alias.document));if(!member?.candidate.data.includes('source')||!validateNativeVersionSetHeld(owner,held,m.profile,g,g))return null;
 const source=member.candidate.root;return {snapshot:{ref:{authority:source.document.authority,document:source.document.id,incarnation:source.document.incarnation,revision:source.sourceRevision},source:source.source,selector:reference,profile:held.scope.profile},sourceProfile:member.candidate.sourceProfile};
}
export function reserveNativeVersionSetSearchLease(owner,domain,handle,reference,c,g){
 if(!inspectNativeVersionSetSearchContext(owner,domain,handle,reference,c,g))return null;const m=pool(owner,domain,handle),alias=m.selectors.get(reference),held=m.parents.get(alias.parent);
 if(m.holds.size>=m.profile.maxHolds||m.issued>=m.profile.maxIssued)return null;const lease=token('native-version-set-search-hold/1'),record={context:reference,parent:alias.parent,published:false};
 if(!copy(lease,m.profile,m.profile.maxSelectorBytes)||!copy(record,m.profile,m.profile.maxSelectorRecordBytes))return null;
 m.holds.set(lease,{...record,held});held.holds++;m.issued++;return lease;
}
export function inspectNativeVersionSetSearchLease(owner,domain,handle,lease,c,g){const m=pool(owner,domain,handle),row=m?.holds.get(lease);return row&&m.parents.get(row.parent)===row.held?inspectNativeVersionSetSearchContext(owner,domain,handle,row.context,c,g):null;}
export function publishNativeVersionSetSearchLease(owner,domain,handle,lease){const m=pool(owner,domain,handle),row=m?.holds.get(lease);if(!row||row.published||row.held.invalidated||owner.faulted||m.parents.get(row.parent)!==row.held)return false;row.published=true;return true;}
function dischargeSearchLease(owner,domain,handle,lease,published,g){const m=pool(owner,domain,handle),row=m?.holds.get(lease);if(!row||row.published!==published||published&&!yes(g,discharge)||m.holds.get(lease)!==row)return false;m.holds.delete(lease);row.held.holds--;return true;}
export function discardNativeVersionSetSearchLease(owner,domain,handle,lease){return dischargeSearchLease(owner,domain,handle,lease,false,null);}
export function retireNativeVersionSetSearchLease(owner,domain,handle,lease,g){return dischargeSearchLease(owner,domain,handle,lease,true,g);}
