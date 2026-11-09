import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {lifecycleInvocationFixture} from './lifecycle-invocation-checks.mjs';
const limits={maxRequestBytes:16384,maxInputNodes:4096,maxInputDepth:128},kinds=['create-scratch','attach-view','detach-view','membership','dispose'];
const variant=k=>['attach-view','detach-view'].includes(k)?'attach-detach-view':k;
export function lifecycleEnvelopeFixture(kind='create-scratch',route='typed'){
 const z=lifecycleInvocationFixture(undefined,kind),issued=kernel.issueInvocation(z.domain,z.controlContext,z.g());assert(issued);
 const request={...structuredClone(z.request),invocation:issued.reference},envelope=()=>({route,request:{...(route==='typed'?{family:16}:{name:'change-document-lifecycle'}),variant:variant(kind),version:1,arguments:structuredClone(request)}}),facts=()=>({...z.g(),outsidePublicationGuard:true});
 const prepare=(e=envelope(),l=limits,c=z.controlContext,g=facts(),inspection=z.inspection,profile=z.profile,domain=z.domain,ticket=issued.ticket)=>kernel.prepareInvocationLifecycleEnvelope(domain,ticket,e,l,profile,inspection,c,g);
 return {...z,...issued,request,envelope,facts,prepare,nativePrepare:r=>kernel.prepareInvocationLifecycleIntent(z.domain,issued.ticket,r,z.profile,z.inspection,z.controlContext,facts()),usage:()=>kernel.invocationUsage(z.domain),retire:g=>kernel.retireInvocation(z.domain,issued.ticket,g??facts())};
}
export function checkLifecycleEnvelope(){let nativeCases=0,denials=0,reentries=0,hostFaults=0;const controls=[],check=(name,f)=>{f();controls.push(name);};
 check('fresh-both-route-preparation-delegates-to-actual-owner-and-publishes-once',()=>{
  for(const kind of kinds)for(const route of ['typed','dynamic']){const x=lifecycleEnvelopeFixture(kind,route),before=x.root(),usage=x.usage(),plan=x.prepare();assert(plan,kind);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert.equal(x.nativePrepare(x.request),plan);const facts=x.facts();assert.equal(kernel.submitInvocation(x.domain,x.ticket,{kind:'lifecycle',plan},route,x.controlContext,x.controlPolicy,facts).status,'accepted');assert.equal(kernel.runInvocation(x.domain,x.ticket,x.controlContext,facts,x.host).status,'settled');assert.equal(x.root().publication,before.publication+1);assert.equal(kernel.observeLifecycleResult(x.domain,x.ticket,x.controlContext,x.controlPolicy,{...facts,lifecycleResultReadable:true,lifecycleResultContextCurrent:true,historicalDocumentReadable:true}).status,'result');assert.equal(x.prepare(),null);nativeCases++;}
 });
 check('ordinary-refusals-preserve-root-slot-eligibility-and-existing-ledgers',()=>{
  for(const field of ['outsidePublicationGuard','preparationReserved','authorized','scope','continuous','notStopped','resourcesReady','lifecycleAllowed','creationAllowed'])for(const value of [false,undefined,1]){const x=lifecycleEnvelopeFixture(),before=x.root(),usage=x.usage();assert.equal(x.prepare(x.envelope(),limits,x.controlContext,{...x.facts(),[field]:value}),null,field);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert(x.prepare());denials++;}
  for(const mutate of [e=>{e.extra=true;},e=>{e.request.arguments.extra=true;},e=>{e.request.arguments.invocation+='foreign';},e=>{e.request.arguments.request.extra=true;},e=>{e.request.variant='membership';},e=>{e.request.arguments.request.source=[999];}]){const x=lifecycleEnvelopeFixture(),e=x.envelope(),before=x.root(),usage=x.usage();mutate(e);assert.equal(x.prepare(e),null);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert(x.prepare());denials++;}
 });
 check('foreign-identity-context-and-inspection-cannot-repair-a-request',()=>{
  for(const role of ['domain','ticket','context','inspection']){const x=lifecycleEnvelopeFixture(),before=x.root(),usage=x.usage();assert.equal(x.prepare(x.envelope(),limits,role==='context'?{...x.controlContext,recipient:'foreign'}:x.controlContext,x.facts(),role==='inspection'?{}:x.inspection,x.profile,role==='domain'?{}:x.domain,role==='ticket'?{}:x.ticket),null);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert(x.prepare());denials++;}
 });
 check('untrusted-request-accessors-refuse-before-native-effects',()=>{
  for(const target of ['envelope','arguments','context','limits']){const x=lifecycleEnvelopeFixture(),e=x.envelope(),c={...x.controlContext},l={...limits},v={envelope:e,arguments:e.request.arguments,context:c,limits:l}[target],key={envelope:'route',arguments:'profile',context:'purpose',limits:'maxInputDepth'}[target];let calls=0;Object.defineProperty(v,key,{enumerable:true,get(){calls++;throw Error('unexpected request getter');}});const before=x.root(),usage=x.usage();assert.equal(x.prepare(e,l,c),null);assert.equal(calls,0);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert(x.prepare());denials++;}
 });
 check('caller-data-is-owned-and-preparation-remains-separate-from-publication',()=>{
  const x=lifecycleEnvelopeFixture(),e=x.envelope(),before=x.root(),plan=x.prepare(e);assert(plan);e.request.arguments.request.source=[999];assert.equal(x.root(),before);assert.equal(x.prepare(e),null);assert.equal(x.nativePrepare(x.request),plan);assert.equal(kernel.submitInvocation(x.domain,x.ticket,{kind:'lifecycle',plan},'typed',x.controlContext,x.controlPolicy,x.facts()).status,'accepted');assert.equal(kernel.runInvocation(x.domain,x.ticket,x.controlContext,x.facts(),x.host).status,'settled');assert.equal(x.root().publication,before.publication+1);
 });
 check('trusted-native-port-can-carry-unserialized-opaque-bookkeeping',()=>{
  const x=lifecycleEnvelopeFixture(),g=x.facts(),opaque=new WeakMap(),before=x.root();g.testOnlyOpaque=opaque;assert(x.prepare(x.envelope(),limits,x.controlContext,g));assert.equal(g.testOnlyOpaque,opaque);assert.equal(x.root(),before);
 });
 check('same-entry-reentry-and-retirement-use-the-existing-preparation-guard',()=>{
  const x=lifecycleEnvelopeFixture(),g=x.facts(),before=x.root(),usage=x.usage();let calls=0;
  Object.defineProperty(g,'authorized',{enumerable:true,get(){calls++;assert.equal(x.prepare(),null);assert.deepEqual(x.retire(),{status:'unavailable'});assert.equal(x.nativePrepare(x.request),null);assert.equal(kernel.prepareInvocationRelationIntent(x.domain,x.ticket,{}, {},x.controlContext,x.facts()),null);assert.deepEqual(kernel.submitInvocation(x.domain,x.ticket,x.job,'typed',x.controlContext,x.controlPolicy,x.facts()),{status:'unavailable'});assert.equal(x.root(),before);reentries+=5;return true;}});
  assert(x.prepare(x.envelope(),limits,x.controlContext,g));assert(calls>0);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert.equal(x.retire().status,'retired');
 });
 check('unexpected-native-host-fault-fences-domain-but-known-slot-still-retires',()=>{
  const x=lifecycleEnvelopeFixture(),g=x.facts(),before=x.root(),usage=x.usage();Object.defineProperty(g,'authorized',{enumerable:true,get(){throw Error('fixed native host-control fault');}});assert.equal(x.prepare(x.envelope(),limits,x.controlContext,g),null);assert.equal(x.root(),before);assert.deepEqual(x.usage(),{...usage,faulted:true});assert.equal(x.prepare(),null);assert.equal(x.retire({...x.facts(),retentionReleased:false}).status,'unavailable');assert.equal(x.retire().status,'retired');assert.equal(x.usage().issued,usage.issued);assert.equal(x.usage().slots,usage.slots-1);assert.equal(x.prepare(),null);hostFaults++;
 });
 return {scope:'selected-F17-lifecycle-envelope-owner-composition',controls,nativeCases,denials,reentries,hostFaults,existingOwnerAndPreparationGuardReused:true,hostFactsRemainNativePorts:true,temporaryParentOrPhysicalAccountingProved:false,fullF17OrImplementationOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkLifecycleEnvelope()));
