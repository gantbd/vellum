// Actual engine root binding, not parser truth or a full analysis facade.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as analysis from './analysis-work.mjs';
import * as kernel from './invocation-publication.mjs';
import {analysisWorkFixture} from './analysis-work-checks.mjs';
import {annotationObservationFixture,annotationObservationContext as executionContext,annotationObservationGates as executionGates,annotationObservationPolicy as executionPolicy} from './annotation-observation-checks.mjs';
const copy=structuredClone,no={status:'unavailable'};
export function analysisOwnerBindingFixture(api=analysis,seed=analysisWorkFixture(),base=annotationObservationFixture().current()){
  const state=copy(base),doc=state.documents[0];doc.source=copy(seed.state.documents[0].source);
  const c={...copy(seed.c),document:copy(doc.document)};
  const row={...copy(c),provider:copy(seed.gates.provider)};delete row.sourceRevision;
  state.analysisContexts=[row];state.views=seed.state.views.map(v=>({...copy(v),document:copy(doc.document),profile:'explicit-positional-selection/1',selections:[{anchor:0,head:0,affinity:'before'}],primary:0,selectionGeneration:1,attachmentGeneration:1,viewportGeneration:1}));
  const z=annotationObservationFixture(undefined,{state}),g={...seed.gates,scope:true,ownerControlAllowed:true,resourcesReady:true,preparationReserved:true};
  const owner=api.newInvocationAnalysisRegistry(z.domain,seed.limits,{maxRootBytes:32768,maxReceipts:16,maxBatches:32,maxEdits:64,maxCoordinate:4294967295,maxPlanBytes:262144},g);assert(owner);
  const root=()=>z.current(),request=(name='one')=>({...seed.request(name),context:copy(c)}),packet=()=>({...seed.packet(),context:copy(c)}),
   admit=(name='one')=>api.admitInvocationAnalysisWork(owner,request(name),g),
   event=(task,kind='result',name='one')=>({...seed.event(task,kind,packet(),name),...(kind==='result'?{packet:packet()}:{})}),
   query=snapshot=>({...seed.query(snapshot),view:copy(state.views[0].view)}),
   ready=()=>{const t=admit();assert.equal(t.status,'admitted');assert.equal(api.submitInvocationAnalysisWork(owner,t.task,g).status,'submitted');assert.equal(api.observeAnalysisWork(owner,event(t.task),seed.ingress).status,'recorded');const a=api.acceptInvocationAnalysisWork(owner,t.task,g);assert.equal(a.status,'analysis');return {task:t.task,snapshot:a.snapshot};};
  return {z,c,g,owner,root,request,packet,admit,event,query,ready};
}
export function checkAnalysisOwnerBinding(api=analysis){
 const seed=analysisWorkFixture(),base=annotationObservationFixture().current(),controls=[];let strictOrdinaryCases=0,strictProtectedCases=0;
 const make=()=>analysisOwnerBindingFixture(api,seed,base);
 const check=(name,fn)=>{fn();controls.push(name);};
 check('actual-domain-only-and-one-nonrenewing-native-owner',()=>{
  const x=make(),parser={maxRootBytes:32768,maxReceipts:16,maxBatches:32,maxEdits:64,maxCoordinate:4294967295,maxPlanBytes:262144};
  for(const domain of [copy(x.z.domain),{},null])assert.equal(api.newInvocationAnalysisRegistry(domain,seed.limits,parser,x.g),null);
  assert.equal(api.newInvocationAnalysisRegistry(x.z.domain,seed.limits,parser,x.g),null);
 });
 check('caller-root-copy-and-other-domain-cannot-bypass-native-owner',()=>{
  const x=make(),y=make(),a=x.admit();assert.equal(a.status,'admitted');
  for(const state of [copy(x.root()),y.root()]){
   assert.deepEqual(api.admitAnalysisWork(x.owner,state,x.request('foreign'),x.g),no);
   assert.equal(api.submitAnalysisWork(x.owner,a.task,state,x.g).status,'not-submitted');
  }
  assert.equal(api.submitInvocationAnalysisWork(x.owner,a.task,x.g).status,'submitted');
  assert.equal(api.observeAnalysisWork(x.owner,x.event(a.task),seed.ingress).status,'recorded');
  for(const state of [copy(x.root()),y.root()])assert.deepEqual(api.acceptAnalysisWork(x.owner,a.task,state,x.g),no);
 });
 check('native-query-uses-owned-current-root-and-preserves-engine-frame',()=>{
  const x=make(),root=x.root(),usage=copy(kernel.invocationUsage(x.z.domain)),a=x.ready();
  for(const operation of ['supported-analysis','supplied-evidence-inspection']){
   const q=x.query(a.snapshot);q.operation=operation;if(operation==='supplied-evidence-inspection')delete q.range;
   assert.equal(api.queryInvocationAnalysis(x.owner,q,x.g).status,operation==='supported-analysis'?'captures':'analysis-summary');
   assert.deepEqual(api.queryAnalysis(x.owner,copy(root),q,x.g),no);
  }
  assert.equal(x.root(),root);assert.deepEqual(kernel.invocationUsage(x.z.domain),usage);
 });
 check('native-owner-follows-real-publication-not-captured-root-pointer',()=>{
  const x=make(),a=x.ready(),before=x.root(),p=x.z.prepare({operation:'update-body'});x.z.execute(p);
  assert.notEqual(x.root(),before);
  assert.equal(api.queryInvocationAnalysis(x.owner,x.query(a.snapshot),x.g).status,'captures');
  assert.deepEqual(api.queryAnalysis(x.owner,before,x.query(a.snapshot),x.g),no);
 });
 check('native-source-edit-stales-old-analysis-despite-retained-root',()=>{
  const x=make(),a=x.ready(),before=x.root();x.z.source();assert.notEqual(x.root(),before);
  assert.deepEqual(api.queryInvocationAnalysis(x.owner,x.query(a.snapshot),x.g),no);
  assert.deepEqual(api.queryAnalysis(x.owner,before,x.query(a.snapshot),x.g),no);
  assert.deepEqual(api.admitInvocationAnalysisWork(x.owner,x.request('stale'),x.g),no);
 });
 check('native-root-fault-fences-ordinary-work-not-late-evidence-or-cleanup',()=>{
  const x=make(),a=x.admit();assert.equal(a.status,'admitted');assert.equal(api.submitInvocationAnalysisWork(x.owner,a.task,x.g).status,'submitted');
  const issued=kernel.issueInvocation(x.z.domain,executionContext,executionGates),plan={contract:'owned-annotation-plan/1',invocation:issued.reference,descriptor:issued.reference,sources:[],annotations:[{kind:'body',document:null,id:'note',incarnation:1,expectedBody:1,body:'surprise'}]};
  assert.equal(kernel.submitInvocation(x.z.domain,issued.ticket,{kind:'compound',plan},'typed',executionContext,executionPolicy,executionGates).status,'accepted');
  assert.equal(kernel.runInvocation(x.z.domain,issued.ticket,executionContext,executionGates,x.z.h).status,'faulted');
  assert.deepEqual(api.admitInvocationAnalysisWork(x.owner,x.request('after-fault'),x.g),no);
  assert.equal(api.observeAnalysisWork(x.owner,x.event(a.task),seed.ingress).status,'recorded');
  assert.deepEqual(api.acceptInvocationAnalysisWork(x.owner,a.task,x.g),no);
  assert.equal(api.controlAnalysisWork(x.owner,a.task,'cancel',x.g).status,'stopped');
  assert.equal(api.observeAnalysisWork(x.owner,x.event(a.task,'quiescence'),seed.ingress).status,'quiescent');
  assert.equal(api.retireAnalysisWork(x.owner,a.task,seed.retirement).status,'retired');
 });
 check('native-ordinary-authority-facts-require-exact-true',()=>{
  const x=make(),a=x.admit();assert.equal(a.status,'admitted');
  const launch=['authorized','sourceReadable','disclosureAllowed','continuous','live','notStopped','providerAvailable','capacity','reservationReady','scope','contextAllowed'];
  for(const key of launch)for(const value of [false,1,undefined]){
   const g={...x.g,[key]:value},before=copy(api.analysisUsage(x.owner));
   assert.deepEqual(api.admitInvocationAnalysisWork(x.owner,x.request('denied'),g),no);
   assert.equal(api.submitInvocationAnalysisWork(x.owner,a.task,g).status,'not-submitted');
   assert.deepEqual(api.analysisUsage(x.owner),before);strictOrdinaryCases+=2;
  }
  assert.equal(api.submitInvocationAnalysisWork(x.owner,a.task,x.g).status,'submitted');api.observeAnalysisWork(x.owner,x.event(a.task),seed.ingress);
  const acceptance=['authorized','sourceReadable','disclosureAllowed','continuous','live','notStopped','acceptanceAllowed','scope','contextAllowed'];
  for(const key of acceptance)for(const value of [false,1,undefined]){assert.deepEqual(api.acceptInvocationAnalysisWork(x.owner,a.task,{...x.g,[key]:value}),no);strictOrdinaryCases++;}
  const accepted=api.acceptInvocationAnalysisWork(x.owner,a.task,x.g);assert.equal(accepted.status,'analysis');
  for(const key of ['authorized','scope','continuous','live','retained','notStopped','replyReady','contextAllowed'])for(const value of [false,1,undefined]){assert.deepEqual(api.queryInvocationAnalysis(x.owner,x.query(accepted.snapshot),{...x.g,[key]:value}),no);strictOrdinaryCases++;}
 });
 check('native-open-grants-refuse-before-consuming-the-domain-slot',()=>{
  const z=annotationObservationFixture(undefined,{state:{...copy(base),analysisContexts:[]}}),parser={maxRootBytes:32768,maxReceipts:16,maxBatches:32,maxEdits:64,maxCoordinate:4294967295,maxPlanBytes:262144},g={ownerControlAllowed:true,resourcesReady:true,preparationReserved:true};
  for(const key of Object.keys(g))for(const value of [false,1,undefined]){assert.equal(api.newInvocationAnalysisRegistry(z.domain,seed.limits,parser,{...g,[key]:value}),null);strictProtectedCases++;}
  assert(api.newInvocationAnalysisRegistry(z.domain,seed.limits,parser,g));
 });
 check('native-producer-control-and-retirement-facts-are-not-truthy',()=>{
  const x=make(),a=x.admit();api.submitInvocationAnalysisWork(x.owner,a.task,x.g);
  for(const key of ['authentic','evidenceAllowed','ingressReady'])for(const value of [false,1,undefined]){assert.deepEqual(api.observeAnalysisWork(x.owner,x.event(a.task),{...seed.ingress,[key]:value}),no);strictProtectedCases++;}
  assert.equal(api.observeAnalysisWork(x.owner,x.event(a.task),seed.ingress).status,'recorded');
  for(const value of [false,1,undefined]){
   assert.equal(api.analysisResultMatches(x.owner,a.task,x.packet(),{nativeBindingAllowed:value}),false);
   assert.deepEqual(api.controlAnalysisWork(x.owner,a.task,'cancel',{...x.g,controlAllowed:value}),no);
   assert.deepEqual(api.observeAnalysisWork(x.owner,x.event(a.task,'quiescence'),{...seed.ingress,quiescenceAttested:value}),no);strictProtectedCases+=3;
  }
  assert.equal(api.analysisResultMatches(x.owner,a.task,x.packet(),{nativeBindingAllowed:true}),true);
  api.controlAnalysisWork(x.owner,a.task,'cancel',x.g);api.observeAnalysisWork(x.owner,x.event(a.task,'quiescence'),seed.ingress);
  for(const key of Object.keys(seed.retirement))for(const value of [false,1,undefined]){assert.deepEqual(api.retireAnalysisWork(x.owner,a.task,{...seed.retirement,[key]:value}),no);strictProtectedCases++;}
  assert.equal(api.retireAnalysisWork(x.owner,a.task,seed.retirement).status,'retired');
  assert.deepEqual(x.admit(),no);
 });
 check('absent-or-malformed-optional-analysis-inventory-refuses-without-enablement',()=>{
  const parser={maxRootBytes:32768,maxReceipts:16,maxBatches:32,maxEdits:64,maxCoordinate:4294967295,maxPlanBytes:262144},g={ownerControlAllowed:true,resourcesReady:true,preparationReserved:true};
  for(const inventory of [undefined,null,{},[null],[{}]]){
   const state=copy(base);if(inventory!==undefined)state.analysisContexts=inventory;
   const z=annotationObservationFixture(undefined,{state}),before=z.current(),usage=copy(kernel.invocationUsage(z.domain));
   assert.equal(api.newInvocationAnalysisRegistry(z.domain,seed.limits,parser,g),null);
   assert.equal(z.current(),before);assert.deepEqual(kernel.invocationUsage(z.domain),usage);
  }
  const z=annotationObservationFixture(undefined,{state:{...copy(base),analysisContexts:[]}}),owner=api.newInvocationAnalysisRegistry(z.domain,seed.limits,parser,g);assert(owner);
  assert.deepEqual(api.admitInvocationAnalysisWork(owner,seed.request(),{...seed.gates,...g}),no);
 });
 return {scope:'actual-engine-analysis-owner-root-binding-only',controls,strictOrdinaryCases,strictProtectedCases,actualDomainAndCurrentRootChecked:true,legacyRawOwnerStillAvailable:true,originalInvocationRetryOrF16ActivationProved:false,allF08F24F28OrEventFacadeProved:false,parentGlobalOrPhysicalFundingProved:false,hostParserTruthConcurrentCrashOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkAnalysisOwnerBinding()));}catch(e){console.error(e.code+': '+String(e.message).slice(0,500));process.exitCode=1;}
