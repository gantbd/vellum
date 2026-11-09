// Actual-domain ordinary-admission fence only; not full F18 acceptance.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {nativeSourceReadApi as api} from './native-source-read-checks.mjs';
import {nativeSourceSaveFixture} from './native-source-save-checks.mjs';
import {discardSourceDestinationTransition} from './source-destination-state.mjs';
import {annotationArtifactContext as artifactContext,annotationArtifactProfile as artifactProfile} from './annotation-artifact-checks.mjs';
import {annotationObservationContext as context,annotationObservationPolicy as policy,annotationInvocationLimits as limits} from './annotation-observation-checks.mjs';
const copy=structuredClone,controlKeys=['ownerControlAllowed','controlContextCurrent','controlScopeValid','protectedControlReady'];
function fixture(){
 const z=nativeSourceSaveFixture(),a=z.z.a;
 assert(['attached','current'].includes(api.attachAnnotationReadProvenance(z.writer,a.x.owner,a.g()).status));
 assert.equal(a.adopt(a.g(),a.t,z.writer).status,'current');
 const facts=(more={})=>({...z.g(),...Object.fromEntries(controlKeys.map(k=>[k,true])),...more});
 return {z,a,facts,seal:(before=z.root(),c=context,g=facts())=>api.sealInvocationOrdinaryAdmission(z.domain,before,c,g),usage:()=>api.invocationUsage(z.domain)};
}
function admittedEdit(x,route='typed'){
 const {z,a}=x,d=z.d(),issued=api.issueInvocation(z.domain,context,a.g());assert(issued);
 const plan={contract:'owned-annotation-plan/1',invocation:issued.reference,descriptor:issued.reference,sources:[{document:copy(z.document),expectedSource:d.revision,expectedAnnotationSet:d.annotationSet,edits:[{lo:0,hi:d.source.length,expected:copy(d.source),ins:[88]}]}],annotations:[]},job={kind:'compound',plan};
 assert.equal(api.submitInvocation(z.domain,issued.ticket,job,route,context,policy,a.g()).status,'accepted');
 return {...issued,job,run:(host={})=>api.runInvocation(z.domain,issued.ticket,context,a.g(),Object.defineProperties({...a.x.z.h,sourceWritable:true,parserSourceReserved:true,parserSourceLimits:{maxDocuments:8,maxRootBytes:8192,maxBatches:32,maxEdits:64,maxJournalBytes:65536}},Object.getOwnPropertyDescriptors(host)))};
}
export function checkAdmissionFence(){
 const controls=[],check=(name,f)=>{try{f();}catch(e){e.message=name+': '+e.message;throw e;}controls.push(name);};let strictCases=0,scopeCases=0,boundaries=0,reentries=0;
 check('actual-four-native-bypasses-close-without-faulting-or-changing-the-root',()=>{
  const x=fixture(),{z,a}=x,before=z.root(),writer=api.sourceSaveUsage(z.writer),codec=api.sourceSaveCodecUsage(z.writer),fence=x.seal();assert(fence);
  assert.equal(api.invocationOrdinaryAdmissionOpen(z.domain),false);assert.equal(x.usage().faulted,false);
  assert.equal(api.issueInvocation(z.domain,context,a.g()),null);
  assert.equal(api.prepareNativeSourceDestination(z.writer,z.domain,z.profile,'ordinary-admit',z.request('denied-preparation'),z.g()),null);
  assert.equal(z.run('ordinary-admit',z.request('denied-source')).status,'unavailable');
  assert.equal(api.admitOrdinaryAnnotationSave(z.writer,z.domain,a.pool,a.request('denied-annotation'),artifactContext,a.binding(artifactContext),artifactProfile,a.g()).status,'unavailable');
  assert.equal(z.root(),before);assert.deepEqual(api.sourceSaveUsage(z.writer),writer);assert.deepEqual(api.sourceSaveCodecUsage(z.writer),codec);
  const y=fixture(),e=admittedEdit(y),root=y.z.root();assert(y.seal());assert.equal(e.run().status,'unavailable');assert.equal(y.z.root(),root);assert.equal(y.usage().starts,0);
 });
 check('strict-original-context-root-and-control-bindings-precede-fence-installation',()=>{
  for(const key of controlKeys)for(const value of [false,1,undefined]){const x=fixture(),before=x.usage();assert.equal(x.seal(x.z.root(),context,x.facts({[key]:value})),null);assert.deepEqual(x.usage(),before);assert(api.invocationOrdinaryAdmissionOpen(x.z.domain));strictCases++;}
  for(const key of Object.keys(context)){const x=fixture(),c={...context,[key]:'foreign'},before=x.usage();assert.equal(x.seal(x.z.root(),c),null);assert.deepEqual(x.usage(),before);scopeCases++;}
  const x=fixture(),other=fixture(),root=x.z.root();for(const before of [copy(root),other.z.root(),null]){assert.equal(x.seal(before),null);assert(api.invocationOrdinaryAdmissionOpen(x.z.domain));scopeCases++;}
  assert.equal(api.sealInvocationOrdinaryAdmission(copy(x.z.domain),root,context,x.facts()),null);scopeCases++;
 });
 check('exact-retry-retains-one-fence-and-one-nonrefundable-root-frame',()=>{
  const x=fixture(),before=x.z.root(),usage=x.usage(),token=x.seal();assert(token);const after=x.usage(),f=api.invocationAdmissionFenceUsage(x.z.domain,context,x.facts());
  assert.equal(after.declaredHeldBytes-usage.declaredHeldBytes,limits.maxRootBytes+Buffer.byteLength(JSON.stringify(token)));assert.equal(f.declaredHeldBytes,after.declaredHeldBytes-usage.declaredHeldBytes);
  assert.equal(x.seal(before),token);assert.deepEqual(x.usage(),after);assert.equal(x.seal(copy(before)),null);assert.deepEqual(x.usage(),after);
  assert.equal(api.invocationAdmissionFenceUsage(x.z.domain,{...context,recipient:'foreign'},x.facts()),null);assert.equal(f.completeOwnerInventoryProved,false);assert.equal(f.shutdownAccepted,false);
 });
 check('actual-parent-headroom-boundaries-refuse-without-installing-a-fence',()=>{
  const charge=limits.maxRootBytes+Buffer.byteLength(JSON.stringify({kind:'ordinary-admission-fence/1'}));
  for(const delta of [-1,0,1]){const x=fixture(),before=x.usage(),n=limits.maxHeldBytes-before.declaredHeldBytes-charge-delta;assert(n>=0);const hold=api.reserveInvocationRegistrationReadCharge(x.z.domain,n,x.facts());assert(hold);const filled=x.usage();assert.equal(Boolean(x.seal()),delta>=0);assert.equal(api.invocationOrdinaryAdmissionOpen(x.z.domain),delta<0);assert.equal(x.usage().declaredHeldBytes,filled.declaredHeldBytes+(delta>=0?charge:0));assert.equal(api.releaseInvocationRegistrationReadCharge(x.z.domain,hold,x.facts({retentionReleased:true,deliveryDrained:true})),true);boundaries++;}
 });
 check('matching-invocation-retry-stop-settlement-and-retirement-remain-separate',()=>{
  for(const route of ['typed','dynamic']){const x=fixture(),e=admittedEdit(x,route),root=x.z.root();assert(x.seal());
   assert.equal(api.submitInvocation(x.z.domain,e.ticket,e.job,route,context,policy,x.a.g()).status,'pending');assert.equal(e.run().status,'unavailable');
   assert.equal(api.stopInvocation(x.z.domain,e.ticket,x.facts()).status,'stop-requested');assert.equal(e.run().status,'settled');assert.equal(x.z.root(),root);assert.equal(api.observeInvocation(x.z.domain,e.ticket,context,policy,x.a.g()).outcome.publication,'notApplied');
   assert.equal(api.submitInvocation(x.z.domain,e.ticket,e.job,route,context,policy,x.a.g()).status,'observe-existing');
   const charge=api.invocationAdmissionFenceUsage(x.z.domain,context,x.facts()).declaredHeldBytes;
   assert.equal(api.retireInvocation(x.z.domain,e.ticket,x.facts({retentionReleased:true,deliveryDrained:true})).status,'retired');assert.equal(api.invocationOrdinaryAdmissionOpen(x.z.domain),false);assert.equal(api.invocationAdmissionFenceUsage(x.z.domain,context,x.facts()).declaredHeldBytes,charge);
  }
 });
 check('an-issued-but-unadmitted-ticket-cannot-cross-the-fence',()=>{
  const x=fixture(),d=x.z.d(),e=api.issueInvocation(x.z.domain,context,x.a.g());assert(e);assert(x.seal());
  const job={kind:'compound',plan:{contract:'owned-annotation-plan/1',invocation:e.reference,descriptor:e.reference,sources:[{document:copy(x.z.document),expectedSource:d.revision,expectedAnnotationSet:d.annotationSet,edits:[]}],annotations:[]}};
  assert.equal(api.submitInvocation(x.z.domain,e.ticket,job,'typed',context,policy,x.a.g()).status,'unavailable');assert.equal(x.usage().starts,0);
 });
 check('an-existing-source-admission-preparation-cannot-install-a-task-after-fencing',()=>{
  const x=fixture(),{z}=x,token=api.prepareNativeSourceDestination(z.writer,z.domain,z.profile,'ordinary-admit',z.request('prepared-before'),z.g());assert(token);const writer=api.sourceSaveUsage(z.writer),root=z.root();assert(x.seal());
  assert.equal(api.installInvocationSourceDestination(z.domain,token,z.g()).status,'unavailable');assert.deepEqual(api.sourceSaveUsage(z.writer),writer);assert.equal(z.root(),root);assert.equal(discardSourceDestinationTransition(token),true);
 });
 check('eligible-pre-admitted-save-drain-and-independent-baselines-still-work',()=>{
  for(const order of [['source','annotation'],['annotation','source']]){const x=fixture(),{z,a}=x;z.edit();const n=a.x.z,p=n.prepare({operation:'update-body'},{body:'fence-annotation'});assert.equal(n.execute(p).outcome.publication,'applied');n.remember();
   const source=z.admit(z.request('original-source')),request=a.request('original-annotation'),r=api.admitOrdinaryAnnotationSave(z.writer,z.domain,a.pool,request,artifactContext,a.binding(artifactContext),artifactProfile,a.g());assert.equal(r.status,'admitted');const annotation=r.task;assert(x.seal());const root=z.root(),issued=api.sourceSaveUsage(z.writer).issued;
   for(const kind of order){const task=kind==='source'?source:annotation;if(kind==='source')assert.equal(z.dispatch(task).status,'dispatched');else{const b=z.root().documents.find(d=>d.document.id===request.document.id).ordinaryBindings.annotations,out=api.dispatchSourceSave(z.writer,task,z.root(),{provider:b.provider,resource:b.resource,registrationGeneration:b.registrationGeneration,identityDomain:b.identityDomain,identityKey:b.identityKey,resourceVersion:request.condition.resourceVersion,presence:'present'},a.g());assert.equal(out.status,'dispatched');z.submitted.set(task,copy(out));}
    assert.equal(z.written(task,{epoch:kind==='source'?'source-read-epoch':'read-epoch',token:'fenced-'+kind}).status,'recorded');assert(['published','installed'].includes(kind==='source'?z.baseline().status:api.installAnnotationWriteBaseline(z.writer,task,z.domain,a.pool,a.g()).status));assert.equal(z.observe(task,'quiescence').status,'recorded');}
   for(const before of root.documents){const now=z.root().documents.find(d=>d.document.id===before.document.id);assert.deepEqual(now.source,before.source);assert.deepEqual(now.records,before.records);assert.equal(now.revision,before.revision);assert.equal(now.annotationSet,before.annotationSet);}
   assert.equal(api.sourceSaveUsage(z.writer).issued,issued);assert.equal(api.invocationOrdinaryAdmissionOpen(z.domain),false);assert.equal(x.usage().faulted,false);
  }
 });
 check('fence-installation-is-excluded-from-active-native-admission-and-running-publication',()=>{
  for(const kind of ['issue','source','annotation']){const x=fixture(),{z,a}=x,g=kind==='annotation'?a.g():z.g();let seen=0;Object.defineProperty(g,'authorized',{get(){seen++;assert.equal(x.seal(),null,kind+'/active-admission');return true;}});
   const result=kind==='issue'?api.issueInvocation(z.domain,context,g):kind==='source'?z.run('ordinary-admit',z.request('section-source'),g):api.admitOrdinaryAnnotationSave(z.writer,z.domain,a.pool,a.request('section-annotation'),artifactContext,a.binding(artifactContext),artifactProfile,g);assert(result);if(kind!=='issue')assert.equal(result.status,'admitted');assert(seen>0);assert(x.seal());reentries++;
  }
  const x=fixture(),e=admittedEdit(x);let seen=0;assert.equal(e.run({get authorized(){seen++;assert.equal(x.seal(),null,'active-publication');return true;}}).status,'settled');assert(seen>0);assert(x.seal());reentries++;
 });
 check('fencing-one-actual-domain-does-not-close-a-peer',()=>{
  const x=fixture(),y=fixture();assert(x.seal());assert(api.invocationOrdinaryAdmissionOpen(y.z.domain));assert(api.issueInvocation(y.z.domain,context,y.a.g()));assert.equal(y.z.run('ordinary-admit',y.z.request('peer-source')).status,'admitted');assert.equal(y.usage().faulted,false);
 });
 check('mutation-sections-unwind-on-fixed-admission-errors-and-cannot-be-released-by-a-copy',()=>{
  for(const kind of ['issue','source','annotation']){const x=fixture(),{z,a}=x,g=kind==='annotation'?a.g():z.g();Object.defineProperty(g,'authorized',{get(){throw Error('fixed-admission-fault');}});
   assert.throws(()=>kind==='issue'?api.issueInvocation(z.domain,context,g):kind==='source'?z.run('ordinary-admit',z.request('throw-source'),g):api.admitOrdinaryAnnotationSave(z.writer,z.domain,a.pool,a.request('throw-annotation'),artifactContext,a.binding(artifactContext),artifactProfile,g),/fixed-admission-fault/);assert(x.seal());}
  const x=fixture(),section=api.beginInvocationAdmissionMutation(x.z.domain);assert(section);assert.equal(api.endInvocationAdmissionMutation(x.z.domain,copy(section)),false);assert.equal(x.seal(),null);assert.equal(api.endInvocationAdmissionMutation(x.z.domain,section),true);assert(x.seal());
 });
 check('fault-containment-is-not-fence-acceptance-and-late-evidence-remains-owned',()=>{
  const x=fixture(),task=x.z.admit(x.z.request('pre-fault'));assert.equal(x.z.dispatch(task).status,'dispatched');const e=admittedEdit(x),root=x.z.root();
  assert.equal(e.run({get authorized(){throw Error('fixed-publication-fault');}}).status,'faulted');assert.equal(x.usage().faulted,true);assert.equal(x.z.root(),root);assert.equal(x.seal(),null);
  assert.equal(x.z.written(task,{epoch:'source-read-epoch',token:'late-fault-evidence'}).status,'recorded');assert.equal(x.z.baseline().status,'unavailable');
 });
 return {scope:'actual-domain-selected-ordinary-admission-fence-primitive',controls,strictCases,scopeCases,boundaries,reentries,
  actualDomainFenceAndFourBypassesChecked:true,actualParentFenceFrameCharged:true,matchingRetryAndProtectedStopRetirementPreserved:true,preAdmittedSourceAnnotationSaveDrainChecked:true,
  completeOwnerInventoryOrAllNativeEntryPathsProved:false,shutdownAcceptanceOrEligibleDispatchInventoryProved:false,publicF18CodecHostClockOrPhysicalTruthProved:false,implementationRefinementOrIndependentCertification:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkAdmissionFence()));}catch(e){console.error(e.code+': '+String(e.message).slice(0,1000));process.exitCode=1;}
