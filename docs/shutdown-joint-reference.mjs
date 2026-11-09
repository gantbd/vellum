// Working composition evidence, not an adopted shutdown implementation.
// One actual authority root and writer, with distinct- and same-document save
// cases. This does not prove all native admission fences or host inventories.
import assert from 'node:assert/strict';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {nativeSourceSaveFixture} from '../formal/native-source-save-checks.mjs';
import {nativeSourceReadApi as api} from '../formal/native-source-read-checks.mjs';
import {nativeSourceDestinationFixture,nativeSourceDestinationSourceEdit} from '../formal/native-source-destinations-checks.mjs';
import {annotationArtifactContext as context,annotationArtifactProfile as profile} from '../formal/annotation-artifact-checks.mjs';
import {annotationObservationContext as executionContext,annotationObservationPolicy as executionPolicy} from '../formal/annotation-observation-checks.mjs';
const copy=structuredClone;
const stamp=d=>copy({document:d.document,live:d.live,lifecycle:d.lifecycle?.generation??0,source:d.revision,annotations:d.annotationSet,serialization:d.serialization,binding:d.binding,ordinaryBindings:d.ordinaryBindings});
// Setup only: an explicitly authorized existing native same-destination SaveAs
// establishes an acknowledged source baseline, never a shutdown cleanup exception.
function sameDocumentSourceFixture(){
 const b=nativeSourceDestinationFixture(api),a=b.a,initial=b.root().documents[0],r=b.request('pre-shutdown-source-basis','save-as');
 Object.assign(r,{provider:copy(initial.binding.provider),resource:copy(initial.binding.resource),expectedRegistration:initial.binding.registrationGeneration});
 const t=b.admit(r);assert.equal(b.dispatch(t,r).status,'dispatched');b.done(t);assert.equal(b.run('activate',{task:t,probe:b.probe(r,{epoch:'epoch',token:'after'},'present')},b.g(r)).status,'activated');assert.equal(a.w.observe(t,'quiescence').status,'recorded');
 const document=copy(initial.document),domain=a.x.z.domain,writer=a.w.owner,profile=b.z.profile,d=()=>b.root().documents.find(d=>equal(d.document,document)),g=(extra={})=>b.g(r,{...Object.fromEntries([...api.nativeOrdinarySourceSaveGates,...api.nativeSourceBaselineGates,'comparisonAllowed'].map(k=>[k,true])),...extra});
 const request=(name='ordinary')=>({contract:'source-save/1',operation:'captured-source-save',origin:{operation:name,effect:'write',attempt:'1'},document:copy(document),expectedSource:d().revision,expectedSerialization:d().serialization.generation,expectedBinding:d().binding.generation,expectedRegistration:d().binding.registrationGeneration,protection:'best-effort',acknowledgment:'ordinary',condition:{policy:'pinned',resourceVersion:copy(d().sourceBaseline.resourceVersion)}}),run=(kind,input)=>api.publishInvocationSourceDestination(domain,writer,profile,kind,input,g());
 const dispatch=task=>{const binding=d().binding,out=api.dispatchSourceSave(writer,task,b.root(),{provider:copy(binding.provider),resource:copy(binding.resource),registrationGeneration:binding.registrationGeneration,identityDomain:binding.identityDomain,identityKey:binding.identityKey,resourceVersion:copy(d().sourceBaseline.resourceVersion)},g());if(out.status==='dispatched')a.w.submitted.set(task,copy(out));return out;};
 return {z:b,domain,writer,profile,root:b.root,document,d,g,request,submitted:a.w.submitted,admit:q=>{const out=run('ordinary-admit',q);assert.equal(out.status,'admitted');return out.task;},dispatch,written:(task,resourceVersion,extra={})=>a.w.result(task,{resourceVersion,...extra}),baseline:()=>run('baseline',{document}),edit:()=>nativeSourceDestinationSourceEdit(b,api)};
}
function fixture(weakening=null,sameDocument=false){
 const z=sameDocument?sameDocumentSourceFixture():nativeSourceSaveFixture(),a=z.z.a,source=z.document,annotation=z.root().documents[0].document,tasks=new Map(),disposed=new Set();
 assert(['attached','current'].includes(api.attachAnnotationReadProvenance(z.writer,a.x.owner,a.g()).status));
 assert.equal(a.adopt(a.g(),a.t,z.writer).status,'current');
 let consent=null,fenced=false,cut=null;
 const doc=id=>z.root().documents.find(d=>equal(d.document,id)),basis=()=>z.root().documents.map(stamp);
 function edit(kind){assert(!fenced);if(kind==='source')z.edit();else{const n=a.x.z,p=n.prepare({operation:'update-body'},{body:'joint-shutdown-'+doc(annotation).annotationSet});assert.equal(n.execute(p).outcome.publication,'applied');n.remember();}}
 function ready(kind){
  if(kind==='source'){const r=api.inspectSourceOutput(z.writer,z.root(),source,z.g());return r.status==='source-output'&&r.comparison==='equal';}
  const c=weakening==='annotation-write-context-substitution'?context:doc(annotation).annotationBaseline?.context;if(!c)return false;
  const r=api.inspectAnnotationOutput(z.writer,z.root(),annotation,z.domain,c,a.binding(c),profile,a.g());return r.status==='annotation-output'&&r.comparison==='equal';
 }
 function evidence(row){return api.inspectSourceSave(z.writer,row.task,a.g());}
 function incompatible(kind){if(weakening==='incompatible-write-bypass')return false;const row=tasks.get(kind);if(!row)return false;const e=evidence(row);if(e.status!=='save-evidence'||e.contested||e.destinationFaulted)return true;const id=kind==='source'?source:annotation,approved=consent?.find(d=>equal(d.document,id))??stamp(doc(id));if(equal(row.basis,approved))return false;return !(e.phase==='queued'&&e.stopped||e.phase==='dispatched'&&e.knowledge.application==='not-written'&&e.safeDisposition&&e.quiescent);}
 function compatible(row){const e=evidence(row);return e.status==='save-evidence'&&!e.stopped&&!e.contested&&!e.destinationFaulted&&equal(row.basis,stamp(doc(row.kind==='source'?source:annotation)))&&(e.phase==='queued'||e.phase==='dispatched'&&e.knowledge.application!=='not-written'&&e.knowledge.synchronization!=='failed');}
 function admit(kind){
  if(fenced||tasks.has(kind))return {status:'refused'};
  const before=stamp(doc(kind==='source'?source:annotation)),request=kind==='source'?z.request('joint-source'):a.request('joint-annotation');
  const r=kind==='source'?{status:'admitted',task:z.admit(request)}:api.admitOrdinaryAnnotationSave(z.writer,z.domain,a.pool,request,context,a.binding(context),profile,a.g());
  if(r.status==='admitted')tasks.set(kind,{task:r.task,request,basis:before,kind});return r;
 }
 function accept(){
  if(fenced)return 'existing-control';if(!consent||!equal(consent,basis()))return 'stale-consent';
  for(const kind of ['source','annotation']){if(incompatible(kind))return 'conflicting-'+kind+'-save';if(!ready(kind)&&!compatible(tasks.get(kind)??{}))return 'needs-'+kind+'-save';}
  fenced=true;return 'accepted';
 }
 function dispatch(kind){const row=tasks.get(kind);if(!row)return {status:'refused'};if(kind==='source')return z.dispatch(row.task);
  const b=doc(annotation).ordinaryBindings.annotations,r=api.dispatchSourceSave(z.writer,row.task,z.root(),{provider:b.provider,resource:b.resource,registrationGeneration:b.registrationGeneration,identityDomain:b.identityDomain,identityKey:b.identityKey,resourceVersion:row.request.condition.resourceVersion,presence:'present'},a.g());
  if(r.status==='dispatched')z.submitted.set(row.task,copy(r));return r;
 }
 function written(kind,extra={}){return z.written(tasks.get(kind).task,{epoch:kind==='source'?'source-read-epoch':'read-epoch',token:'joint-'+kind},extra);}
 function install(kind){return kind==='source'?z.baseline():api.installAnnotationWriteBaseline(z.writer,tasks.get(kind).task,z.domain,a.pool,a.g());}
 function dispose(kind){
  if(!fenced)return 'refused';if(disposed.has(kind))return 'already-disposed';
  const id=kind==='source'?source:annotation,index=z.root().documents.findIndex(d=>equal(d.document,id)),d=doc(id);
  if(!equal(consent[index],stamp(d)))return 'stale-consent';
  for(const required of sameDocument?['source','annotation']:[kind]){const e=tasks.has(required)?evidence(tasks.get(required)):null;if((!ready(required)||incompatible(required)||e&&(e.contested||e.destinationFaulted))&&weakening!==required+'-readiness-bypass')return 'waiting-for-'+required+'-save';}
  const r={contract:'document-lifecycle/1',operation:'dispose',document:copy(id),expectedLifecycle:d.lifecycle?.generation??0,expectedSource:d.revision,expectedAnnotations:d.annotationSet,expectedSerialization:d.serialization.generation,expectedBinding:d.binding?.generation??0,sourceDisposition:sameDocument||kind==='source'?'retain':'discard',annotationDisposition:sameDocument||kind==='annotation'?'retain':'discard'},g=z.g({...a.g(),...Object.fromEntries(api.nativeRetainedLifecycleRequiredGates(r).map(k=>[k,true])),annotationHistoryAbsentAttested:true});
  const out=api.publishInvocationRetainedLifecycle(z.domain,r,z.profile,z.writer,g);if(out.status==='applied')for(const k of sameDocument?['source','annotation']:[kind])disposed.add(k);return out.status;
 }
 function restoreSource(){assert(!fenced);const d=doc(source),issued=api.issueInvocation(z.domain,executionContext,a.g());assert(issued);const ins=d.sourceBaseline.bytes.slice(d.serialization.bom?3:0),plan={contract:'owned-annotation-plan/1',invocation:issued.reference,descriptor:issued.reference,sources:[{document:copy(source),expectedSource:d.revision,expectedAnnotationSet:d.annotationSet,edits:[{lo:0,hi:d.source.length,expected:copy(d.source),ins:copy(ins)}]}],annotations:[]};assert.equal(api.submitInvocation(z.domain,issued.ticket,{kind:'compound',plan},'typed',executionContext,executionPolicy,a.g()).status,'accepted');assert.equal(api.runInvocation(z.domain,issued.ticket,executionContext,a.g(),{...a.x.z.h,sourceWritable:true,parserSourceReserved:true,parserSourceLimits:{maxDocuments:8,maxRootBytes:8192,maxBatches:32,maxEdits:64,maxJournalBytes:65536}}).status,'settled');}
 return {z,a,source,annotation,tasks,doc,edit,ready,admit,accept,dispatch,written,install,dispose,restoreSource,
  decide:()=>{assert(!fenced);consent=basis();},isFenced:()=>fenced,
  cutoff:()=>{assert(fenced);if(!cut)cut=copy({writer:api.sourceSaveUsage(z.writer),ready:{source:ready('source'),annotation:ready('annotation')},disposed:[...disposed]});return copy(cut);},cut:()=>copy(cut),
  usage:()=>api.sourceSaveUsage(z.writer),evidence:kind=>evidence(tasks.get(kind))};
}
export function checkJointShutdown(weakening=null){
 const controls=[],check=(name,f)=>{f();controls.push(name);};
 check('clean-loaded-baselines-need-no-redundant-save',()=>{const x=fixture(weakening);assert(x.ready('source'));assert(x.ready('annotation'));x.decide();assert.equal(x.accept(),'accepted');assert.equal(x.tasks.size,0);});
 check('historical-equality-cannot-hide-an-incompatible-captured-write',()=>{for(const dispatched of [false,true]){const x=fixture(weakening);x.edit('source');x.admit('source');if(dispatched)assert.equal(x.dispatch('source').status,'dispatched');x.restoreSource();assert(x.ready('source'));x.decide();const root=x.z.root();assert.equal(x.accept(),'conflicting-source-save');assert(!x.isFenced());assert.equal(x.z.root(),root);if(dispatched){assert.equal(x.written('source').status,'recorded');assert.equal(x.accept(),'conflicting-source-save');}}});
 check('both-dirty-products-need-independent-pre-fence-admission',()=>{const x=fixture(weakening);x.edit('source');x.edit('annotation');x.decide();assert.equal(x.accept(),'needs-source-save');x.admit('source');assert.equal(x.accept(),'needs-annotation-save');assert(!x.isFenced());x.admit('annotation');assert.equal(x.accept(),'accepted');for(const k of ['source','annotation'])assert.equal(x.admit(k).status,'refused');});
 check('one-success-never-satisfies-the-other-save-choice',()=>{for(const first of ['source','annotation']){const second=first==='source'?'annotation':'source',x=fixture(weakening);x.edit('source');x.edit('annotation');x.admit('source');x.admit('annotation');x.decide();assert.equal(x.accept(),'accepted');assert.equal(x.dispatch(first).status,'dispatched');assert.equal(x.written(first).status,'recorded');assert(['published','installed'].includes(x.install(first).status));assert(x.ready(first));assert(!x.ready(second));const before=x.z.root();assert.equal(x.dispose(second),'waiting-for-'+second+'-save');assert.equal(x.z.root(),before);}});
 check('both-completion-orders-use-the-same-root-writer-and-native-disposal',()=>{for(const order of [['source','annotation'],['annotation','source']]){const x=fixture(weakening);for(const k of order)x.edit(k);for(const k of order)x.admit(k);x.decide();x.accept();for(const k of order){assert.equal(x.dispatch(k).status,'dispatched');x.written(k);assert(['published','installed'].includes(x.install(k).status));}for(const k of order){const peers=x.z.root().documents.filter(d=>!equal(d.document,k==='source'?x.source:x.annotation));assert.equal(x.dispose(k),'applied');assert.deepEqual(x.z.root().documents.filter(d=>!equal(d.document,k==='source'?x.source:x.annotation)),peers);assert.equal(x.dispose(k),'already-disposed');}assert(x.usage().tasks>=2);}});
 check('cutoff-preserves-both-unknown-effects-and-allows-late-baselines',()=>{const x=fixture(weakening);for(const k of ['source','annotation']){x.edit(k);x.admit(k);}x.decide();x.accept();for(const k of ['source','annotation'])x.dispatch(k);const before=copy(x.usage()),cut=x.cutoff();assert.deepEqual(x.usage(),before);for(const k of ['annotation','source']){assert.equal(x.evidence(k).knowledge.application,'unknown');x.written(k);assert(['published','installed'].includes(x.install(k).status));}assert.deepEqual(x.cut(),cut);for(const k of ['source','annotation'])assert.equal(x.dispose(k),'applied');assert(x.isFenced());});
 check('failed-disposition-acceptance-does-not-undo-an-admitted-save',()=>{const x=fixture(weakening);x.edit('source');x.edit('annotation');x.admit('source');const before=copy(x.usage());x.decide();assert.equal(x.accept(),'needs-annotation-save');assert.deepEqual(x.usage(),before);assert(!x.isFenced());assert.equal(x.evidence('source').phase,'queued');});
 check('late-contradiction-does-not-rollback-disposal-or-become-safe-release',()=>{const x=fixture(weakening);for(const k of ['source','annotation']){x.edit(k);x.admit(k);}x.decide();x.accept();for(const k of ['source','annotation']){x.dispatch(k);x.written(k);x.install(k);assert.equal(x.dispose(k),'applied');}const root=x.z.root();for(const k of ['source','annotation']){assert.equal(x.written(k,{application:'not-written',observedBytes:0,synchronization:'not-requested',resourceVersion:null}).status,'conflict');assert(x.evidence(k).contested);}assert.equal(x.z.root(),root);});
 return {scope:'working-joint-authority-source-annotation-shutdown',controls,sameActualRootAndWriterChecked:true,distinctDocumentSaveLegs:true,allNativeFencesSameDocumentPairOrCompleteShutdownProved:false,hostProviderClockOrPhysicalTruthProved:false,adopted:false};
}
export function checkSameDocumentShutdown(weakening=null){
 const controls=[],check=(name,f)=>{f();controls.push(name);};
 check('same-document-initial-qualified-products-need-no-new-save',()=>{const x=fixture(weakening,true);assert.deepEqual(x.source,x.annotation);assert(x.ready('source'));assert(x.ready('annotation'));x.decide();assert.equal(x.accept(),'accepted');assert.equal(x.dispose('source'),'applied');assert.equal(x.dispose('annotation'),'already-disposed');});
 check('both-save-orders-require-both-independent-baselines-before-one-disposal',()=>{for(const first of ['source','annotation']){const second=first==='source'?'annotation':'source',x=fixture(weakening,true);x.edit('source');x.edit('annotation');assert(!x.ready('source'));assert(!x.ready('annotation'));x.admit('source');x.admit('annotation');x.decide();assert.equal(x.accept(),'accepted');assert.equal(x.dispatch(first).status,'dispatched');assert.equal(x.written(first).status,'recorded');assert(['published','installed'].includes(x.install(first).status));assert(x.ready(first));assert(!x.ready(second));const before=x.z.root();for(const entry of ['source','annotation']){assert.equal(x.dispose(entry),'waiting-for-'+second+'-save');assert.equal(x.z.root(),before);}assert.equal(x.dispatch(second).status,'dispatched');assert.equal(x.written(second).status,'recorded');assert(['published','installed'].includes(x.install(second).status));const peers=x.z.root().documents.filter(d=>!equal(d.document,x.source));assert.equal(x.dispose(first),'applied');assert.deepEqual(x.z.root().documents.filter(d=>!equal(d.document,x.source)),peers);assert.equal(x.dispose(second),'already-disposed');assert(x.usage().tasks>=3);}});
 check('same-document-cutoff-preserves-two-unknown-effects-and-historical-cut',()=>{const x=fixture(weakening,true);x.edit('source');x.edit('annotation');for(const k of ['source','annotation'])x.admit(k);x.decide();assert.equal(x.accept(),'accepted');for(const k of ['source','annotation'])assert.equal(x.dispatch(k).status,'dispatched');const usage=copy(x.usage()),cut=x.cutoff();assert.deepEqual(x.usage(),usage);for(const k of ['annotation','source']){assert.equal(x.evidence(k).knowledge.application,'unknown');x.written(k);assert(['published','installed'].includes(x.install(k).status));}assert.deepEqual(x.cut(),cut);assert.equal(x.dispose('source'),'applied');assert.equal(x.dispose('annotation'),'already-disposed');});
 return {scope:'working-same-document-source-annotation-shutdown',controls,sameActualDocumentRootAndWriterChecked:true,independentOrdinarySaveBaselinesChecked:true,preShutdownSetupUsesExistingAuthorizedSameDestinationSaveAs:true,allNativeFencesOrCompleteShutdownProved:false,hostProviderClockOrPhysicalTruthProved:false,adopted:false};
}
// Negative evidence: these observations expose the wrapper-only fence. They are
// not successful shutdown invariants and must stop reproducing after a real
// authority fence replaces the wrapper. No qualified formal input is modified.
export function inspectNativeShutdownFenceGaps(){
 const counterexamples=[],record=(name,f)=>{f();counterexamples.push(name);};
 const accepted=()=>{const x=fixture();x.decide();assert.equal(x.accept(),'accepted');assert(x.isFenced());return x;};
 record('actual-invocation-issuance-remains-open-after-wrapper-acceptance',()=>{
  const x=accepted(),before=api.invocationUsage(x.z.domain),issued=api.issueInvocation(x.z.domain,executionContext,x.a.g());
  assert(issued);assert.equal(api.invocationUsage(x.z.domain).issued,before.issued+1);assert(x.isFenced());
 });
 record('actual-source-save-admission-bypasses-wrapper-fence',()=>{
  const x=accepted(),before=api.sourceSaveUsage(x.z.writer),root=x.z.root();
  assert.equal(x.admit('source').status,'refused');
  const task=x.z.admit(x.z.request('post-wrapper-fence-source'));assert(task);
  assert.equal(api.sourceSaveUsage(x.z.writer).tasks,before.tasks+1);assert.equal(x.z.root(),root);assert(x.isFenced());
 });
 record('actual-annotation-save-admission-bypasses-wrapper-fence',()=>{
  const x=accepted(),before=api.sourceSaveUsage(x.z.writer),root=x.z.root();
  assert.equal(x.admit('annotation').status,'refused');
  const out=api.admitOrdinaryAnnotationSave(x.z.writer,x.z.domain,x.a.pool,x.a.request('post-wrapper-fence-annotation'),context,x.a.binding(context),profile,x.a.g());
  assert.equal(out.status,'admitted');assert.equal(api.sourceSaveUsage(x.z.writer).tasks,before.tasks+1);assert.equal(x.z.root(),root);assert(x.isFenced());
 });
 record('previously-admitted-authored-publication-bypasses-wrapper-fence',()=>{
  const x=fixture(),d=x.doc(x.source),issued=api.issueInvocation(x.z.domain,executionContext,x.a.g());assert(issued);
  const plan={contract:'owned-annotation-plan/1',invocation:issued.reference,descriptor:issued.reference,sources:[{document:copy(x.source),expectedSource:d.revision,expectedAnnotationSet:d.annotationSet,edits:[{lo:0,hi:d.source.length,expected:copy(d.source),ins:[88]}]}],annotations:[]};
  assert.equal(api.submitInvocation(x.z.domain,issued.ticket,{kind:'compound',plan},'typed',executionContext,executionPolicy,x.a.g()).status,'accepted');
  x.decide();assert.equal(x.accept(),'accepted');const before=x.z.root();
  assert.equal(api.runInvocation(x.z.domain,issued.ticket,executionContext,x.a.g(),{...x.a.x.z.h,sourceWritable:true,parserSourceReserved:true,parserSourceLimits:{maxDocuments:8,maxRootBytes:8192,maxBatches:32,maxEdits:64,maxJournalBytes:65536}}).status,'settled');
  assert.notEqual(x.z.root(),before);assert.equal(x.doc(x.source).revision,d.revision+1);assert.deepEqual(x.doc(x.source).source,[88]);assert(x.isFenced());
 });
 return {scope:'negative-working-wrapper-fence-audit',counterexamples,actualNativeAdmissionOrPublicationFenceProved:false,shutdownRequirementSatisfied:false,adopted:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){const baseline=checkJointShutdown(),mutations=[];for(const name of ['source-readiness-bypass','annotation-readiness-bypass','incompatible-write-bypass','annotation-write-context-substitution']){assert.throws(()=>checkJointShutdown(name),{code:'ERR_ASSERTION'},name);mutations.push(name);}const sameDocument=checkSameDocumentShutdown(),sameDocumentMutations=[];for(const name of ['source-readiness-bypass','annotation-readiness-bypass']){assert.throws(()=>checkSameDocumentShutdown(name),{code:'ERR_ASSERTION'},name);sameDocumentMutations.push(name);}console.log(JSON.stringify({...baseline,mutations,sameDocument,sameDocumentMutations,baselineBeforeMutationChecked:true}));}
