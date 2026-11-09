// Owned F26 local bridge; no exporter, callback registration or native job kind.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {inspectDiagnosticCapture,bindDiagnosticDrain,inspectDiagnosticDrainBinding,drainDiagnosticBinding,releaseDiagnosticDrainBinding} from './diagnostic-capture.mjs';
import {decodeTyped,decodeDynamic,planRoute} from './operation-catalog.mjs';
import {beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment,releaseInvocationOwnerAttachment,beginInvocationAdmissionMutation,endInvocationAdmissionMutation,invocationOrdinaryAdmissionOpen} from './invocation-publication.mjs';
const C=JSON.parse(fs.readFileSync(new URL('./diagnostic-drain.json',import.meta.url))),Q=JSON.parse(fs.readFileSync(new URL('./diagnostic-capture.json',import.meta.url))),families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url))),owners=new WeakMap();
const size=v=>Buffer.byteLength(JSON.stringify(v)),nat=v=>Number.isSafeInteger(v)&&v>=0,exact=(v,keys)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort()),gates=(g,keys)=>keys.every(k=>g?.[k]===true),token=kind=>Object.freeze({kind}),no=()=>({status:'unavailable'}),text=s=>typeof s==='string'&&s.length>0&&Buffer.from(s).toString('utf8')===s;
const taskToken=()=>token('diagnostic-drain-task/1'),bindingG=()=>({outsidePublicationGuard:true});
function outside(m,c,g){return !!m&&equal(c,m.context)&&g?.outsidePublicationGuard===true;}
function drop(m,e){if(e.batch!==null){e.batch=null;m.releasedPayloads++;}}
function fence(m,fault=false){m.stopped=true;m.faulted||=fault;for(const e of m.tasks.values()){if(e.phase==='pending'){e.phase='terminal';e.disposition='notPerformed';}drop(m,e);}}
function current(m,c,g){if(!outside(m,c,g))return false;if(!gates(g,C.authorityGates)){fence(m);return false;}return !m.stopped;}
const inspect=m=>inspectDiagnosticDrainBinding(m.binding,m.domain,m.queueContext,bindingG());
function control(task,e){return {status:'control',task,phase:e.phase,admitted:e.intent!==null,...(e.phase==='terminal'?{drain:e.disposition}:{})};}
function result(task,e,batch){return {status:'result',task,admitted:e.intent!==null,drain:e.disposition,batch};}
export function diagnosticDrainCharge(l){if(!exact(l,C.limits)||!Object.values(l).every(nat))return null;const n=2*l.maxContextBytes+l.maxBindingBytes+l.maxTasks*(l.maxRequestBytes+l.maxBatchBytes+2*l.maxResultBytes+2*l.maxControlReplyBytes);return nat(n)&&n<=l.maxHeldBytes?n:null;}
export function openDiagnosticDrain(domain,queueOwner,queueContext,c,limits,g){
 const attachment=beginInvocationOwnerAttachment(domain,'diagnostic-drain',limits?.maxHeldBytes,token('diagnostic-drain-owner/1'));if(!attachment)return null;
 try{
 const charge=diagnosticDrainCharge(limits);if(charge===null||!gates(g,C.openGates)||!exact(c,C.contextFields)||!Object.values(c).every(text)||c.profile!==C.profile||size(c)>limits.maxContextBytes||!exact(queueContext,Q.contextFields)||!Object.values(queueContext).every(text)||size(queueContext)>limits.maxContextBytes||!equal({...c,profile:Q.profile},queueContext))return null;
 const q=inspectDiagnosticCapture(queueOwner,domain,queueContext,g);if(!q)return null;
 const task=taskToken(),maximumControl=control(task,{phase:'terminal',intent:null,disposition:'notPerformed'}),overhead=size(result(task,{intent:null,disposition:'notPerformed'},null))-4,request={contract:C.contract,task,generation:q.generation,limit:q.maxDrainRecords};
 if(limits.maxBindingBytes<size({kind:'diagnostic-drain-binding/1',context:queueContext})||limits.maxRequestBytes<size(request)||limits.maxBatchBytes<q.maxReplyBytes||!nat(q.maxReplyBytes+overhead)||limits.maxResultBytes<q.maxReplyBytes+overhead||limits.maxControlReplyBytes<size(maximumControl))return null;
 const binding=bindDiagnosticDrain(queueOwner,domain,queueContext,g);if(!binding)return null;const owner=token('diagnostic-drain-owner/1');owners.set(owner,{domain,parentAttachment:attachment,binding,queueContext:Object.freeze(structuredClone(queueContext)),context:Object.freeze(structuredClone(c)),limits:Object.freeze(structuredClone(limits)),charge,queueReplyBytes:q.maxReplyBytes,tasks:new Map(),issued:0,stopped:false,faulted:false,releasedPayloads:0});if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native diagnostic drain attachment invariant');return owner;
 }finally{abortInvocationOwnerAttachment(domain,attachment);}
}
export function issueDiagnosticDrain(owner,c,g){const m=owners.get(owner),section=m&&beginInvocationAdmissionMutation(m.domain);if(!section)return no();try{if(!current(m,c,g)||!gates(g,C.admissionGates)||m.tasks.size>=m.limits.maxTasks||m.issued>=m.limits.maxIssued)return no();const q=inspect(m);if(!q||q.mode!=='enabled')return no();const task=taskToken();m.tasks.set(task,{phase:'issued',generation:q.generation,intent:null,disposition:'notPerformed',batch:null});m.issued++;return {status:'issued',task,generation:q.generation};}finally{endInvocationAdmissionMutation(m.domain,section);}}
function normalize(request,route){const identity=route==='typed'?'family':route==='dynamic'?'name':null;if(!identity||!exact(request,[identity,'variant','version','arguments'])||!exact(request.arguments,C.argumentFields)||request.arguments.contract!==C.contract)return null;const d=(route==='typed'?decodeTyped:decodeDynamic)(families,Object.fromEntries([identity,'variant','version'].map(k=>[k,request[k]])));return d?.descriptor.id===C.family&&d.variant===C.variant?{decoded:d,args:request.arguments}:null;}
export function submitDiagnosticDrain(owner,request,route,c,g){
 const m=owners.get(owner),section=m&&beginInvocationAdmissionMutation(m.domain,true);if(!section)return no();try{if(!current(m,c,g)||!gates(g,C.admissionGates))return no();const r=normalize(request,route),e=r&&m.tasks.get(r.args.task);if(!e||r.args.generation!==e.generation||!Number.isSafeInteger(r.args.limit)||r.args.limit<1||size(r.args)>m.limits.maxRequestBytes)return no();
 const routed=planRoute(r.decoded,{schemaValid:true,enabled:true,grant:g.drainAllowed===true,current:true,ordinaryCapacity:g.resourcesReady===true,releaseAllowed:false});if(routed.status!=='routed'||routed.sink!=='hostRequest')return no();
 if(e.phase!=='issued')return e.intent&&e.intent.generation===r.args.generation&&e.intent.limit===r.args.limit?{status:'observe-existing',task:r.args.task}:no();
 if(!invocationOrdinaryAdmissionOpen(m.domain))return no();
 const q=inspect(m);if(!q||q.mode!=='enabled'||q.generation!==e.generation||r.args.limit>q.maxDrainRecords)return no();e.intent=Object.freeze({generation:r.args.generation,limit:r.args.limit});e.phase='pending';return {status:'accepted',task:r.args.task};
 }finally{endInvocationAdmissionMutation(m.domain,section);}
}
function validBatch(b,e,m){return exact(b,['status','generation','coverage','records','remaining'])&&b.status==='batch'&&b.generation===e.generation&&b.coverage==='optional-capture'&&typeof b.remaining==='boolean'&&Array.isArray(b.records)&&b.records.length<=e.intent.limit&&b.records.every(r=>exact(r,['schema','site','milestone'])&&r.schema===Q.record&&Q.sites.includes(r.site)&&Q.milestones.includes(r.milestone))&&size(b)<=m.limits.maxBatchBytes&&size(b)<=m.queueReplyBytes;}
export function runDiagnosticDrain(owner,task,c,g){
 const m=owners.get(owner);if(!current(m,c,g)||!gates(g,C.serviceGates))return no();const e=m.tasks.get(task);if(!e||e.phase!=='pending')return no();
 const q=inspect(m);if(!q||q.mode!=='enabled'||q.generation!==e.generation||g.drainAllowed!==true){e.phase='terminal';e.disposition='notPerformed';return {status:'settled'};}
 try{const b=drainDiagnosticBinding(m.binding,m.domain,m.queueContext,{contract:Q.contract,operation:'drain',generation:e.generation,limit:e.intent.limit},{...g,releaseAllowed:g.queueReleaseAllowed===true});if(exact(b,['status'])&&b.status==='unavailable'){e.disposition='notPerformed';e.phase='terminal';return {status:'settled'};}if(!validBatch(b,e,m))throw new Error('invalid private drain result');e.batch=b;e.disposition='performed';e.phase='terminal';return {status:'settled'};}
 catch{e.phase='terminal';e.disposition='unknown';fence(m,true);return {status:'faulted'};}
}
export function queryDiagnosticDrain(owner,task,facet,c,g){
 const m=owners.get(owner),e=m?.tasks.get(task);if(!e||!outside(m,c,g))return no();
 if(facet==='control'){if(!gates(g,C.controlGates))return no();const reply=control(task,e);return size(reply)<=m.limits.maxControlReplyBytes?reply:no();}
 if(facet!=='result'||!current(m,c,g)||!gates(g,C.resultGates)||e.phase!=='terminal')return no();const q=inspect(m);if(!q||q.generation!==e.generation)drop(m,e);
 const batch=e.batch===null?null:{...structuredClone(e.batch),generation:e.batch.generation},reply=result(task,e,batch);return size(reply)<=m.limits.maxResultBytes?reply:no();
}
export function stopDiagnosticDrainTask(owner,task,c,g){const m=owners.get(owner),e=m?.tasks.get(task);if(!e||!outside(m,c,g)||!gates(g,['ownerControlAllowed','controlAllowed']))return no();if(e.phase!=='terminal'){e.phase='terminal';e.disposition='notPerformed';}drop(m,e);return {status:'stopped'};}
export function retireDiagnosticDrainTask(owner,task,g){const m=owners.get(owner),e=m?.tasks.get(task);if(!e||e.phase==='pending'||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained','outsidePublicationGuard']))return no();drop(m,e);m.tasks.delete(task);return {status:'retired'};}
export function stopDiagnosticDrain(owner,c,g){const m=owners.get(owner);if(!outside(m,c,g)||!gates(g,['ownerControlAllowed','controlAllowed']))return no();fence(m);return {status:'stopped'};}
export function retireDiagnosticDrain(owner,g){const m=owners.get(owner);if(!m||!m.stopped||m.tasks.size||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained','outsidePublicationGuard']))return no();const r=releaseDiagnosticDrainBinding(m.binding,m.domain,m.queueContext,g);if(r.status!=='retired')return no();if(!releaseInvocationOwnerAttachment(m.domain,m.parentAttachment,g))return no();owners.delete(owner);return {status:'retired'};}
export function diagnosticDrainUsage(owner){const m=owners.get(owner);return m?{tasks:m.tasks.size,issued:m.issued,phases:[...m.tasks.values()].map(e=>e.phase),retainedBatches:[...m.tasks.values()].filter(e=>e.batch!==null).length,releasedPayloads:m.releasedPayloads,stopped:m.stopped,faulted:m.faulted,declaredHeldBytes:m.charge}:null;}
