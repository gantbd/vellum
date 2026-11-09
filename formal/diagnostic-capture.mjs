// Sequential reference, not installed critical-path instrumentation or an SDK.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {invocationAuthorityContext,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment,releaseInvocationOwnerAttachment} from './invocation-publication.mjs';
const C=JSON.parse(fs.readFileSync(new URL('./diagnostic-capture.json',import.meta.url))),owners=new WeakMap(),domains=new WeakMap(),drainBindings=new WeakMap();
const size=v=>Buffer.byteLength(JSON.stringify(v)),nat=v=>Number.isSafeInteger(v)&&v>=0,exact=(v,keys)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort()),gates=(g,keys)=>keys.every(k=>g?.[k]===true),token=kind=>Object.freeze({kind}),no=()=>({status:'unavailable'});
const text=s=>typeof s==='string'&&s.length>0&&Buffer.from(s).toString('utf8')===s;
const counters=()=>Object.fromEntries(C.counters.map(k=>[k,{value:0,exact:true}]));
const recordValid=r=>exact(r,['schema','site','milestone'])&&r.schema===C.record&&C.sites.includes(r.site)&&C.milestones.includes(r.milestone);
const generation=()=>token('diagnostic-generation/1');
function increment(m,key,n=1){const c=m.counters[key],room=m.limits.maxCounter-c.value;if(n>room){c.value=m.limits.maxCounter;c.exact=false;}else c.value+=n;}
function discard(m){increment(m,'discarded',m.queue.length);m.queue=[];m.bytes=0;}
function stop(m,mode='stopped'){discard(m);m.preparations.clear();m.mode=mode;}
// The complete primitive context was bounded at open; compare it without encoding.
function scopeValid(m,c){return exact(c,C.contextFields)&&equal(c,m.context);}
function current(m,c,g){if(!m||!scopeValid(m,c))return false;if(!gates(g,C.authorityGates)){stop(m);return false;}return m.mode==='enabled';}
function health(m){return {status:'health',generation:m.generation,coverage:'optional-capture',mode:m.mode,clock:m.clock,counters:structuredClone(m.counters),occupancy:{records:m.queue.length,bytes:m.bytes}};}
export function diagnosticCaptureCharge(l){
 if(!exact(l,C.limits)||!Object.values(l).every(nat)||l.maxGenerations<1||l.maxRetentionSteps<1||l.maxDrainRecords<1)return null;
 const largest=Math.max(...C.sites.flatMap(site=>C.milestones.map(milestone=>size({schema:C.record,site,milestone})))),sample={status:'batch',generation:generation(),coverage:'optional-capture',records:[{schema:C.record,site:'INS-01',milestone:'discarded'}],remaining:false};
 const worstHealth={status:'health',generation:generation(),coverage:'optional-capture',mode:'disabled',clock:'unobserved',counters:Object.fromEntries(C.counters.map(k=>[k,{value:l.maxCounter,exact:false}])),occupancy:{records:l.maxRecords,bytes:l.maxQueueBytes}};
 if(l.maxRecordBytes<largest||l.maxReplyBytes<size(sample)||l.maxHealthReplyBytes<size(worstHealth))return null;
 const charge=l.maxContextBytes+l.maxQueueBytes+l.maxPreparations*l.maxRecordBytes+2*l.maxReplyBytes+2*l.maxHealthReplyBytes;
 return nat(charge)&&charge<=l.maxHeldBytes?charge:null;
}
export function openDiagnosticCapture(domain,c,limits,g){
 const attachment=beginInvocationOwnerAttachment(domain,'diagnostic-capture',limits?.maxHeldBytes,token('diagnostic-capture-owner/1'));if(!attachment)return null;
 try{
 const native=invocationAuthorityContext(domain),charge=diagnosticCaptureCharge(limits);
 if(!native||domains.has(domain)||charge===null||!exact(c,C.contextFields)||!Object.values(c).every(text)||c.authority!==native.authority||c.engine!==native.engine||c.profile!==C.profile||size(c)>limits.maxContextBytes||!gates(g,['ownerControlAllowed','resourcesReady','preparationReserved']))return null;
 const owner=token('diagnostic-capture-owner/1');owners.set(owner,{domain,parentAttachment:attachment,context:Object.freeze(structuredClone(c)),limits:Object.freeze(structuredClone(limits)),charge,preparations:new Map(),issued:0,generations:1,generation:generation(),mode:'enabled',clock:'unobserved',queue:[],bytes:0,counters:counters(),drainHold:null,drainHoldSpent:false});domains.set(domain,owner);if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native diagnostic capture attachment invariant');return owner;
 }finally{abortInvocationOwnerAttachment(domain,attachment);}
}
// Only this preparation inspects/copies caller data; capture accepts an owned token.
export function prepareDiagnosticCapture(owner,c,record,g){
 const m=owners.get(owner);if(!current(m,c,g)||!gates(g,C.prepareGates))return null;
 if(!recordValid(record)){increment(m,'invalid');return null;}
 if(m.preparations.size>=m.limits.maxPreparations||m.issued>=m.limits.maxPreparedIssued){increment(m,'preparationRefused');return null;}
 const bytes=size(record);if(bytes>m.limits.maxRecordBytes){increment(m,'invalid');return null;}
 const ref=token('diagnostic-preparation/1');m.preparations.set(ref,{record:Object.freeze(structuredClone(record)),bytes,generation:m.generation});m.issued++;return ref;
}
export function captureDiagnostic(owner,c,prepared,g){
 const m=owners.get(owner);if(!current(m,c,g)||!gates(g,C.captureGates))return no();const p=m.preparations.get(prepared);if(!p||p.generation!==m.generation)return no();
 m.preparations.delete(prepared);increment(m,'offered');
 if(m.queue.length>=m.limits.maxRecords||p.bytes>m.limits.maxQueueBytes-m.bytes){increment(m,'dropped');return {status:'dropped'};}
 m.queue.push({record:p.record,bytes:p.bytes,remaining:m.limits.maxRetentionSteps});m.bytes+=p.bytes;increment(m,'captured');return {status:'captured'};
}
export function drainDiagnostics(owner,c,request,g){
 const m=owners.get(owner);if(!current(m,c,g)||!gates(g,C.drainGates)||!exact(request,['contract','operation','generation','limit'])||request.contract!==C.contract||request.operation!=='drain'||request.generation!==m.generation||!Number.isSafeInteger(request.limit)||request.limit<1||request.limit>m.limits.maxDrainRecords)return no();
 const n=Math.min(request.limit,m.queue.length),reply={status:'batch',generation:m.generation,coverage:'optional-capture',records:m.queue.slice(0,n).map(p=>p.record),remaining:n<m.queue.length};
 if(size(reply)>m.limits.maxReplyBytes)return no();
 const removed=m.queue.splice(0,n);m.bytes-=removed.reduce((v,p)=>v+p.bytes,0);increment(m,'drained',n);return reply;
}
export function readDiagnosticHealth(owner,c,g){const m=owners.get(owner);if(!m||!scopeValid(m,c)||!gates(g,C.healthGates))return no();const reply=health(m);return size(reply)<=m.limits.maxHealthReplyBytes?reply:no();}
export function advanceDiagnosticRetention(owner,c,steps,g){
 const m=owners.get(owner);if(!m||!scopeValid(m,c)||m.mode!=='enabled'||!gates(g,['ownerControlAllowed','outsidePublicationGuard'])||!Number.isSafeInteger(steps)||steps<1||steps>m.limits.maxRetentionSteps)return no();
 if(g.clockValid!==true){m.clock='invalid';stop(m);return {status:'stopped'};}
 m.clock='valid';const kept=[];let discarded=0;for(const p of m.queue){if(steps>=p.remaining){m.bytes-=p.bytes;discarded++;}else{p.remaining-=steps;kept.push(p);}}m.queue=kept;increment(m,'discarded',discarded);return {status:'advanced'};
}
export function stopDiagnosticCapture(owner,c,g){const m=owners.get(owner);if(!m||!scopeValid(m,c)||g?.ownerControlAllowed!==true)return no();stop(m,g.disabled===true?'disabled':'stopped');return {status:'stopped'};}
export function resetDiagnosticCapture(owner,c,g){const m=owners.get(owner);if(!m||!scopeValid(m,c)||m.mode==='enabled'||!gates(g,['ownerControlAllowed','resourcesReady','preparationReserved','outsidePublicationGuard'])||m.generations>=m.limits.maxGenerations)return no();m.generation=generation();m.generations++;m.mode='enabled';m.clock='unobserved';m.counters=counters();return {status:'reset',generation:m.generation};}
export function retireDiagnosticPreparation(owner,ref,g){const m=owners.get(owner);if(!m?.preparations.has(ref)||!gates(g,['ownerControlAllowed','retentionReleased']))return no();m.preparations.delete(ref);return {status:'retired'};}
export function retireDiagnosticCapture(owner,g){const m=owners.get(owner);if(!m||m.mode==='enabled'||m.preparations.size||m.queue.length||m.drainHold!==null||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return no();if(!releaseInvocationOwnerAttachment(m.domain,m.parentAttachment,g))return no();owners.delete(owner);return {status:'retired'};}
// Trusted component ports, not public recipient selectors or health permissions.
export function inspectDiagnosticCapture(owner,domain,c,g){const m=owners.get(owner);if(!m||m.domain!==domain||!scopeValid(m,c)||!gates(g,['ownerControlAllowed','outsidePublicationGuard']))return null;return Object.freeze({generation:m.generation,mode:m.mode,maxDrainRecords:m.limits.maxDrainRecords,maxReplyBytes:m.limits.maxReplyBytes});}
export function bindDiagnosticDrain(owner,domain,c,g){const m=owners.get(owner);if(!inspectDiagnosticCapture(owner,domain,c,g)||m.drainHoldSpent||!gates(g,['bindingReady','preparationReserved']))return null;const binding=token('diagnostic-drain-binding/1');m.drainHold=binding;m.drainHoldSpent=true;drainBindings.set(binding,{owner,domain});return binding;}
function boundDrain(binding,domain,c,g){const b=drainBindings.get(binding),m=b&&owners.get(b.owner);return m&&b.domain===domain&&m.drainHold===binding&&scopeValid(m,c)&&g?.outsidePublicationGuard===true?{b,m}:null;}
export function inspectDiagnosticDrainBinding(binding,domain,c,g){const x=boundDrain(binding,domain,c,g);return x?Object.freeze({generation:x.m.generation,mode:x.m.mode,maxDrainRecords:x.m.limits.maxDrainRecords,maxReplyBytes:x.m.limits.maxReplyBytes}):null;}
export function drainDiagnosticBinding(binding,domain,c,request,g){const x=boundDrain(binding,domain,c,g);return x?drainDiagnostics(x.b.owner,c,request,g):no();}
export function releaseDiagnosticDrainBinding(binding,domain,c,g){const x=boundDrain(binding,domain,c,g);if(!x||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return no();x.m.drainHold=null;drainBindings.delete(binding);return {status:'retired'};}
// Trusted supervision only: not a recipient metric label or public native clock.
export function diagnosticCaptureUsage(owner){const m=owners.get(owner);return m?{preparations:m.preparations.size,issued:m.issued,generations:m.generations,queued:m.queue.length,bytes:m.bytes,mode:m.mode,declaredHeldBytes:m.charge}:null;}
