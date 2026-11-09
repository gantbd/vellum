import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {lifecycleActivationArgumentFixture} from './lifecycle-activation-argument-checks.mjs';
import {lifecycleActivationInputForms} from './lifecycle-activation-arguments.mjs';
export const activationEnvelopeIds=['source-load','annotation-restore','annotation-baseline','source-saveas','source-rebind','view-container-create','view-container-retire','workspace-container-create','workspace-container-retire'];
const limits={maxRequestBytes:32768,maxInputNodes:8192,maxInputDepth:128};
export function activationEnvelopeFixture(id='source-load',route='typed'){
 const z=lifecycleActivationArgumentFixture(id),issued=kernel.issueInvocation(z.domain,z.controlContext,z.g());assert(issued);
 const request={...structuredClone(z.request),invocation:issued.reference},form=lifecycleActivationInputForms().find(f=>f.id===id);
 const ports=id==='source-load'||id==='source-saveas'?{authority:z.authority}:id.startsWith('annotation-')?{authority:z.authority,inspection:z.inspection}:id==='source-rebind'?{inspection:z.inspection}:{};
 const facts=()=>({...z.g(),outsidePublicationGuard:true});
 const envelope=()=>({route,request:{...(route==='typed'?{family:16}:{name:'change-document-lifecycle'}),variant:form.variant,version:1,arguments:structuredClone(request)}});
 const prepare=(e=envelope(),p=ports,c=z.controlContext,g=facts(),l=limits,domain=z.domain,ticket=issued.ticket)=>kernel.prepareInvocationLifecycleActivationEnvelope(domain,ticket,e,l,z.profile,p,c,g);
 const nativePrepare=()=>id==='source-load'?kernel.prepareInvocationLoadIntent(z.domain,issued.ticket,request,z.profile,z.authority,z.controlContext,facts()):id.startsWith('annotation-')?kernel.prepareInvocationAnnotationLoadIntent(z.domain,issued.ticket,request,z.profile,z.authority,z.inspection,z.controlContext,facts()):id==='source-saveas'?kernel.prepareInvocationSaveAsIntent(z.domain,issued.ticket,request,z.profile,z.authority,z.controlContext,facts()):id==='source-rebind'?kernel.prepareInvocationLifecycleIntent(z.domain,issued.ticket,request,z.profile,z.inspection,z.controlContext,facts()):kernel.prepareInvocationRelationIntent(z.domain,issued.ticket,request,z.profile,z.controlContext,facts());
 const usage=()=>kernel.invocationUsage(z.domain),retire=(g=facts())=>kernel.retireInvocation(z.domain,issued.ticket,g);
 const submit=p=>kernel.submitInvocation(z.domain,issued.ticket,p.job,p.route,z.controlContext,z.controlPolicy,facts());
 const run=()=>kernel.runInvocation(z.domain,issued.ticket,z.controlContext,facts(),z.host);
 const outcome=()=>kernel.observeInvocation(z.domain,issued.ticket,z.controlContext,z.controlPolicy,facts());
 const read=()=>id.includes('-container-')?kernel.observeRelationResult(z.domain,issued.ticket,z.controlContext,z.controlPolicy,{...facts(),relationResultReadable:true,relationResultContextCurrent:true,historicalTargetReadable:true}):kernel.observeLifecycleResult(z.domain,issued.ticket,z.controlContext,z.controlPolicy,{...facts(),lifecycleResultReadable:true,lifecycleResultContextCurrent:true,historicalDocumentReadable:true});
 return {...z,...issued,id,route,request,ports,facts,envelope,prepare,nativePrepare,usage,retire,submit,run,outcome,read};
}
export function checkActivationEnvelope(){
 const controls=[],check=(name,f)=>{f();controls.push(name);};let nativeCases=0,denials=0,reentries=0,hostFaults=0,opaquePorts=0;
 check('fresh-nine-form-both-route-native-owner-publication-and-release',()=>{
  for(const id of activationEnvelopeIds)for(const route of ['typed','dynamic']){
   const x=activationEnvelopeFixture(id,route),before=x.root(),usage=x.usage(),p=x.prepare();assert(p,id);assert.equal(p.route,route);assert.equal(p.job.kind,id.includes('-container-')?'relation':'lifecycle');assert(Object.isFrozen(p)&&Object.isFrozen(p.job));assert.equal(x.nativePrepare(),p.job.plan);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);
   assert.equal(x.submit(p).status,'accepted');assert.equal(x.run().status,'settled');const publication=x.outcome().outcome.publication;assert(['applied','unchanged'].includes(publication));assert.equal(x.root().publication,before.publication+(publication==='applied'?1:0));assert.equal(x.read().status,'result');assert.equal(x.prepare(),null);nativeCases++;
  }
 });
 check('ordinary-refusals-preserve-native-root-slot-and-eligibility',()=>{
  for(const id of activationEnvelopeIds){
   for(const field of ['outsidePublicationGuard','preparationReserved','authorized','scope','continuous','notStopped','resourcesReady'])for(const value of [false,undefined,1]){
    const x=activationEnvelopeFixture(id),before=x.root(),usage=x.usage();assert.equal(x.prepare(x.envelope(),x.ports,x.controlContext,{...x.facts(),[field]:value}),null,id+'/'+field);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert(x.prepare());denials++;
   }
   for(const mutate of [e=>{e.extra=true;},e=>{e.request.arguments.extra=true;},e=>{e.request.arguments.invocation+='foreign';},e=>{e.request.variant='dispose';},e=>{e.route='network';}]){
    const x=activationEnvelopeFixture(id),e=x.envelope(),before=x.root(),usage=x.usage();mutate(e);assert.equal(x.prepare(e),null);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert(x.prepare());denials++;
   }
  }
 });
 check('closed-native-port-record-preserves-original-handle-roles',()=>{
  for(const id of activationEnvelopeIds){
   const seed=activationEnvelopeFixture(id);
   const cases=[p=>null,p=>[],p=>({...p,extra:{}}),...Object.keys(seed.ports).flatMap(key=>[p=>{const missing={...p};delete missing[key];return missing;},p=>({...p,[key]:{}})])];
   for(const change of cases){const x=activationEnvelopeFixture(id),before=x.root(),usage=x.usage();assert.equal(x.prepare(x.envelope(),change(x.ports)),null,id);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert(x.prepare());denials++;}
   const x=activationEnvelopeFixture(id),ports={...x.ports};let calls=0;Object.defineProperty(ports,'extra',{enumerable:true,get(){calls++;throw Error('port getter');}});assert.equal(x.prepare(x.envelope(),ports),null);assert.equal(calls,0);assert(x.prepare());denials++;
  }
 });
 check('foreign-owner-ticket-context-cannot-repair-original-native-roles',()=>{
  for(const id of activationEnvelopeIds)for(const role of ['domain','ticket','context']){
   const x=activationEnvelopeFixture(id),before=x.root(),usage=x.usage();assert.equal(x.prepare(x.envelope(),x.ports,role==='context'?{...x.controlContext,recipient:'foreign'}:x.controlContext,x.facts(),limits,role==='domain'?{}:x.domain,role==='ticket'?{}:x.ticket),null);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert(x.prepare());denials++;
  }
 });
 check('untrusted-data-refuses-without-evaluating-host-control-port',()=>{
  for(const id of activationEnvelopeIds){const x=activationEnvelopeFixture(id),e=x.envelope(),g=x.facts();let calls=0;Object.defineProperty(e.request.arguments,'profile',{enumerable:true,get(){calls++;throw Error('request getter');}});Object.defineProperty(g,'authorized',{enumerable:true,get(){calls++;throw Error('host getter');}});const before=x.root(),usage=x.usage();assert.equal(x.prepare(e,x.ports,x.controlContext,g),null);assert.equal(calls,0);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert(x.prepare());denials++;}
 });
 check('request-is-owned-but-native-host-bookkeeping-is-not-json-cloned',()=>{
  for(const id of activationEnvelopeIds){const x=activationEnvelopeFixture(id),e=x.envelope(),g=x.facts(),opaque=new WeakMap();g.testOnlyOpaque=opaque;const before=x.root(),p=x.prepare(e,x.ports,x.controlContext,g);assert(p);assert.equal(g.testOnlyOpaque,opaque);e.request.arguments.invocation='changed';assert.equal(x.root(),before);assert.equal(x.nativePrepare(),p.job.plan);assert.equal(x.submit(p).status,'accepted');assert.equal(x.run().status,'settled');assert.equal(x.read().status,'result');opaquePorts++;}
 });
 check('shared-preparation-guard-excludes-recursion-sibling-native-submit-and-retire',()=>{
  for(const id of activationEnvelopeIds){const x=activationEnvelopeFixture(id),g=x.facts(),before=x.root(),usage=x.usage();let calls=0;Object.defineProperty(g,'authorized',{enumerable:true,get(){calls++;assert.equal(x.prepare(),null);assert.equal(x.nativePrepare(),null);assert.equal(kernel.prepareInvocationRelationIntent(x.domain,x.ticket,{}, {},x.controlContext,x.facts()),null);assert.equal(x.retire().status,'unavailable');assert.equal(kernel.submitInvocation(x.domain,x.ticket,x.job,'typed',x.controlContext,x.controlPolicy,x.facts()).status,'unavailable');assert.equal(x.root(),before);reentries+=5;return true;}});assert(x.prepare(x.envelope(),x.ports,x.controlContext,g));assert(calls>0);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert.equal(x.retire().status,'retired');}
 });
 check('unexpected-native-control-fault-fences-domain-retaining-known-ticket-discharge',()=>{
  for(const id of activationEnvelopeIds){const x=activationEnvelopeFixture(id),g=x.facts(),before=x.root(),usage=x.usage();Object.defineProperty(g,'authorized',{enumerable:true,get(){throw Error('native control fault');}});assert.equal(x.prepare(x.envelope(),x.ports,x.controlContext,g),null);assert.equal(x.root(),before);assert.deepEqual(x.usage(),{...usage,faulted:true});assert.equal(x.prepare(),null);assert.equal(x.retire({...x.facts(),retentionReleased:false}).status,'unavailable');assert.equal(x.retire().status,'retired');assert.equal(x.usage().issued,usage.issued);assert.equal(x.usage().slots,usage.slots-1);assert.equal(x.prepare(),null);hostFaults++;}
 });
 return {scope:'selected-F17-activation-envelope-owner-composition',controls,forms:9,nativeCases,denials,reentries,hostFaults,opaquePorts,existingNativeJobsAndOwnerReused:true,authorityPortsRemainOutsideData:true,temporaryParentOrPhysicalAccountingProved:false,fullF17OrImplementationOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkActivationEnvelope()));
