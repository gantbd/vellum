import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as facade from './observation-stop-facade.mjs';
import * as kernel from './invocation-publication.mjs';
import * as publication from './native-publication-observation.mjs';
import {invocationPublicationFixture} from './invocation-publication-checks.mjs';
import {nativeSourceContextFixture} from './native-source-context-checks.mjs';
import {nativeAnnotationQueryFixture} from './native-annotation-query-checks.mjs';
import {nativeVersionSetQueryFixture} from './native-version-set-query-checks.mjs';
import {historyContextFixture,historyContextAcquire,historyContextGates,historyContextScope} from './history-context-checks.mjs';
import {engineSettingsReadFixture} from './engine-settings-read-checks.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {registrationReadFixture} from './registration-read-checks.mjs';
import {registrationEventFixture} from './registration-event-checks.mjs';
import * as registryEvents from './registration-events.mjs';
import {invocationObservationFixture,invocationObservationContext} from './invocation-observation-checks.mjs';
import {nativeOperationObservationFixture} from './native-operation-observation-checks.mjs';
const copy=structuredClone,no={status:'unavailable'},bounds={maxRequestBytes:32768,maxInputNodes:4096,maxInputDepth:32},kinds=facade.observationStopKinds();
const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url))),index=families.findIndex(f=>f.id==='F19');
const envelope=(context,route='typed')=>({route,request:{...(route==='typed'?{family:index}:{name:families[index].name}),variant:'stop-observation',version:1,arguments:{contract:'read-context/1',operation:'stop-observation',context}}});
function fixture(kind){
 let x,domain,owner,context,scope,usage,root;
 switch(kind){
  case 'source':x=nativeSourceContextFixture();domain=x.z.domain;owner=x.handle;context=x.acquire().context;scope=x.c;usage=x.usage;root=x.z.root;break;
  case 'annotation':x=nativeAnnotationQueryFixture();domain=x.x.z.domain;owner=x.handle;context=x.context;scope=x.x.scope;usage=x.usage;root=x.x.z.current;break;
  case 'version-set':x=nativeVersionSetQueryFixture();domain=x.z.x.z.domain;owner=x.z.handle;context=x.out.participants[0].snapshot;scope=Object.fromEntries(['recipient','purpose','destination','policy','continuity','profile'].map(k=>[k,x.z.x.host[k]]));usage=x.z.usage;root=x.z.x.z.root;break;
  case 'history':x=historyContextFixture();domain=x.domain;owner=x.history;context=historyContextAcquire(undefinedApi(),x).context;scope=historyContextScope;usage=()=>kernel.invocationUsage(domain);root=x.current;break;
  case 'settings':x=engineSettingsReadFixture();domain=x.domain;owner=x.pool;context=x.acquire().context;scope=x.c;usage=x.usage;root=x.root;break;
  case 'discovery':x=nativeDiscoveryFixture();domain=x.domain;owner=x.owner;context=x.open().context;scope=x.scope;usage=x.usage;root=x.z.root;break;
  case 'resource-catalog':x=registrationReadFixture();domain=x.domain;owner=x.owner;context=x.acquire().context;scope=x.scope;usage=x.usage;root=x.z.root;break;
  case 'registration-stream':x=registrationEventFixture();domain=x.domain;owner=x.owner;context=x.open().stream;scope=x.scope;usage=x.usage;root=x.z.root;break;
  case 'registration-receipt':{x=registrationEventFixture();domain=x.domain;owner=x.owner;scope=x.scope;const command=registryEvents.issueRegistrationObservationCommand(owner,'establish-bounded-stream',null,scope,x.g()),preparation=x.prepare();assert(command&&preparation);assert.equal(registryEvents.requestRegistrationObservationCommand(owner,command,'establish-bounded-stream',null,scope,preparation,x.g()).status,'command-settled');context=registryEvents.openRegistrationObservationReceipt(owner,command,scope,{kind:'registry-baseline',allowed:true},x.g({operationReceiptAdmissionAllowed:true}));usage=x.usage;root=x.z.root;break;}
  case 'publication-stream':{
   x=invocationPublicationFixture();domain=x.domain;const before=x.root();scope={authority:before.authority,recipient:'reader',purpose:'observe',destination:'client',policy:'p1',continuity:'c1',profile:'recipient-publication-observation-reference/2',documents:[{document:copy(before.documents[0].document),source:true,annotations:[],views:[],collection:null}],outcomePolicy:{publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'}};
   const limits={maxRootBytes:262144,maxDocuments:8,maxRows:64,maxViews:16,maxProjectionBytes:65536,maxReplyBytes:131072,maxQueue:4,maxOutcomes:8,maxOutcomeBytes:4096,maxIssued:128,maxHeldBytes:2097152,maxBindings:64,maxBindingBytes:65536};
   const g=Object.fromEntries(['authorized','scope','continuous','live','notStopped','projectionAllowed','resourcesReady','coverageComplete','releaseAllowed','retained','transferReady','baselineAllowed','outcomeCaptureAllowed','ownerControlAllowed'].map(k=>[k,true]));
   owner=publication.openInvocationPublicationObserver(domain,scope,limits,g).owner;context=owner;usage=()=>publication.invocationPublicationObserverUsage(owner);root=x.root;break;
  }
  case 'execution':x=invocationObservationFixture();assert.equal(x.admit().status,'accepted');domain=x.domain;owner=x.open();context=owner;scope=invocationObservationContext;usage=()=>kernel.invocationUsage(domain);root=x.current;break;
  case 'native-operation':x=nativeOperationObservationFixture('source-load');domain=x.domain;owner=x.h;context=owner;scope=x.c;usage=()=>kernel.invocationUsage(domain);root=()=>kernel.invocationAuthorityRoot(domain);break;
 }
 assert(domain&&owner&&context&&scope,kind);return {x,kind,domain,owner,context,scope,usage,root,authority:kernel.invocationAuthorityContext(domain)};
}
import * as histories from './history-contexts.mjs';
function undefinedApi(){return {...kernel,...histories};}
export function checkObservationStopFacade(api=facade){
 const controls=[];let deliveries=0,denials=0,resolvedDeliveries=0,resolvedDenials=0;const check=(name,fn)=>{fn();controls.push(name);};
 const call=(z,route='typed',patch={})=>api.dispatchBoundObservationStop(patch.domain??z.domain,patch.authority??z.authority,patch.target??{kind:z.kind,owner:z.owner},patch.envelope??envelope(z.context,route),patch.scope??z.scope,patch.g??{ownerControlAllowed:true},patch.bounds??bounds);
 check('twelve-fixed-native-kinds-both-routes-idempotent-protected-stop',()=>{assert.equal(api.observationStopKinds().length,12);for(const kind of kinds)for(const route of ['typed','dynamic']){const z=fixture(kind),before=z.root(),parent=copy(kernel.invocationUsage(z.domain));assert.equal(call(z,route,{g:{ownerControlAllowed:true,authorized:false,resourcesReady:false,ordinaryReady:false}}).status,'stopped',kind);assert.equal(call(z,route).status,'stopped',kind);assert.equal(z.root(),before);assert.deepEqual(kernel.invocationUsage(z.domain),parent);deliveries++;}});
 check('actual-domain-and-owner-not-equal-labels-or-copied-identities',()=>{for(const kind of kinds){const z=fixture(kind),other=fixture(kind),before=copy(z.usage());for(const patch of [{domain:other.domain,authority:other.authority},{target:{kind,owner:copy(z.owner)}},{target:{kind,owner:other.owner}},{authority:{...z.authority,purpose:'foreign'}}]){assert.deepEqual(call(z,'typed',patch),no,kind);assert.deepEqual(z.usage(),before);denials++;}assert.equal(call(z).status,'stopped');}});
 check('wrong-scope-and-role-refuse-without-poisoning-valid-context',()=>{for(const kind of kinds){const z=fixture(kind),before=copy(z.usage());for(const patch of [{scope:{...z.scope,purpose:'foreign'}},{envelope:envelope(copy(z.context))},{target:{kind:kinds[(kinds.indexOf(kind)+1)%kinds.length],owner:z.owner}}]){assert.deepEqual(call(z,'typed',patch),no,kind);assert.deepEqual(z.usage(),before);denials++;}assert.equal(call(z).status,'stopped');}});
 check('strict-current-control-and-closed-request-vocabulary',()=>{for(const kind of kinds){const z=fixture(kind),before=copy(z.usage());for(const value of [false,1,'true',null]){assert.deepEqual(call(z,'typed',{g:{ownerControlAllowed:value}}),no);denials++;}for(const mutate of [e=>e.request.arguments.extra=true,e=>e.request.arguments.operation='retire',e=>e.request.variant='revoke-observation-grant',e=>e.request.version=2]){const e=envelope(z.context);mutate(e);assert.deepEqual(call(z,'typed',{envelope:e}),no);denials++;}assert.deepEqual(z.usage(),before);assert.equal(call(z).status,'stopped');}});
 check('private-capture-precedes-facts-and-never-traverses-opaque-roles',()=>{for(const kind of kinds){const z=fixture(kind),scope=copy(z.scope),e=envelope(z.context),target={kind,owner:z.owner};let count=0;const g={get ownerControlAllowed(){count++;scope.purpose='mutated';e.request.arguments.context={};target.owner={};return true;}};assert.equal(call(z,'dynamic',{scope,envelope:e,target,g}).status,'stopped',kind);assert.equal(count,1);const y=fixture(kind),bad=envelope(y.context);let opaqueReads=0;const token=Object.freeze({get forbidden(){opaqueReads++;throw Error('opaque traversal');}});bad.request.arguments.context=token;assert.deepEqual(call(y,'typed',{envelope:bad}),no);assert.equal(opaqueReads,0);}});
 check('accessors-and-bounds-refuse-before-current-fact-suppliers',()=>{const z=fixture('source');let facts=0,getters=0;const g={get ownerControlAllowed(){facts++;return true;}},e=envelope(z.context);Object.defineProperty(e.request.arguments,'operation',{enumerable:true,get(){getters++;return 'stop-observation';}});assert.deepEqual(call(z,'typed',{envelope:e,g}),no);assert.deepEqual(call(z,'typed',{g,bounds:{...bounds,maxRequestBytes:1}}),no);assert.equal(facts,0);assert.equal(getters,0);assert.equal(call(z).status,'stopped');});
 check('history-stop-is-entry-local-not-whole-owner-cancellation',()=>{const z=fixture('history'),other=historyContextAcquire(undefinedApi(),z.x);assert.equal(call(z).status,'stopped');assert.equal(histories.closeHistoryInspection(z.owner,other.context,historyContextScope.recipient,historyContextGates).status,'stopped');});
 const resolved=(z,route='typed',patch={})=>api.dispatchObservationStop(patch.domain??z.domain,patch.authority??z.authority,patch.envelope??envelope(z.context,route),patch.scope??z.scope,patch.g??{ownerControlAllowed:true},patch.bounds??bounds);
 check('local-opaque-reference-resolves-eleven-classes-without-caller-owner',()=>{for(const kind of kinds)for(const route of ['typed','dynamic']){const z=fixture(kind),root=z.root(),parent=copy(kernel.invocationUsage(z.domain));assert.equal(resolved(z,route).status,'stopped',kind);assert.equal(resolved(z,route).status,'stopped',kind);assert.equal(z.root(),root);assert.deepEqual(kernel.invocationUsage(z.domain),parent);resolvedDeliveries++;}});
 check('local-resolution-rejects-foreign-copied-wrong-scope-and-extra-owner',()=>{for(const kind of kinds){const z=fixture(kind),other=fixture(kind),before=copy(z.usage()),extra=envelope(z.context);extra.request.arguments.owner=z.owner;for(const patch of [{domain:other.domain,authority:other.authority},{envelope:envelope(copy(z.context))},{scope:{...z.scope,purpose:'foreign'}},{g:{ownerControlAllowed:1}},{envelope:extra}]){assert.deepEqual(resolved(z,'typed',patch),no,kind);assert.deepEqual(z.usage(),before);resolvedDenials++;}assert.equal(resolved(z).status,'stopped');}});
 check('local-resolution-captures-before-facts-and-rejects-noncontext-aliases',()=>{for(const kind of kinds){const z=fixture(kind),scope=copy(z.scope),e=envelope(z.context);let facts=0;assert.equal(resolved(z,'typed',{scope,envelope:e,g:{get ownerControlAllowed(){facts++;scope.purpose='mutated';e.request.arguments.context={};return true;}}}).status,'stopped',kind);assert.equal(facts,1);}for(const kind of ['annotation','version-set']){const z=fixture(kind),page=kind==='annotation'?z.x.query():z.x.query(z.x.annotation()),alias=page.records[0].reference,before=copy(z.usage());assert.deepEqual(resolved(z,'dynamic',{envelope:envelope(alias,'dynamic')}),no);assert.deepEqual(z.usage(),before);resolvedDenials++;assert.equal(resolved(z).status,'stopped');}});
 return {scope:'closed-first-party-F19-stop-composition',controls,deliveries,denials,resolvedDeliveries,resolvedDenials,kinds:[...kinds],actualAttachmentAndOriginalScopeChecked:true,localOpaqueTargetResolutionChecked:true,newIssuerRegistryProducerCancellationOrPolicyMutation:false,hostileWirePhysicalConcurrentOrIndependentProof:false};
}
const mutants=[
 ['actual-attachment-ignored',"!invocationOwnerAttachmentMatches(domain,a,owner,attachmentKinds[kinds.indexOf(kind)])","false"],
 ['history-original-scope-ignored','historyInspectionClaimMatches(owner,c)?','true?'],
 ['truthy-control-fact','const allowed=g?.ownerControlAllowed===true','const allowed=Boolean(g?.ownerControlAllowed)'],
 ['scope-capture-ignored','c=captured.value.claimedScope','c=claimedScope'],
 ['request-extra-fields-ignored',"!exact(r,['contract','operation','context'])","!r"],
 ['execution-context-role-ignored',"case 'execution':result=r.context===owner?","case 'execution':result=true?"],
 ['local-lookup-control-not-strict','const permitted=g?.ownerControlAllowed===true','const permitted=Boolean(g?.ownerControlAllowed)'],
 ['local-lookup-original-scope-not-captured','data=captured.value,scope=data.claimedScope,authority=','data=captured.value,scope=claimedScope,authority='],
 ['local-lookup-matches-first-owner',"const kind=stopKind(domain,entry,reference)","const kind=entry.kind==='history'?'history':stopKind(domain,entry,reference)"],
];
export async function checkObservationStopMutations(){const raw=fs.readFileSync(new URL('./observation-stop-facade.mjs',import.meta.url),'utf8'),url=s=>'data:text/javascript;base64,'+Buffer.from(s.replace(/from '(\.\/[^']+)'/g,(_,p)=>`from ${JSON.stringify(new URL(p,import.meta.url).href)}`)).toString('base64');assert.deepEqual(checkObservationStopFacade(await import(url(raw))),checkObservationStopFacade());const names=[];for(const [name,from,to]of mutants){assert.equal(raw.split(from).length,2,name);const api=await import(url(raw.replace(from,to)));assert.throws(()=>checkObservationStopFacade(api),{code:'ERR_ASSERTION'},name);names.push(name);}return {baseline:checkObservationStopFacade(),mutations:names,copiedFacadeBaseline:true,constructionFailuresCounted:false};}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkObservationStopMutations()));}catch(e){console.error(String(e.message).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[data-module]'));process.exitCode=1;}
