// Selected retained-outcome receipt integration, not a new execution machine.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {pathToFileURL,fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
import * as kernel from './invocation-publication.mjs';
import * as annotations from './annotation-editing.mjs';
import * as reversals from './history-reversal.mjs';
import * as observation from './invocation-observation.mjs';
import {projectedEditResultValid} from './projected-edit-results.mjs';
import * as admission from './observation-admission.mjs';
import {normalizeInvocationObservationArguments,invocationObservationProducerRequest,sameInvocationObservationArguments,invocationReceiptEngineDescriptor} from './invocation-observation-arguments.mjs';
import * as discovery from './native-discovery.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {nativeLocalCommandFixture} from './native-local-command-checks.mjs';
import {atomicAnnotationResultFixture} from './atomic-annotation-result-checks.mjs';
import {lifecycleInvocationFixture} from './lifecycle-invocation-checks.mjs';
import {relationInvocationFixture} from './relation-invocation-checks.mjs';
import {registrationInvocationFixture} from './registration-invocation-checks.mjs';
import {scopedRegistrationInvocationFixture} from './registration-scoped-checks.mjs';
import {settingsCreatedResultFixture} from './settings-created-result-checks.mjs';
import {invocationObservationLimits as originalLimits,invocationObservationGates as baseGates} from './invocation-observation-checks.mjs';
import {observationAdmissionProfile,observationAdmissionLimits} from './observation-admission-checks.mjs';
const api={...kernel,...annotations,...reversals},copy=structuredClone,no={status:'unavailable'};
const context={authority:'a',engine:'native-membership-engine',recipient:'reader',purpose:'edit',destination:'client',profile:'execution',continuity:'c1'};
const policy={publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'};
const inputLimits={maxRequestBytes:8192,maxInputNodes:2048,maxInputDepth:32};
const limits={...originalLimits,maxHeldBytes:262144};
const ceiling=(enabled=true)=>Object.fromEntries(observation.invocationReceiptKinds().map(k=>[k,enabled]));
function fixture(kind='annotation',options={}){
 let z;
 switch(kind){
  case 'annotation':z=nativeLocalCommandFixture();break;
  case 'reversal':z=nativeLocalCommandFixture(undefined,'F14','undo');break;
  case 'atomic-annotation':z=atomicAnnotationResultFixture(api);{const job=z.prepare();assert(job);z.submit=()=>api.submitInvocation(z.domain,z.ticket,job,'typed',context,policy,z.g);}break;
  case 'relation':z=relationInvocationFixture();break;
  case 'lifecycle':z=lifecycleInvocationFixture();break;
  case 'host-registration':z=registrationInvocationFixture();break;
  case 'scoped-registration':z=scopedRegistrationInvocationFixture();break;
  case 'created-settings':z=settingsCreatedResultFixture(kernel);break;
 }
 const c=z.c??z.controlContext??context,p=z.policy??z.controlPolicy??policy;
 const native=typeof z.g==='function'?z.g():z.g??(typeof z.h==='function'?z.h():z.h);
 const g={...baseGates,...native,...(kind==='relation'?{relationResultReadable:true,relationResultContextCurrent:true,historicalTargetReadable:true}:kind==='lifecycle'?{lifecycleResultReadable:true,lifecycleResultContextCurrent:true,historicalDocumentReadable:true}:{}),capacityActivationAuthorized:true,operationReceiptAdmissionAllowed:true,operationReceiptAllowed:true};
 assert.equal(z.submit().status,'accepted');
 const handle=observation.openInvocationObservation(z.domain,z.ticket,c,p,options.limits??limits,g,options.ceiling===undefined?ceiling():options.ceiling);assert(handle,kind);
 const facet={kind,...(kind==='atomic-annotation'?{index:0}:{})};
 const request=(extra={})=>({contract:'execution-receipt/1',operation:'retained-outcome',invocation:z.reference,context:handle,profile:c.profile,facet:copy(facet),...extra});
 const query=(extra={},facts=g,scope=c,pol=p)=>observation.queryInvocationReceipt(handle,request(extra),scope,pol,facts);
 return {z,c,p,g,handle,facet,request,query};
}
export function checkInvocationReceipts(){
 assert.deepEqual(observation.invocationReceiptKinds(),['annotation','atomic-annotation','reversal','relation','lifecycle','host-registration','scoped-registration','created-settings']);
 const controls=[],check=(name,f)=>{f();controls.push(name);};let nativeCases=0,denials=0;
 check('all-eight-facets-use-original-native-receipts-without-running-again',()=>{
  for(const kind of observation.invocationReceiptKinds()){
   const x=fixture(kind);assert.deepEqual(x.query(),no);assert.equal(x.z.run().status,'settled');
   const root=x.z.root(),used=copy(kernel.invocationUsage(x.z.domain)),u=copy(observation.invocationObservationUsage(x.handle));
   const first=x.query();assert.equal(first.status,'operation-receipt',kind);assert.equal(first.receipt.result.invocation,x.z.reference);
   for(const route of ['typed','dynamic']){
    const envelope={route,request:{...(route==='typed'?{family:5}:{name:'inspect-execution'}),variant:'retained-outcome',version:1,arguments:x.request()}};
    const n=normalizeInvocationObservationArguments(envelope,x.c,inputLimits,inputLimits.maxRequestBytes,true);assert(n);assert.equal(n.context,x.handle);
    const value=observation.queryInvocationReceipt(x.handle,invocationObservationProducerRequest(n),x.c,x.p,x.g);assert.deepEqual(value,first);nativeCases++;
   }
   assert.equal(x.z.root(),root);assert.deepEqual(kernel.invocationUsage(x.z.domain),used);assert.deepEqual(observation.invocationObservationUsage(x.handle),u);
   if(kind==='created-settings')assert.equal(first.receipt.result.created[0].reference,x.z.detail().result.created[0].reference);
  }
 });
 check('old-contexts-and-false-facet-ceilings-cannot-gain-receipt-access',()=>{
  for(const cap of [null,ceiling(false)]){const x=fixture('annotation',{ceiling:cap});x.z.run();assert.deepEqual(x.query({}, {...x.g,details:true}),no);denials++;}
  const cap=ceiling(),x=fixture('annotation',{ceiling:cap});cap.annotation=false;x.z.run();assert.equal(x.query().status,'operation-receipt');
 });
 check('receipt-grant-is-independent-of-generic-summary-details',()=>{
  const x=fixture();x.z.run();assert.equal(x.p.details,false);assert.equal(x.query().status,'operation-receipt');
  for(const value of [false,undefined,1,'true']){assert.deepEqual(x.query({}, {...x.g,operationReceiptAllowed:value}),no);denials++;}
  const g={...x.g,annotationResultGrant:{...x.g.annotationResultGrant,identity:false,contentVersion:false,targetGeneration:false}};assert.deepEqual(x.query({},g),no);denials++;
 });
 check('wrong-original-context-invocation-facet-or-receipt-role-refuses',()=>{
  const x=fixture(),y=fixture();x.z.run();y.z.run();
  for(const patch of [{context:copy(x.handle)},{context:y.handle},{invocation:y.z.reference+'-foreign'},{contract:'execution-receipt/2'},{profile:'foreign'},{facet:{kind:'unknown'}},{facet:{kind:'annotation',index:0}},{facet:{kind:'atomic-annotation',index:0}},{extra:true}]){assert.deepEqual(x.query(patch),no);denials++;}
  for(const key of Object.keys(x.c)){assert.deepEqual(x.query({},x.g,{...x.c,[key]:'foreign'}),no);denials++;}
 });
 check('typed-dynamic-capture-is-equal-closed-and-does-not-evaluate-accessors',()=>{
  const x=fixture(),make=route=>({route,request:{...(route==='typed'?{family:5}:{name:'inspect-execution'}),variant:'retained-outcome',version:1,arguments:x.request()}});
  const a=normalizeInvocationObservationArguments(make('typed'),x.c,inputLimits,8192,true),b=normalizeInvocationObservationArguments(make('dynamic'),x.c,inputLimits,8192,true);assert(sameInvocationObservationArguments(a,b));
  let calls=0;const e=make('typed');Object.defineProperty(e.request.arguments.facet,'kind',{enumerable:true,get(){calls++;return 'annotation';}});assert.equal(normalizeInvocationObservationArguments(e,x.c,inputLimits,8192,true),null);assert.equal(calls,0);
  for(const facet of [{kind:'unknown'},{kind:'atomic-annotation',index:-1},{kind:'atomic-annotation',index:0,extra:true}]){const e=make('typed');e.request.arguments.facet=facet;assert.equal(normalizeInvocationObservationArguments(e,x.c,inputLimits,8192,true),null);denials++;}
 });
 check('pooled-decoder-reuses-prepaid-workspace-and-preserves-control',()=>{
  const x=fixture(),pool=admission.openObservationAdmission(x.z.domain,observationAdmissionProfile(),observationAdmissionLimits,x.g,true);assert(pool);
  const r=admission.admitPooledObservation(pool,x.z.ticket,x.c,x.p,limits,x.g,inputLimits,ceiling());assert.equal(r.status,'context');x.z.run();
  for(const route of ['typed','dynamic']){
   const argumentsValue=x.request({context:r.context}),envelope={route,request:{...(route==='typed'?{family:5}:{name:'inspect-execution'}),variant:'retained-outcome',version:1,arguments:argumentsValue}};
   const before=copy(admission.observationAdmissionUsage(pool));assert.equal(admission.queryDecodedPooledObservation(pool,r.reservation,envelope,x.c,x.p,x.g).status,'operation-receipt');assert.deepEqual(admission.observationAdmissionUsage(pool),before);
   const unavailable=admission.queryDecodedPooledObservation(pool,r.reservation,envelope,x.c,x.p,{...x.g,operationReceiptAllowed:false});assert.deepEqual(unavailable,no);
   const control={...envelope,request:{...envelope.request,variant:'minimal-control-status',arguments:{contract:'execution-observation/1',operation:'minimal-control-status',invocation:x.z.reference,context:r.context,profile:x.c.profile}}};assert.equal(admission.queryDecodedPooledObservation(pool,r.reservation,control,x.c,x.p,{...x.g,ordinaryReady:false,resourcesReady:false,controlReady:true}).status,'execution');
  }
 });
 check('receipt-query-borrow-blocks-reentry-and-retirement-without-changing-command',()=>{
  const x=fixture();x.z.run();let calls=0;const g={...x.g};Object.defineProperty(g,'operationReceiptAllowed',{get(){calls++;assert.deepEqual(x.query(),no);assert.deepEqual(observation.retireInvocationObservation(x.handle,x.g),no);return true;}});
  assert.equal(x.query({},g).status,'operation-receipt');assert.equal(calls,1);assert.equal(observation.retireInvocationObservation(x.handle,x.g).status,'retired');assert.deepEqual(x.query(),no);
 });
 check('forgetting-receipts-does-not-reexecute-or-resurrect-them',()=>{const x=fixture();x.z.run();const root=x.z.root(),starts=kernel.invocationUsage(x.z.domain).starts;assert.equal(x.query().status,'operation-receipt');assert.equal(kernel.forgetInvocationResult(x.z.domain,x.z.ticket,x.g).status,'forgotten');assert.deepEqual(x.query(),no);assert.equal(x.z.root(),root);assert.equal(kernel.invocationUsage(x.z.domain).starts,starts);});
 check('optional-receipt-fault-does-not-disable-summary-or-protected-control',()=>{
  const x=fixture();x.z.run();const root=x.z.root(),used=copy(kernel.invocationUsage(x.z.domain)),g={...x.g};Object.defineProperty(g,'operationReceiptAllowed',{get(){throw Error('selected receipt fault injection');}});
  assert.deepEqual(x.query({},g),no);assert.deepEqual(x.query(),no);
  const request=operation=>({contract:'execution-observation/1',operation,invocation:x.z.reference,context:x.handle,profile:x.c.profile});
  assert.equal(observation.queryInvocationObservation(x.handle,request('retained-outcome'),x.c,x.p,x.g).status,'outcome');
  assert.equal(observation.queryInvocationObservation(x.handle,request('minimal-control-status'),x.c,x.p,{...x.g,ordinaryReady:false,resourcesReady:false}).status,'execution');
  assert.equal(x.z.root(),root);assert.deepEqual(kernel.invocationUsage(x.z.domain),used);assert.equal(observation.retireInvocationObservation(x.handle,x.g).status,'retired');
 });
 check('recognized-observation-revocation-absorbs-regrant-without-cancelling-producer',()=>{const x=fixture();x.z.run();const root=x.z.root();assert.deepEqual(x.query({}, {...x.g,scope:false}),no);assert.deepEqual(x.query(),no);assert.equal(x.z.root(),root);assert.equal(observation.invocationObservationUsage(x.handle).live,false);});
 check('actual-accepted-registry-discovery-describes-receipt-facet-without-authorizing-it',()=>{
  const x=fixture(),template=nativeDiscoveryFixture(),c=kernel.invocationAuthorityContext(x.z.domain),scope={...c,purpose:'discover-contracts',policy:'receipt-policy',profile:'owned-contract-discovery/1'},g=template.g;
  const l={...template.l,maxContexts:1,maxPreparations:1,maxEntries:1,maxVariants:1,maxSelectors:8,maxPageRows:1,maxRootBytes:65536,maxPacketBytes:16384,maxCaptureBytes:16384,maxRowBytes:8192,maxReplyBytes:16384,maxMetadataBytes:8192,maxHeldBytes:131072};
  const owner=discovery.openNativeDiscoveryRegistry(x.z.domain,l,g());assert(owner);
  const {family,...descriptor}=invocationReceiptEngineDescriptor();assert.equal(family,'F06');assert(Object.isFrozen(descriptor.variants[0].requirements.schema));
  const rows=[{command:{authority:c.authority,id:'receipt-handler',incarnation:1},...descriptor}],target={identity:'receipt-target',basis:{}};
  const cut=()=>{const p=discovery.prepareNativeDiscoveryCut(owner,scope,rows,template.visibility(rows),target,template.advice(rows),g());assert(p);return p;};
  let p=cut();const context=discovery.acquireNativeDiscovery(owner,scope,p,g());assert.equal(context.status,'context');discovery.retireNativeDiscoveryCut(owner,p,g());
  const read=request=>{const p=cut();try{return discovery.queryNativeDiscovery(owner,{contract:'contract-discovery/1',profile:scope.profile,context:context.context,...request},scope,p,g());}finally{discovery.retireNativeDiscoveryCut(owner,p,g());}};
  const page=read({operation:'list',limit:1,continuation:null});assert.equal(page.status,'page');const value=read({operation:'inspect',entry:page.entries[0].reference});assert.equal(value.status,'descriptor');assert.deepEqual(value.descriptor.variants[0].input.fields,descriptor.variants[0].input.fields);
  x.z.run();assert.deepEqual(x.query({context:page.entries[0].reference}),no);assert.deepEqual(x.query({}, {...x.g,operationReceiptAllowed:false}),no);assert.equal(x.query().status,'operation-receipt');
 });
 return {scope:'selected-F06-retained-operation-receipts',controls,nativeCases,denials,facets:observation.invocationReceiptKinds().length,originalNativeReleaseAndOpaqueRolesChecked:true,newExecutionOrReceiptReferenceIssuer:false,fullFacadeShutdownOrPhysicalOrIndependentProof:false};
}
function checkWrongNativeOriginal(){
 const x=fixture();x.z.run();const root=x.z.root(),used=copy(kernel.invocationUsage(x.z.domain));
 const native=x.z.result();assert.equal(native.result.invocation,'wrong-native-original');assert(projectedEditResultValid(native,'F13'));
 assert.deepEqual(x.query(),no);assert.deepEqual(x.query(),no);
 const request=operation=>({contract:'execution-observation/1',operation,invocation:x.z.reference,context:x.handle,profile:x.c.profile});
 assert.equal(observation.queryInvocationObservation(x.handle,request('retained-outcome'),x.c,x.p,x.g).status,'outcome');
 assert.equal(observation.queryInvocationObservation(x.handle,request('minimal-control-status'),x.c,x.p,{...x.g,ordinaryReady:false,resourcesReady:false}).status,'execution');
 assert.equal(x.z.root(),root);assert.deepEqual(kernel.invocationUsage(x.z.domain),used);
 return {wrongNativeOriginalRefused:true,summaryAndProtectedControlPreserved:true};
}
export function checkInvocationReceiptMutations(){
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),scope=createCaptureScope(),mutations=[];
 const keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
 try{
  process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
  const run=(d,args=[])=>spawnSync(process.execPath,[path.join(d,'formal/invocation-receipt-checks.mjs'),...args],{encoding:'utf8',env:process.env,timeout:50000,maxBuffer:1048576});
  const original=captureInputs(root),positive=run(original.directory);assert.equal(positive.status,0,positive.stderr);assert(!positive.error&&!positive.signal);
  const rules=[
   ['captured-facet-ceiling-ignored','invocation-observation.mjs','r.ceiling[request.facet.kind]!==true','false'],
   ['current-receipt-grant-ignored','invocation-observation.mjs',"g?.operationReceiptAllowed!==true",'false'],
   ['original-observation-role-ignored','invocation-observation.mjs','request.context!==context','false'],
   ['receipt-reentry-admitted','invocation-observation.mjs','if(!r||r.busy||r.faulted)','if(!r||r.faulted)'],
   ['receipt-fault-disables-control','invocation-observation.mjs','catch{r.faulted=true;return unavailable();}','catch{return resultFault(e);}'],
   ['opaque-receipt-roles-cloned','invocation-observation.mjs','facet:copy(request.facet),receipt:value','facet:copy(request.facet),receipt:copy(value)'],
   ['pooled-receipt-dispatched-as-summary','observation-admission.mjs','queryInvocationReceipt(r.context,request,n.record.context,packet.value.policy,packet.value.facts)','queryInvocationObservation(r.context,request,n.record.context,packet.value.policy,packet.value.facts)']
  ];
  for(const [name,file,from,to]of rules){const graph=captureInputs(root);assert.deepEqual(graph.hashes,original.hashes);const target=path.join(graph.directory,'formal',file),raw=fs.readFileSync(target,'utf8');assert.equal(raw.split(from).length-1,1,name+': construction');fs.writeFileSync(target,raw.replace(from,to));const r=run(graph.directory);assert(!r.error&&!r.signal,name+': process failure');assert.equal(r.status,1,name+': '+r.stderr.slice(-1000));assert(r.stderr.includes('ERR_ASSERTION')&&!r.stderr.includes('SyntaxError'),name+': nonsemantic failure');mutations.push(name);}
  // Corrupt the released invocation and its dependent publication label after
  // the native producer's own checks, preserving the native structural invariant.
  // The same injected graph must pass before removing the facade correlation guard.
  const graph=captureInputs(root);assert.deepEqual(graph.hashes,original.hashes);
  const target=path.join(graph.directory,'formal/invocation-publication.mjs'),raw=fs.readFileSync(target,'utf8'),anchor='return releaseAnnotationResult(entry.annotationReceipt,entry.annotationCeiling,g,o.limits.maxResultBytes);';
  assert.equal(raw.split(anchor).length-1,1);fs.writeFileSync(target,raw.replace(anchor,"const released=releaseAnnotationResult(entry.annotationReceipt,entry.annotationCeiling,g,o.limits.maxResultBytes);return released.status==='unavailable'?released:{...released,result:{...released.result,invocation:'wrong-native-original',...(Object.hasOwn(released.result,'publication')?{publication:'wrong-native-original'}:{})}};"));
  const fault=run(graph.directory,['--wrong-native-original']);assert(!fault.error&&!fault.signal);assert.equal(fault.status,0,fault.stderr);assert.equal(JSON.parse(fault.stdout).wrongNativeOriginalRefused,true);
  const facade=path.join(graph.directory,'formal/invocation-observation.mjs'),source=fs.readFileSync(facade,'utf8'),guard="||value.status!=='unavailable'&&value.result.invocation!==e.reference";
  assert.equal(source.split(guard).length-1,1);fs.writeFileSync(facade,source.replace(guard,''));
  const mutant=run(graph.directory,['--wrong-native-original']);assert(!mutant.error&&!mutant.signal);assert.equal(mutant.status,1,mutant.stderr);assert(mutant.stderr.includes('ERR_ASSERTION')&&!mutant.stderr.includes('SyntaxError'));mutations.push('wrong-native-original-released');
  passed=true;return {scope:'selected-F06-retained-operation-receipt-sensitivity',baseline:JSON.parse(positive.stdout),mutations,wrongNativeOriginalFaultChecked:true,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,wholeFacadeOrIndependentProof:false};
 }finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(process.argv.includes('--mutations')?checkInvocationReceiptMutations():process.argv.includes('--wrong-native-original')?checkWrongNativeOriginal():checkInvocationReceipts()));
