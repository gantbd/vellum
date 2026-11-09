import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import * as annotation from './annotation-editing.mjs';
import * as reversal from './history-reversal.mjs';
import {engineInventory} from './engine-facade.mjs';
import {nativeLocalCommandInputForms,normalizeNativeLocalCommandArguments} from './native-local-command-arguments.mjs';
import {annotationMembershipEditingFixture} from './annotation-membership-editing-checks.mjs';
const original={...kernel,...annotation,...reversal},copy=structuredClone,id=n=>({authority:'a',id:n,incarnation:1});
const context={authority:'a',engine:'native-membership-engine',recipient:'reader',purpose:'edit',destination:'client',profile:'execution',continuity:'c1'},policy={publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'};
const inputLimits={maxRequestBytes:65536,maxInputNodes:16384,maxInputDepth:64};
function fixture(api=original,family='F13',operation='create',targetKind='group',options={}){
 const base=annotationMembershipEditingFixture(api,'create',options),g={...base.h,outsidePublicationGuard:true,preparationReserved:true};
 let issued={ticket:base.ticket,reference:base.reference},h=base.h,request;
 if(family==='F13'){
  const common={contract:'annotation-authoring/1',operation,invocation:issued.reference,document:id('A'),profile:'execution',dependencies:[]},ref={document:id('A'),id:'note',incarnation:1},target={document:id('A'),sourceRevision:1,target:{kind:'point',offset:1,affinity:'after',profile:'position'}};
  request={...common,...(operation==='create'?{body:'new',target}:operation==='update-body'?{annotation:ref,expectedContent:1,body:'changed'}:operation==='reattach'?{annotation:ref,expectedTarget:1,target}:{annotation:ref,expectedContent:1,expectedTarget:1})};
  g.annotationHost=h;g.annotationResultGrant={invocation:issued.reference,annotation:{document:id('A'),id:operation==='create'?'fresh':'note',incarnation:1},identity:true,contentVersion:true,targetGeneration:true};
 }else{
  const seed={contract:'owned-annotation-plan/1',invocation:base.reference,descriptor:base.reference,sources:['A','B'].map(name=>({document:id(name),expectedSource:1,expectedAnnotationSet:1,edits:[{lo:0,hi:0,expected:[],ins:[88]}]})),annotations:[]};
  assert.equal(base.submit({kind:'compound',plan:seed}).status,'accepted');assert.equal(base.run({...h,sourceWritable:true}).status,'settled');
  const group=base.root().groups[0];h={...h,groups:[{reference:'original-group',internal:group.id,authority:'a'}],scopes:[{reference:'original-scope',internal:group.scopes[0],authority:'a'}],limits:{maxDependencies:8,maxParticipants:8,maxChanges:8,maxPlanBytes:65536,...h.viewWorkLimits}};
  if(operation==='redo'){
   const p=api.issueInvocation(base.domain,context,g);assert(p);const r={contract:'history-reversal/1',operation:'undo',invocation:p.reference,authority:'a',target:{kind:'group',group:'original-group'},profile:'execution',dependencies:[]},plan=api.prepareReversal(base.root(),r,h);assert(plan);assert.equal(api.submitInvocation(base.domain,p.ticket,{kind:'reversal',plan},'typed',context,policy,g).status,'accepted');assert.equal(api.runInvocation(base.domain,p.ticket,context,g,h).status,'settled');
  }
  issued=api.issueInvocation(base.domain,context,g);assert(issued);
  request={contract:'history-reversal/1',operation,invocation:issued.reference,authority:'a',target:targetKind==='group'?{kind:'group',group:'original-group'}:{kind:'scope-frontier',scope:'original-scope',expectedGroup:'original-group'},profile:'execution',dependencies:[]};
  g.historyHost=h;g.reversalResultGrant={invocation:issued.reference,authority:'a',target:copy(request.target),discloseTarget:true};
 }
 const row=engineInventory().find(f=>f.id===family),envelope=(r=request,route='typed')=>({route,request:{...(route==='typed'?{family:row.family}:{name:row.name}),variant:r.operation,version:1,arguments:r}});
 return {...base,...issued,request,g,h,family,envelope,
  submit:(r=request,route='typed',flags=g,limits=inputLimits,c=context)=>api.submitInvocationNativeLocalCommand(base.domain,issued.ticket,envelope(r,route),limits,c,policy,flags),
  run:(flags=g)=>api.runInvocation(base.domain,issued.ticket,context,g,flags),
  outcome:()=>api.observeInvocation(base.domain,issued.ticket,context,policy,g),
  result:(flags=g)=>family==='F13'?api.observeAnnotationResult(base.domain,issued.ticket,context,policy,flags):api.observeReversalResult(base.domain,issued.ticket,context,policy,flags)};
}
export function checkNativeLocalCommands(api=original){
 const controls=[],control=(name,f)=>{f();controls.push(name);};let variantCases=0,omissions=0,permissionCases=0;
 const variants=[...['create','update-body','reattach','delete'].map(op=>['F13',op]),...['undo','redo'].flatMap(op=>['group','scope-frontier'].map(target=>['F14',op,target]))];
 control('all-six-variants-and-both-history-target-forms-bind-native-replay-and-receipts',()=>{
  for(const [family,operation,target] of variants){const z=fixture(api,family,operation,target),before=z.root(),usage=z.usage();assert.deepEqual(z.submit(),{status:'accepted'});assert.equal(z.usage().declaredHeldBytes-usage.declaredHeldBytes,z.usage().ordinaryArgumentBytes);assert.equal(z.submit(z.request,'dynamic').status,'pending');assert.equal(z.run().status,'settled');assert.equal(z.root().publication,before.publication+1);assert.equal(z.outcome().outcome.publication,'applied');assert.equal(z.result().result.operation,operation);assert.equal(z.result().result.status,'applied');const root=z.root();assert.equal(z.submit(z.request,'dynamic').status,'observe-existing');assert.equal(z.run().status,'unavailable');assert.equal(z.root(),root);variantCases++;}
 });
 control('annotation-body-and-target-do-not-require-source-write-permission',()=>{
  for(const operation of ['create','update-body','reattach','delete']){const z=fixture(api,'F13',operation);assert.equal(z.h.sourceWritable,false);assert.equal(z.submit().status,'accepted');z.run();assert.equal(z.outcome().outcome.publication,'applied');}
  const z=fixture(api,'F13','update-body');z.request.body='old';z.h.versionsAvailable=false;assert.equal(z.submit().status,'accepted');const before=z.root();z.run();assert.equal(z.root(),before);assert.equal(z.outcome().outcome.publication,'unchanged');assert.equal(z.result().result.contentVersion,1);
 });
 control('each-native-owner-authority-and-original-limit-binding-is-current',()=>{
  for(const [family,operation] of [['F13','update-body'],['F14','undo']])for(const key of ['authorized',family==='F13'?'annotationWritable':'invocationFresh'])for(const value of [false,1]){const z=fixture(api,family,operation);assert.equal(z.submit().status,'accepted');const before=z.root();z.h[key]=value;z.run();assert.equal(z.root(),before);assert.equal(z.outcome().outcome.publication,'notApplied');permissionCases++;}
  for(const [family,operation] of [['F13','create'],['F14','undo']])for(const change of ['owner','limits']){const z=fixture(api,family,operation);assert.equal(z.submit().status,'accepted');const before=z.root();if(change==='owner')z.h.owner={};else if(family==='F13')z.h.annotationLimits={...z.h.annotationLimits,maxRecords:31};else z.h.limits={...z.h.limits,maxChanges:7};z.run();assert.equal(z.root(),before);assert.equal(z.outcome().outcome.publication,'notApplied');}
 });
 control('changed-public-intent-never-replaces-an-original-prepared-claim',()=>{
  for(const [family,operation] of [['F13','update-body'],['F14','undo']]){const z=fixture(api,family,operation),saved=copy(z.request);assert.equal(z.submit().status,'accepted');if(family==='F13')z.request.body='substitute';else z.request.target={kind:'group',group:'different'};assert.equal(z.submit().status,'unavailable');z.run();assert.equal(z.submit(saved,'dynamic').status,'observe-existing');if(family==='F13')assert.equal(z.root().documents[0].records.find(r=>r.id==='note').body,'changed');else assert.equal(Buffer.from(z.root().documents[0].source).toString(),'aé🙂z');}
 });
 control('create-allocation-is-original-and-both-point-profiles-survive',()=>{
  const z=fixture(api,'F13','create');assert.equal(z.submit().status,'accepted');z.h.annotationAllocation={...z.h.annotationAllocation,id:'replacement'};const before=z.root();z.run();assert.equal(z.root(),before);assert.equal(z.outcome().outcome.publication,'notApplied');
  for(const operation of ['create','reattach'])for(const profile of ['position','content']){const x=fixture(api,'F13',operation);x.request.target.target.profile=profile;assert.equal(x.submit().status,'accepted');x.run();const name=operation==='create'?'fresh':'note';assert.equal(x.root().documents[0].records.find(r=>r.id===name).target.target.profile,profile);}
 });
 control('queued-scope-frontier-never-follows-a-newer-group-or-reprepares-on-retry',()=>{
  const z=fixture(api,'F14','undo','scope-frontier');assert.equal(z.submit().status,'accepted');const issued=api.issueInvocation(z.domain,context,z.g);assert(issued);const d=z.root().documents[0],plan={contract:'owned-annotation-plan/1',invocation:issued.reference,descriptor:issued.reference,sources:[{document:id('A'),expectedSource:d.revision,expectedAnnotationSet:d.annotationSet,edits:[{lo:0,hi:0,expected:[],ins:[81]}]}],annotations:[]};
  assert.equal(api.submitInvocation(z.domain,issued.ticket,{kind:'compound',plan},'typed',context,policy,z.g).status,'accepted');assert.equal(api.runInvocation(z.domain,issued.ticket,context,z.g,{...z.h,sourceWritable:true}).status,'settled');z.h.groups.push({reference:'new-group',internal:issued.reference,authority:'a'});const before=z.root();assert.equal(z.submit().status,'pending');assert.equal(z.submit({...z.request,target:{...z.request.target,expectedGroup:'new-group'}}).status,'unavailable');z.run();assert.equal(z.root(),before);assert.equal(z.outcome().outcome.publication,'notApplied');assert.equal(z.result().result.status,'not-published');assert.equal(Object.hasOwn(z.result().result,'target'),false);
 });
 control('queued-history-participant-basis-is-not-refreshed-after-an-independent-body-edit',()=>{
  const z=fixture(api,'F14','undo','scope-frontier');assert.equal(z.submit().status,'accepted');const issued=api.issueInvocation(z.domain,context,z.g);assert(issued);const request={contract:'annotation-authoring/1',operation:'update-body',invocation:issued.reference,document:id('A'),profile:'execution',dependencies:[],annotation:{document:id('A'),id:'note',incarnation:1},expectedContent:1,body:'later'},plan=api.prepareAnnotationEdit(z.root(),request,z.h,z.h.annotationLimits);assert(plan);assert.equal(api.submitInvocation(z.domain,issued.ticket,{kind:'annotation',plan},'typed',context,policy,z.g).status,'accepted');assert.equal(api.runInvocation(z.domain,issued.ticket,context,z.g,z.h).status,'settled');const before=z.root();assert.equal(z.submit().status,'pending');z.run();assert.equal(z.root(),before);assert.equal(z.outcome().outcome.publication,'notApplied');assert.equal(z.root().documents[0].records[0].body,'later');
 });
 control('closed-native-fields-typed-dynamic-products-and-accessor-refusal',()=>{
  for(const [family,operation] of variants.slice(0,6)){const z=fixture(api,family,operation),a=normalizeNativeLocalCommandArguments(z.envelope(),context,inputLimits,65536),b=normalizeNativeLocalCommandArguments(z.envelope(z.request,'dynamic'),context,inputLimits,65536);assert(a&&b);assert.deepEqual(a.record,b.record);for(const key of Object.keys(z.request)){const r=copy(z.request),before=z.usage();delete r[key];assert.equal(z.submit(r).status,'unavailable');assert.deepEqual(z.usage(),before);omissions++;}assert.equal(z.submit({...z.request,callback:()=>true}).status,'unavailable');}
  const z=fixture(api,'F13','update-body'),before=z.usage();let invoked=false;Object.defineProperty(z.request,'body',{enumerable:true,get(){invoked=true;return 'bad';}});assert.equal(z.submit().status,'unavailable');assert.equal(invoked,false);assert.deepEqual(z.usage(),before);assert.equal(nativeLocalCommandInputForms().length,6);
 });
 control('actual-parent-headroom-and-native-profile-bounds-refuse-without-spending',()=>{
  for(const [family,operation] of [['F13','create'],['F14','undo']]){const seed=fixture(api,family,operation),base=seed.usage().declaredHeldBytes;assert.equal(seed.submit().status,'accepted');const charge=seed.usage().ordinaryArgumentBytes;
   for(const delta of [-1,0,1]){const z=fixture(api,family,operation,'group',{limits:{maxHeldBytes:base+charge+delta}}),before=z.usage();assert.equal(z.submit().status,delta<0?'unavailable':'accepted');if(delta<0)assert.deepEqual(z.usage(),before);}
   const z=fixture(api,family,operation),before=z.usage();if(family==='F13')z.h.annotationLimits={...z.h.annotationLimits,maxPlanBytes:65537};else z.h.limits={...z.h.limits,maxPlanBytes:65537};assert.equal(z.submit().status,'accepted');assert.equal(z.usage().ordinaryArgumentBytes,charge+2);z.run();assert.equal(z.outcome().outcome.publication,'applied');
   for(const delta of [-1,0,1]){const x=fixture(api,family,operation,'group',{limits:{maxHeldBytes:base+charge+2+delta}}),prior=x.usage();if(family==='F13')x.h.annotationLimits={...x.h.annotationLimits,maxPlanBytes:65537};else x.h.limits={...x.h.limits,maxPlanBytes:65537};assert.equal(x.submit().status,delta<0?'unavailable':'accepted');if(delta<0)assert.deepEqual(x.usage(),prior);}
   for(const value of [NaN,Infinity,-1,0.5]){const x=fixture(api,family,operation),prior=x.usage();if(family==='F13')x.h.annotationLimits={...x.h.annotationLimits,maxRecords:value};else x.h.limits={...x.h.limits,maxChanges:value};assert.equal(x.submit().status,'unavailable');assert.deepEqual(x.usage(),prior);}
  }
 });
 control('every-declared-native-limit-coordinate-is-required-before-preparation',()=>{
  for(const [family,operation,field] of [['F13','create','annotationLimits'],['F13','create','membershipLimits'],['F14','undo','limits'],['F14','undo','viewLimits']]){
   const seed=fixture(api,family,operation);
   for(const key of Object.keys(seed.h[field])){const z=fixture(api,family,operation),before=z.usage(),root=z.root();z.h[field]={...z.h[field]};delete z.h[field][key];assert.equal(z.submit().status,'unavailable',field+'.'+key);assert.deepEqual(z.usage(),before);assert.equal(z.root(),root);omissions++;}
  }
 });
 control('captured-result-ceilings-and-current-grants-remain-separate-from-execution',()=>{
  const z=fixture(api,'F13','create');z.g.annotationResultGrant.identity=false;z.g.annotationResultGrant.contentVersion=false;z.g.annotationResultGrant.targetGeneration=false;assert.equal(z.submit().status,'accepted');z.run();assert.equal(z.outcome().outcome.publication,'applied');z.g.annotationResultGrant.identity=true;z.g.annotationResultGrant.contentVersion=true;z.g.annotationResultGrant.targetGeneration=true;assert.equal(z.result().status,'unavailable');
  const x=fixture(api,'F14','undo');x.g.reversalResultGrant.discloseTarget=false;assert.equal(x.submit().status,'accepted');x.run();x.g.reversalResultGrant.discloseTarget=true;assert.equal(Object.hasOwn(x.result().result,'target'),false);
 });
 control('stop-fault-forget-and-protected-retirement-do-not-refund-issuance',()=>{
  for(const [family,operation] of [['F13','create'],['F14','undo']])for(const fault of [false,true]){const z=fixture(api,family,operation),before=z.root(),base=z.usage().declaredHeldBytes;assert.equal(z.submit().status,'accepted');if(fault)Object.defineProperty(z.h,'authorized',{get(){throw Error('injected');}});else api.stopInvocation(z.domain,z.ticket,z.g);assert.equal(z.run().status,fault?'faulted':'settled');assert.equal(z.root(),before);assert.equal(z.outcome().outcome.publication,fault?'unknown':'notApplied');assert.equal(api.retireInvocation(z.domain,z.ticket,{...z.g,deliveryDrained:false}).status,'unavailable');assert.equal(api.retireInvocation(z.domain,z.ticket,z.g).status,'retired');assert.equal(z.usage().declaredHeldBytes,base);assert.equal(z.submit().status,'unavailable');}
  const z=fixture(api,'F14','undo');z.submit();z.run();assert.equal(api.forgetInvocationResult(z.domain,z.ticket,z.g).status,'forgotten');assert.equal(z.result().status,'unavailable');assert.equal(z.submit().status,'observe-existing');
 });
 return {scope:'standalone-native-F13-F14-facade-binding-only',controls,variantCases,omissions,permissionCases,
  actualOriginalNativePlanAndReceiptsChecked:true,actualParentFundingChecked:true,
  independentSourceAnnotationAuthorityChecked:true,U01Selected:false,
  physicalIssuerHostOrFullFacadeProved:false,fullQualificationOrIndependentAssurance:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeLocalCommands()));
export {fixture as nativeLocalCommandFixture};
