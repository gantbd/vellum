import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as observation from './native-operation-observation.mjs';
import {sourceLoadRequestFixture} from './source-load-request-checks.mjs';
import {annotationLoadRequestFixture} from './annotation-load-request-checks.mjs';
import {sourceSaveRequestFixture,annotationSaveRequestFixture} from './ordinary-save-request-checks.mjs';
const copy=structuredClone,no={status:'unavailable'},kinds=['source-load','annotation-load','source-save','annotation-save'];
const limits={maxWaiters:2,maxIssued:64,maxWaitSteps:3,maxContextBytes:8192,maxWaitBytes:256,maxReferenceBytes:128,maxReplyBytes:1048576,maxControlReplyBytes:256,maxHeldBytes:8388608};
const fields=['authorized','scope','continuous','live','resourcesReady','retained','transferReady','preparationReserved','registryReady','retentionReady','controlAllowed','operationReceiptAdmissionAllowed','operationReceiptAllowed','replyReady','sourceReadable','annotationReadable','readAllowed','dispositionReadable','ordinaryReady','controlReady','waitCapacity','waitClockValid','ownerControlAllowed','retentionReleased','deliveryDrained'];
const good=Object.fromEntries(fields.map(k=>[k,true])),policy=c=>({publication:false,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:c.recipient});
const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url))),family=families.find(f=>f.id==='F06');
function fixture(kind,api=observation,options={}){
 const x=kind==='source-load'?sourceLoadRequestFixture():kind==='annotation-load'?annotationLoadRequestFixture():kind==='source-save'?sourceSaveRequestFixture():annotationSaveRequestFixture();
 const c=copy(x.context),p=policy(c),request=kind.endsWith('load')?x.request():x.args(),a=x.invoke(request);assert.equal(a.status,'admitted');
 const owner=kind==='annotation-save'?x.x.owner:kind==='source-save'?x.writer:x.owner,domain=kind==='annotation-save'?x.z.domain:x.domain;
 const h=api.openNativeOperationObservation(kind,owner,a.task,c,p,{...limits,...options.limits},{...good,...options.grants});
 const form=(op,extra={})=>({contract:op==='retained-outcome'?'native-operation-receipt/1':'native-operation-observation/1',operation:op,task:a.task,context:h,profile:c.profile,...(op==='retained-outcome'?{facet:{kind}}:{}),...extra});
 const query=(op='status',g={},q=p,r=form(op),scope=c)=>api.queryNativeOperationObservation(h,r,scope,q,{...good,...g});
 const envelope=(r,route)=>({[route==='typed'?'family':'name']:route==='typed'?families.indexOf(family):family.name,variant:r.operation,version:1,arguments:r});
 const stop=()=>kind==='source-load'?x.r.controlResourceRead(owner,a.task,'cancel',{controlAllowed:true}):kind==='annotation-load'?x.api.controlInvocationAnnotationRead(owner,a.task,'cancel',x.g()):x.api.controlSourceSave(owner,a.task,'cancel',{controlAllowed:true});
 return {x,c,p,owner,domain,task:a.task,h,kind,api,form,query,envelope,stop};
}
export {fixture as nativeOperationObservationFixture,limits as nativeOperationObservationLimits,good as nativeOperationObservationGates};
export function checkNativeOperationObservation(api=observation){
 const controls=[];let routeDeliveries=0;const check=(n,f)=>{f();controls.push(n);};
 check('four-actual-parent-kinds-and-two-closed-routes',()=>{
  for(const kind of kinds){const z=fixture(kind,api);assert(z.h);const s=z.query();assert.equal(s.status,'native-execution');assert.equal(s.phase,'pending');assert.equal(api.nativeOperationReceiptValid(s.receipt,kind),true);
   const n=api.nativeOperationObservationUsage(z.h).ordinaryIssued;
   for(const route of ['typed','dynamic'])for(const op of ['status','retained-outcome','minimal-control-status']){
    const r=api.invokeNativeOperationObservation(z.h,z.envelope(z.form(op),route),route,z.c,z.p,good);assert.equal(r.status,op==='retained-outcome'?'native-operation-receipt':'native-execution');assert.equal(r.observation,op==='minimal-control-status'?z.query(op).observation:s.observation);routeDeliveries++;
    assert(!Object.hasOwn(r,'publication'));assert(!Object.hasOwn(r.receipt??{},'publication'));assert(!Object.hasOwn(r.receipt??{},'export'));
   }assert.equal(api.nativeOperationObservationUsage(z.h).ordinaryIssued,n);
  }
 });
 check('actual-parent-scope-kind-and-task-not-copied-markers',()=>{
  for(const kind of kinds){const z=fixture(kind,api);for(const owner of [{},{...z.owner}])assert.equal(api.openNativeOperationObservation(kind,owner,z.task,z.c,z.p,limits,good),null);
   for(const key of Object.keys(z.c))assert.deepEqual(z.query('status',{},z.p,z.form('status'),{...z.c,[key]:z.c[key]+'-foreign'}),no);
   assert.deepEqual(z.query('status',{},z.p,z.form('status',{task:{...z.task}})),no);
   assert.deepEqual(z.query('status',{},z.p,z.form('status',{context:{...z.h}})),no);
   assert.deepEqual(z.query('retained-outcome',{},z.p,z.form('retained-outcome',{facet:{kind:kind==='source-load'?'annotation-load':'source-load'}})),no);
  }
 });
 check('actual-raw-task-does-not-acquire-command-parent-authority',()=>{
  const z=fixture('source-load',api),request=z.x.request('raw-only'),a=z.x.r.admitInvocationSourceRead(z.owner,request,z.x.grants());assert.equal(a.status,'admitted');assert.equal(api.openNativeOperationObservation(z.kind,z.owner,a.task,z.c,z.p,limits,good),null);
 });
 check('explicit-captured-ceilings-and-strict-current-native-grants',()=>{
  for(const kind of kinds){const z=fixture(kind,api);for(const key of ['operationReceiptAllowed',kind.startsWith('annotation')?'annotationReadable':'sourceReadable',kind.endsWith('load')?'readAllowed':'dispositionReadable'])for(const value of [false,1,undefined]){assert.deepEqual(z.query('retained-outcome',{[key]:value}),no);assert.equal(z.query('minimal-control-status',{[key]:value,ordinaryReady:false,resourcesReady:false}).status,'native-execution');}
   assert.deepEqual(z.query('retained-outcome',{}, {...z.p,effect:false}),no);assert.equal(z.query().receipt.kind,kind);
   const h=api.openNativeOperationObservation(kind,z.owner,z.task,z.c,{...z.p,effect:false},limits,{...good,operationReceiptAdmissionAllowed:false});assert(h);
   assert.deepEqual(api.queryNativeOperationObservation(h,{...z.form('retained-outcome'),context:h},z.c,z.p,good),no);
   assert.equal(api.openNativeOperationObservation(kind,z.owner,z.task,z.c,z.p,limits,{...good,operationReceiptAdmissionAllowed:1}),null);
   assert.equal(api.openNativeOperationObservation(kind,z.owner,z.task,z.c,{...z.p,publication:true},limits,good),null);
  }
 });
 check('observer-stop-and-wait-expiry-do-not-stop-producer',()=>{
  for(const kind of kinds){const z=fixture(kind,api),s=z.query(),r=z.form('bounded-observation-wait',{after:s.observation,budget:2});
   const w=api.startNativeOperationWait(z.h,r,z.c,z.p,good);assert.equal(w.status,'waiting');
   assert.equal(api.advanceNativeOperationWait(z.h,w.wait,'tick',z.c,z.p,good).status,'waiting');assert.equal(api.advanceNativeOperationWait(z.h,w.wait,'tick',z.c,z.p,good).status,'observation-ended');
   assert.equal(z.query().stopped,false);assert.equal(api.retireNativeOperationObservation(z.h,good).status,'unavailable');
   assert.equal(api.retireNativeOperationWait(z.h,w.wait,good).status,'retired');assert.equal(api.closeNativeOperationObservation(z.h,z.c,good).status,'stopped');assert.deepEqual(z.query(),no);
   const next=api.openNativeOperationObservation(kind,z.owner,z.task,z.c,z.p,limits,good);assert(next);assert.equal(api.queryNativeOperationObservation(next,{...z.form('status'),context:next},z.c,z.p,good).stopped,false);
   assert.equal(api.retireNativeOperationObservation(z.h,good).status,'retired');
  }
 });
 check('native-stop-is-not-quiescence-or-known-nonapplication',()=>{
  for(const kind of kinds){const z=fixture(kind,api);assert(z.h);const before=z.query();assert(['stopped','unchanged'].includes(z.stop().status));
   const s=z.query('minimal-control-status',{ordinaryReady:false,resourcesReady:false,operationReceiptAllowed:false});assert.equal(s.phase,'terminal');assert.equal(s.stopped,true);assert.equal(s.quiescent,false);assert.notEqual(s.observation,before.observation);
   const r=z.query('retained-outcome');assert.equal(r.status,'native-operation-receipt');if(kind.endsWith('save'))assert.equal(r.receipt.knowledge.application,'unknown');else assert.deepEqual(r.receipt.witnesses,[]);
  }
 });
 check('read-completion-and-late-chunk-conflict-never-disclose-content',()=>{
  for(const kind of ['source-load','annotation-load']){
   const z=fixture(kind,api),x=z.x;const before=kind==='source-load'?x.root():x.root();
   if(kind==='source-load')x.events.set(z.task,x.request());
   assert.equal(x.submit(z.task).status,'submitted');x.complete(z.task);
   const r=z.query('retained-outcome');assert.equal(r.status,'native-operation-receipt');assert.equal(r.receipt.witnesses.length,1);assert.equal(r.receipt.witnesses[0].kind,'end');assert.equal(r.receipt.quiescent,false);assert.equal(x.root(),before);
   const conflict=x.observe(z.task,'chunk',{sequence:0,offset:0,bytes:[1]});assert.equal(conflict.status,'conflict');
   const late=z.query('retained-outcome');assert.equal(late.receipt.contested,true);assert(!JSON.stringify(late).includes('"bytes"'));assert.equal(late.receipt.witnesses.length,1);
   assert.equal(x.observe(z.task,'quiescence').status,'quiescent');assert.equal(z.query('minimal-control-status').quiescent,true);
  }
 });
 check('actual-write-acknowledgment-is-neither-baseline-adoption-nor-quiescence',()=>{
  for(const kind of ['source-save','annotation-save']){
   const z=fixture(kind,api),x=z.x,root=kind==='source-save'?x.root():x.z.current();assert.equal(x.dispatch(z.task).status,'dispatched');
   assert.equal((kind==='source-save'?x.written(z.task):x.x.result(z.task)).status,'recorded');
   const r=z.query('retained-outcome');assert.equal(r.status,'native-operation-receipt');assert.equal(r.receipt.knowledge.application,'written');assert.equal(r.receipt.quiescent,false);assert.equal(kind==='source-save'?x.root():x.z.current(),root);
   assert(!Object.hasOwn(r.receipt,'baseline'));assert(!Object.hasOwn(r.receipt,'bindingApplied'));assert(!Object.hasOwn(r.receipt,'export'));
   assert.equal((kind==='source-save'?x.observe(z.task,'quiescence'):x.x.observe(z.task,'quiescence')).status,'recorded');assert.equal(z.query('minimal-control-status').quiescent,true);
  }
 });
 check('retired-producer-becomes-unavailable-without-resurrection-or-reexecution',()=>{
  const z=fixture('source-load',api);z.stop();assert.equal(z.x.r.retireInvocationSourceRead(z.owner,z.task,{retirementAllowed:true,retentionReleased:true,deliveryDrained:true}).status,'retired');
  assert.deepEqual(z.query('retained-outcome'),no);assert.equal(api.nativeOperationObservationUsage(z.h).live,false);assert.equal(api.openNativeOperationObservation(z.kind,z.owner,z.task,z.c,z.p,limits,good),null);assert.equal(api.retireNativeOperationObservation(z.h,good).status,'retired');
 });
 check('wait-observes-real-native-change-not-a-receipt-query',()=>{
  for(const kind of kinds){const z=fixture(kind,api),s=z.query();z.query('retained-outcome');const w=api.startNativeOperationWait(z.h,z.form('bounded-observation-wait',{after:s.observation,budget:2}),z.c,z.p,good);assert.equal(w.status,'waiting');assert.equal(api.advanceNativeOperationWait(z.h,w.wait,'poll',z.c,z.p,good).status,'waiting');z.stop();const out=api.advanceNativeOperationWait(z.h,w.wait,'poll',z.c,z.p,good);assert.equal(out.status,'observed');assert.equal(out.result.stopped,true);assert.equal(api.advanceNativeOperationWait(z.h,w.wait,'poll',z.c,z.p,good).status,'unavailable');}
 });
 check('ordinary-exhaustion-preserves-protected-control-and-stopped-wait-capacity',()=>{
  for(const kind of kinds){const z=fixture(kind,api,{limits:{maxIssued:1}});assert(z.h);const s=z.query();const w=api.startNativeOperationWait(z.h,z.form('bounded-observation-wait',{after:s.observation,budget:1}),z.c,z.p,good);assert.equal(w.status,'waiting');z.stop();assert.deepEqual(z.query(),no);assert.equal(z.query('minimal-control-status',{ordinaryReady:false,resourcesReady:false}).stopped,true);assert.equal(api.advanceNativeOperationWait(z.h,w.wait,'expire',z.c,z.p,good).status,'observation-ended');assert.equal(api.nativeOperationObservationUsage(z.h).waiters,1);assert.equal(api.retireNativeOperationWait(z.h,w.wait,good).status,'retired');}
 });
 check('input-snapshot-and-busy-guard-precede-current-fact-suppliers',()=>{
  for(const kind of kinds){const z=fixture(kind,api),r=z.form('retained-outcome'),c=copy(z.c),p=copy(z.p);let once=false;const g={...good,get authorized(){if(!once){once=true;c.recipient+='-foreign';p.effect=false;r.facet.kind='foreign';assert.deepEqual(z.query('minimal-control-status'),no);assert.equal(api.retireNativeOperationObservation(z.h,good).status,'unavailable');}return true;}};
   assert.equal(api.queryNativeOperationObservation(z.h,r,c,p,g).status,'native-operation-receipt');assert.equal(z.query('retained-outcome').receipt.kind,kind);
  }
 });
 check('observer-admission-reserves-actual-parent-before-current-fact-reentry',()=>{
  const z=fixture('source-load',api);let entered=false;const g={...good,get authorized(){entered=true;assert.equal(api.openNativeOperationObservation(z.kind,z.owner,z.task,z.c,z.p,limits,good),null);return true;}};
  assert(api.openNativeOperationObservation(z.kind,z.owner,z.task,z.c,z.p,limits,g));assert(entered);
 });
 check('known-scope-loss-is-absorbing-not-temporary-receipt-denial',()=>{
  for(const kind of kinds){const z=fixture(kind,api);assert.deepEqual(z.query('retained-outcome',{operationReceiptAllowed:false}),no);assert.equal(z.query().status,'native-execution');assert.deepEqual(z.query('status',{scope:false}),no);assert.equal(api.nativeOperationObservationUsage(z.h).live,false);assert.deepEqual(z.query(),no);assert.equal(api.retireNativeOperationObservation(z.h,good).status,'retired');}
 });
 check('unexpected-result-path-fault-quarantines-receipt-not-control-or-producer',()=>{
  for(const kind of kinds){const z=fixture(kind,api);const g={...good,get operationReceiptAllowed(){throw Error('trusted fact supplier fault');}};assert.deepEqual(api.queryNativeOperationObservation(z.h,z.form('retained-outcome'),z.c,z.p,g),no);assert.equal(api.nativeOperationObservationUsage(z.h).receiptFaulted,true);assert.deepEqual(z.query('retained-outcome'),no);const s=z.query('minimal-control-status');assert.equal(s.status,'native-execution');assert.equal(s.stopped,false);assert.equal(api.retireNativeOperationObservation(z.h,good).status,'retired');}
 });
 check('closed-contracts-versions-selectors-and-wait-ownership',()=>{
  const z=fixture('source-load',api),r=z.form('retained-outcome');for(const x of [{...r,contract:'execution-receipt/1'},{...r,facet:{kind:z.kind,index:0}},{...r,extra:true},{...r,task:{...z.task}},{...r,profile:'foreign'}])assert.deepEqual(api.queryNativeOperationObservation(z.h,x,z.c,z.p,good),no);
  for(const route of ['typed','dynamic']){const e=z.envelope(z.form('status'),route);for(const v of [{...e,version:2},{...e,extra:true},{...e,variant:'retained-outcome'}])assert.deepEqual(api.invokeNativeOperationObservation(z.h,v,route,z.c,z.p,good),no);}
  const s=z.query();assert.deepEqual(api.startNativeOperationWait(z.h,z.form('bounded-observation-wait',{after:{...s.observation},budget:1}),z.c,z.p,good),no);
  for(const budget of [0,4,1.5,undefined])assert.deepEqual(api.startNativeOperationWait(z.h,z.form('bounded-observation-wait',{after:s.observation,budget}),z.c,z.p,good),no);
 });
 check('complete-prepaid-observer-and-independent-control-reserves',()=>{
  const charge=api.nativeOperationObservationCharge(limits);assert.equal(charge,5*limits.maxReplyBytes+2*limits.maxControlReplyBytes+3*limits.maxContextBytes+(limits.maxWaiters+1)*limits.maxWaitBytes+(limits.maxIssued+4)*limits.maxReferenceBytes);assert.equal(api.nativeOperationObservationCharge({...limits,maxHeldBytes:charge-1}),null);assert.equal(api.nativeOperationObservationCharge({...limits,maxHeldBytes:charge}),charge);
  const z=fixture('source-load',api);for(const change of [{maxReplyBytes:1},{maxControlReplyBytes:1},{maxReferenceBytes:1},{maxContextBytes:1},{maxWaiters:Infinity},{maxIssued:Number.MAX_SAFE_INTEGER}])assert.equal(api.openNativeOperationObservation(z.kind,z.owner,z.task,z.c,z.p,{...limits,...change},good),null);
 });
 return {scope:'owned-native-operation-observation-and-retained-evidence',controls,routeDeliveries,producerDispatchPublicationOrBaselineAdoptionAdded:false,oldReceiptProfileChanged:false,hostClockPhysicalConcurrentCrashOrIndependentProof:false};
}
export async function checkNativeOperationObservationMutations(){
 const source=fs.readFileSync(new URL('./native-operation-observation.mjs',import.meta.url),'utf8');
 const edits=[
  ['original-scope-ignored','!equal(e.context,c)','false'],
  ['actual-task-identity-ignored','r.task===e.task','true'],
  ['captured-effect-ceiling-ignored','p[k]&&e.ceiling[k]','p[k]'],
  ['current-receipt-grant-not-strict',"gates(g,receiptGates(e.kind))","receiptGates(e.kind).every(k=>g?.[k])"],
  ['protected-control-uses-ordinary-capacity',"g?.controlReady===true","g?.controlReady===true&&g?.ordinaryReady===true"],
  ['observer-stop-stops-native-producer',"function close(e){e.live=false;","function close(e){if(e.kind==='source-load')controlResourceRead(e.owner,e.task,'cancel',{controlAllowed:true});e.live=false;"],
  ['wait-tick-does-not-consume-budget','w.remaining--;',''],
  ['wait-foreign-reference-accepted',"e.refs.get(r.after)?.facet!=='ordinary'",'false'],
  ['ordinary-exhaustion-unbounded','e.issued>=e.limits.maxIssued','false'],
  ['native-receipt-query-creates-false-progress',"receipt:s.evidence?r:null}),last=","receipt:s.evidence?r:null,operation}),last="],
  ['pre-fact-input-not-captured','context:copy(c),policy:copy(p)','context:c,policy:p'],
  ['reentrant-query-not-excluded','if(!e||e.busy)return no();e.busy=true;','if(!e)return no();e.busy=true;'],
  ['scope-loss-not-absorbing',"{close(e);return null;}",'{return null;}'],
  ['extra-protected-reservation-omitted','base+2*limits.maxReferenceBytes+2*limits.maxContextBytes+3*limits.maxReplyBytes+limits.maxControlReplyBytes','base'],
  ['observer-admission-facts-before-parent-reservation','const attachment=beginInvocationOwnerAttachment',"if(!gates(g,catalog.admissionGates))return null;const attachment=beginInvocationOwnerAttachment"],
 ];
 const load=async(edit)=>{let s=source;if(edit){const [name,before,after]=edit;const count=s.split(before).length-1;assert.equal(count,name==='original-scope-ignored'?3:name==='reentrant-query-not-excluded'||name==='scope-loss-not-absorbing'?2:1,name+': exact anchor count');s=s.replaceAll(before,after);if(name==='observer-stop-stops-native-producer')s="import {controlResourceRead} from './resource-read.mjs';\n"+s;}s=s.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,import.meta.url).href)).replace(/new URL\('(\.\/[^']+)',import.meta.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,import.meta.url).href)+')');return import('data:text/javascript;base64,'+Buffer.from(s+'\n// '+(edit?.[0]??'baseline')).toString('base64'));};
 checkNativeOperationObservation(await load(null));const killed=[];for(const edit of edits){const a=await load(edit);let caught=false;try{checkNativeOperationObservation(a);}catch(e){assert.equal(e.code,'ERR_ASSERTION',edit[0]+': '+String(e.message).slice(0,160));caught=true;}assert(caught,edit[0]);killed.push(edit[0]);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{const baseline=checkNativeOperationObservation();console.log(JSON.stringify(process.argv.includes('--reference-only')?baseline:{baseline,mutations:await checkNativeOperationObservationMutations(),copiedFacadeBaseline:true,constructionFailuresCounted:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
