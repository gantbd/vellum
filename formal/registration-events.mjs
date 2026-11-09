// Full permitted-state observations after publication; never an activation audit.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {beginInvocationAdmissionMutation,endInvocationAdmissionMutation,invocationOrdinaryAdmissionOpen,invocationAuthorityRoot,invocationAuthorityContext,invocationUsage,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment,releaseInvocationOwnerAttachment} from './invocation-publication.mjs';
import {nativeResourceCatalogValid} from './resource-registration-state.mjs';
import {copySettingsData} from './semantic-settings.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./registration-events.json',import.meta.url))),descriptors=JSON.parse(fs.readFileSync(new URL('./registration-reads.json',import.meta.url))),owners=new WeakMap(),domains=new WeakMap(),copy=structuredClone;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort()),natural=n=>Number.isSafeInteger(n)&&n>=0,size=v=>Buffer.byteLength(JSON.stringify(v)),gates=(g,ks)=>ks.every(k=>g?.[k]===true),no=()=>({status:'unavailable'}),token=kind=>Object.freeze({kind});
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;},label=(s,l)=>typeof s==='string'&&s.length>0&&Buffer.byteLength(s)<=l&&Buffer.from(s).toString('utf8')===s;
const identity=(v,l)=>exact(v,['authority','id','incarnation'])&&label(v.authority,l)&&label(v.id,l)&&natural(v.incarnation),key=s=>JSON.stringify([s.table,s.provider.authority,s.provider.id,s.provider.incarnation,s.resource.authority,s.resource.id,s.resource.incarnation]);
const slots=root=>[...root.resourceRegistrations.map(row=>({table:'read',row})),...root.writeRegistrations.map(row=>({table:'write',row}))],slot=s=>({table:s.table,provider:copy(s.row.provider),resource:copy(s.row.resource)});
const slotValid=(s,l)=>['read','write'].includes(s.table)&&identity(s.provider,l)&&identity(s.resource,l);
function scopeValid(m,c){return exact(c,catalog.contextFields)&&Object.entries(c).filter(([k])=>k!=='descriptors').every(([,v])=>label(v,m.limits.maxContextBytes))&&c.authority===m.authority&&c.engine===m.engine&&c.profile===catalog.profile&&exact(c.descriptors,['read','write'])&&Object.values(c.descriptors).every(v=>typeof v==='boolean')&&size(c)<=m.limits.maxContextBytes;}
function registrationCommandOwnerFaulted(m){for(const p of m.packets.values())if(p.kind==='command'&&p.phase==='faulted'&&p.executionStarted)return true;return false;}
function currentRoot(m,g){if(registrationCommandOwnerFaulted(m))return null;if(invocationUsage(m.domain)?.faulted!==false||!gates(g,catalog.authorityGates)||g?.registrationObservationInterpretationCurrent!==true||!equal(g.registrationObservationProfile,m.profile))return null;const root=invocationAuthorityRoot(m.domain);return nativeResourceCatalogValid(root,m.profile)&&size(root)<=m.limits.maxRootBytes?root:null;}
function stopped(m,s){if(!s.stopped){m.bindings-=s.refs.size;s.refs.clear();s.queue=[];s.baseline=null;s.stopped=true;}}
function access(m,stream,c,g){const s=m?.streams.get(stream);if(!s||s.stopped||!scopeValid(m,c)||!equal(s.scope,c))return null;if(!currentRoot(m,g)){stopped(m,s);return null;}return s;}
const gap=s=>{s.gap=true;s.queue=[];s.baseline=null;return {status:'gap',baseline:'required'};};
const descriptor=(table,row)=>Object.fromEntries((table==='write'?descriptors.writeFields:descriptors.readFields[row.profile]).map(k=>[k,copy(row[k])]));
const entries=p=>p.map(r=>({reference:r.reference,...copy(r.value)}));
const envelope=(status,generation,p,before)=>status==='baseline'?{status,coverage:'captured-policy-current',generation,entries:entries(p)}:{status,coverage:'captured-policy-current',before,after:generation,entries:entries(p)};
export function registrationEventCharge(l){if(!exact(l,catalog.limits)||!Object.values(l).every(natural))return null;const n=l.maxProfileBytes+l.maxPreparations*l.maxPacketBytes+l.maxStreams*(l.maxContextBytes+2*l.maxProjectionBytes+2*l.maxBindingBytes+(l.maxQueue+2)*l.maxReplyBytes);return natural(n)&&n<=l.maxHeldBytes?n:null;}
export function openRegistrationEventRegistry(domain,profile,limits,g){
 const attachment=beginInvocationOwnerAttachment(domain,'registration-events',limits?.maxHeldBytes,token('registration-event-registry/1'));if(!attachment)return null;
 try{const c=invocationAuthorityContext(domain),charge=registrationEventCharge(limits);if(!c||invocationUsage(domain)?.faulted!==false||domains.has(domain)||charge===null||!gates(g,['ownerControlAllowed','resourcesReady','preparationReserved'])||!nativeResourceCatalogValid(invocationAuthorityRoot(domain),profile)||size(profile)>limits.maxProfileBytes)return null;const owner=token('registration-event-registry/1');owners.set(owner,{domain,parentAttachment:attachment,authority:c.authority,engine:c.engine,profile:freeze(copy(profile)),limits:freeze(copy(limits)),charge,streams:new Map(),packets:new Map(),bindings:0,issued:0});domains.set(domain,owner);if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native registration event attachment invariant');return owner;}
 finally{abortInvocationOwnerAttachment(domain,attachment);}
}
export function prepareRegistrationEventSnapshot(owner,stream,c,facts,g){
 const admissionDomain=owners.get(owner)?.domain??null;const admissionSection=stream===null?beginInvocationAdmissionMutation(admissionDomain):null;if(stream===null&&!admissionSection)return null;try{const m=owners.get(owner);if(!m||!scopeValid(m,c)||stream!==null&&!access(m,stream,c,g)||!gates(g,catalog.preparationGates)||m.packets.size>=m.limits.maxPreparations)return null;const root=currentRoot(m,g);if(!root||!Array.isArray(facts))return null;const all=slots(root);if(all.length>m.limits.maxRows||facts.length!==all.length||size({root,scope:c,facts})>m.limits.maxPacketBytes||facts.some(f=>!exact(f,['table','provider','resource','captureAllowed','currentAllowed','captureDescriptor','currentDescriptor','order'])||!slotValid(f,m.limits.maxContextBytes)||!['captureAllowed','currentAllowed','captureDescriptor','currentDescriptor'].every(k=>typeof f[k]==='boolean')||!f.captureAllowed&&f.captureDescriptor||!f.currentAllowed&&f.currentDescriptor||(!(f.captureAllowed&&f.currentAllowed)?f.order!==null:!natural(f.order))))return null;const keys=facts.map(key),expected=all.map(s=>key(slot(s)));if(new Set(keys).size!==facts.length||!equal([...keys].sort(),[...expected].sort()))return null;const order=facts.filter(f=>f.captureAllowed&&f.currentAllowed).map(f=>f.order).sort((a,b)=>a-b);if(!equal(order,order.map((_,i)=>i)))return null;const packet=token('registration-event-snapshot/1');m.packets.set(packet,{kind:'snapshot',stream,root,scope:freeze(copy(c)),facts:freeze(copy(facts))});return packet;
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
function take(m,packet,kind,stream,c){const p=m.packets.get(packet);if(!p||p.kind!==kind||p.stream!==stream||p.root!==invocationAuthorityRoot(m.domain)||!equal(p.scope,c))return null;m.packets.delete(packet);return p;}
function project(m,s,p){const all=slots(p.root),refs=new Map(s.refs),projection=[];let issued=m.issued;for(const f of p.facts.filter(f=>f.captureAllowed&&f.currentAllowed).sort((a,b)=>a.order-b.order)){const k=key(f),actual=all.find(x=>key(slot(x))===k);if(!actual)return null;let binding=refs.get(k);if(!binding){if(m.bindings+(refs.size-s.refs.size)>=m.limits.maxBindings||issued>=m.limits.maxIssued)return null;binding={reference:token('registration-observation-entry/1'),slot:slot(actual)};refs.set(k,binding);issued++;}const value={table:f.table,...(s.scope.descriptors[f.table]&&f.captureDescriptor&&f.currentDescriptor?{descriptor:descriptor(f.table,actual.row)}:{})};projection.push({key:k,reference:binding.reference,value});}if(size(projection)>m.limits.maxProjectionBytes||size([...refs])>m.limits.maxBindingBytes)return null;return {refs,projection,issued};}
function install(m,s,p){m.bindings+=p.refs.size-s.refs.size;s.refs=p.refs;s.baseline=p.projection;m.issued=p.issued;}
export function establishRegistrationObservation(owner,c,packet,g){
 const admissionDomain=owners.get(owner)?.domain??null;const admissionSection=beginInvocationAdmissionMutation(admissionDomain);if(!admissionSection)return no();try{const m=owners.get(owner);if(g?.outsidePublicationGuard!==true||!m||!scopeValid(m,c)||!currentRoot(m,g)||!gates(g,catalog.releaseGates)||m.streams.size>=m.limits.maxStreams||m.limits.maxQueue<1)return no();const p=take(m,packet,'snapshot',null,c);if(!p)return no();const s={scope:freeze(copy(c)),refs:new Map(),queue:[],baseline:null,generation:null,gap:false,stopped:false},stage=project(m,s,p);if(!stage||stage.issued+2>m.limits.maxIssued)return no();const stream=token('registration-observation/1'),generation=token('registration-observation-generation/1'),reply={stream,...envelope('baseline',generation,stage.projection)};if(size(reply)>m.limits.maxReplyBytes)return no();install(m,s,stage);m.issued+=2;s.generation=generation;m.streams.set(stream,s);return reply;
 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function sampleRegistrationObservation(owner,stream,c,packet,g){const m=owners.get(owner),s=m&&access(m,stream,c,g);if(!s||g?.outsidePublicationGuard!==true)return no();if(g?.resourcesReady!==true)return gap(s);if(s.gap)return gap(s);const p=take(m,packet,'snapshot',stream,c);if(!p)return no();const stage=project(m,s,p);if(!stage)return gap(s);if(equal(stage.projection,s.baseline))return {status:'unchanged'};const after=token('registration-observation-generation/1'),reply=envelope('projection',after,stage.projection,s.generation);if(s.queue.length>=m.limits.maxQueue||stage.issued>=m.limits.maxIssued||size(reply)>m.limits.maxReplyBytes)return gap(s);s.queue.push({before:s.generation,after,projection:stage.projection});install(m,s,stage);m.issued++;s.generation=after;return {status:'queued'};}
export function prepareRegistrationEventDelivery(owner,stream,c,facts,g){const m=owners.get(owner),s=m&&access(m,stream,c,g);if(!s||s.gap||!s.queue.length||!gates(g,catalog.deliveryGates)||m.packets.size>=m.limits.maxPreparations||!Array.isArray(facts))return null;const head=s.queue[0],wanted=head.projection.map(r=>r.key);if(facts.length!==wanted.length||facts.some(f=>!exact(f,['table','provider','resource','visible','descriptor'])||!slotValid(f,m.limits.maxContextBytes)||typeof f.visible!=='boolean'||typeof f.descriptor!=='boolean'||!f.visible&&f.descriptor))return null;const keys=facts.map(key);if(new Set(keys).size!==facts.length||!equal([...keys].sort(),[...wanted].sort()))return null;const root=invocationAuthorityRoot(m.domain);if(size({root,scope:c,head,facts})>m.limits.maxPacketBytes)return null;const packet=token('registration-event-delivery/1');m.packets.set(packet,{kind:'delivery',stream,scope:freeze(copy(c)),root,head,facts:freeze(copy(facts))});return packet;}
export function releaseRegistrationEvent(owner,stream,c,packet,g){const m=owners.get(owner),s=m&&access(m,stream,c,g);if(!s||g?.outsidePublicationGuard!==true)return no();if(g?.releaseAllowed!==true||g?.retained!==true){stopped(m,s);return no();}if(!gates(g,catalog.releaseGates))return no();if(s.gap)return gap(s);if(!s.queue.length)return {status:'empty'};const p=take(m,packet,'delivery',stream,c),head=s.queue[0];if(!p||p.head!==head)return no();const byKey=new Map(p.facts.map(f=>[key(f),f]));if(head.projection.some(r=>!byKey.get(r.key)?.visible||Object.hasOwn(r.value,'descriptor')&&!byKey.get(r.key).descriptor))return gap(s);const reply=envelope('projection',head.after,head.projection,head.before);if(size(reply)>m.limits.maxReplyBytes)return gap(s);s.queue.shift();return reply;}
export function resyncRegistrationObservation(owner,stream,c,packet,g){const m=owners.get(owner),s=m&&access(m,stream,c,g);if(!s||g?.outsidePublicationGuard!==true)return no();if(g?.releaseAllowed!==true||g?.retained!==true){stopped(m,s);return no();}if(!gates(g,catalog.releaseGates)||g.baselineAllowed!==true)return no();const p=take(m,packet,'snapshot',stream,c);if(!p)return no();const stage=project(m,s,p);if(!stage||stage.issued>=m.limits.maxIssued)return no();const generation=token('registration-observation-generation/1'),reply=envelope('baseline',generation,stage.projection);if(size(reply)>m.limits.maxReplyBytes)return no();s.queue=[];install(m,s,stage);m.issued++;s.gap=false;s.generation=generation;return reply;}
export function retireRegistrationEventPreparation(owner,packet,g){const m=owners.get(owner);if(!m?.packets.has(packet)||['command','command-receipt'].includes(m.packets.get(packet).kind)||!gates(g,['ownerControlAllowed','retentionReleased']))return no();m.packets.delete(packet);return {status:'retired'};}
export function stopRegistrationObservation(owner,stream,c,g){const m=owners.get(owner),s=m?.streams.get(stream);if(!s||!equal(s.scope,c)||g?.ownerControlAllowed!==true)return no();stopped(m,s);return {status:'stopped'};}
export function registrationObservationStopTargetOwned(owner,reference){return owners.get(owner)?.streams.has(reference)===true;}
export function retireRegistrationObservation(owner,stream,g){const m=owners.get(owner),s=m?.streams.get(stream);if(!s?.stopped||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return no();m.streams.delete(stream);return {status:'retired'};}
export function retireRegistrationEventRegistry(owner,g){const m=owners.get(owner);if(!m)return no();const f=Object.fromEntries(['ownerControlAllowed','retentionReleased','deliveryDrained'].map(k=>[k,g?.[k]===true]));if(owners.get(owner)!==m||m.streams.size||m.packets.size||!Object.values(f).every(Boolean))return no();if(!releaseInvocationOwnerAttachment(m.domain,m.parentAttachment,f))return no();owners.delete(owner);return {status:'retired'};}
// Trusted supervision only; never a recipient stream clock or ordinary metric label.
export function registrationEventUsage(owner){const m=owners.get(owner);return m?{streams:m.streams.size,preparations:m.packets.size,bindings:m.bindings,issued:m.issued,queued:[...m.streams.values()].reduce((n,s)=>n+s.queue.length,0),gaps:[...m.streams.values()].filter(s=>s.gap).length,stopped:[...m.streams.values()].filter(s=>s.stopped).length,declaredHeldBytes:m.charge}:null;}

// Separate original command identity, issued by the existing native registry.
// Commands and technical preparations share its prepaid bounded packet slots;
// neither a new owner/global issuer nor a semantic alias for a snapshot packet.
const commandOperations=['establish-bounded-stream','authorized-resynchronization'];
function boundedCommandData(value,bytes){return natural(bytes)&&natural(bytes+1)?copySettingsData(value,{maxRequestBytes:bytes,maxInputNodes:bytes+1,maxInputDepth:bytes+1})?.value??null:null;}
function commandFacts(m,g){const keys=[...new Set([...catalog.authorityGates,...catalog.releaseGates,'ownerControlAllowed','outsidePublicationGuard','baselineAllowed'])],out=Object.fromEntries(keys.map(k=>[k,g?.[k]===true]));out.registrationObservationInterpretationCurrent=g?.registrationObservationInterpretationCurrent===true;out.registrationObservationProfile=boundedCommandData(g?.registrationObservationProfile,m.limits.maxProfileBytes);return out;}
function commandBinding(m,command,operation,stream,c){const p=m?.packets.get(command);return p?.kind==='command'&&p.operation===operation&&p.stream===stream&&equal(p.scope,c)?p:null;}
export function registrationObservationCommandOwned(owner,command){return owners.get(owner)?.packets.get(command)?.kind==='command';}
export function issueRegistrationObservationCommand(owner,operation,stream,c,g){
 const m=owners.get(owner);if(!m||!commandOperations.includes(operation)||operation==='establish-bounded-stream'&&stream!==null||operation==='authorized-resynchronization'&&!m.streams.has(stream))return null;
 const scope=boundedCommandData(c,m.limits.maxContextBytes);if(!scope||!scopeValid(m,scope)||m.limits.maxPreparations<2)return null;
 const section=beginInvocationAdmissionMutation(m.domain);if(!section)return null;
 try{
  const f=commandFacts(m,g),root=currentRoot(m,f),s=stream===null?null:m.streams.get(stream);
  if(owners.get(owner)!==m||!root||f.ownerControlAllowed!==true||!gates(f,catalog.releaseGates)||f.outsidePublicationGuard!==true||m.packets.size>=m.limits.maxPreparations||m.issued>=m.limits.maxIssued||stream!==null&&(!s||s.stopped||!equal(s.scope,scope)))return null;
  const record={kind:'command',operation,stream,scope,root,generation:s?.generation??null,phase:'issued',executionStarted:false,result:null,historicalSlots:null},marker={kind:'registration-observation-command/1'};
  // Reserve the whole terminal reply before reference issuance or native work.
  const held=size([marker,{...record,phase:'executing'}])+m.limits.maxReplyBytes+m.limits.maxBindingBytes;
  if(!natural(held)||held>m.limits.maxPacketBytes||m.limits.maxReplyBytes<Math.max(size(no()),size({status:'command-settled',command:marker}),size({status:'observe-existing',command:marker})))return null;
  const command=token(marker.kind);m.packets.set(command,record);m.issued++;return command;
 }catch(cause){throw new Error('registration-observation-command-protocol-fault',{cause});}
 finally{endInvocationAdmissionMutation(m.domain,section);}
}
export function requestRegistrationObservationCommand(owner,command,operation,stream,c,preparation,g){
 const m=owners.get(owner);if(!m)return no();const scope=boundedCommandData(c,m.limits.maxContextBytes),p=scope&&commandBinding(m,command,operation,stream,scope);if(!p||p.phase==='executing'||p.phase==='faulted')return no();
 if(p.phase==='settled'){try{const granted=g?.ownerControlAllowed===true;return granted&&m.packets.get(command)===p&&p.phase==='settled'?{status:'observe-existing',command}:no();}catch(cause){throw new Error('registration-observation-command-protocol-fault',{cause});}}
 const prepared=m.packets.get(preparation),s=stream===null?null:m.streams.get(stream);
 if(p.phase!=='issued'||registrationCommandOwnerFaulted(m)||!invocationOrdinaryAdmissionOpen(m.domain)||prepared?.kind!=='snapshot'||prepared.stream!==stream||!equal(prepared.scope,scope)||prepared.root!==p.root||p.root!==invocationAuthorityRoot(m.domain)||stream!==null&&(!s||s.stopped||s.generation!==p.generation))return no();
 p.phase='executing';
 try{
  // The command is busy before trusted suppliers may reenter. Ordinary values
  // were already captured; native execution below receives only private facts.
  const f=commandFacts(m,g);
  if(f.ownerControlAllowed!==true||registrationCommandOwnerFaulted(m)||!invocationOrdinaryAdmissionOpen(m.domain)||m.packets.get(command)!==p||m.packets.get(preparation)!==prepared||p.root!==invocationAuthorityRoot(m.domain)||stream!==null&&m.streams.get(stream)?.generation!==p.generation){p.phase='issued';return no();}
  p.executionStarted=true;
  const result=operation==='establish-bounded-stream'?establishRegistrationObservation(owner,scope,preparation,f):resyncRegistrationObservation(owner,stream,scope,preparation,f);
  if(result.status==='baseline'){
   const nativeStream=m.streams.get(result.stream??stream),bindings=[...nativeStream.refs.values()];
   const historical=result.entries.map(entry=>{const binding=bindings.find(b=>b.reference===entry.reference);if(!binding)throw Error('original historical slot invariant');return {reference:entry.reference,slot:copy(binding.slot)};});
   if(size(historical)>m.limits.maxBindingBytes)throw Error('original historical slot bound');
   p.historicalSlots=freeze(historical);
  }
  p.result=freeze(result);p.phase='settled';return {status:'command-settled',command};
 }catch(cause){p.phase='faulted';throw new Error('registration-observation-command-protocol-fault',{cause});}
}
// First-party retained-fact seam, not a recipient receipt-release authorization.
// The original result remains as-observed, including its original opaque roles.
export function nativeRegistrationObservationCommandState(owner,command,c){
 const m=owners.get(owner),scope=m&&boundedCommandData(c,m.limits.maxContextBytes),p=scope&&m.packets.get(command);if(p?.kind!=='command'||!equal(p.scope,scope))return null;
 const result=p.result&&{...p.result,...(p.result.entries?{entries:p.result.entries.map(({reference,...value})=>({reference,...copy(value)}))}:{})};
 return {domain:m.domain,operation:p.operation,phase:p.phase,stream:p.stream,result};
}
export function retireRegistrationObservationCommand(owner,command,c,g){
 const m=owners.get(owner),scope=m&&boundedCommandData(c,m.limits.maxContextBytes),p=scope&&m.packets.get(command),phase=p?.phase,result=p?.result,started=p?.executionStarted;if(p?.kind!=='command'||p.phase==='executing'||p.phase==='faulted'&&p.executionStarted||!equal(p.scope,scope)||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])||m.packets.get(command)!==p||p.phase!==phase||p.result!==result||p.executionStarted!==started)return no();
 if(registrationCommandBorrowed(m,command))return no();
 m.packets.delete(command);return {status:'retired'};
}

// Receipt contexts are funded children in the existing registry packet ceiling.
// Their retained native command claims are actual records, not discharge flags.
function registrationCommandBorrowed(m,command){for(const q of m.packets.values())if(q.kind==='command-receipt'&&q.command===command)return true;return false;}
function receiptBinding(m,context,command,scope){const q=m?.packets.get(context),p=m?.packets.get(command);return q?.kind==='command-receipt'&&q.command===command&&equal(q.scope,scope)&&p?.kind==='command'&&p.phase==='settled'&&p.result?.status==='baseline'&&p.historicalSlots&&equal(p.scope,scope)?{q,p}:null;}
export function registrationObservationReceiptOwned(owner,context){return owners.get(owner)?.packets.get(context)?.kind==='command-receipt';}
// Trusted authorizer input, never a recipient query or disclosure grant. It
// avoids asking a host to reconstruct removed historical slots from a newer root.
export function registrationObservationReceiptAuthorizationBasis(owner,context,command,c){const m=owners.get(owner),scope=m&&boundedCommandData(c,m.limits.maxContextBytes),b=scope&&receiptBinding(m,context,command,scope);return b&&!b.q.stopped?b.p.historicalSlots.map(h=>copy(h.slot)):null;}
// First-party result correlation; a schema alone cannot establish provenance.
export function validateRegistrationObservationReceiptReply(owner,context,command,c,reply){
 const m=owners.get(owner),b=receiptBinding(m,context,command,c);if(!b||b.q.stopped)return false;const result=b.p.result,r=reply?.receipt;
 if(!exact(reply,['status','facet','receipt'])||reply.status!=='operation-receipt'||!exact(reply.facet,['kind'])||reply.facet.kind!=='registry-baseline'||!r||size(reply)>m.limits.maxReplyBytes||!equal(r,result)||r.generation!==result.generation||r.stream!==result.stream||r.entries.some((entry,i)=>entry.reference!==result.entries[i].reference)){
  b.q.stopped=true;throw new Error('registration-observation-receipt-protocol-fault');
 }return true;
}
export function openRegistrationObservationReceipt(owner,command,c,ceiling,g){
 const m=owners.get(owner);if(!m)return null;const input=boundedCommandData({scope:c,ceiling},m.limits.maxContextBytes),p=m.packets.get(command);
 if(!input||!exact(input.ceiling,['kind','allowed'])||input.ceiling.kind!=='registry-baseline'||input.ceiling.allowed!==true||p?.kind!=='command'||p.phase!=='settled'||p.result?.status!=='baseline'||!p.historicalSlots||!equal(input.scope,p.scope))return null;
 const section=beginInvocationAdmissionMutation(m.domain);if(!section)return null;
 try{
  const f=commandFacts(m,g),admitted=g?.operationReceiptAdmissionAllowed===true;
  if(!admitted||!currentRoot(m,f)||!gates(f,catalog.releaseGates)||f.ownerControlAllowed!==true||owners.get(owner)!==m||m.packets.get(command)!==p||p.phase!=='settled'||m.packets.size>=m.limits.maxPreparations||m.issued>=m.limits.maxIssued)return null;
  const context=token('registration-observation-receipt-context/1'),record={kind:'command-receipt',command,scope:freeze(input.scope),ceiling:freeze(input.ceiling),stopped:false,busy:false};
  const reply={status:'operation-receipt',facet:{kind:'registry-baseline'},receipt:p.result},held=size([context,record])+m.limits.maxReplyBytes+m.limits.maxBindingBytes;
  if(!natural(held)||held>m.limits.maxPacketBytes||size(reply)>m.limits.maxReplyBytes)return null;
  m.packets.set(context,record);m.issued++;return context;
 }catch(cause){throw new Error('registration-observation-receipt-protocol-fault',{cause});}finally{endInvocationAdmissionMutation(m.domain,section);}
}
function receiptFacts(m,q,g){
 const keys=[...new Set([...catalog.authorityGates,...catalog.releaseGates,...catalog.deliveryGates,'registrationObservationInterpretationCurrent','operationReceiptAllowed','receiptGrantActive'])],raw=Object.fromEntries(keys.map(k=>[k,g?.[k]])),f=Object.fromEntries(keys.map(k=>[k,raw[k]===true]));
 f.registrationObservationProfile=boundedCommandData(g?.registrationObservationProfile,m.limits.maxProfileBytes);
 if(['authorized','scope','continuous','live','retained','receiptGrantActive'].some(k=>raw[k]===false))q.stopped=true;
 return f;
}
function receiptCurrent(owner,m,context,command,scope,q,p,f){return owners.get(owner)===m&&m.packets.get(context)===q&&!q.stopped&&receiptBinding(m,context,command,scope)?.p===p&&q.ceiling.allowed===true&&currentRoot(m,f)&&gates(f,[...catalog.releaseGates,...catalog.deliveryGates,'operationReceiptAllowed','receiptGrantActive']);}
export function prepareRegistrationObservationReceipt(owner,context,command,c,facts,g){
 const m=owners.get(owner);if(!m)return null;const input=boundedCommandData({scope:c,facts},m.limits.maxPacketBytes),b=input&&receiptBinding(m,context,command,input.scope);if(!b||b.q.stopped||b.q.busy)return null;const {q,p}=b;q.busy=true;
 try{
  const f=receiptFacts(m,q,g);if(!receiptCurrent(owner,m,context,command,input.scope,q,p,f)||m.packets.size>=m.limits.maxPreparations||!Array.isArray(input.facts)||input.facts.length!==p.historicalSlots.length)return null;
  const rows=input.facts;if(rows.some(r=>!exact(r,['table','provider','resource','visible','descriptor'])||!slotValid(r,m.limits.maxContextBytes)||typeof r.visible!=='boolean'||typeof r.descriptor!=='boolean'||!r.visible&&r.descriptor))return null;
  const keys=rows.map(key),wanted=p.historicalSlots.map(h=>key(h.slot));if(new Set(keys).size!==rows.length||!equal([...keys].sort(),[...wanted].sort()))return null;
  const packet=token('registration-observation-receipt-delivery/1'),record={kind:'command-receipt-delivery',context,command,scope:freeze(input.scope),root:invocationAuthorityRoot(m.domain),result:p.result,facts:freeze(rows)};
  if(size([packet,record])>m.limits.maxPacketBytes)return null;m.packets.set(packet,record);return packet;
 }catch(cause){q.stopped=true;throw new Error('registration-observation-receipt-protocol-fault',{cause});}finally{q.busy=false;}
}
export function releaseRegistrationObservationReceipt(owner,context,command,c,packet,g){
 const m=owners.get(owner),scope=m&&boundedCommandData(c,m.limits.maxContextBytes),b=scope&&receiptBinding(m,context,command,scope);if(!b||b.q.stopped||b.q.busy)return no();const {q,p}=b;q.busy=true;
 try{
  const f=receiptFacts(m,q,g);if(!receiptCurrent(owner,m,context,command,scope,q,p,f))return no();const prepared=m.packets.get(packet);
  if(prepared?.kind!=='command-receipt-delivery'||prepared.context!==context||prepared.command!==command||!equal(prepared.scope,scope)||prepared.result!==p.result||prepared.root!==invocationAuthorityRoot(m.domain))return no();
  m.packets.delete(packet);const rights=new Map(prepared.facts.map(r=>[key(r),r]));
  if(p.historicalSlots.some((h,i)=>!rights.get(key(h.slot))?.visible||Object.hasOwn(p.result.entries[i],'descriptor')&&!rights.get(key(h.slot)).descriptor))return no();
  const result={...p.result,entries:p.result.entries.map(({reference,...value})=>({reference,...copy(value)}))},reply={status:'operation-receipt',facet:{kind:'registry-baseline'},receipt:result};
  if(size(reply)>m.limits.maxReplyBytes)throw Error('receipt reply invariant');return reply;
 }catch(cause){q.stopped=true;throw new Error('registration-observation-receipt-protocol-fault',{cause});}finally{q.busy=false;}
}
export function stopRegistrationObservationReceipt(owner,context,c,g){const m=owners.get(owner),scope=m&&boundedCommandData(c,m.limits.maxContextBytes),q=m?.packets.get(context);if(q?.kind!=='command-receipt'||!equal(q.scope,scope))return no();try{if(g?.ownerControlAllowed!==true||m.packets.get(context)!==q)return no();q.stopped=true;return {status:'stopped'};}catch(cause){q.stopped=true;throw new Error('registration-observation-receipt-protocol-fault',{cause});}}
export function retireRegistrationObservationReceipt(owner,context,c,g){const m=owners.get(owner),scope=m&&boundedCommandData(c,m.limits.maxContextBytes),q=m?.packets.get(context);if(q?.kind!=='command-receipt'||q.busy||!q.stopped||!equal(q.scope,scope))return no();try{if(!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])||m.packets.get(context)!==q||q.busy||!q.stopped)return no();for(const p of m.packets.values())if(p.kind==='command-receipt-delivery'&&p.context===context)return no();m.packets.delete(context);return {status:'retired'};}catch(cause){throw new Error('registration-observation-receipt-protocol-fault',{cause});}}
