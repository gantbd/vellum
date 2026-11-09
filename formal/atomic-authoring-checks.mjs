import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import * as editing from './annotation-editing.mjs';
import * as reversal from './history-reversal.mjs';
import {annotationMembershipEditingFixture} from './annotation-membership-editing-checks.mjs';
import {normalizeAtomicAuthoringArguments,atomicAuthoringInputForm} from './atomic-authoring-arguments.mjs';
const original={...kernel,...editing,...reversal},copy=structuredClone,id=n=>({authority:'a',id:n,incarnation:1});
const context={authority:'a',engine:'native-membership-engine',recipient:'reader',purpose:'edit',destination:'client',profile:'execution',continuity:'c1'};
const policy={publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'};
const inputLimits={maxRequestBytes:65536,maxInputNodes:16384,maxInputDepth:64};
function fixture(api=original,options={}){
 const seed=annotationMembershipEditingFixture(api),state=copy(seed.root()),d=state.documents[0];for(const name of ['other','peer-note']){d.records.push({...copy(d.records[0]),id:name});d.annotationLifecycle.entries.push({...copy(d.annotationLifecycle.entries[0]),id:name});}
 const z=annotationMembershipEditingFixture(api,'create',{state,...options});
 const target={basis:'candidate',target:{kind:'range',start:8,end:9}};
 const request={contract:'atomic-source-annotation/1',operation:'exact-batch',invocation:z.reference,profile:'bounded-atomic-authoring/1',
  sources:[{document:id('A'),expectedSource:1,edits:[{lo:8,hi:8,expected:[],ins:[88]}]}],
  annotations:[
   {operation:'create',document:id('A'),dependencies:[],body:'created',target:copy(target)},
   {operation:'update-body',document:id('A'),dependencies:[],annotation:{document:id('A'),id:'note',incarnation:1},expectedContent:1,body:'changed'},
   {operation:'reattach',document:id('A'),dependencies:[],annotation:{document:id('A'),id:'note',incarnation:1},expectedTarget:1,target:copy(target)},
   {operation:'delete',document:id('A'),dependencies:[],annotation:{document:id('A'),id:'other',incarnation:1},expectedContent:1,expectedTarget:1}],
  reads:[],scopes:[{profile:'bounded-source-scope/1',document:id('A'),expectedSource:1,replacementRanges:[],insertionSites:[{first:8,last:8}]}]};
 const g={...z.h,...Object.fromEntries(['scope','continuous','live','resourcesReady','sourceReadable','sourceWritable','allTargetsWritable','outsidePublicationGuard','preparationReserved'].map(k=>[k,true])),viewWritable:false,
  sourceScopeLimits:{maxScopes:8,maxReplacementRanges:8,maxInsertionIntervals:8,maxIdentifierBytes:64},
  atomicAnnotationHosts:request.annotations.map((a,component)=>({component,invocation:z.reference,document:copy(a.document),operation:a.operation,host:{...z.h,sourceWritable:false}})),
  atomicAnnotationResultGrants:['fresh','note','note','other'].map(n=>({invocation:z.reference,annotation:{document:id('A'),id:n,incarnation:1},identity:true,contentVersion:true,targetGeneration:true}))};
 const envelope=(r=request,route='typed')=>({route,request:{...(route==='typed'?{family:10}:{name:'change-source'}),variant:'exact-batch',version:1,arguments:r}});
 return {...z,request,g,envelope,
  submit:(r=request,route='typed',flags=g,limits=inputLimits)=>api.submitInvocationAtomicAuthoring(z.domain,z.ticket,envelope(r,route),limits,context,policy,flags),
  run:(h=g)=>api.runInvocation(z.domain,z.ticket,context,g,h),
  result:(index,h=g)=>api.observeAtomicAnnotationResult(z.domain,z.ticket,index,context,policy,h)};
}
function onlyBody(z){z.request.sources=[];z.request.scopes=[];z.request.annotations=[z.request.annotations[1]];z.g.atomicAnnotationHosts=[{...z.g.atomicAnnotationHosts[1],component:0}];z.g.atomicAnnotationResultGrants=[z.g.atomicAnnotationResultGrants[1]];}
function otherBody(api,z){
 const issued=api.issueInvocation(z.domain,context,z.g);assert(issued);const r={contract:'annotation-authoring/1',operation:'update-body',invocation:issued.reference,document:id('A'),profile:'execution',dependencies:[],annotation:{document:id('A'),id:'peer-note',incarnation:1},expectedContent:1,body:'later'},p=api.prepareAnnotationEdit(z.root(),r,z.h,z.al);assert(p);assert.equal(api.submitInvocation(z.domain,issued.ticket,{kind:'annotation',plan:p},'typed',context,policy,z.g).status,'accepted');assert.equal(api.runInvocation(z.domain,issued.ticket,context,z.g,z.h).status,'settled');
}
function otherSource(api,z,name='B'){
 const d=z.root().documents.find(d=>d.document.id===name),issued=api.issueInvocation(z.domain,context,z.g);assert(issued);
 const plan={contract:'owned-annotation-plan/1',invocation:issued.reference,descriptor:issued.reference,sources:[{document:copy(d.document),expectedSource:d.revision,expectedAnnotationSet:d.annotationSet,edits:[{lo:0,hi:0,expected:[],ins:[81]}]}],annotations:[]};
 const h={...z.g,sourceReadable:true,sourceWritable:true,allTargetsWritable:true};assert.equal(api.submitInvocation(z.domain,issued.ticket,{kind:'compound',plan},'typed',context,policy,h).status,'accepted');assert.equal(api.runInvocation(z.domain,issued.ticket,context,h,h).status,'settled');
}
export function checkAtomicAuthoring(api=original){
 const controls=[],control=(name,f)=>{f();controls.push(name);};let omissions=0,permissionCases=0;
 control('public-all-four-source-and-historical-receipts-use-one-owned-publication',()=>{
  const z=fixture(api),before=z.usage();assert.deepEqual(z.submit(),{status:'accepted'});assert(z.usage().declaredHeldBytes>before.declaredHeldBytes);assert.equal(z.usage().declaredHeldBytes-before.declaredHeldBytes,z.usage().ordinaryArgumentBytes);assert(z.usage().ordinaryArgumentBytes>=2*65536+2*262144);assert.equal(z.submit(z.request,'dynamic').status,'pending');assert.equal(z.run().status,'settled');
  assert.equal(Buffer.from(z.root().documents[0].source).toString(),'aé🙂zX');assert.equal(z.root().publication,1);assert.equal(z.root().groups.length,1);assert.equal(z.root().groups[0].changes.length,5);assert.equal(z.root().groups[0].scopes.length,4);
  for(let index=0;index<4;index++){const r=z.result(index);assert.equal(r.status,'annotation-result');assert.equal(r.result.operation,z.request.annotations[index].operation);assert.equal(r.result.contentVersion,[1,2,2,1][index]);}
  const root=z.root();assert.equal(z.submit(z.request,'dynamic').status,'observe-existing');assert.equal(z.run().status,'unavailable');assert.equal(z.root(),root);assert.equal(z.usage().starts,1);
 });
 control('source-authority-never-substitutes-for-each-annotation-authority',()=>{
  for(const phase of ['admission','service'])for(const index of [0,1,2,3])for(const value of [false,1]){const z=fixture(api),before=z.root(),usage=z.usage();if(phase==='service')assert.equal(z.submit().status,'accepted');z.g.atomicAnnotationHosts[index].host.annotationWritable=value;
   assert.equal(phase==='admission'?z.submit().status:z.run().status,phase==='admission'?'unavailable':'settled');assert.equal(z.root(),before);if(phase==='admission')assert.deepEqual(z.usage(),usage);else assert.equal(z.read().outcome.publication,'notApplied');permissionCases++;}
 });
 control('source-write-permission-is-separate-and-current',()=>{
  for(const key of ['sourceReadable','sourceWritable','allTargetsWritable','scope','historyReady'])for(const value of [false,1]){const z=fixture(api);assert.equal(z.submit().status,'accepted');const before=z.root();z.g[key]=value;z.run();assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'notApplied');permissionCases++;}
  const z=fixture(api);onlyBody(z);z.g.sourceReadable=false;z.g.sourceWritable=false;assert.equal(z.submit().status,'accepted');z.run();assert.equal(z.read().outcome.publication,'applied');assert.equal(z.root().documents[0].records.find(r=>r.id==='note').body,'changed');
 });
 control('actual-create-allocation-is-original-owned-and-not-caller-supplied',()=>{
  for(const patch of [{id:'chosen'},{annotation:{document:id('A'),id:'chosen',incarnation:1}}]){const z=fixture(api),before=z.usage();Object.assign(z.request.annotations[0],patch);assert.equal(z.submit().status,'unavailable');assert.deepEqual(z.usage(),before);}
  for(const patch of [{allocationCurrent:false},{annotationAllocation:{invocation:'foreign',document:id('A'),id:'fresh',incarnation:1}}]){const z=fixture(api);assert.equal(z.submit().status,'accepted');Object.assign(z.g.atomicAnnotationHosts[0].host,patch);const before=z.root();z.run();assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'notApplied');}
  const z=fixture(api);assert.equal(z.submit().status,'accepted');z.g.atomicAnnotationHosts[0].host.annotationAllocation={invocation:z.reference,document:id('A'),id:'substitute',incarnation:1};z.run();assert.equal(z.read().outcome.publication,'notApplied');
 });
 control('before-and-same-proposal-candidate-targets-preserve-explicit-point-profiles',()=>{
  for(const basis of ['before','candidate'])for(const profile of ['position','content']){const z=fixture(api);for(const index of [0,2])z.request.annotations[index].target={basis,...(basis==='before'?{sourceRevision:1}:{}),target:{kind:'point',offset:1,affinity:'after',profile}};assert.equal(z.submit().status,'accepted');z.run();for(const n of ['note','fresh']){const r=z.root().documents[0].records.find(r=>r.id===n);assert.equal(r.target.target.profile,profile);assert.equal(r.target.sourceRevision,basis==='before'?1:2);assert.equal(r.placement.sourceRevision,2);}}
  for(const target of [{basis:'candidate',descriptor:'foreign',target:{kind:'document'}},{basis:'candidate',sourceRevision:2,target:{kind:'document'}},{basis:'before',sourceRevision:1,target:{kind:'range',start:8,end:9}},{basis:'candidate',target:{kind:'point',offset:2,affinity:'after',profile:'position'}}]){const z=fixture(api),before=z.usage();z.request.annotations[2].target=target;assert.equal(z.submit().status,'unavailable');assert.deepEqual(z.usage(),before);}
 });
 control('same-record-fields-combine-but-conflicts-never-become-sequential-script',()=>{
  for(const mutate of [z=>z.request.annotations.push(copy(z.request.annotations[1])),z=>{z.request.annotations[3].annotation.id='note';},z=>{z.request.annotations[1].annotation.id='fresh';}]){const z=fixture(api);mutate(z);if(z.request.annotations.length===5){z.g.atomicAnnotationHosts.push({...z.g.atomicAnnotationHosts[1],component:4});z.g.atomicAnnotationResultGrants.push(copy(z.g.atomicAnnotationResultGrants[1]));}const before=z.usage();assert.equal(z.submit().status,'unavailable');assert.deepEqual(z.usage(),before);assert.equal(z.root().publication,0);}
 });
 control('final-invalid-component-refuses-before-admission-and-refunds-provisional-work',()=>{
  const z=fixture(api),before=z.usage();z.request.annotations[3].expectedTarget=2;assert.equal(z.submit().status,'unavailable');assert.deepEqual(z.usage(),before);assert.equal(z.usage().faulted,false);z.request.annotations[3].expectedTarget=1;assert.equal(z.submit().status,'accepted');z.run();assert.equal(z.root().publication,1);
 });
 control('original-private-companion-stamps-are-not-refreshed-by-public-retry',()=>{
  const z=fixture(api);assert.equal(z.submit().status,'accepted');otherBody(api,z);const before=z.root();assert.equal(z.submit(z.request,'dynamic').status,'pending');z.run();assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'notApplied');
 });
 control('cross-document-source-and-declared-read-dependencies-remain-one-transaction',()=>{
  const z=fixture(api);z.request.sources.push({document:id('B'),expectedSource:1,edits:[{lo:0,hi:1,expected:[112],ins:[80]}]});z.request.scopes.push({profile:'bounded-source-scope/1',document:id('B'),expectedSource:1,replacementRanges:[{lo:0,hi:4}],insertionSites:[]});assert.equal(z.submit().status,'accepted');z.run();assert.equal(z.root().publication,1);assert.equal(z.root().groups[0].changes.length,6);assert.equal(Buffer.from(z.root().documents[1].source).toString(),'Peer');
  for(const stale of [false,true]){const a=fixture(api),read={kind:'source',document:id('B'),expectedSource:1};a.request.reads=[read];a.g.readDependencyLimits={maxDependencies:8,maxIdentifierBytes:64};a.g.readDependencyAuthority={contract:'native-read-authority/2',context:copy(context),grants:[{dependency:read,readAllowed:true,presenceAllowed:true,equalityAllowed:true}]};assert.equal(a.submit().status,'accepted');if(stale)otherSource(api,a);const before=a.root();a.run();assert.equal(a.read().outcome.publication,stale?'notApplied':'applied');if(stale)assert.equal(a.root(),before);}
 });
 control('body-only-authoring-preserves-compatible-current-source-and-target-mapping',()=>{
  const z=fixture(api);onlyBody(z);z.g.sourceReadable=false;z.g.sourceWritable=false;assert.equal(z.submit().status,'accepted');otherSource(api,z,'A');const mapped=copy(z.root().documents[0].records.find(r=>r.id==='note').placement);z.run();assert.equal(z.read().outcome.publication,'applied');assert.equal(z.root().documents[0].records.find(r=>r.id==='note').body,'changed');assert.deepEqual(z.root().documents[0].records.find(r=>r.id==='note').placement,mapped);
 });
 control('candidate-target-without-source-write-still-pins-original-source-meaning',()=>{
  const z=fixture(api);z.request.sources=[];z.request.scopes=[];z.request.annotations=[{...z.request.annotations[2],target:{basis:'candidate',target:{kind:'point',offset:1,affinity:'after',profile:'position'}}}];z.g.atomicAnnotationHosts=[{...z.g.atomicAnnotationHosts[2],component:0}];z.g.atomicAnnotationResultGrants=[z.g.atomicAnnotationResultGrants[2]];assert.equal(z.submit().status,'accepted');otherSource(api,z,'A');const before=z.root();z.run();assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'notApplied');
 });
 control('complete-final-candidate-must-fit-every-captured-component-root-bound',()=>{
  const seed=fixture(api);seed.request.annotations[1].body='L'.repeat(2000);assert.equal(seed.submit().status,'accepted');seed.run();assert.equal(seed.read().outcome.publication,'applied');const bytes=Buffer.byteLength(JSON.stringify(seed.root()));
  const z=fixture(api);z.request.annotations[1].body='L'.repeat(2000);z.g.atomicAnnotationHosts[1].host.annotationLimits={...z.al,maxRootBytes:bytes-1};assert.equal(z.submit().status,'accepted');const before=z.root();z.run();assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'notApplied');
 });
 control('host-component-binding-and-original-native-owner-are-current',()=>{
  for(const patch of [{component:0},{invocation:'foreign'},{document:id('B')},{operation:'delete'}]){const z=fixture(api);assert.equal(z.submit().status,'accepted');Object.assign(z.g.atomicAnnotationHosts[1],patch);const before=z.root();z.run();assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'notApplied');}
  const z=fixture(api);assert.equal(z.submit().status,'accepted');z.g.atomicAnnotationHosts[1].host.owner={};z.run();assert.equal(z.read().outcome.publication,'notApplied');
 });
 control('closed-typed-dynamic-capture-and-original-caller-mutation-isolation',()=>{
  const z=fixture(api);const a=normalizeAtomicAuthoringArguments(z.envelope(),context,inputLimits,65536),b=normalizeAtomicAuthoringArguments(z.envelope(z.request,'dynamic'),context,inputLimits,65536);assert(a&&b);assert.deepEqual(a.record,b.record);const saved=copy(z.request);assert.equal(z.submit().status,'accepted');z.request.annotations[1].body='later';assert.equal(z.submit().status,'unavailable');z.run();assert.equal(z.root().documents[0].records.find(r=>r.id==='note').body,'changed');assert.equal(z.submit(saved,'dynamic').status,'observe-existing');
  for(const key of atomicAuthoringInputForm().fields){const x=fixture(api),r=copy(x.request);delete r[key];const before=x.usage();assert.equal(x.submit(r).status,'unavailable');assert.deepEqual(x.usage(),before);omissions++;}
  for(const patch of [{operation:'execute'},{profile:'unknown'},{contract:'atomic-source-annotation/2'}]){const x=fixture(api);Object.assign(x.request,patch);assert.equal(x.submit().status,'unavailable');assert.equal(x.usage().faulted,false);}
  let called=false;const x=fixture(api),before=x.usage();Object.defineProperty(x.request.annotations[1],'body',{enumerable:true,get(){called=true;return 'unsafe';}});assert.equal(x.submit().status,'unavailable');assert.equal(called,false);assert.deepEqual(x.usage(),before);
 });
 control('actual-parent-total-headroom-boundaries-include-receipt-and-bundle-work',()=>{
  const z=fixture(api),base=z.usage().declaredHeldBytes;assert.equal(z.submit().status,'accepted');const charge=z.usage().declaredHeldBytes-base;
  for(const delta of [-1,0,1]){const x=fixture(api,{limits:{maxHeldBytes:base+charge+delta}}),before=x.usage();assert.equal(x.submit().status,delta<0?'unavailable':'accepted');assert.equal(x.usage().declaredHeldBytes,before.declaredHeldBytes+(delta<0?0:charge));}
 });
 control('stop-and-fault-preserve-original-root-and-protected-responsibility',()=>{
  for(const fault of [false,true]){const z=fixture(api),base=z.usage().declaredHeldBytes,before=z.root();assert.equal(z.submit().status,'accepted');if(fault)Object.defineProperty(z.g.atomicAnnotationHosts[1].host,'authorized',{get(){throw Error('injected');}});else api.stopInvocation(z.domain,z.ticket,z.g);assert.equal(z.run().status,fault?'faulted':'settled');assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,fault?'unknown':'notApplied');assert.equal(api.retireInvocation(z.domain,z.ticket,{...z.g,deliveryDrained:false}).status,'unavailable');assert.equal(api.retireInvocation(z.domain,z.ticket,z.g).status,'retired');assert.equal(z.usage().declaredHeldBytes,base);}
 });
 return {scope:'owned-public-mixed-source-annotation-binding-only',controls,omissions,permissionCases,
  actualSharedPublicationAndReceiptsChecked:true,perComponentNativeAuthorityChecked:true,
  originalCandidateAllocationAndPrivateStampsChecked:true,actualParentTotalFundingChecked:true,
  U01Selected:false,actualIssuerPolicyOrPhysicalTruthProved:false,
  fullQualificationOrIndependentAssurance:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkAtomicAuthoring()));
export {fixture as atomicAuthoringFixture};
