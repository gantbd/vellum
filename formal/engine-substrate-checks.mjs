// Isolated post-import reference probe, not a sandbox or production conformance.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {readFileSync as esmRead} from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {spawnSync as esmSpawn} from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
const base=new URL('.',import.meta.url);
const load=name=>import(new URL(name,base));
const [{checkWorkspaceReads},{checkEngineSettings},{resourceReadFixture},{sourceSaveFixture},{checkAnnotationResourceRead},{sourceSerializationFixture}]=await Promise.all([
  load('workspace-read-checks.mjs'),load('engine-settings-checks.mjs'),load('resource-read-checks.mjs'),load('source-save-checks.mjs'),load('annotation-resource-read-checks.mjs'),load('source-serialization-checks.mjs')]);
// Prepare complete source-save module variants before interception. A coherent
// unchanged variant must pass the same path; import/parse failures are not kills.
const sourceURL=new URL('source-save.mjs',base),source=fs.readFileSync(sourceURL,'utf8');
const marker='export function dispatchSourceSave(owner,task,state,probe,g){';
assert(source.includes(marker));
const lower=s=>s.replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,sourceURL).href+q);
const prepared=[];
for(const [name,injection] of [['baseline',''],['hidden-read',"fs.readFileSync('not-accessed');"],['hidden-rename',"fs.renameSync('not-accessed','not-accessed');"],['hidden-process',"probeSpawn('not-launched');"]]) {
  const prefix=injection.includes('probeSpawn')?"import {spawnSync as probeSpawn} from 'node:child_process';\n":'';
  const code=lower(prefix+source.replace(marker,marker+injection));
  prepared.push({name,api:await import('data:text/javascript;base64,'+Buffer.from(code).toString('base64')+'#'+name)});
}
const serializationPrepared=[];
const [{checkBasisLifecycle,basisLifecycleFixture},{basisLifecycleTestGraph}]=await Promise.all([import('./basis-lifecycle-checks.mjs'),import('./basis-lifecycle-test-graph.mjs')]);
const basisPrepared=[],basisMarker='if(p.scoped){const current=lowerScopedSettings';
for(const [name,injection,call] of [['baseline','',''],['basis-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-accessed');",'fs.readFileSync'],['basis-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');",'fs.renameSync'],['basis-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']])basisPrepared.push({name,call,api:await basisLifecycleTestGraph(injection?[name,basisMarker,"if(p.scoped){if(p.value.scoped.request.contract===basisLifecycleInput.contract){"+injection+"}const current=lowerScopedSettings"]:null)});
const [{checkCreatedAtomic},{createdSettingsTestGraph},{scopedLifecycleFixture}]=await Promise.all([import('./settings-created-result-checks.mjs'),import('./settings-created-result-test-graph.mjs'),load('scoped-lifecycle-checks.mjs')]);
const createdPrepared=[];
const createdMarker='export function commitCreatedSettingsPublication(stage,hold,owner,domain,ticket,before,result,c,g){';
for(const [name,injection,call] of [['baseline','',''],['created-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-accessed');",'fs.readFileSync'],['created-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');",'fs.renameSync'],['created-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']])createdPrepared.push({name,call,api:await createdSettingsTestGraph(injection?[name,'state',createdMarker,createdMarker+injection]:null)});
const serializationMarker='export function publishSourceSerialization(before,plan,h,owner,domain,ticket){';
const moduleCode=(file,overrides={},mutation=null,prefix='')=>{
  const url=new URL(file,base);let code=fs.readFileSync(url,'utf8');
  if(mutation){assert.equal(code.split(serializationMarker).length,2);code=code.replace(serializationMarker,serializationMarker+mutation);}
  code=prefix+code;
  code=code.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,url).href));
  code=code.replace(/new URL\('(\.\.?\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,url).href)+',import.meta.url)');
  return 'data:text/javascript;base64,'+Buffer.from(code).toString('base64');
};
for(const [name,injection,call] of [['baseline','',''],['metadata-hidden-read',"probeFs.readFileSync('not-accessed');",'fs.readFileSync'],['metadata-hidden-rename',"probeFs.renameSync('not-accessed','not-accessed');",'fs.renameSync'],['metadata-hidden-process',"probeLaunch('not-launched');",'child_process.spawnSync']]){
 const prefix=injection?"import probeFs from 'node:fs';\nimport {spawnSync as probeLaunch} from 'node:child_process';\n":'';
 const state=moduleCode('source-serialization-state.mjs',{},injection||null,prefix),editing=moduleCode('editing-intents.mjs',{'./source-serialization-state.mjs':state});
 const kernel=moduleCode('invocation-publication.mjs',{'./source-serialization-state.mjs':state,'./editing-intents.mjs':editing});
 serializationPrepared.push({name,call,api:await import(kernel)});
}
const held=[],attempts=[],unwrapped=[];
let report;
try {
  for(const [label,o] of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(o)) {
    const descriptor=Object.getOwnPropertyDescriptor(o,key);
    if(typeof o[key]!=='function')continue;
    if(!descriptor.configurable&&!descriptor.writable){unwrapped.push(label+'.'+key);continue;}
    held.push({o,key,descriptor});
    Object.defineProperty(o,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{
      attempts.push(label+'.'+key);const e=new Error('denied host call');e.code='SUBSTRATE_PROBE_DENIED';throw e;
    }});
  }
  assert.deepEqual(unwrapped,[],'every discovered callable export must be intercepted');
  syncBuiltinESMExports();
  const controls=[()=>fs.readFileSync('not-accessed'),()=>fs.renameSync('not-accessed','not-accessed'),()=>fsp.readFile('not-accessed'),()=>cp.spawnSync('not-launched'),()=>esmRead('not-accessed'),()=>esmSpawn('not-launched')];
  for(const invoke of controls)assert.throws(invoke,{code:'SUBSTRATE_PROBE_DENIED'});
  assert.equal(attempts.length,controls.length);attempts.length=0;
  const workspace=checkWorkspaceReads(),settings=checkEngineSettings(),annotation=checkAnnotationResourceRead();
  const created=checkCreatedAtomic(createdPrepared[0].api);assert.deepEqual(attempts,[]);
  const basis=checkBasisLifecycle(basisPrepared[0].api);assert.deepEqual(attempts,[]);
  const r=resourceReadFixture(),t=r.admit();r.submit(t);r.complete(t);assert.equal(r.activateTask(t).status,'created');
  const s=sourceSaveFixture(),a=s.admit();s.start(a);assert.equal(s.result(a).status,'recorded');
  let serializationRoutes=0;
  for(const route of ['typed','dynamic']){const x=sourceSerializationFixture(undefined,{route});assert.equal(x.submit(route).status,'accepted');assert.equal(x.run().status,'settled');assert.equal(x.root().documents[0].serialization.generation,2);serializationRoutes++;}
  assert.deepEqual(attempts,[]);
  const mutationKills=[];
  for(const {name,api} of prepared) {
    const run=()=>{const x=sourceSaveFixture(api),t=x.admit();x.start(t);assert.equal(x.result(t).status,'recorded');};
    if(name==='baseline'){run();assert.deepEqual(attempts,[]);continue;}
    assert.throws(run,{code:'SUBSTRATE_PROBE_DENIED'},name);assert.equal(attempts.length,1,name);
    mutationKills.push(name);attempts.length=0;
  }
  const serializationMutationKills=[];let serializationMutationCases=0,serializationBaselineRoutes=0;
  for(const {name,call,api}of serializationPrepared){
   for(const route of ['typed','dynamic']){
    const x=sourceSerializationFixture(api,{route}),before=x.root(),issued=api.invocationUsage(x.domain).issued;assert.equal(x.submit(route).status,'accepted');
    const outcome=x.run();
    if(name==='baseline'){assert.equal(outcome.status,'settled');assert.equal(x.root().documents[0].serialization.generation,2);assert.deepEqual(attempts,[]);serializationBaselineRoutes++;continue;}
    assert.equal(outcome.status,'faulted',name);assert.equal(x.root(),before,name);assert.deepEqual(attempts,[call],name);
    const observed=api.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome;
    assert.equal(observed.publication,'unknown',name);assert.equal(observed.control,'faulted',name);
    assert.equal(api.issueInvocation(x.domain,x.c,x.h()),null,name);
    assert.equal(api.retireInvocation(x.domain,x.ticket,x.h()).status,'retired',name);
    assert.equal(api.invocationUsage(x.domain).issued,issued,name);
    serializationMutationCases++;attempts.length=0;
   }
   if(name!=='baseline')serializationMutationKills.push(name);
  }
  let createdResultBaselineRoutes=0,createdResultMutationCases=0;const createdResultMutationKills=[];
  for(const {name,call,api}of createdPrepared){for(const route of ['typed','dynamic']){
   const x=scopedLifecycleFixture(api,{route,host:{settingsCreatedResultReferencesAllowed:true,settingsCreatedResultReadable:true,settingsCreatedResultContextCurrent:true,historicalSettingsLayersReadable:true}}),before=x.root(),usage=api.invocationUsage(x.domain),hold=api.prepareInvocationSettingsCreatedResult(x.domain,x.ticket,x.c,x.h());assert(hold);assert.equal(x.submit(route).status,'accepted');const out=x.run();
   if(name==='baseline'){assert.equal(out.status,'settled');assert.equal(api.observeSettingsCreatedResult(x.domain,x.ticket,x.c,x.policy,x.h()).status,'result');assert.deepEqual(attempts,[]);createdResultBaselineRoutes++;continue;}
   assert.equal(out.status,'faulted',name);assert.equal(x.root(),before,name);assert.deepEqual(attempts,[call],name);assert.equal(x.read().outcome.publication,'unknown',name);assert.equal(api.observeSettingsCreatedResult(x.domain,x.ticket,x.c,x.policy,x.h()).status,'unavailable');assert.equal(api.issueInvocation(x.domain,x.c,x.h()),null);assert.equal(api.retireInvocation(x.domain,x.ticket,x.h()).status,'retired');assert.equal(api.invocationUsage(x.domain).declaredHeldBytes,usage.declaredHeldBytes);assert.equal(api.invocationUsage(x.domain).issued,usage.issued);createdResultMutationCases++;attempts.length=0;
  }if(name!=='baseline')createdResultMutationKills.push(name);}
  let basisLifecycleBaselineRoutes=0,basisLifecycleMutationCases=0;const basisLifecycleMutationKills=[];
  for(const {name,call,api}of basisPrepared){for(const route of ['typed','dynamic']){
   const x=basisLifecycleFixture(api,{noPrepare:true}),before=x.root(),usage=api.invocationUsage(x.domain),plan=x.prepare(x.request,[x.writeBasis],x.lifecycleBasis,x.h(),route);assert(plan);assert.equal(x.submit(plan,route).status,'accepted');const out=x.run();
   if(name==='baseline'){assert.equal(out.status,'settled');assert.equal(x.outcome().outcome.publication,'applied');assert.deepEqual(attempts,[]);basisLifecycleBaselineRoutes++;continue;}
   assert.equal(out.status,'faulted',name);assert.equal(x.root(),before,name);assert.deepEqual(attempts,[call],name);assert.equal(x.outcome().outcome.publication,'unknown');assert.equal(api.issueInvocation(x.domain,x.c,x.h()),null);assert.equal(api.retireInvocation(x.domain,x.ticket,x.h()).status,'retired');assert.equal(api.invocationUsage(x.domain).issued,usage.issued);for(const [retire,b]of [[api.retireInvocationSettingsLifecycleBasis,x.lifecycleBasis],[api.retireInvocationSettingsWriteBasis,x.writeBasis]]){assert.equal(retire(x.domain,b,x.c,x.h({retentionReleased:false})),false);assert(retire(x.domain,b,x.c,x.h()));}basisLifecycleMutationCases++;attempts.length=0;
  }if(name!=='baseline')basisLifecycleMutationKills.push(name);}
  report={scope:'selected-post-import-reference-execution-only',wrappedFunctions:held.length,unwrapped,basisLifecycleControls:basis.controls.length,basisLifecycleBaselineRoutes,basisLifecycleMutationCases,basisLifecycleMutationKills,basisLifecycleMutationBaselineChecked:true,
    denialControls:controls.length,workspaceControls:workspace.controls.length,settingsControls:settings.controls.length,
    annotationReadControls:annotation.controls.length,sourceReadActivated:true,saveEvidenceRecorded:true,
    attemptedHostCalls:attempts.length,createdResultControls:created.controls.length,createdResultBaselineRoutes,createdResultMutationCases,createdResultMutationKills,createdResultMutationBaselineChecked:true,serializationRoutes,serializationBaselineRoutes,serializationMutationCases,serializationMutationKills,serializationMutationBaselineChecked:true,mutationBaselineChecked:true,mutationKills,importIndependenceProved:false,allEnginePathsProved:false,
    actualExternalWriteProved:false,hostTruthProved:false,implementationRefinement:false};
} finally {
  for(const {o,key,descriptor} of held.reverse())Object.defineProperty(o,key,descriptor);
  syncBuiltinESMExports();
}
for(const {o,key,descriptor} of held)assert.deepEqual(Object.getOwnPropertyDescriptor(o,key),descriptor);
console.log(JSON.stringify(report));
