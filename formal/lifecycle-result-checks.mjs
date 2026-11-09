import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as resultApi from './lifecycle-results.mjs';
import * as kernel from './invocation-publication.mjs';
import {releaseLifecycleInvocationResult} from './lifecycle-invocation-state.mjs';
import {releaseRelationInvocationResult} from './relation-invocation-state.mjs';
import {activationEnvelopeFixture,activationEnvelopeIds} from './lifecycle-activation-envelope-checks.mjs';
import {lifecycleEnvelopeFixture} from './lifecycle-envelope-checks.mjs';
import {lifecycleInvocationFixture} from './lifecycle-invocation-checks.mjs';
import {loadInvocationFixture} from './load-invocation-checks.mjs';
import {annotationLoadInvocationFixture} from './annotation-load-invocation-checks.mjs';
import {annotationNativeLoadDonor,annotationNativeLoadPack} from './annotation-native-load-checks.mjs';
const copy=structuredClone,document={authority:'a',id:'document',incarnation:1};
function current(x,kind,g=x.facts(),ticket=x.ticket,c=x.controlContext){
 return kind==='relation'?kernel.observeRelationResult(x.domain,ticket,c,x.controlPolicy,g):kernel.observeLifecycleResult(x.domain,ticket,c,x.controlPolicy,g);
}
export function checkLifecycleResults(api=resultApi){
 const controls=[],check=(n,f)=>{f();controls.push(n);},decl=api.lifecycleReceiptDeclarations(),samples=[];
 let nativeCases=0,denials=0,releaseCases=0,boundaries=0,shapeCases=0;
 const expect=(value,kind,valid)=>{let out;assert.doesNotThrow(()=>{out=api.nativeLifecycleResultValid(value,kind);});assert.equal(out,valid);valid?shapeCases++:denials++;};
 const capture=(x,kind,read,route='typed')=>{assert.equal(x.submit(route).status,'accepted');assert.equal(x.run().status,'settled');const r=read();expect(r,kind,true);samples.push({kind,receipt:copy(r.result)});nativeCases++;return r;};
 check('declarations-retain-four-existing-shapes-and-internal-discharge-is-not-a-public-verb',()=>{
  assert.deepEqual(Object.keys(decl),['identityFields','document','load','annotations','relation']);assert.equal(decl.document.operations.length,10);assert.deepEqual(decl.load.activation,{created:'applied',joined:'unchanged','already-activated':'unchanged'});assert.deepEqual(decl.annotations.phases,{restore:{restored:'applied',unchanged:'unchanged'},'adopt-baseline':{installed:'applied',current:'unchanged'}});assert(Object.isFrozen(decl)&&Object.isFrozen(decl.document.fields)&&Object.isFrozen(decl.annotations.phases.restore));
 });
 check('fresh-actual-native-publications-and-original-current-result-release',()=>{
  for(const id of activationEnvelopeIds)for(const route of ['typed','dynamic']){
   const x=activationEnvelopeFixture(id,route),p=x.prepare();assert(p);const kind=p.job.kind;assert.equal(x.submit(p).status,'accepted');assert.equal(x.run().status,'settled');const r=x.read();expect(r,kind,true);samples.push({kind,receipt:copy(r.result)});nativeCases++;
   const changed=copy(r);changed.result.invocation='caller-change';assert.deepEqual(x.read(),r);assert.equal(current(x,kind,x.facts(),copy(x.ticket)).status,'unavailable');assert.equal(current(x,kind,x.facts(),x.ticket,{...x.controlContext,recipient:'other'}).status,'unavailable');releaseCases+=2;
  }
  for(const id of ['create-scratch','attach-view','detach-view','membership','dispose'])for(const route of ['typed','dynamic']){
   const x=lifecycleEnvelopeFixture(id,route),plan=x.prepare();assert(plan);assert.equal(kernel.submitInvocation(x.domain,x.ticket,{kind:'lifecycle',plan},route,x.controlContext,x.controlPolicy,x.facts()).status,'accepted');assert.equal(kernel.runInvocation(x.domain,x.ticket,x.controlContext,x.facts(),x.host).status,'settled');const r=current(x,'lifecycle',{...x.facts(),lifecycleResultReadable:true,lifecycleResultContextCurrent:true,historicalDocumentReadable:true});expect(r,'lifecycle',true);samples.push({kind:'lifecycle',receipt:copy(r.result)});nativeCases++;
  }
 });
 check('native-unchanged-joined-repeat-and-private-discharge-receipts-remain-valid',()=>{
  for(const mode of ['created','joined','already-activated'])for(const route of ['typed','dynamic']){const x=loadInvocationFixture(undefined,mode),r=capture(x,'lifecycle',x.read,route);assert.equal(r.result.activation,mode);assert.equal(r.result.status,mode==='created'?'applied':'unchanged');}
  for(const mode of ['restore','adopt-baseline','current'])for(const route of ['typed','dynamic']){const x=annotationLoadInvocationFixture(undefined,mode),r=capture(x,'lifecycle',x.result,route);assert.equal(r.result.disposition,mode==='restore'?'restored':mode==='current'?'current':'installed');}
  for(const route of ['typed','dynamic']){const pack=annotationNativeLoadPack(annotationNativeLoadDonor().z);pack.snapshot.records=[];pack.binding.identities=[];pack.raw=Array.from(Buffer.from(JSON.stringify(pack.snapshot)));const x=annotationLoadInvocationFixture(undefined,'restore',pack),r=capture(x,'lifecycle',x.result,route);assert.equal(r.result.status,'unchanged');assert.equal(r.result.disposition,'unchanged');}
  for(const kind of ['release-binding','release-annotations','release-provenance'])for(const route of ['typed','dynamic']){const x=lifecycleInvocationFixture(undefined,kind);capture(x,'lifecycle',x.read,route);}
 });
 check('closed-receipt-fields-identity-types-and-discriminators-refuse-ordinary-surprises',()=>{
  for(const {kind,receipt}of samples){
   expect({status:'result',result:Object.fromEntries(Object.entries(copy(receipt)).toReversed())},kind,true);
   const nullReceipt=Object.assign(Object.create(null),copy(receipt)),identityField=kind==='relation'?'target':'document';nullReceipt[identityField]=Object.assign(Object.create(null),nullReceipt[identityField]);expect(Object.assign(Object.create(null),{status:'result',result:nullReceipt}),kind,true);
   for(const key of Object.keys(receipt)){const r=copy(receipt);delete r[key];expect({status:'result',result:r},kind,false);}
   for(const r of [null,{},[],{...copy(receipt),extra:true},{...copy(receipt),invocation:''},{...copy(receipt),invocation:1},{...copy(receipt),status:'unknown'},{...copy(receipt),[kind==='relation'?'kind':'operation']:'unknown'}])expect({status:'result',result:r},kind,false);
   for(const value of [null,{},[],{...document,extra:true},{...document,authority:''},{...document,id:''},{...document,incarnation:-1},{...document,incarnation:1.5},{...document,incarnation:'1'},{...document,id:'\ud800'}])expect({status:'result',result:{...copy(receipt),[kind==='relation'?'target':'document']:value}},kind,false);
  }
  for(const operation of decl.document.operations)for(const status of ['applied','unchanged'])expect({status:'result',result:{invocation:'i',operation,status,document:copy(document)}},'lifecycle',true);
  for(const [activation,status]of Object.entries(decl.load.activation)){const r={invocation:'i',operation:'source-load',activation,status,document:copy(document)};expect({status:'result',result:r},'lifecycle',true);expect({status:'result',result:{...r,status:status==='applied'?'unchanged':'applied'}},'lifecycle',false);}
  for(const [phase,dispositions]of Object.entries(decl.annotations.phases))for(const [disposition,status]of Object.entries(dispositions)){const r={invocation:'i',operation:'annotation-load',phase,disposition,status,document:copy(document)};expect({status:'result',result:r},'lifecycle',true);for(const patch of [{phase:'unknown'},{disposition:'unknown'},{status:status==='applied'?'unchanged':'applied'},{phase:phase==='restore'?'adopt-baseline':'restore'}])expect({status:'result',result:{...r,...patch}},'lifecycle',false);}
  for(const kind of decl.relation.kinds)for(const status of decl.relation.statuses)expect({status:'result',result:{invocation:'i',kind,status,target:copy(document)}},'relation',true);
  expect({status:'result',result:{invocation:'copied-label',operation:'membership',status:'unchanged',document:{authority:'other-authority',id:'not-a-capability',incarnation:1}}},'lifecycle',true);
 });
 check('outer-results-accessors-prototypes-and-key-coercion-never-become-execution',()=>{
  for(const kind of ['lifecycle','relation']){expect({status:'unavailable'},kind,true);for(const value of [null,undefined,1,[],{}, {status:'unavailable',result:null},{status:'result'},{status:'result',result:null},{status:'result',result:{},extra:true}])expect(value,kind,false);}
  expect({status:'unavailable'},'unknown',false);
  for(const target of ['receipt','identity','outer']){const receipt={invocation:'i',operation:'source-load',activation:'created',status:'applied',document:copy(document)},outer={status:'result',result:receipt};let calls=0;const value=target==='receipt'?receipt:target==='identity'?receipt.document:outer,key=target==='receipt'?'activation':target==='identity'?'id':'status';Object.defineProperty(value,key,{enumerable:true,get(){calls++;throw Error('unexpected getter');}});expect(outer,'lifecycle',false);assert.equal(calls,0);}
  for(const patch of [{activation:{toString(){throw Error('coercion');}}},{activation:Symbol('activation')}])expect({status:'result',result:{invocation:'i',operation:'source-load',activation:'created',status:'applied',document:copy(document),...patch}},'lifecycle',false);
  for(const patch of [{phase:{toString(){throw Error('coercion');}}},{disposition:{toString(){throw Error('coercion');}}}])expect({status:'result',result:{invocation:'i',operation:'annotation-load',phase:'restore',disposition:'restored',status:'applied',document:copy(document),...patch}},'lifecycle',false);
  for(const value of [Object.assign(Object.create({}),{invocation:'i',operation:'membership',status:'unchanged',document:copy(document)}),Object.defineProperty({invocation:'i',operation:'membership',status:'unchanged',document:copy(document)},'extra',{value:1}),{invocation:'i',operation:'membership',status:'unchanged',document:copy(document),[Symbol('extra')]:1}])expect({status:'result',result:value},'lifecycle',false);
 });
 check('complete-release-envelope-boundaries-and-ceilings-remain-independent-of-shape',()=>{
  for(const kind of ['lifecycle','relation']){const {receipt}=samples.find(s=>s.kind===kind),release=kind==='lifecycle'?releaseLifecycleInvocationResult:releaseRelationInvocationResult,fields=kind==='lifecycle'?['lifecycleResultReadable','lifecycleResultContextCurrent','historicalDocumentReadable']:['relationResultReadable','relationResultContextCurrent','historicalTargetReadable'],g=Object.fromEntries(fields.map(k=>[k,true])),bytes=Buffer.byteLength(JSON.stringify({status:'result',result:receipt}));
   for(const delta of [-1,0,1]){assert.equal(release(receipt,true,g,bytes+delta).status,delta<0?'unavailable':'result');boundaries++;}
   assert.equal(release(receipt,false,g,bytes).status,'unavailable');assert.equal(release({...receipt,extra:true},true,g,bytes+100).status,'unavailable');releaseCases+=2;
   for(let mask=0;mask<8;mask++){assert.equal(release(receipt,true,Object.fromEntries(fields.map((k,i)=>[k,!!(mask&(1<<i))])),bytes).status,mask===7?'result':'unavailable');releaseCases++;}
  }
 });
 return {scope:'selected-native-lifecycle-closed-result-products',controls,nativeCases,denials,releaseCases,boundaries,shapeCases,structuralShapes:4,constructionAndReleaseShareValidator:true,shapeDoesNotAuthenticateOrSelectPublicQuery:true,fullF17PublicProductsOrPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkLifecycleResults()));
