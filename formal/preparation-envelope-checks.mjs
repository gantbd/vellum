// Selected parent-envelope preflight, not physical memory or whole-admission proof.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {pathToFileURL,fileURLToPath} from 'node:url';
import {projectInputs} from './evidence-harness.mjs';
const size=v=>Buffer.byteLength(JSON.stringify(v)),copy=structuredClone;
const sourceRoot=fileURLToPath(new URL('..',import.meta.url));
const mutations=[
 ['registration-wrapper-preflight','registration-invocation-state.mjs',"size({kind:'registration',plan:value})",'size(value)'],
 ['relation-wrapper-preflight','relation-invocation-state.mjs',"size({kind:'relation',plan:value})",'size(value)'],
 ['lifecycle-wrapper-preflight','lifecycle-invocation-state.mjs',"size({kind:'lifecycle',plan:value})",'size(value)'],
 ['saveas-wrapper-preflight','saveas-invocation-state.mjs',"size({kind:'lifecycle',plan:value})",'size(value)'],
 ['source-load-wrapper-preflight','load-invocation-state.mjs',"size({kind:'lifecycle',plan:value})",'size(value)'],
 ['annotation-load-wrapper-preflight','annotation-load-invocation-state.mjs',"size({kind:'lifecycle',plan:value})",'size(value)'],
 ['scoped-registration-wrapper-preflight','registration-invocation-state.mjs',"||size({kind:'registration',plan:p.value})>owner.limits.maxIntentBytes",''],
 ['configured-line-wrapper-preflight','configured-line-state.mjs',"||Buffer.byteLength(JSON.stringify({kind:'line',plan:payload.value}))>owner.limits.maxIntentBytes",'']
];
const ownerExport='export function __envelopeBoundaryOwner(domain){return domains.get(domain);}';
async function checkGraph(directory){
 const load=name=>import(pathToFileURL(path.join(directory,'formal',name)).href),k=await load('invocation-publication.mjs');
 const modules=await Promise.all(['registration-invocation-checks.mjs','relation-invocation-checks.mjs','lifecycle-invocation-checks.mjs','saveas-invocation-checks.mjs','load-invocation-checks.mjs','annotation-load-invocation-checks.mjs','registration-scoped-checks.mjs','configured-line-checks.mjs','registration-invocation-state.mjs','relation-invocation-state.mjs','lifecycle-invocation-state.mjs','configured-line-state.mjs'].map(load));
 const [registration,relation,lifecycle,saveas,sourceLoad,annotationLoad,scoped,configured,registrationState,relationState,lifecycleState,lineState]=modules;
 const rows=[
  ['registration',()=>registration.registrationInvocationFixture(),'registration','prepareInvocationRegistrationIntent',[],'registrationPrepared'],
  ['relation',()=>relation.relationInvocationFixture(),'relation','prepareInvocationRelationIntent',[],'relationPrepared'],
  ['lifecycle',()=>lifecycle.lifecycleInvocationFixture(),'lifecycle','prepareInvocationLifecycleIntent',['inspection'],'lifecyclePrepared'],
  ['saveas',()=>saveas.saveAsInvocationFixture(),'lifecycle','prepareInvocationSaveAsIntent',['authority'],'lifecyclePrepared'],
  ['load',()=>sourceLoad.loadInvocationFixture(),'lifecycle','prepareInvocationLoadIntent',['authority'],'lifecyclePrepared'],
  ['annotation-load',()=>annotationLoad.annotationLoadInvocationFixture(),'lifecycle','prepareInvocationAnnotationLoadIntent',['authority','inspection'],'lifecyclePrepared'],
  ['scoped-registration',()=>scoped.scopedRegistrationInvocationFixture(undefined,'empty'),'registration',null,[],'registrationPrepared'],
  ['configured-line',()=>configured.configuredLineFixture(),'line',null,[],'configuredLinePrepared']
 ];
 let cases=0,retries=0,pendingRetries=0,refusalRecovery=0,leaseCases=0;
 for(const [name,factory,kind,method,args,slotField]of rows)for(const offset of [-1,0,1])for(const route of ['typed','dynamic']){
  const x=factory(),o=k.__envelopeBoundaryOwner(x.domain),old=o.limits,c=x.controlContext,g=x.flags??x.g(),p=x.engineProfile??x.profile;
  const capture=plan=>kind==='registration'?registrationState.captureRegistrationInvocation(plan,o,x.domain,plan===x.plan?x.ticket:issued.ticket,old.maxIntentBytes):kind==='relation'?relationState.captureRelationInvocation(plan,o,x.domain,plan===x.plan?x.ticket:issued.ticket,old.maxIntentBytes):kind==='line'?lineState.configuredLineIntent(plan,old.maxIntentBytes,o,x.domain,plan===x.plan?x.ticket:issued.ticket):lifecycleState.captureLifecycleInvocation(plan,o,x.domain,plan===x.plan?x.ticket:issued.ticket,old.maxIntentBytes);
  const captured=capture(x.plan);assert(captured,name);const issued=k.issueInvocation(x.domain,c,g);assert(issued);assert.equal(issued.reference.length,x.reference.length);
  const request={...copy(x.request),invocation:issued.reference},value=copy(captured.value);
  // These fixtures issue equal-length references. Actual fresh captures below
  // check the predicted byte count rather than trusting that length alone.
  value.request.invocation=issued.reference;if(value.scoped)value.scoped.request.invocation=issued.reference;
  const wrapperBytes=size({kind,plan:value});assert(wrapperBytes>size(value));
  if(name==='scoped-registration')assert(size({value,bases:x.refs})<wrapperBytes);
  const prepare=()=>method?k[method](x.domain,issued.ticket,request,p,...args.map(a=>x[a]),c,g):name==='scoped-registration'?x.prepare(request,p,x.refs,x.scope,g,x.owner,issued.ticket):k.prepareInvocationConfiguredLine(x.domain,issued.ticket,{route,request:{...(route==='typed'?{family:10}:{name:'change-source'}),variant:'ordinary-edit',version:1,arguments:request}},p,c,g);
  try{
   o.limits={...old,maxIntentBytes:wrapperBytes+offset};
   const root=x.root(),floor=o.registrationFloor,usage=k.invocationUsage(x.domain),settings=k.invocationSemanticSettingsUsage(x.domain),beforeSlot=o.slots.get(issued.ticket)[slotField];assert.equal(beforeSlot,kind==='line'?undefined:null);
   const plan=prepare();assert.equal(Boolean(plan),offset>=0,name+'/'+route+'/'+offset);
   assert.equal(x.root(),root);assert.equal(o.registrationFloor,floor);assert.deepEqual(k.invocationUsage(x.domain),usage);
   if(offset<0){
    assert.equal(o.slots.get(issued.ticket)[slotField],beforeSlot);assert.deepEqual(k.invocationSemanticSettingsUsage(x.domain),settings);
    o.limits=old;assert(prepare(),name+'/recovery');refusalRecovery++;
   }else{
    assert.equal(o.slots.get(issued.ticket)[slotField],plan);assert.equal(size({kind,plan:capture(plan).value}),wrapperBytes,name+'/actual-capture');
    const held=k.invocationSemanticSettingsUsage(x.domain);
    if(kind==='line'){assert.equal(held.contexts,settings.contexts+1);assert.equal(held.issued,settings.issued+1);leaseCases++;}else assert.deepEqual(held,settings);
    assert.equal(prepare(),plan);assert.deepEqual(k.invocationSemanticSettingsUsage(x.domain),held);retries++;
    const job={kind,plan};assert.equal(k.submitInvocation(x.domain,issued.ticket,job,route,c,x.controlPolicy,g).status,'accepted');
    assert.equal(k.submitInvocation(x.domain,issued.ticket,job,route==='typed'?'dynamic':'typed',c,x.controlPolicy,g).status,'pending');pendingRetries++;
    assert.equal(x.root(),root);assert.equal(o.registrationFloor,floor);
   }
   cases++;
  }finally{o.limits=old;}
 }
 return {cases,paths:rows.length,retries,pendingRetries,refusalRecovery,leaseCases};
}
export const preparationEnvelopeMutationNames=Object.freeze(mutations.map(m=>m[0]));
export async function checkOwnedPreparationEnvelopes(){
 const inputs=projectInputs(sourceRoot),directory=fs.mkdtempSync(path.join(os.tmpdir(),'vellum-parent-envelope-graphs-'));
 try{
  let index=0;
  const graph=mutation=>{
   const target=path.join(directory,String(index++));fs.mkdirSync(target);fs.mkdirSync(path.join(target,'formal'));
   for(const file of inputs)fs.copyFileSync(path.join(sourceRoot,file),path.join(target,file));
   const kernel=path.join(target,'formal/invocation-publication.mjs'),source=fs.readFileSync(kernel,'utf8');assert(!source.includes(ownerExport));fs.writeFileSync(kernel,source+'\n'+ownerExport+'\n');
   if(mutation){const file=path.join(target,'formal',mutation[1]),source=fs.readFileSync(file,'utf8');assert(source.includes(mutation[2]),mutation[0]);fs.writeFileSync(file,source.replace(mutation[2],mutation[3]));}
   return target;
  };
  const baseline=await checkGraph(graph(null));assert.deepEqual(baseline,{cases:48,paths:8,retries:32,pendingRetries:32,refusalRecovery:16,leaseCases:4});
  const killed=[];for(const mutation of mutations){let failed=false;const target=graph(mutation);try{await checkGraph(target);}catch(e){assert.equal(e.code,'ERR_ASSERTION',mutation[0]+': '+String(e.message).slice(0,150));failed=true;}assert(failed,mutation[0]);killed.push(mutation[0]);}
  return {scope:'owned-preparation-parent-envelope-only',...baseline,mutationBaselineChecked:true,mutations:killed,generatedGraphTestOwnerExposure:true,actualPrivatePreparationAndSubmitChecked:true,refusalFramesAndLeasePreflightChecked:true,publicWireOrPhysicalAccountingProved:false,hostConcurrentCrashOrIndependentRefinementProved:false};
 }finally{fs.rmSync(directory,{recursive:true,force:true});}
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkOwnedPreparationEnvelopes()));}catch(e){console.error(String(e.stack??e));process.exitCode=1;}
