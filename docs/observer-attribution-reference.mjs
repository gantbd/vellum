// Working facade-ordering evidence. This is not an installed public F19 gateway.
import assert from 'node:assert/strict';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {invocationPublicationFixture,invocationCompoundJob} from '../formal/invocation-publication-checks.mjs';
import * as observer from '../formal/publication-observation.mjs';

const copy=structuredClone;
const limits={maxRootBytes:262144,maxDocuments:8,maxRows:64,maxViews:16,maxProjectionBytes:65536,maxReplyBytes:131072,maxQueue:4,maxOutcomes:8,maxOutcomeBytes:4096,maxIssued:128,maxHeldBytes:2097152,maxBindings:64,maxBindingBytes:65536};
const facts=Object.fromEntries(['authorized','scope','continuous','live','notStopped','projectionAllowed','resourcesReady','coverageComplete','releaseAllowed','retained','transferReady','baselineAllowed','outcomeCaptureAllowed','ownerControlAllowed'].map(k=>[k,true]));
function fixture(route){
 const producer=invocationPublicationFixture(),before=producer.root(),scope={authority:before.authority,recipient:'reader',purpose:'observe',destination:'client',policy:'p1',continuity:'c1',profile:'recipient-publication-observation-reference/2',documents:[{document:copy(before.documents[0].document),source:true,annotations:[],views:[],collection:null}],outcomePolicy:{publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'}};
 const opened=observer.openPublicationObserver(before,scope,limits,facts);assert(opened);
 assert.equal(producer.submit(invocationCompoundJob(producer.reference,before),route).status,'accepted');assert.equal(producer.run().status,'settled');const after=producer.root();assert.equal(producer.read().outcome.publication,'applied');
 assert.equal(observer.observePublicationRoot(opened.owner,after,scope,facts).status,'queued');
 // The reference is resolved by actual identity, never by its descriptive fields.
 const reference=Object.freeze({kind:'working-observation-reference'}),bindings=new WeakMap([[reference,{owner:opened.owner,scope:copy(scope)}]]);
 let nativeCalls=0;
 function release(requestReference,callerScope,trustedCurrentScope=scope,trustedFacts=facts,unguarded=false){
  const binding=bindings.get(requestReference);
  // This comparison illustrates attribution ordering, not a hostile decoder or
  // selected public context schema. Both records are finite normalized values.
  if(!binding||!unguarded&&!equal(callerScope,binding.scope))return {status:'unavailable'};
  nativeCalls++;
  return observer.releasePublicationEvent(binding.owner,unguarded?callerScope:trustedCurrentScope,trustedFacts);
 }
 return {producer,before,after,scope,owner:opened.owner,reference,release,calls:()=>nativeCalls,usage:()=>observer.publicationObserverUsage(opened.owner)};
}
export function checkObserverAttribution(){
 const controls=[],check=(name,f)=>{f();controls.push(name);};let foreignCases=0,lossCases=0,roleCases=0,counterexamples=0;
 const fields=['authority','recipient','purpose','destination','policy','continuity','profile','epoch','documents','extra'];
 const changed=(scope,key)=>{const c=copy(scope);if(key==='epoch')c.outcomePolicy.epoch++;else if(key==='documents')c.documents=[];else c[key]='foreign';return c;};
 check('foreign-request-refuses-before-trusted-observer-loss-processing',()=>{
  for(const route of ['typed','dynamic'])for(const field of fields){const x=fixture(route),usage=x.usage(),root=x.producer.root(),out=x.producer.read();assert.equal(x.release(x.reference,changed(x.scope,field)).status,'unavailable');assert.equal(x.calls(),0);assert.deepEqual(x.usage(),usage);assert.equal(x.release(x.reference,x.scope).status,'projection');assert.equal(x.usage().stopped,false);assert.equal(x.producer.root(),root);assert.deepEqual(x.producer.read(),out);foreignCases++;}
 });
 check('copied-foreign-and-wrong-role-references-do-not-reach-native-helper',()=>{
  for(const route of ['typed','dynamic']){const x=fixture(route),foreign=fixture(route),usage=x.usage();for(const reference of [copy(x.reference),foreign.reference,x.owner,{}]){assert.equal(x.release(reference,x.scope).status,'unavailable');assert.equal(x.calls(),0);assert.deepEqual(x.usage(),usage);roleCases++;}assert.equal(x.release(x.reference,x.scope).status,'projection');}
 });
 check('independently-attributed-real-loss-remains-absorbing',()=>{
  for(const route of ['typed','dynamic'])for(const field of ['authorized','continuous','live','epoch']){const x=fixture(route),root=x.producer.root(),out=x.producer.read(),c=field==='epoch'?changed(x.scope,'epoch'):x.scope,g=field==='epoch'?facts:{...facts,[field]:false};assert.equal(x.release(x.reference,x.scope,c,g).status,'unavailable');assert.equal(x.calls(),1);assert.equal(x.usage().stopped,true);assert.equal(x.usage().queued,0);assert.equal(x.release(x.reference,x.scope).status,'unavailable');assert.equal(observer.resyncPublicationObserver(x.owner,root,x.scope,facts).status,'unavailable');assert.equal(x.producer.root(),root);assert.deepEqual(x.producer.read(),out);lossCases++;}
 });
 check('unguarded-adaptation-counterexample-retains-native-trusted-semantics',()=>{
  for(const route of ['typed','dynamic']){const x=fixture(route);assert.equal(x.release(x.reference,changed(x.scope,'recipient'),x.scope,facts,true).status,'unavailable');assert.equal(x.calls(),1);assert.equal(x.usage().stopped,true);assert.equal(x.release(x.reference,x.scope).status,'unavailable');counterexamples++;}
 });
 return {scope:'working-F19-observer-attribution-ordering',controls,foreignCases,roleCases,lossCases,counterexamples,producerRoutes:['typed','dynamic'],F19TypedDynamicFacadeTested:false,actualNativeObserverAndProducerUsed:true,adoptedHelperSemanticsUnchanged:true,installedPublicGatewayOrContextSumProved:false,hostIdentityPolicyTruthOrHostileDecoderProved:false,fullQualificationOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkObserverAttribution()));
