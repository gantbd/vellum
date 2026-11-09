import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as analysis from './analysis-work.mjs';
import * as kernel from './invocation-publication.mjs';
import {analysisOwnerBindingFixture} from './analysis-owner-binding-checks.mjs';
import {analysisWorkFixture} from './analysis-work-checks.mjs';
import {annotationObservationFixture,annotationObservationGates as executionGates,annotationObservationPolicy as executionPolicy} from './annotation-observation-checks.mjs';
import {analysisQueryClassificationBytes,normalizeAnalysisQueryArguments,sameAnalysisQueryArguments,analysisQueryProducerRequest} from './analysis-query-arguments.mjs';
const copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const nodes=v=>1+(v&&typeof v==='object'?Object.values(v).reduce((n,x)=>n+nodes(x),0):0);
const depth=v=>v&&typeof v==='object'&&Object.keys(v).length?1+Math.max(...Object.values(v).map(depth)):0;
const limits={maxRequestBytes:4096,maxInputNodes:4096,maxInputDepth:128},no={status:'unavailable'};
export function checkAnalysisQueryGateway(api=analysis){
 const seed=analysisWorkFixture(),base=annotationObservationFixture().current(),controls=[];let nativeCases=0,denials=0,boundaries=0,reentries=0;
 const make=(patch={})=>{
  const x=analysisOwnerBindingFixture(api,{...seed,limits:{...seed.limits,...patch}},base),accepted=x.ready(),context=kernel.invocationAuthorityContext(x.z.domain);
  const request=(operation='supported-analysis')=>{const r=x.query(accepted.snapshot);r.operation=operation;if(operation==='supplied-evidence-inspection')delete r.range;return r;};
  const envelope=(operation='supported-analysis',route='typed')=>({route,request:{...(route==='typed'?{family:7}:{name:'read-derived-analysis'}),variant:operation,version:1,arguments:request(operation)}});
  const query=(e=envelope(),p=limits,c=context,g=x.g)=>api.queryDecodedInvocationAnalysis(x.owner,e,p,c,g);
  return {...x,accepted,context,request,envelope,query};
 };
 const frame=x=>({root:x.root(),owner:copy(api.analysisUsage(x.owner)),domain:copy(kernel.invocationUsage(x.z.domain))});
 const same=(x,b)=>{assert.equal(x.root(),b.root);assert.deepEqual(api.analysisUsage(x.owner),b.owner);assert.deepEqual(kernel.invocationUsage(x.z.domain),b.domain);};
 const check=(name,fn)=>{fn();controls.push(name);};
 check('both-routes-and-products-use-actual-owned-snapshot',()=>{
  const x=make(),before=frame(x);
  for(const op of ['supported-analysis','supplied-evidence-inspection'])for(const route of ['typed','dynamic']){assert.equal(x.query(x.envelope(op,route)).status,op==='supported-analysis'?'captures':'analysis-summary');same(x,before);nativeCases++;}
 });
 check('empty-query-remains-authorized-native-range-not-a-point',()=>{
  const x=make(),e=x.envelope(),before=frame(x);e.request.arguments.range=[0,0];assert.deepEqual(x.query(e).captures,[]);same(x,before);
  assert.deepEqual(x.query(e,limits,x.context,{...x.g,authorized:false}),no);same(x,before);
 });
 check('canonical-meaning-is-route-independent-but-original-role-sensitive',()=>{
  const x=make(),e=x.envelope(),typed=normalizeAnalysisQueryArguments(e,x.context,limits,seed.limits.maxQueryBytes),dynamic=normalizeAnalysisQueryArguments(x.envelope('supported-analysis','dynamic'),x.context,limits,seed.limits.maxQueryBytes);
  assert(typed&&dynamic);assert.equal(typed.record.request.snapshot,null);assert.equal(sameAnalysisQueryArguments(typed,dynamic),true);assert.equal(analysisQueryProducerRequest(typed).snapshot,x.accepted.snapshot);
  e.request.arguments.snapshot=copy(x.accepted.snapshot);const copied=normalizeAnalysisQueryArguments(e,x.context,limits,seed.limits.maxQueryBytes);assert(copied);assert.equal(sameAnalysisQueryArguments(typed,copied),false);
  const original=copy(typed.record);e.request.arguments.range[0]=99;assert.deepEqual(typed.record,original);
 });
 check('foreign-originals-copies-and-caller-scope-cannot-borrow-analysis',()=>{
  const x=make(),other=make(),before=frame(x);
  for(const snapshot of [copy(x.accepted.snapshot),other.accepted.snapshot,{},null]){const e=x.envelope();e.request.arguments.snapshot=snapshot;assert.deepEqual(x.query(e),no);same(x,before);denials++;}
  for(const field of Object.keys(x.context)){assert.deepEqual(x.query(x.envelope(),limits,{...x.context,[field]:'foreign'}),no);same(x,before);denials++;}
  assert.equal(normalizeAnalysisQueryArguments(x.envelope(),{...x.context,extra:'injected'},limits,seed.limits.maxQueryBytes),null);assert.deepEqual(x.query(x.envelope(),limits,{...x.context,extra:'injected'}),no);same(x,before);
 });
 check('opaque-foreign-reference-never-reaches-json-hooks',()=>{
  const x=make(),before=frame(x),e=x.envelope();let calls=0;const snapshot={};Object.defineProperty(snapshot,'toJSON',{get(){calls++;throw Error('opaque traversal');}});e.request.arguments.snapshot=snapshot;
  assert.deepEqual(x.query(e),no);assert.equal(calls,0);same(x,before);
 });
 check('routes-discriminants-versions-and-fields-remain-closed',()=>{
  const x=make(),before=frame(x),e=x.envelope();
  const cases=[{...e,extra:true},{...e,route:'remote'},...[
   {family:6},{variant:'supplied-evidence-inspection'},{version:2},{extra:true},
   {arguments:{...e.request.arguments,operation:'unknown'}},{arguments:{...e.request.arguments,contract:'foreign'}},
   {arguments:{...e.request.arguments,extra:true}},{arguments:{...e.request.arguments,view:{...e.request.arguments.view,authority:'foreign'}}},
   {arguments:{...e.request.arguments,range:[2,1]}},{arguments:{...e.request.arguments,range:[0,0,0]}}
  ].map(p=>({...e,request:{...e.request,...p}}))];
  for(const value of cases){assert.equal(normalizeAnalysisQueryArguments(value,x.context,limits,seed.limits.maxQueryBytes),null);assert.deepEqual(x.query(value),no);same(x,before);denials++;}
 });
 check('accessors-refuse-without-running-or-faulting-owner',()=>{
  for(const site of ['envelope','arguments','limits','context','facts']){
   const x=make(),e=x.envelope(),p={...limits},c={...x.context},g={...x.g},before=frame(x);let calls=0;
   const value={envelope:e,arguments:e.request.arguments,limits:p,context:c,facts:g}[site],key={envelope:'route',arguments:'view',limits:'maxRequestBytes',context:'recipient',facts:'authorized'}[site];
   Object.defineProperty(value,key,{enumerable:true,get(){calls++;throw Error('ordinary getter');}});
   assert.deepEqual(x.query(e,p,c,g),no);assert.equal(calls,0);same(x,before);denials++;
  }
 });
 check('current-release-facts-remain-strict-and-independent',()=>{
  const x=make(),before=frame(x);
  for(const key of ['authorized','scope','continuous','live','retained','notStopped','replyReady','contextAllowed'])for(const value of [false,1,undefined]){assert.deepEqual(x.query(x.envelope(),limits,x.context,{...x.g,[key]:value}),no);same(x,before);denials++;}
  assert.equal(x.query().status,'captures');same(x,before);
 });
 check('actual-annotation-preservation-and-source-staleness-stay-native',()=>{
  const x=make(),plan=x.z.prepare({operation:'update-body'});x.z.execute(plan);let before=frame(x);assert.equal(x.query().status,'captures');same(x,before);
  x.z.source();before=frame(x);assert.deepEqual(x.query(),no);same(x,before);
 });
 check('complete-gateway-and-native-query-headroom-have-exact-boundary',()=>{
  const probe=make(),held=api.analysisUsage(probe.owner).heldBytes,frames=analysisQueryClassificationBytes+7*Math.min(limits.maxRequestBytes,seed.limits.maxQueryBytes),native=2*(seed.limits.maxEnvelopeBytes+Math.max(size({status:'captures'}),size({status:'analysis-summary'})));
  for(const delta of [-1,0,1]){const x=make({maxHeldBytes:held+frames+native+delta}),before=frame(x);assert.equal(x.query().status,delta<0?'unavailable':'captures');same(x,before);boundaries++;}
 });
 check('classification-reservation-precedes-limit-descriptor-access',()=>{
  const probe=make(),held=api.analysisUsage(probe.owner).heldBytes;
  for(const delta of [-1,0,1]){const x=make({maxHeldBytes:held+analysisQueryClassificationBytes+delta}),before=frame(x);let calls=0;const p=new Proxy({...limits},{getPrototypeOf(value){calls++;return Object.getPrototypeOf(value);}});assert.deepEqual(x.query(x.envelope(),p),no);assert.equal(calls,delta<0?0:1);same(x,before);boundaries++;}
 });
 check('full-frame-reservation-precedes-envelope-descriptor-access',()=>{
  const probe=make(),held=api.analysisUsage(probe.owner).heldBytes,charge=analysisQueryClassificationBytes+7*Math.min(limits.maxRequestBytes,seed.limits.maxQueryBytes);
  for(const delta of [-1,0,1]){const x=make({maxHeldBytes:held+charge+delta}),before=frame(x);let calls=0;const e=new Proxy(x.envelope(),{getPrototypeOf(value){calls++;return Object.getPrototypeOf(value);}});assert.deepEqual(x.query(e),no);assert.equal(calls,delta<0?0:1);same(x,before);boundaries++;}
 });
 check('argument-and-fact-byte-node-depth-limits-are-independent',()=>{
  for(const dimension of ['maxRequestBytes','maxInputNodes','maxInputDepth']){
   const x=make(),e=x.envelope(),p={...limits},shadow={...e.request.arguments,snapshot:null};
   const frames=()=>[{route:e.route,request:{family:7,variant:e.request.variant,version:1,arguments:shadow}},x.context,{request:shadow,context:x.context,limits:p,opaqueBindings:['analysis-snapshot']},{facts:x.g}];
   const measure=dimension==='maxRequestBytes'?size:dimension==='maxInputNodes'?nodes:depth;
   for(let i=0;i<6;i++)p[dimension]=Math.max(...frames().map(measure));
   const minimum=p[dimension];assert.equal(minimum,Math.max(...frames().map(measure)));
   for(const delta of [-1,0,1]){const before=frame(x);assert.equal(x.query(e,{...p,[dimension]:minimum+delta}).status,delta<0?'unavailable':'captures',dimension);same(x,before);boundaries++;}
  }
 });
 check('caller-result-mutation-does-not-rewrite-native-packet',()=>{
  const x=make(),before=frame(x),first=x.query(),expected=copy(first);first.context.sourceRevision=999;if(first.captures.length)first.captures[0][1]='changed';assert.deepEqual(x.query(),expected);same(x,before);
 });
 check('classification-and-complete-frame-reentry-stay-excluded',()=>{
  for(const site of ['limits','envelope']){
   const x=make(),before=frame(x);let observed=0;
   const trap=target=>new Proxy(target,{getPrototypeOf(v){
    const u=api.analysisUsage(x.owner),expected=analysisQueryClassificationBytes+(site==='envelope'?7*Math.min(limits.maxRequestBytes,seed.limits.maxQueryBytes):0);assert.equal(u.heldBytes-before.owner.heldBytes,expected);assert.equal(u.queryBusy,true);
    assert.deepEqual(x.query(),no);assert.deepEqual(api.queryInvocationAnalysis(x.owner,x.request(),x.g),no);assert.deepEqual(api.controlAnalysisWork(x.owner,x.accepted.task,'cancel',{controlAllowed:true}),no);observed++;reentries+=3;return Object.getPrototypeOf(v);
   }});
   assert.equal(x.query(site==='envelope'?trap(x.envelope()):x.envelope(),site==='limits'?trap({...limits}):limits).status,'captures');assert(observed>0);same(x,before);
  }
 });
 check('injected-capture-fault-refunds-and-keeps-protected-discharge',()=>{
  const x=make(),before=frame(x),bad=new Proxy(x.envelope(),{getPrototypeOf(){throw Error('injected capture fault');}});
  assert.deepEqual(x.query(bad),no);assert.equal(x.root(),before.root);assert.deepEqual(kernel.invocationUsage(x.z.domain),before.domain);
  assert.deepEqual(api.analysisUsage(x.owner),{...before.owner,queryFaulted:true});assert.deepEqual(x.query(),no);
  let calls=0;const blocked=new Proxy({...limits},{getPrototypeOf(value){calls++;return Object.getPrototypeOf(value);}});assert.deepEqual(x.query(x.envelope(),blocked),no);assert.equal(calls,0);
  assert.equal(api.controlAnalysisWork(x.owner,x.accepted.task,'cancel',{controlAllowed:true}).status,'stopped');
  assert.equal(api.observeAnalysisWork(x.owner,x.event(x.accepted.task,'quiescence'),seed.ingress).status,'quiescent');
  assert.equal(api.retireAnalysisWork(x.owner,x.accepted.task,seed.retirement).status,'retired');assert.equal(api.analysisUsage(x.owner).issued,before.owner.issued);
 });
 check('actual-domain-fault-refuses-before-caller-decoding',()=>{
  const x=make(),issued=kernel.issueInvocation(x.z.domain,x.context,executionGates);assert(issued);
  const plan={contract:'owned-annotation-plan/1',invocation:issued.reference,descriptor:'fault',sources:[],annotations:[{kind:'body',document:null,id:'note',incarnation:1,expectedBody:1,body:'surprise'}]};
  assert.equal(kernel.submitInvocation(x.z.domain,issued.ticket,{kind:'compound',plan},'typed',x.context,executionPolicy,executionGates).status,'accepted');
  assert.equal(kernel.runInvocation(x.z.domain,issued.ticket,x.context,executionGates,x.z.h).status,'faulted');
  const before=frame(x);let calls=0;const blocked=new Proxy({...limits},{getPrototypeOf(value){calls++;return Object.getPrototypeOf(value);}});assert.deepEqual(x.query(x.envelope(),blocked),no);assert.equal(calls,0);same(x,before);
 });
 return {scope:'actual-native-analysis-query-facade-only',controls,nativeCases,denials,boundaries,reentries,originalScopeAndSnapshotPreserved:true,completeTemporaryFramesAndNativeWorkspaceChecked:true,proxyUseIsFaultInjectionNotHostileContainment:true,conditionalDiscoveryAndF24F28OrFullF08Proved:false,hostProviderPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkAnalysisQueryGateway()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
