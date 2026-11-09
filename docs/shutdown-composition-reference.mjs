// Working proof reference, outside the qualified suite and production engine.
// Composes actual normalized source-read/write and F17 lifecycle relations,
// with a separately checked native annotation save/readiness/disposal leg.
// Joint source/annotation authority fencing and complete host inventories are NOT proved.
import assert from 'node:assert/strict';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {sourceSaveFixture,sourceSaveGates,sourceSaveRetirement} from '../formal/source-save-checks.mjs';
import {inspectSourceOutput,publishSourceSaveBaseline,sourceSaveUsage,retireSourceSave,controlSourceSave} from '../formal/source-save.mjs';
import {observeResourceRead,resourceReadUsage,resourceReadStatus,retireResourceRead} from '../formal/resource-read.mjs';
import {applyDocumentLifecycle} from '../formal/lifecycle-products.mjs';
import {nativeRetainedLifecycleFixture} from '../formal/native-retained-lifecycle-checks.mjs';
import {installAnnotationWriteBaseline,inspectSourceSave} from '../formal/source-save.mjs';
import {annotationArtifactContext as annotationSaveContext} from '../formal/annotation-artifact-checks.mjs';

const copy=structuredClone,key=d=>JSON.stringify(d.document),natural=n=>Number.isSafeInteger(n)&&n>=0;
const limits={maxDocuments:16,maxRelations:64,maxRootBytes:1048576,maxRequestBytes:65536,maxSourceBytes:8192,maxTagBytes:512};
const flags=['lifecycleAllowed','coverageComplete','disposeAllowed','sourceRetentionReady','annotationRetentionReady','sourceDiscardAllowed','annotationDiscardAllowed','retentionComplete','effectsRetained','historyRetained','snapshotsRetained','providersRetained','viewsDetachable','membershipsDetachable','observerScopesFenced','bindingReleaseAllowed','retentionDischargeAllowed','historyEvidenceReleased','snapshotEvidenceReleased','readEvidenceReleased','writeEvidenceReleased','providerEvidenceReleased','dispositionReadable','baselinePublicationAllowed','comparisonAllowed'];
const facts={...sourceSaveGates,...Object.fromEntries(flags.map(k=>[k,true]))};
const entries=['source-edit','ordinary-save','create','attach','membership','activation','read'];
const basis=d=>({document:copy(d.document),lifecycle:d.lifecycle?.generation??0,source:d.revision,annotations:d.annotationSet,serialization:d.serialization.generation,binding:d.binding?.generation??0});
const rootFrame=s=>s.documents.map(d=>({document:d.document,source:d.source,revision:d.revision,records:d.records,annotationSet:d.annotationSet}));

export function shutdownCompositionFixture(weakening=null){
 const x=sourceSaveFixture(),identity=Object.freeze({kind:'shutdown-reference-control/1'}),tasks=[],disposed=new Set(),savedOutputs=new Map(),retiredWrites=new Set(),retiredReads=new Set();
 let phase='decision',intent=null,started=null,budget=null,remaining=null,last=null,epoch=null,cut=null;
 const fenced=()=>phase!=='decision'&&phase!=='cancelled';
 const sourceReady=i=>{const r=inspectSourceOutput(x.owner,x.state,x.state.documents[i].document,facts);return r.status==='source-output'&&r.comparison==='equal';};
 const sourceAdmitted=i=>tasks.some(t=>{const e=x.inspect(t.task);return t.which===i&&equal(t.basis,basis(x.state.documents[i]))&&e.status==='save-evidence'&&!e.stopped&&!e.contested&&!e.destinationFaulted&&(e.phase==='queued'||e.phase==='dispatched'&&e.knowledge.application!=='not-written'&&e.knowledge.synchronization!=='failed');});
 // A historical equal baseline cannot promise the final output while another
 // captured attempt can still overwrite it. This selected profile refuses a
 // conflicting attempt rather than inventing a compensating save after fencing.
 const incompatibleWrite=i=>tasks.some(t=>{if(t.which!==i||retiredWrites.has(t.task))return false;const e=x.inspect(t.task);if(e.status!=='save-evidence'||e.contested||e.destinationFaulted)return true;if(equal(t.basis,intent?.versions[i]??basis(x.state.documents[i])))return false;return !(e.phase==='queued'&&e.stopped||e.phase==='dispatched'&&e.knowledge.application==='not-written'&&e.safeDisposition&&e.quiescent);});
 const validIntent=choices=>Array.isArray(choices)&&choices.length===x.state.documents.length&&choices.length<=limits.maxDocuments&&choices.every(c=>c&&equal(Object.keys(c).sort(),['annotation','source'])&&['save','discard'].includes(c.source)&&['save','discard'].includes(c.annotation));
 function decide(choices){if(phase!=='decision'||!validIntent(choices))return 'refused';intent={authority:x.state.authority,versions:x.state.documents.map(basis),choices:copy(choices)};return 'decision-captured';}
 function cancel(){if(phase!=='decision')return 'refused';phase='cancelled';return 'cancelled';}
 function entryAllowed(kind){return entries.includes(kind)&&(!fenced()||weakening==='fence-bypassed');}
 function admit(i=0){if(!entryAllowed('ordinary-save')||tasks.length>=8)return {status:'refused'};const captured=basis(x.state.documents[i]),task=x.admit('shutdown-save-'+tasks.length,{},i);tasks.push({task,which:i,basis:captured});return {status:'admitted',task};}
 function accept(now,wait=3,clockEpoch='test-epoch'){
  if(phase!=='decision'){
   if(weakening==='retry-renews-wait'&&fenced()){started=now;remaining=budget;last=now;}
   return 'existing-control';
  }
  if(!intent||!equal(intent.versions,x.state.documents.map(basis))||!natural(now)||!natural(wait)||typeof clockEpoch!=='string'||!clockEpoch)return 'refused';
  for(let i=0;i<intent.choices.length;i++){
   if(intent.choices[i].source==='save'&&incompatibleWrite(i))return 'conflicting-save';
   if(intent.choices[i].source==='save'&&!sourceReady(i)&&!sourceAdmitted(i)&&weakening!=='missing-save-admission')return 'needs-save';
   // No fabricated annotation baseline or supplied-true readiness in this profile.
   if(intent.choices[i].annotation==='save')return 'unsupported-annotation-save';
  }
  phase='draining';started=now;last=now;epoch=clockEpoch;budget=wait;remaining=wait;return 'accepted';
 }
 function dispatch(task){if(!tasks.some(t=>t.task===task)||retiredWrites.has(task))return 'refused';const out=x.dispatch(task);if(out.status==='dispatched')x.submitted.set(task,copy(out));return out.status;}
 function cancelQueued(task){if(!tasks.some(t=>t.task===task)||x.inspect(task).phase!=='queued')return 'refused';return controlSourceSave(x.owner,task,'cancel',{controlAllowed:true}).status;}
 function observe(task,kind,data={}){if(!tasks.some(t=>t.task===task)||!x.submitted.has(task)||retiredWrites.has(task)||!['written','result','disposition','quiescence','causal'].includes(kind))return 'refused';return kind==='written'?x.result(task,data).status:x.observe(task,kind,data).status;}
 function publishBaseline(i=0){if(!fenced()||disposed.has(i))return 'refused';const before=rootFrame(x.state),r=publishSourceSaveBaseline(x.owner,x.state,x.state.documents[i].document,facts);if(['published','unchanged'].includes(r.status))x.state=r.state;assert.deepEqual(rootFrame(x.state),before);return r.status;}
 function dispose(i,g={}){
  if(!fenced()||!intent||!natural(i)||i>=intent.choices.length)return 'refused';if(disposed.has(i))return 'already-disposed';
  const d=x.state.documents[i],choice=intent.choices[i],h={...facts,...g};
  if(!equal(intent.versions[i],basis(d)))return 'stale-consent';
  if(choice.source==='save'&&(!sourceReady(i)||incompatibleWrite(i)))return 'waiting-for-save';
  if(choice.annotation!=='discard')return 'unsupported-annotation-save';
  if(weakening==='annotation-discard-bypass')h.annotationDiscardAllowed=true;
  const r=applyDocumentLifecycle(x.state,{contract:'document-lifecycle/1',operation:'dispose',document:copy(d.document),expectedLifecycle:d.lifecycle?.generation??0,expectedSource:d.revision,expectedAnnotations:d.annotationSet,expectedSerialization:d.serialization.generation,expectedBinding:d.binding?.generation??0,sourceDisposition:choice.source==='discard'?'discard':'retain',annotationDisposition:'discard'},h,limits,x.owner);
  if(r.status==='applied'){if(choice.source==='save')savedOutputs.set(i,copy({source:d.source,revision:d.revision,serialization:d.serialization,baseline:d.sourceBaseline}));x.state=r.state;disposed.add(i);}return r.status;
 }
 function report(){return {phase,started,budget,remaining,disposed:disposed.size,writer:copy(sourceSaveUsage(x.owner)),reader:copy(resourceReadUsage(x.readOwner))};}
 function poll(now,clockEpoch=epoch,valid=true){
  if(phase!=='draining')return report();
  if(valid!==true||clockEpoch!==epoch||!natural(now)||now<last){phase='cutoff';remaining=null;cut=copy({...report(),reason:'clock-unavailable'});return report();}
  last=now;remaining=Math.max(0,budget-(now-started));
  if(remaining===0){phase='cutoff';cut=copy({...report(),reason:'budget-ended'});}
  return report();
 }
 // V1 supplies no successor-owner transfer here. Explicit host authority can
 // end waiting, but the same actual owners must keep serving their obligations.
 // This private reference port is not a selected public F18 schema.
 function forceClose(control,g){
  if(control!==identity||g?.controlAllowed!==true||!fenced())return 'unavailable';
  if(phase==='complete')return 'complete';
  if(weakening==='force-claims-complete')phase='complete';
  else if(phase==='draining'){phase='cutoff';if(!cut)cut=copy({...report(),reason:'authorized-wait-ended'});}
  return phase==='complete'?'complete':'retained';
 }
 function releaseBinding(i){const d=x.state.documents[i];if(!disposed.has(i))return 'refused';const r=applyDocumentLifecycle(x.state,{contract:'document-lifecycle/1',operation:'release-binding',document:copy(d.document),expectedLifecycle:d.lifecycle.generation,expectedBinding:d.binding?.generation??0},facts,limits,x.owner);if(['applied','unchanged'].includes(r.status))x.state=r.state;return r.status;}
 function quiesceReads(){for(let i=0;i<x.readTasks.length;i++){if(retiredReads.has(x.readTasks[i]))continue;const row=x.state.resourceRegistrations[i],r=observeResourceRead(x.readOwner,{contract:'resource-read-evidence/1',task:x.readTasks[i],origin:{operation:'load-'+i,effect:'read',attempt:'1'},provider:row.provider,resource:row.resource,kind:'quiescence'},{authentic:true,evidenceAllowed:true,ingressReady:true,readAttested:true,quiescenceAttested:true});assert(['quiescent','duplicate'].includes(r.status));}}
 function finish(){
  if(phase==='complete')return 'complete';
  if(!fenced()||disposed.size!==intent.choices.length||intent.choices.some((c,i)=>{if(c.source!=='save')return false;const d=x.state.documents[i];return !equal(savedOutputs.get(i),{source:d.source,revision:d.revision,serialization:d.serialization,baseline:d.sourceBaseline})||incompatibleWrite(i);})||tasks.some(t=>{if(retiredWrites.has(t.task))return false;const e=x.inspect(t.task);if(e.status!=='save-evidence')return true;if(e.phase==='queued')return !e.stopped;return e.phase!=='dispatched'||!e.quiescent||!e.safeDisposition||e.contested||e.knowledge.application==='unknown';})||x.readTasks.some(t=>!retiredReads.has(t)&&!resourceReadStatus(x.readOwner,t)?.quiescent))return 'retained';
  for(let i=0;i<x.state.documents.length;i++){
   if(x.state.documents[i].binding!==null&&releaseBinding(i)!=='applied')return 'retained';
   const d=x.state.documents[i];if(d.lifecycle.phase==='provenance-released')continue;
   const r=applyDocumentLifecycle(x.state,{contract:'document-lifecycle/1',operation:'release-provenance',document:copy(d.document),expectedLifecycle:d.lifecycle.generation},facts,limits,x.owner);if(r.status!=='applied')return 'retained';x.state=r.state;
  }
  for(const t of tasks)if(!retiredWrites.has(t.task)){assert.equal(retireSourceSave(x.owner,t.task,x.state,sourceSaveRetirement).status,'retired');retiredWrites.add(t.task);}
  for(const t of x.readTasks)if(!retiredReads.has(t)){assert.equal(retireResourceRead(x.readOwner,t,x.state,sourceSaveRetirement).status,'retired');retiredReads.add(t);}
  assert.equal(sourceSaveUsage(x.owner).tasks,0);assert.equal(resourceReadUsage(x.readOwner).tasks,0);phase='complete';return 'complete';
 }
 return {x,identity,tasks,decide,cancel,entryAllowed,admit,accept,dispatch,cancelQueued,observe,publishBaseline,dispose,report,poll,forceClose,releaseBinding,quiesceReads,finish,sourceReady,cut:()=>copy(cut)};
}

const choices=(source='discard',annotation='discard')=>Array.from({length:2},()=>({source,annotation}));
const dirty=z=>{z.x.state.documents[0].source.push(88);z.x.state.documents[0].revision++;}; // Explicit trusted environment transition, not a native F11 proof.

// Selected annotation leg of the same consent/readiness/disposal contract.
// The local control fence is not proof of all native entry/final-admission routes.
function nativeAnnotationLeg(weakening=null){
 const z=nativeRetainedLifecycleFixture(),n=z.a.x.z,tasks=[],captured=new Map();
 let consent=null,fenced=false,cutoff=false;
 const d=()=>z.root().documents[0],stamp=()=>copy([d().document,d().live,d().lifecycle?.generation??0,d().revision,d().annotationSet,d().serialization,d().binding,d().ordinaryBindings]);
 const evidence=t=>inspectSourceSave(z.a.w.owner,t,z.a.g());
 const ready=()=>{
  const c=d().annotationBaseline?.context,r=c&&z.a.compare(z.a.g(),c);
  return r?.status==='annotation-output'&&r.comparison==='equal'&&!tasks.some(t=>{const e=evidence(t);return e.status!=='save-evidence'||e.contested||e.destinationFaulted||e.phase==='dispatched'&&!e.requestedStrengthSatisfied||!equal(captured.get(t),consent??stamp())&&!(e.phase==='queued'&&e.stopped||e.phase==='dispatched'&&e.knowledge.application==='not-written'&&e.safeDisposition&&e.quiescent);});
 };
 function edit(){const p=n.prepare({operation:'update-body'},{body:'shutdown-edit-'+d().annotationSet});assert.equal(n.execute(p).outcome.publication,'applied');n.remember();}
 function admit(){if(fenced)return {status:'refused'};const before=stamp(),r=z.a.admit('shutdown-annotation-'+tasks.length);if(r.status==='admitted'){tasks.push(r.task);captured.set(r.task,before);}return r;}
 function accept(){
  if(fenced)return 'existing-control';
  if(!consent||!equal(consent,stamp()))return 'stale-consent';
  if(!ready()&&!tasks.some(t=>{const e=evidence(t);return equal(captured.get(t),consent)&&e.status==='save-evidence'&&!e.stopped&&!e.contested&&!e.destinationFaulted&&(e.phase==='queued'||e.phase==='dispatched'&&e.knowledge.application!=='not-written'&&e.knowledge.synchronization!=='failed');}))return 'needs-save';
  fenced=true;return 'accepted';
 }
 function dispose(){
  if(!fenced)return 'refused';
  if(!equal(consent,stamp())&&weakening!=='annotation-consent-bypass')return 'stale-consent';
  if(!ready()&&weakening!=='annotation-readiness-bypass')return 'waiting-for-save';
  const request=z.request('dispose');request.sourceDisposition='discard';request.annotationDisposition='retain';
  return z.run('dispose',z.g(request),request).status;
 }
 return {z,n,tasks,d,evidence,ready,edit,admit,accept,dispose,decide:()=>{consent=stamp();},cutoff:()=>{assert(fenced);cutoff=true;},isCutoff:()=>cutoff,isFenced:()=>fenced,
  dispatch:t=>z.a.dispatch(t),written:(t,data={})=>z.a.w.result(t,data),observe:(t,k)=>z.a.w.observe(t,k),
  install:t=>installAnnotationWriteBaseline(z.a.w.owner,t,n.domain,z.a.pool,z.a.g()),
  retire:t=>retireSourceSave(z.a.w.owner,t,z.root(),sourceSaveRetirement)};
}
export function checkNativeAnnotationShutdown(weakening=null){
 const controls=[],check=(name,fn)=>{fn();controls.push(name);};
 check('equal-loaded-annotation-output-needs-no-invented-write',()=>{const a=nativeAnnotationLeg(weakening);assert(a.ready());a.decide();assert.equal(a.accept(),'accepted');assert.equal(a.admit().status,'refused');assert.equal(a.dispose(),'applied');});
 check('dirty-annotations-need-their-own-pre-fence-admission',()=>{const a=nativeAnnotationLeg(weakening);a.edit();assert(!a.ready());a.decide();const root=a.z.root();assert.equal(a.accept(),'needs-save');assert(!a.isFenced());assert.equal(a.z.root(),root);assert.equal(a.admit().status,'admitted');assert.equal(a.accept(),'accepted');assert.equal(a.dispose(),'waiting-for-save');});
 check('older-admitted-annotation-capture-cannot-fulfil-new-consent',()=>{const a=nativeAnnotationLeg(weakening);a.edit();a.admit();a.edit();a.decide();const root=a.z.root();assert.equal(a.accept(),'needs-save');assert(!a.isFenced());assert.equal(a.z.root(),root);});
 check('failed-sync-before-acceptance-cannot-fence-out-a-replacement-save',()=>{const a=nativeAnnotationLeg(weakening);a.edit();const t=a.admit().task;a.dispatch(t);a.written(t,{synchronization:'failed'});assert.equal(a.evidence(t).requestedStrengthSatisfied,false);a.decide();const root=a.z.root();assert.equal(a.accept(),'needs-save');assert(!a.isFenced());assert.equal(a.z.root(),root);assert.equal(a.admit().status,'admitted');});
 check('faulted-destination-cannot-borrow-an-uncontested-queued-save',()=>{const a=nativeAnnotationLeg(weakening);a.edit();const first=a.admit().task,second=a.admit().task;a.dispatch(first);a.written(first);a.written(first,{application:'not-written',observedBytes:0,synchronization:'not-requested',resourceVersion:null});assert.equal(a.evidence(second).contested,false);assert.equal(a.evidence(second).destinationFaulted,true);a.decide();const root=a.z.root();assert.equal(a.accept(),'needs-save');assert(!a.isFenced());assert.equal(a.z.root(),root);});
 check('qualified-post-fence-annotation-baseline-enables-real-native-disposal',()=>{const a=nativeAnnotationLeg(weakening);a.edit();const t=a.admit().task;a.decide();assert.equal(a.accept(),'accepted');const peer=copy(a.z.root().documents[1]);assert.equal(a.dispatch(t).status,'dispatched');assert.equal(a.written(t).status,'recorded');assert.equal(a.install(t).status,'installed');assert(a.ready());assert.deepEqual(a.d().annotationBaseline.context,annotationSaveContext);assert.equal(a.dispose(),'applied');assert.deepEqual(a.z.root().documents[1],peer);assert.equal(a.retire(t).status,'unavailable');});
 check('annotation-unknown-and-sync-failure-never-satisfy-save-consent',()=>{for(const mode of ['unknown','sync-failed']){const a=nativeAnnotationLeg(weakening);a.edit();const t=a.admit().task;a.decide();a.accept();a.dispatch(t);if(mode==='sync-failed')a.written(t,{synchronization:'failed'});assert(!a.ready());const root=a.z.root();assert.equal(a.dispose(),'waiting-for-save');assert.equal(a.z.root(),root);assert.equal(a.retire(t).status,'unavailable');}});
 check('changed-annotation-consent-cannot-be-repaired-by-a-later-save',()=>{const a=nativeAnnotationLeg(weakening);a.edit();const t=a.admit().task;a.decide();a.accept();a.edit();a.dispatch(t);a.written(t);a.install(t);const root=a.z.root();assert.equal(a.dispose(),'stale-consent');assert.equal(a.z.root(),root);});
 check('cutoff-retains-annotation-holds-and-permits-attributed-late-baseline',()=>{const a=nativeAnnotationLeg(weakening);a.edit();const t=a.admit().task;a.decide();a.accept();a.dispatch(t);const usage=copy(a.z.a.usage());a.cutoff();assert(a.isCutoff());assert(a.isFenced());assert.deepEqual(a.z.a.usage(),usage);assert.equal(a.retire(t).status,'unavailable');a.written(t);assert.equal(a.install(t).status,'installed');assert.equal(a.dispose(),'applied');assert.equal(a.admit().status,'refused');});
 check('late-annotation-contradiction-retains-native-evidence-without-rollback',()=>{const a=nativeAnnotationLeg(weakening);a.edit();const t=a.admit().task;a.decide();a.accept();a.dispatch(t);a.written(t);a.install(t);assert.equal(a.dispose(),'applied');const root=a.z.root();assert.equal(a.written(t,{application:'not-written',observedBytes:0,synchronization:'not-requested',resourceVersion:null}).status,'conflict');assert.equal(a.z.root(),root);assert(a.evidence(t).contested);assert.equal(a.retire(t).status,'unavailable');});
 return {scope:'selected-native-annotation-shutdown-save-readiness-leg',controls,actualAnnotationSaveBaselineAndDisposalChecked:true,sourceAndAnnotationJointAuthorityFenceOrCompleteShutdownProved:false,hostClockProviderTruthOrPhysicalReleaseProved:false,adopted:false};
}
export function checkShutdownComposition(weakening=null){
 const controls=[],check=(name,f)=>{f();controls.push(name);};
 check('save-choice-needs-real-admission-before-fence',()=>{const z=shutdownCompositionFixture(weakening);dirty(z);z.decide(choices('save'));assert.equal(z.accept(10),'needs-save');assert(z.entryAllowed('ordinary-save'));const a=z.admit();assert.equal(a.status,'admitted');assert.equal(z.accept(10),'accepted');assert.equal(z.admit().status,'refused');assert.equal(z.dispose(0),'waiting-for-save');});
 check('independent-annotation-save-is-never-inferred-from-source',()=>{const z=shutdownCompositionFixture(weakening);z.decide(choices('save','save'));assert.equal(z.accept(10),'unsupported-annotation-save');assert(z.entryAllowed('ordinary-save'));});
 check('source-or-annotation-consent-change-refuses-acceptance',()=>{for(const field of ['revision','annotationSet']){const z=shutdownCompositionFixture(weakening);z.decide(choices());z.x.state.documents[0][field]++;assert.equal(z.accept(10),'refused');assert(z.entryAllowed('create'));}});
 check('reference-fence-classifies-all-listed-ordinary-entry-kinds',()=>{const z=shutdownCompositionFixture(weakening);z.decide(choices());assert.equal(z.accept(10),'accepted');for(const kind of entries)assert.equal(z.entryAllowed(kind),false);assert.equal(z.entryAllowed('execute-unknown'),false);});
 check('retry-does-not-renew-wait-origin-or-budget',()=>{const z=shutdownCompositionFixture(weakening);z.decide(choices());z.accept(10,3);z.poll(11);const before=z.report();assert.equal(z.accept(12,99),'existing-control');assert.deepEqual(z.report(),before);z.poll(13);assert.equal(z.report().phase,'cutoff');assert.equal(z.report().started,10);});
 check('annotation-discard-needs-independent-current-authority',()=>{const z=shutdownCompositionFixture(weakening);z.decide(choices());z.accept(10);const root=copy(z.x.state);assert.equal(z.dispose(0,{annotationDiscardAllowed:false}),'unavailable');assert.deepEqual(z.x.state,root);assert.equal(z.dispose(0),'applied');});
 check('actual-disposal-progress-is-once-only-and-peer-preserving',()=>{const z=shutdownCompositionFixture(weakening);z.decide(choices());z.accept(10);const peer=copy(z.x.state.documents[1]);assert.equal(z.dispose(0),'applied');assert.deepEqual(z.x.state.documents[1],peer);const root=z.x.state;assert.equal(z.dispose(0),'already-disposed');assert.equal(z.x.state,root);assert.equal(z.report().disposed,1);});
 check('actual-source-save-baseline-can-complete-after-fence',()=>{const z=shutdownCompositionFixture(weakening);dirty(z);const a=z.admit();z.decide(choices('save'));assert.equal(z.accept(10),'accepted');assert.equal(z.dispatch(a.task),'dispatched');assert.equal(z.observe(a.task,'written'),'recorded');assert.equal(z.publishBaseline(),'published');assert(z.sourceReady(0));assert.equal(z.dispose(0),'applied');});
 check('unknown-issued-effect-and-actual-key-barrier-survive-cutoff',()=>{const z=shutdownCompositionFixture(weakening),a=z.admit();z.dispatch(a.task);z.decide(choices());z.accept(10,1);assert.equal(z.dispose(0),'applied');const usage=copy(sourceSaveUsage(z.x.owner));z.poll(11);assert.equal(z.report().phase,'cutoff');assert.deepEqual(sourceSaveUsage(z.x.owner),usage);assert.equal(z.releaseBinding(0),'unavailable');assert.equal(z.finish(),'retained');const cut=z.cut();z.observe(a.task,'written');z.observe(a.task,'disposition');assert.deepEqual(z.cut(),cut);assert.equal(z.releaseBinding(0),'applied');assert.equal(z.finish(),'retained');});
 check('late-contradiction-stays-contested-with-no-disposal-rollback',()=>{const z=shutdownCompositionFixture(weakening),a=z.admit();z.dispatch(a.task);z.decide(choices());z.accept(10,0);z.dispose(0);z.poll(10);const root=z.x.state;z.observe(a.task,'written');z.observe(a.task,'written',{application:'not-written',observedBytes:0,synchronization:'not-requested',resourceVersion:null});assert(z.x.inspect(a.task).contested);assert.equal(z.x.state,root);assert.equal(z.releaseBinding(0),'unavailable');assert.equal(z.finish(),'retained');});
 check('invalid-clock-ends-wait-without-freeing-owner-obligations',()=>{for(const [now,epoch,valid]of [[9,'test-epoch',true],[11,'other',true],[11,'test-epoch',false]]){const z=shutdownCompositionFixture(weakening);z.decide(choices());z.accept(10,3);const usage=copy(z.report().reader);z.poll(now,epoch,valid);assert.equal(z.report().phase,'cutoff');assert.equal(z.report().remaining,null);assert.deepEqual(z.report().reader,usage);}});
 check('cancel-before-acceptance-does-not-fence-or-revive-the-control',()=>{const z=shutdownCompositionFixture(weakening);z.decide(choices());const root=z.x.state;assert.equal(z.cancel(),'cancelled');assert.equal(z.x.state,root);assert(z.entryAllowed('create'));assert.equal(z.decide(choices()),'refused');assert.equal(z.accept(10),'existing-control');assert.equal(z.report().phase,'cancelled');});
 check('duplicate-dispatch-and-quiescence-do-not-create-another-attempt',()=>{const z=shutdownCompositionFixture(weakening),a=z.admit();assert.equal(z.dispatch(a.task),'dispatched');const usage=copy(sourceSaveUsage(z.x.owner));assert.equal(z.dispatch(a.task),'not-dispatched');assert.deepEqual(sourceSaveUsage(z.x.owner),usage);z.quiesceReads();const reads=copy(resourceReadUsage(z.x.readOwner));z.quiesceReads();assert.deepEqual(resourceReadUsage(z.x.readOwner),reads);});
 check('quiescence-alone-cannot-release-an-unknown-effect-key',()=>{const z=shutdownCompositionFixture(weakening),a=z.admit();z.dispatch(a.task);z.decide(choices());z.accept(10);z.dispose(0);z.observe(a.task,'quiescence');assert.equal(z.x.inspect(a.task).knowledge.application,'unknown');assert.equal(z.releaseBinding(0),'unavailable');assert.equal(z.finish(),'retained');});
 check('missing-retention-support-frames-the-actual-disposal-root',()=>{const z=shutdownCompositionFixture(weakening);z.decide(choices());z.accept(10);const root=z.x.state;assert.equal(z.dispose(0,{retentionComplete:false}),'unavailable');assert.equal(z.x.state,root);assert.equal(z.report().disposed,0);});
 check('positive-closure-drains-real-read-and-write-owners',()=>{const z=shutdownCompositionFixture(weakening),a=z.admit();z.decide(choices());z.accept(10);z.dispatch(a.task);z.observe(a.task,'written');z.observe(a.task,'disposition');z.observe(a.task,'quiescence');assert.equal(z.dispose(0),'applied');assert.equal(z.dispose(1),'applied');assert.equal(z.finish(),'retained');z.quiesceReads();assert.equal(z.finish(),'complete');assert.equal(z.report().reader.heldBytes,0);assert.equal(z.report().writer.heldBytes,0);const final=copy(z.report()),root=z.x.state;assert.equal(z.finish(),'complete');z.quiesceReads();assert.deepEqual(z.report(),final);assert.equal(z.x.state,root);assert.equal(z.dispatch(a.task),'refused');});
 check('queued-cancel-needs-no-nonexistent-effect-receipts',()=>{const z=shutdownCompositionFixture(weakening),a=z.admit();assert.equal(z.cancelQueued(a.task),'stopped');z.decide(choices());assert.equal(z.accept(10),'accepted');assert.equal(z.dispose(0),'applied');assert.equal(z.dispose(1),'applied');z.quiesceReads();assert.equal(z.finish(),'complete');assert.equal(z.report().writer.tasks,0);});
 check('retired-task-cannot-count-as-admitted-save',()=>{const z=shutdownCompositionFixture(weakening);dirty(z);const a=z.admit();z.cancelQueued(a.task);assert.equal(retireSourceSave(z.x.owner,a.task,z.x.state,sourceSaveRetirement).status,'retired');z.decide(choices('save'));assert.equal(z.accept(10),'conflicting-save');assert(z.entryAllowed('ordinary-save'));});
 check('known-not-written-task-cannot-fulfil-save',()=>{const z=shutdownCompositionFixture(weakening);dirty(z);const a=z.admit();z.dispatch(a.task);z.observe(a.task,'written',{application:'not-written',observedBytes:0,synchronization:'not-requested',resourceVersion:null});z.observe(a.task,'disposition');z.observe(a.task,'quiescence');z.decide(choices('save'));assert.equal(z.accept(10),'needs-save');});
 check('historical-equal-baseline-cannot-hide-incompatible-write',()=>{for(const dispatched of [false,true]){const z=shutdownCompositionFixture(weakening),original=copy(z.x.state.documents[0].source);dirty(z);const a=z.admit();if(dispatched)z.dispatch(a.task);z.x.state.documents[0].source=original;z.x.state.documents[0].revision++;assert(z.sourceReady(0));z.decide(choices('save'));assert.equal(z.accept(10),'conflicting-save');assert(z.entryAllowed('ordinary-save'));if(dispatched){z.observe(a.task,'written');z.observe(a.task,'disposition');z.observe(a.task,'quiescence');assert.equal(z.accept(10),'conflicting-save');}}});
 check('positive-save-closure-keeps-approved-basis-after-disposal',()=>{const z=shutdownCompositionFixture(weakening);dirty(z);const a=z.admit();z.decide(choices('save'));assert.equal(z.accept(10),'accepted');z.dispatch(a.task);z.observe(a.task,'written');assert.equal(z.publishBaseline(),'published');z.observe(a.task,'disposition');z.observe(a.task,'quiescence');assert.equal(z.dispose(0),'applied');assert.equal(z.dispose(1),'applied');z.quiesceReads();assert.equal(z.finish(),'complete');});
 check('late-contradiction-invalidates-retained-save-witness',()=>{const z=shutdownCompositionFixture(weakening);dirty(z);const a=z.admit();z.decide(choices('save'));z.accept(10);z.dispatch(a.task);z.observe(a.task,'written');z.publishBaseline();z.observe(a.task,'disposition');z.observe(a.task,'quiescence');assert.equal(z.dispose(0),'applied');assert.equal(z.dispose(1),'applied');z.observe(a.task,'written',{application:'not-written',observedBytes:0,synchronization:'not-requested',resourceVersion:null});z.quiesceReads();assert.equal(z.finish(),'retained');assert.equal(z.report().disposed,2);});
 check('forced-close-needs-original-control-and-current-strict-host-authority',()=>{const z=shutdownCompositionFixture(weakening);z.decide(choices());assert.equal(z.forceClose(z.identity,{controlAllowed:true}),'unavailable');z.accept(10);for(const id of [copy(z.identity),{},null]){const before=z.report();assert.equal(z.forceClose(id,{controlAllowed:true}),'unavailable');assert.deepEqual(z.report(),before);}for(const allowed of [false,1,undefined]){const before=z.report();assert.equal(z.forceClose(z.identity,{controlAllowed:allowed}),'unavailable');assert.deepEqual(z.report(),before);}});
 check('forced-wait-end-retains-real-unknown-owner-and-original-budget',()=>{const z=shutdownCompositionFixture(weakening),a=z.admit();z.dispatch(a.task);z.decide(choices());z.accept(10,3);const before=z.report(),root=z.x.state,effect=copy(z.x.inspect(a.task));assert.equal(z.forceClose(z.identity,{controlAllowed:true}),'retained');const after=z.report();assert.equal(after.phase,'cutoff');assert.equal(after.started,before.started);assert.equal(after.budget,before.budget);assert.equal(after.remaining,before.remaining);assert.deepEqual(after.writer,before.writer);assert.deepEqual(after.reader,before.reader);assert.deepEqual(z.x.inspect(a.task),effect);assert.equal(z.x.state,root);for(const kind of entries)assert.equal(z.entryAllowed(kind),false);const cut=z.cut();assert.equal(z.forceClose(z.identity,{controlAllowed:true}),'retained');assert.deepEqual(z.cut(),cut);assert.equal(z.observe(a.task,'written'),'recorded');assert.equal(z.x.inspect(a.task).knowledge.application,'written');assert.deepEqual(z.cut(),cut);assert.equal(z.finish(),'retained');});
 check('forced-close-after-cutoff-cannot-rewrite-cut-or-release-holds',()=>{const z=shutdownCompositionFixture(weakening),a=z.admit();z.dispatch(a.task);z.decide(choices());z.accept(10,0);z.poll(10);const cut=z.cut(),before=z.report();assert.equal(z.forceClose(z.identity,{controlAllowed:true}),'retained');assert.deepEqual(z.cut(),cut);assert.deepEqual(z.report(),before);assert.equal(z.releaseBinding(0),'refused');});
 check('normal-discharged-completion-is-not-strengthened-or-repeated-by-force',()=>{const z=shutdownCompositionFixture(weakening);z.decide(choices());z.accept(10);z.dispose(0);z.dispose(1);z.quiesceReads();assert.equal(z.finish(),'complete');const before=z.report(),root=z.x.state;assert.equal(z.forceClose(z.identity,{controlAllowed:true}),'complete');assert.deepEqual(z.report(),before);assert.equal(z.x.state,root);});
 return {scope:'working-source-writer-retained-lifecycle-shutdown-composition',controls,actualSourceReadWriteAndLifecycleRelationsChecked:true,allNativeRoutesAnnotationSaveOrCompleteOwnerInventoryProved:false,productionClockServiceOrPhysicalReclamationProved:false,adopted:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
 const baseline=checkShutdownComposition(),mutations=[];
 for(const name of ['fence-bypassed','retry-renews-wait','missing-save-admission','annotation-discard-bypass','force-claims-complete']){assert.throws(()=>checkShutdownComposition(name),{code:'ERR_ASSERTION'},name);mutations.push(name);}
 const annotations=checkNativeAnnotationShutdown(),annotationMutations=[];
 for(const name of ['annotation-consent-bypass','annotation-readiness-bypass']){assert.throws(()=>checkNativeAnnotationShutdown(name),{code:'ERR_ASSERTION'},name);annotationMutations.push(name);}
 console.log(JSON.stringify({...baseline,mutations,annotations,annotationMutations,baselineBeforeMutationChecked:true},null,2));
}
