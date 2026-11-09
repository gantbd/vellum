// Working N03 composition evidence, not a public codec or complete F19 sum.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {lifecycleEnvelopeFixture} from '../formal/lifecycle-envelope-checks.mjs';
import {activationEnvelopeFixture,activationEnvelopeIds} from '../formal/lifecycle-activation-envelope-checks.mjs';
import * as kernel from '../formal/invocation-publication.mjs';
import {requestInvocationStop} from '../formal/invocation-control.mjs';
import * as observer from '../formal/publication-observation.mjs';
import {nativeLifecycleResultValid} from '../formal/lifecycle-results.mjs';
const copy=structuredClone,forms=[...['create-scratch','attach-view','detach-view','membership','dispose'].map(id=>({id,activation:false})),...activationEnvelopeIds.map(id=>({id,activation:true}))];
const limits={maxRootBytes:262144,maxDocuments:8,maxRows:64,maxViews:16,maxProjectionBytes:65536,maxReplyBytes:131072,maxQueue:4,maxOutcomes:8,maxOutcomeBytes:4096,maxIssued:128,maxHeldBytes:2097152,maxBindings:64,maxBindingBytes:65536};
const g=Object.fromEntries(['authorized','scope','continuous','live','notStopped','projectionAllowed','resourcesReady','coverageComplete','releaseAllowed','retained','transferReady','baselineAllowed','outcomeCaptureAllowed','ownerControlAllowed'].map(k=>[k,true]));
function fixture(form,route='typed',annotationFacet=false){
 const x=form.activation?activationEnvelopeFixture(form.id,route):lifecycleEnvelopeFixture(form.id,route),root=x.root(),c={authority:root.authority,recipient:'reader',purpose:'observe',destination:'client',policy:'p1',continuity:'c1',profile:'recipient-publication-observation-reference/2',documents:[{document:copy(root.documents[0].document),source:true,annotations:[],views:[],collection:null}],outcomePolicy:{publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'}};
 if(annotationFacet){assert.equal(form.id,'annotation-restore');const identity=x.request.request.binding.identities[0];c.documents[0].annotations=[{id:identity.id,incarnation:identity.incarnation,reference:'retained-note',body:true,target:true}];}
 // Capture the selected facet before publication, not after its result is known.
 const selected=observer.openPublicationObserver(root,c,limits,g);assert(selected);
 const prepared=x.prepare();assert(prepared,form.id);const job=form.activation?prepared.job:{kind:'lifecycle',plan:prepared},submit=()=>kernel.submitInvocation(x.domain,x.ticket,job,route,x.controlContext,x.controlPolicy,x.facts()),run=()=>kernel.runInvocation(x.domain,x.ticket,x.controlContext,x.facts(),x.host),outcome=()=>kernel.observeInvocation(x.domain,x.ticket,x.controlContext,x.controlPolicy,x.facts());
 const receipt=(extra={})=>job.kind==='relation'?kernel.observeRelationResult(x.domain,x.ticket,x.controlContext,x.controlPolicy,{...x.facts(),relationResultReadable:true,relationResultContextCurrent:true,historicalTargetReadable:true,...extra}):kernel.observeLifecycleResult(x.domain,x.ticket,x.controlContext,x.controlPolicy,{...x.facts(),lifecycleResultReadable:true,lifecycleResultContextCurrent:true,historicalDocumentReadable:true,...extra});
 return {x,c,owner:selected.owner,job,submit,run,outcome,receipt};
}
function correlateReceipt(form,z,before,after,out,receipt){
 const result=receipt.result,identityEqual=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
 assert.equal(result.invocation,z.x.reference,form.id+'/invocation');
 if(z.job.kind==='relation'){
  const kind=form.id.startsWith('view-')?'view':'workspace',created=form.id.endsWith('-create');
  assert.equal(out.outcome.publication,'applied');assert.equal(result.kind,kind);assert.equal(result.status,created?'created':'retired');
  const collection=kind==='view'?'views':'workspaces',added=after[collection].filter(row=>!before[collection].some(old=>identityEqual(old[kind],row[kind])));
  if(created)assert.equal(added.length,1);
  assert.deepEqual(result.target,created?added[0][kind]:z.x.request.target);
 }else{
  const added=after.documents.filter(row=>!before.documents.some(old=>identityEqual(old.document,row.document))),id=form.id;
  assert.equal(result.status,out.outcome.publication);
  assert.equal(result.operation,id==='source-saveas'?'source-save-as':id.startsWith('annotation-')?'annotation-load':id);
  if(id==='create-scratch'||id==='source-load')assert.equal(added.length,1);
  const document=id==='create-scratch'||id==='source-load'?added[0].document:id==='annotation-restore'?z.x.request.request.binding.document:id==='annotation-baseline'||id==='source-saveas'?before.documents[0].document:z.x.request.request.document;
  assert.deepEqual(result.document,document,id+'/document');
  if(id==='source-load')assert.equal(result.activation,'created');
  if(id.startsWith('annotation-')){assert.equal(result.phase,id==='annotation-restore'?'restore':'adopt-baseline');assert.equal(result.disposition,id==='annotation-restore'?'restored':'installed');}
 }
}
export function checkLifecycleOutcomeEvents(weakening=null){
 const controls=[],rows=[],control=(name,f)=>{f();controls.push(name);};let nativeCases=0,stopCases=0,withheldCases=0,annotationFacetCases=0;
 control('lifecycle-publication-receipt-and-source-projection-have-distinct-meanings',()=>{
  for(const form of forms)for(const route of ['typed','dynamic']){const z=fixture(form,route),before=z.x.root();assert.equal(z.submit().status,'accepted');assert.equal(z.run().status,'settled');const after=z.x.root(),out=z.outcome(),receipt=z.receipt();assert(['applied','unchanged'].includes(out.outcome.publication));assert.equal(out.outcome.effect,'none');assert(nativeLifecycleResultValid(receipt,z.job.kind));assert.equal(receipt.status,'result');
   const correlated=copy(receipt);if(weakening==='foreign-invocation-receipt')correlated.result.invocation='wrong-invocation';if(weakening==='wrong-receipt-operation'&&correlated.result.operation)correlated.result.operation='dispose';if(weakening==='wrong-receipt-target'){const target=correlated.result.document??correlated.result.target;target.id='foreign';}if(weakening==='wrong-receipt-status')correlated.result.status='unchanged';correlateReceipt(form,z,before,after,out,correlated);
   const observed=observer.observePublicationRoot(z.owner,after,z.c,g),expected=form.id==='dispose'?'gap':'unchanged';
   const classification=weakening==='publication-implies-source-event'&&out.outcome.publication==='applied'?'queued':observed.status;assert.equal(classification,expected,form.id);assert.equal(observer.releasePublicationEvent(z.owner,z.c,g).status,expected==='gap'?'gap':'empty');assert.equal(z.x.root(),after);assert.deepEqual(z.outcome(),out);
   assert.equal(kernel.submitInvocation(z.x.domain,z.x.ticket,z.job,route==='typed'?'dynamic':'typed',z.x.controlContext,z.x.controlPolicy,z.x.facts()).status,'observe-existing');assert.equal(z.run().status,'unavailable');assert.equal(z.x.root(),after);assert.equal(after.publication,before.publication+(out.outcome.publication==='applied'?1:0));rows.push({form:form.id,route,publication:out.outcome.publication,effect:out.outcome.effect,sourceProjection:expected,receipt:receipt.result.operation??receipt.result.kind});nativeCases++;
  }
 });
 control('pending-cancellation-neither-publishes-nor-produces-a-source-event',()=>{
  for(const form of forms)for(const route of ['typed','dynamic']){const z=fixture(form,route),before=z.x.root();assert.equal(z.submit().status,'accepted');assert.equal(requestInvocationStop(z.x.domain,z.x.ticket,z.x.controlContext,{...z.x.facts(),ownerControlAllowed:true,controlContextCurrent:true,controlScopeValid:true,protectedControlReady:true}).status,'stop-requested');assert.equal(z.run().status,'settled');assert.equal(z.x.root(),before);const out=z.outcome();assert.equal(out.outcome.publication,'notApplied');assert.equal(out.outcome.control,'stopped');assert.equal(out.outcome.effect,'none');assert.equal(z.receipt().status,'unavailable');assert.equal(observer.observePublicationRoot(z.owner,z.x.root(),z.c,g).status,'unchanged');assert.equal(observer.releasePublicationEvent(z.owner,z.c,g).status,'empty');stopCases++;
  }
 });
 control('observer-loss-and-receipt-withholding-do-not-erase-writer-publication',()=>{
  for(const form of forms){const z=fixture(form),before=z.x.root();z.submit();z.run();const root=z.x.root(),out=z.outcome(),denied=z.job.kind==='relation'?{relationResultReadable:false}:{lifecycleResultReadable:false};assert.equal(z.receipt(denied).status,'unavailable');assert.equal(observer.releasePublicationEvent(z.owner,z.c,{...g,authorized:false}).status,'unavailable');assert.equal(observer.observePublicationRoot(z.owner,root,z.c,g).status,'unavailable');
   const current=weakening==='observer-loss-erases-writer-outcome'?{status:'unavailable'}:z.outcome();assert.deepEqual(current,out);assert.equal(z.x.root(),root);assert.equal(root.publication,before.publication+(out.outcome.publication==='applied'?1:0));assert.equal(observer.publicationObserverUsage(z.owner).stopped,true);withheldCases++;
  }
 });
 control('permitted-restored-annotation-delivers-exact-committed-facets-despite-withheld-command-receipt',()=>{
  for(const route of ['typed','dynamic']){
   const form={id:'annotation-restore',activation:true},z=fixture(form,route,true),before=z.x.root();assert.equal(z.submit().status,'accepted');assert.equal(z.run().status,'settled');const after=z.x.root(),out=z.outcome();correlateReceipt(form,z,before,after,out,z.receipt());
   assert.equal(z.receipt({lifecycleResultReadable:false}).status,'unavailable');assert.equal(observer.observePublicationRoot(z.owner,after,z.c,g).status,'queued');
   const event=observer.releasePublicationEvent(z.owner,z.c,g);assert.equal(event.status,'projection');const scope=z.c.documents[0],document=after.documents.find(d=>JSON.stringify(d.document)===JSON.stringify(scope.document)),identity=z.x.request.request.binding.identities[0],record=document.records.find(r=>r.id===identity.id&&r.incarnation===identity.incarnation);assert(record);
   assert.deepEqual(event.projection,{documents:[{document:copy(scope.document),source:{revision:document.revision,bytes:copy(document.source)},annotations:[{reference:'retained-note',content:{body:record.body,version:record.bodyVersion},location:{placement:copy(record.placement),targetGeneration:record.targetGeneration}}],views:[]}]});
   assert.equal(observer.releasePublicationEvent(z.owner,z.c,g).status,'empty');assert.deepEqual(z.outcome(),out);assert.equal(z.x.root(),after);annotationFacetCases++;
  }
 });
 return {scope:'working-F17-native-outcome-source-event-composition',controls,nativeCases,stopCases,withheldCases,annotationFacetCases,rows,
  actualNativeLifecycleOutcomeReceiptAndObserverCompared:true,namedSourceAndSelectedAnnotationProjectionOnly:true,
  publicReceiptRouteOrCompleteF19ContextSumSelected:false,allViewsCollectionsEffectsOrNativeFencesProved:false,
  hostPolicyPhysicalConcurrentOrIndependentProof:false,adopted:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){const baseline=checkLifecycleOutcomeEvents(),unsafeAlternatives=[];for(const name of ['publication-implies-source-event','observer-loss-erases-writer-outcome','foreign-invocation-receipt','wrong-receipt-operation','wrong-receipt-target','wrong-receipt-status']){assert.throws(()=>checkLifecycleOutcomeEvents(name),{code:'ERR_ASSERTION'},name);unsafeAlternatives.push(name);}console.log(JSON.stringify({...baseline,unsafeAlternatives}));}
