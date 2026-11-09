import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import * as editing from './annotation-editing.mjs';
import * as reversal from './history-reversal.mjs';
import {annotationMembershipEditingFixture} from './annotation-membership-editing-checks.mjs';
const original={...kernel,...editing,...reversal},copy=structuredClone;
const id=n=>({authority:'a',id:n,incarnation:1});
const context={authority:'a',engine:'native-membership-engine',recipient:'reader',purpose:'edit',destination:'client',profile:'execution',continuity:'c1'};
const policy={publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'};
function fixture(api,options={}){
 const seed=annotationMembershipEditingFixture(api),state=copy(seed.root()),d=state.documents[0];
 d.records.push({...copy(d.records[0]),id:'other'});
 d.annotationLifecycle.entries.push({...copy(d.annotationLifecycle.entries[0]),id:'other'});
 const z=annotationMembershipEditingFixture(api,'create',{state,...options});
 const target={basis:'candidate',descriptor:z.reference,target:{kind:'range',start:8,end:9}};
 const plan={contract:'owned-annotation-plan/1',invocation:z.reference,descriptor:z.reference,
  sources:[{document:id('A'),expectedSource:1,expectedAnnotationSet:1,edits:[{lo:8,hi:8,expected:[],ins:[88]}]}],
  annotations:[
   {kind:'create',document:id('A'),id:'fresh',incarnation:1,body:'created',target:copy(target)},
   {kind:'body',document:id('A'),id:'note',incarnation:1,expectedBody:1,body:'changed'},
   {kind:'reattach',document:id('A'),id:'note',incarnation:1,expectedTarget:1,target:copy(target)},
   {kind:'delete',document:id('A'),id:'other',incarnation:1,expectedBody:1,expectedTarget:1}]};
 const h={...z.h,sourceWritable:true,membershipAllocationPlan:copy(plan),membershipAllocations:[{document:id('A'),id:'fresh',incarnation:1}]};
 const grants=plan.annotations.map(a=>({invocation:z.reference,annotation:{document:copy(a.document),id:a.id,incarnation:a.incarnation},identity:true,contentVersion:true,targetGeneration:true}));
 const g={...h,outsidePublicationGuard:true,preparationReserved:true,atomicAnnotationResultGrants:grants};
 const prepare=(p=plan,flags=g,c=context)=>api.prepareInvocationAtomicAnnotationReceipts(z.domain,z.ticket,p,c,flags);
 const read=(index,flags=g,p=policy,c=context)=>api.observeAtomicAnnotationResult(z.domain,z.ticket,index,c,p,flags);
 return {...z,h,plan,g,prepare,read,run:(checks=h)=>z.run(checks)};
}
export {fixture as atomicAnnotationResultFixture};
export function checkAtomicAnnotationCoordinator(api=original){
 const controls=[],control=(name,f)=>{f();controls.push(name);};
 control('same-root-and-terminal-receipts-install-once-with-original-component-order',()=>{
  const z=fixture(api),before=z.usage(),job=z.prepare();assert(job);assert(z.usage().declaredHeldBytes>before.declaredHeldBytes);
  assert.equal(api.invocationAtomicAnnotationResultUsage(z.domain,z.ticket,context).retainedReceipts,0);
  for(let i=0;i<4;i++)assert.equal(z.read(i).status,'unavailable');
  assert.equal(z.submit(job).status,'accepted');assert.equal(z.run().status,'settled');
  assert.equal(z.root().publication,1);assert.equal(z.root().groups.length,1);assert.equal(z.root().groups[0].changes.length,5);assert.equal(z.root().groups[0].scopes.length,4);assert.equal(api.invocationAtomicAnnotationResultUsage(z.domain,z.ticket,context).retainedReceipts,4);
  for(const [i,operation] of ['create','update-body','reattach','delete'].entries()){
   const r=z.read(i);assert.equal(r.status,'annotation-result');assert.equal(r.result.operation,operation);assert.equal(r.result.invocation,z.reference);assert.equal(r.result.membership,i===3?'retained-absent':'present');assert.equal(r.result.contentVersion,[1,2,2,1][i]);assert.equal(r.result.targetGeneration,[1,2,2,1][i]);
  }
  assert.equal(z.prepare(copy(z.plan)),job);assert.equal(z.submit(job).status,'observe-existing');assert.equal(z.run().status,'unavailable');assert.equal(z.root().publication,1);
 });
 control('historical-receipts-do-not-recompute-from-later-native-row',()=>{
  const z=fixture(api),job=z.prepare();z.submit(job);z.run();const receipt=z.read(1);
  const issued=api.issueInvocation(z.domain,context,z.g);assert(issued);
  const request={contract:'annotation-authoring/1',operation:'update-body',invocation:issued.reference,document:id('A'),profile:'execution',dependencies:[],annotation:{document:id('A'),id:'note',incarnation:1},expectedContent:2,body:'later'};
  const p=api.prepareAnnotationEdit(z.root(),request,z.h,z.al);assert(p);
  assert.equal(api.submitInvocation(z.domain,issued.ticket,{kind:'annotation',plan:p},'typed',context,policy,z.g).status,'accepted');assert.equal(api.runInvocation(z.domain,issued.ticket,context,z.g,z.h).status,'settled');
  assert.equal(z.root().documents[0].records.find(r=>r.id==='note').bodyVersion,3);assert.deepEqual(z.read(1),receipt);assert.equal(z.read(1).result.contentVersion,2);
 });
 control('captured-and-current-component-ceilings-never-widen-or-cross-bind',()=>{
  const z=fixture(api);z.g.atomicAnnotationResultGrants[1].contentVersion=false;
  const job=z.prepare();z.submit(job);z.run();
  const current=copy(z.g);current.atomicAnnotationResultGrants[1].contentVersion=true;
  assert.equal(Object.hasOwn(z.read(1,current).result,'contentVersion'),false);
  for(const patch of [{invocation:'other'},{annotation:{document:id('A'),id:'other',incarnation:1}},{identity:false,contentVersion:false,targetGeneration:false}]){const flags=copy(z.g);Object.assign(flags.atomicAnnotationResultGrants[1],patch);assert.equal(z.read(1,flags).status,'unavailable');}
  for(const i of [-1,4,1.5,'1'])assert.equal(z.read(i).status,'unavailable');
  assert.equal(z.read(0,{...z.g,atomicAnnotationResultGrants:[]}).status,'unavailable');
  assert.equal(z.read(0,z.g,policy,{...context,recipient:'other'}).status,'unavailable');
 });
 control('known-whole-plan-refusal-retains-only-not-published-component-facts',()=>{
  const z=fixture(api);z.plan.annotations[3].expectedTarget=2;z.h.membershipAllocationPlan=copy(z.plan);const before=z.root(),job=z.prepare();assert(job);z.submit(job);z.run();
  assert.equal(z.root(),before);for(let i=0;i<4;i++){const r=z.read(i);assert.equal(r.result.status,'not-published');assert.equal(Object.hasOwn(r.result,'annotation'),false);}
 });
 control('missing-admission-result-grants-cannot-be-added-after-publication',()=>{
  const z=fixture(api),later=copy(z.g);delete z.g.atomicAnnotationResultGrants;const job=z.prepare();assert(job);z.submit(job);z.run();assert.equal(z.root().publication,1);for(let i=0;i<4;i++)assert.equal(z.read(i,later).status,'unavailable');
 });
 control('stop-spends-once-without-losing-reservation-or-creating-records',()=>{
  const z=fixture(api),before=z.root(),job=z.prepare(),charge=z.usage().declaredHeldBytes;z.submit(job);assert.equal(api.stopInvocation(z.domain,z.ticket,z.g).status,'stop-requested');z.run();assert.equal(z.root(),before);assert.equal(z.read(0).result.status,'not-published');assert.equal(z.usage().declaredHeldBytes,charge);assert.equal(z.submit(job).status,'observe-existing');
 });
 control('protocol-fault-exposes-no-success-vector-and-retains-original-root-and-charge',()=>{
  const z=fixture(api),before=z.root(),job=z.prepare(),charge=z.usage().declaredHeldBytes;z.submit(job);Object.defineProperty(z.h,'authorized',{get(){throw Error('injected');}});assert.equal(z.run().status,'faulted');assert.equal(z.root(),before);assert.equal(z.usage().declaredHeldBytes,charge);for(let i=0;i<4;i++)assert.equal(z.read(i).status,'unavailable');
 });
 control('faulted-owner-still-permits-only-attested-protected-hold-retirement',()=>{
  const z=fixture(api),base=z.usage().declaredHeldBytes,job=z.prepare();z.submit(job);Object.defineProperty(z.h,'authorized',{get(){throw Error('injected');}});assert.equal(z.run().status,'faulted');const charge=z.usage().declaredHeldBytes;
  assert.equal(api.retireInvocation(z.domain,z.ticket,{...z.g,deliveryDrained:false}).status,'unavailable');assert.equal(z.usage().declaredHeldBytes,charge);assert.equal(api.retireInvocation(z.domain,z.ticket,z.g).status,'retired');assert.equal(z.usage().declaredHeldBytes,base);assert.equal(api.issueInvocation(z.domain,context,z.g),null);
 });
 control('actual-parent-headroom-refusal-is-exact-and-does-not-spend-ticket',()=>{
  const z=fixture(api),before=z.usage();assert(z.prepare());const charge=z.usage().declaredHeldBytes-before.declaredHeldBytes;
  for(const delta of [-1,0,1]){const a=fixture(api,{limits:{maxHeldBytes:before.declaredHeldBytes+charge+delta}}),prior=a.usage(),job=a.prepare();assert.equal(!!job,delta>=0);assert.equal(a.usage().declaredHeldBytes,prior.declaredHeldBytes+(job?charge:0));if(!job){assert.equal(a.root().publication,0);assert.equal(a.usage().starts,0);}}
 });
 control('changed-or-copied-private-plan-cannot-replace-original-preparation',()=>{
  const z=fixture(api),job=z.prepare(),charge=z.usage().declaredHeldBytes;
  const altered=copy(z.plan);altered.annotations[1].body='substitute';assert.equal(z.prepare(altered),null);assert.equal(z.usage().declaredHeldBytes,charge);assert.equal(z.submit({kind:'compound',plan:copy(job.plan)}).status,'unavailable');z.submit(job);z.run();assert.equal(z.root().documents[0].records.find(r=>r.id==='note').body,'changed');
 });
 control('retention-drain-forget-and-owner-controlled-retirement-discharge-once',()=>{
  const z=fixture(api),before=z.usage(),job=z.prepare();z.submit(job);z.run();
  for(const key of ['retentionReleased','deliveryDrained'])for(const value of [false,undefined,0,1,'yes']){assert.equal(api.forgetInvocationResult(z.domain,z.ticket,{...z.g,[key]:value}).status,'unavailable');assert.equal(z.read(0).status,'annotation-result');}
  const beforeForget=z.usage().declaredHeldBytes;
  assert.equal(api.forgetInvocationResult(z.domain,z.ticket,{...z.g,ownerControlAllowed:false}).status,'forgotten');assert.equal(z.read(0).status,'unavailable');assert.equal(api.invocationAtomicAnnotationResultUsage(z.domain,z.ticket,context).retainedReceipts,0);assert.equal(z.usage().declaredHeldBytes,beforeForget);
  const held=z.usage().declaredHeldBytes;for(const key of ['ownerControlAllowed','retentionReleased','deliveryDrained']){assert.equal(api.retireInvocation(z.domain,z.ticket,{...z.g,[key]:false}).status,'unavailable');assert.equal(z.usage().declaredHeldBytes,held);}
  assert.equal(api.retireInvocation(z.domain,z.ticket,z.g).status,'retired');assert.equal(z.usage().declaredHeldBytes,before.declaredHeldBytes);assert.equal(api.invocationAtomicAnnotationResultUsage(z.domain,z.ticket,context),null);assert.equal(api.retireInvocation(z.domain,z.ticket,z.g).status,'unavailable');assert.equal(z.submit(job).status,'unavailable');
 });
 return {scope:'owned-native-atomic-annotation-receipt-coordinator-only',controls,
  actualParentReservationChecked:true,actualCoordinatorReceiptInstallationChecked:true,
  historicalCurrentReleaseChecked:true,protectedDischargeChecked:true,
  publicMixedIngressOrU01Selected:false,physicalOrIssuerTruthProved:false,
  completeQualificationOrIndependentAssurance:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkAtomicAnnotationCoordinator()));
