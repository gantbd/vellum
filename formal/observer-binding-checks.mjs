// Native release attribution only; not a complete public F19 gateway or codec.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as original from './publication-observation.mjs';
import {invocationPublicationFixture,invocationCompoundJob} from './invocation-publication-checks.mjs';
const copy=structuredClone,legs=['event','outcome'];
const limits={maxRootBytes:262144,maxDocuments:8,maxRows:64,maxViews:16,maxProjectionBytes:65536,maxReplyBytes:131072,maxQueue:4,maxOutcomes:8,maxOutcomeBytes:4096,maxIssued:128,maxHeldBytes:2097152,maxBindings:64,maxBindingBytes:65536};
const facts=Object.fromEntries(['authorized','scope','continuous','live','notStopped','projectionAllowed','resourcesReady','coverageComplete','releaseAllowed','retained','transferReady','baselineAllowed','outcomeCaptureAllowed','ownerControlAllowed'].map(k=>[k,true]));
function fixture(api,route,recipient='reader'){
 const producer=invocationPublicationFixture(),before=producer.root(),scope={authority:before.authority,recipient,purpose:'observe',destination:'client',policy:'p1',continuity:'c1',profile:'recipient-publication-observation-reference/2',documents:[{document:copy(before.documents[0].document),source:true,annotations:[],views:[],collection:null}],outcomePolicy:{publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient}};
 const opened=api.openPublicationObserver(before,scope,limits,facts);assert(opened);assert.equal(producer.submit(invocationCompoundJob(producer.reference,before),route).status,'accepted');assert.equal(producer.run().status,'settled');const after=producer.root(),out=producer.read();assert.equal(out.outcome.publication,'applied');
 assert.equal(api.observePublicationRoot(opened.owner,after,scope,facts).status,'queued');const result=api.capturePublicationOutcome(opened.owner,before,{status:'applied',state:after},producer.reference,scope,facts);assert(result);
 const release=(leg,claimed=scope,current=scope,g=facts,owner=opened.owner,handle=result)=>leg==='event'?api.releaseBoundPublicationEvent(owner,claimed,current,g):api.releaseBoundPublicationOutcome(owner,handle,claimed,current,g);
 return {producer,before,after,out,scope,owner:opened.owner,result,release,usage:()=>api.publicationObserverUsage(opened.owner)};
}
const changed=(scope,key)=>{const c=copy(scope);if(key==='epoch')c.outcomePolicy.epoch++;else if(key==='documents')c.documents=[];else if(key==='missing')delete c.purpose;else{c[key]='foreign';if(key==='recipient')c.outcomePolicy.recipient='foreign';}return c;};
export function checkObserverBinding(api=original){
 const controls=[],check=(name,f)=>{f();controls.push(name);};let foreignCases=0,roleCases=0,lossCases=0,pressureCases=0,ceilingCases=0;
 const success=leg=>leg==='event'?'projection':'outcome';
 check('foreign-and-malformed-claims-preserve-original-queue-outcome-and-producer',()=>{
  for(const route of ['typed','dynamic'])for(const leg of legs)for(const field of ['authority','recipient','purpose','destination','policy','continuity','profile','epoch','documents','extra','missing']){
   const x=fixture(api,route),usage=x.usage();assert.equal(x.release(leg,changed(x.scope,field)).status,'unavailable',leg+'/'+field);assert.deepEqual(x.usage(),usage);assert.equal(x.release(leg).status,success(leg));assert.equal(x.usage().stopped,false);assert.equal(x.producer.root(),x.after);assert.deepEqual(x.producer.read(),x.out);foreignCases++;
  }
 });
 check('actual-owner-and-result-roles-cannot-be-substituted',()=>{
  for(const route of ['typed','dynamic'])for(const leg of legs){const x=fixture(api,route),foreign=fixture(api,route,'other'),usage=x.usage();for(const owner of [copy(x.owner),x.result,foreign.owner,x.producer.domain,{}]){assert.equal(x.release(leg,x.scope,x.scope,facts,owner).status,'unavailable');assert.deepEqual(x.usage(),usage);roleCases++;}
   if(leg==='outcome')for(const handle of [copy(x.result),x.owner,foreign.result,{}]){assert.equal(x.release(leg,x.scope,x.scope,facts,x.owner,handle).status,'unavailable');assert.deepEqual(x.usage(),usage);roleCases++;}assert.equal(x.release(leg).status,success(leg));}
 });
 check('independently-attributed-loss-still-fences-event-and-outcome-release',()=>{
  for(const route of ['typed','dynamic'])for(const leg of legs)for(const field of ['authorized','continuous','live','epoch']){const x=fixture(api,route),current=field==='epoch'?changed(x.scope,'epoch'):x.scope,g=field==='epoch'?facts:{...facts,[field]:false};assert.equal(x.release(leg,x.scope,current,g).status,'unavailable');assert.equal(x.usage().stopped,true);assert.equal(x.usage().queued,0);assert.equal(x.release(leg).status,'unavailable');assert.equal(api.resyncPublicationObserver(x.owner,x.after,x.scope,facts).status,'unavailable');assert.equal(x.producer.root(),x.after);assert.deepEqual(x.producer.read(),x.out);lossCases++;}
 });
 check('temporary-release-pressure-does-not-become-caller-or-policy-loss',()=>{
  for(const route of ['typed','dynamic'])for(const leg of legs)for(const field of ['resourcesReady','releaseAllowed','retained','transferReady']){const x=fixture(api,route),usage=x.usage();assert.equal(x.release(leg,x.scope,x.scope,{...facts,[field]:false}).status,'unavailable');assert.deepEqual(x.usage(),usage);assert.equal(x.release(leg).status,success(leg));pressureCases++;}
 });
 check('caller-permission-flags-cannot-replace-trusted-current-policy',()=>{
  for(const route of ['typed','dynamic']){const x=fixture(api,route),claimed=copy(x.scope),current=copy(x.scope);claimed.outcomePolicy.publication=false;assert.equal(x.release('outcome',claimed).outcome.publication,'applied');current.outcomePolicy.publication=false;assert.equal(x.release('outcome',x.scope,current).outcome.publication,'withheld');assert.equal(x.usage().stopped,false);assert.equal(x.release('event',claimed).status,'projection');ceilingCases+=3;}
 });
 return {scope:'native-publication-observation-original-claim-binding',controls,foreignCases,roleCases,lossCases,pressureCases,ceilingCases,producerRoutes:['typed','dynamic'],actualNativeGuardAndObserverCompared:true,legacyTrustedHelperSemanticsUnchanged:true,newPublicVerbOrResultRouteSelected:false,heterogeneousF19OrHostileIngressProved:false,hostPolicyPhysicalConcurrentOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkObserverBinding()));}catch(e){console.error(e.message);process.exitCode=1;}
