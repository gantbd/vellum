import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import {invokeAnnotationLoadRequest} from './source-load-request.mjs';
import {annotationNativeLoadFixture,annotationNativeLoadApi,annotationNativeLoadLimits,annotationNativeLoadSourceEdit} from './annotation-native-load-checks.mjs';
import {annotationArtifactProfile} from './annotation-artifact-checks.mjs';
const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url))),family=families.find(f=>f.id==='F20'),copy=structuredClone,no={status:'unavailable'};
const release=['authorized','scope','continuous','live','notStopped','readAllowed','annotationReadable','retained','replyReady','transferReady','resourcesReady'];
function fixture(api=annotationNativeLoadApi,invoke=invokeAnnotationLoadRequest,options={}){
 const x=annotationNativeLoadFixture(api,undefined,options),domain=x.z.domain,context=api.invocationAuthorityContext(domain),g=(more={})=>x.g({transferReady:true,...more});
 const envelope=(request,route)=>({[route==='typed'?'family':'name']:route==='typed'?families.indexOf(family):family.name,variant:'annotation-load-candidate',version:1,arguments:request});
 return {...x,api,domain,context,g,envelope,invoke:(r,route='typed',c=context,h=g())=>invoke(x.owner,envelope(r,route),route,c,h),root:()=>x.z.current(),frame:()=>({read:api.invocationAnnotationReadUsage(x.owner),engine:api.invocationUsage(domain)})};
}
export {fixture as annotationLoadRequestFixture};
export function checkAnnotationLoadRequest(api=annotationNativeLoadApi,invoke=invokeAnnotationLoadRequest){
 const controls=[];let routeDeliveries=0,denials=0;const check=(n,f)=>{f();controls.push(n);};
 check('private-request-and-scope-precede-current-fact-reads',()=>{
  for(const route of ['typed','dynamic','native'])for(const key of ['authorized','readAllowed','annotationReadable'])for(const existing of [false,true]){
   const x=fixture(api,invoke),request=x.request(),original=copy(request),context=copy(x.context),scope=copy(context);
   const first=existing?x.invoke(original):null;if(first)assert.equal(first.status,'admitted');
   const g=x.g();Object.defineProperty(g,key,{get(){request.origin.attempt='retargeted';context[Object.keys(context)[0]]='foreign-scope';return true;}});
   const a=route==='native'?api.requestInvocationAnnotationRead(x.owner,request,context,g):x.invoke(request,route,context,g);
   assert.equal(a.status,existing?'observe-existing':'admitted');if(first)assert.equal(a.task,first.task);
   const repeated=x.invoke(original,'typed',scope);assert.equal(repeated.status,'observe-existing');assert.equal(repeated.task,a.task);
   assert.deepEqual(x.invoke(request,'typed',scope),no);
  }
 });
 check('both-routes-recover-original-task-after-real-source-change',()=>{
  for(const route of ['typed','dynamic']){
   const x=fixture(api,invoke),r=x.request(),original=copy(r),a=x.invoke(r,route);assert.equal(a.status,'admitted');
   const admitted=copy(x.frame());for(const other of ['typed','dynamic']){const q=x.invoke(original,other);assert.equal(q.status,'observe-existing');assert.equal(q.task,a.task);assert.equal(api.invocationAnnotationReadStatus(x.owner,a.task).phase,'ready');assert.deepEqual(x.frame(),admitted);routeDeliveries++;}
   assert.equal(x.submit(a.task).status,'submitted');x.complete(a.task);
   assert.equal(api.inspectInvocationAnnotationCandidate(x.owner,a.task,x.g()).status,'annotation-resource-candidate');
   annotationNativeLoadSourceEdit(api,x);assert.equal(api.inspectInvocationAnnotationCandidate(x.owner,a.task,x.g()).status,'unavailable');
   const root=x.root(),frame=copy(x.frame());
   for(const other of ['typed','dynamic']){const q=x.invoke(original,other);assert.equal(q.status,'observe-existing');assert.equal(q.task,a.task);assert.deepEqual(x.frame(),frame);routeDeliveries++;}
   assert.deepEqual(x.invoke(x.request(),route),no);assert.deepEqual(x.frame(),frame);assert.equal(x.root(),root);
   assert.equal(api.controlInvocationAnnotationRead(x.owner,a.task,'cancel',x.g()).status,'stopped');
   assert.equal(x.invoke(original,route).task,a.task);assert(api.invocationAnnotationReadStatus(x.owner,a.task).stopped);
  }
 });
 check('complete-original-scope-and-actual-native-owner-before-lookup',()=>{
  const x=fixture(api,invoke),r=x.request();assert.equal(x.invoke(r).status,'admitted');
  for(const key of Object.keys(x.context))for(const value of [undefined,x.context[key]+'-foreign']){const before=copy(x.frame());assert.deepEqual(x.invoke(r,'typed',{...x.context,[key]:value}),no);assert.deepEqual(x.frame(),before);denials++;}
  for(const owner of [{},{...x.owner}]){assert.deepEqual(invoke(owner,x.envelope(r,'typed'),'typed',x.context,x.g()),no);denials++;}
 });
 check('owned-parent-blocks-attempt-relabel-and-raw-bypass',()=>{
  const x=fixture(api,invoke),request=x.request(),a=x.invoke(request);assert.equal(a.status,'admitted');const frame=copy(x.frame());
  for(const patch of [{attempt:'another-attempt'},{effect:'another-effect'}]){
   const changed={...request,origin:{...request.origin,...patch}};assert.deepEqual(x.invoke(changed,'dynamic'),no);
   assert.deepEqual(api.admitInvocationAnnotationRead(x.owner,changed,x.g()),no);assert.deepEqual(x.frame(),frame);
  }
  assert.equal(api.controlInvocationAnnotationRead(x.owner,a.task,'cancel',x.g()).status,'stopped');
  assert.equal(api.retireInvocationAnnotationRead(x.owner,a.task,x.g()).status,'retired');
  const retired=copy(x.frame()),changed={...request,origin:{...request.origin,attempt:'after-retirement'}};
  assert.deepEqual(x.invoke(changed),no);assert.deepEqual(api.admitInvocationAnnotationRead(x.owner,changed,x.g()),no);assert.deepEqual(x.frame(),retired);
 });
 check('unowned-raw-task-does-not-become-a-parent-operation',()=>{
  const x=fixture(api,invoke),request=x.request(),a=api.admitInvocationAnnotationRead(x.owner,request,x.g());assert.equal(a.status,'admitted');
  const changed={...request,origin:{...request.origin,attempt:'not-a-parent'}};
  assert.deepEqual(x.invoke(request),no);assert.deepEqual(x.invoke(changed),no);
  assert.equal(api.controlInvocationAnnotationRead(x.owner,a.task,'cancel',x.g()).status,'stopped');
  assert.equal(api.retireInvocationAnnotationRead(x.owner,a.task,x.g()).status,'retired');
  const retired=copy(x.frame());assert.deepEqual(x.invoke(changed),no);assert.deepEqual(x.frame(),retired);
 });
 check('complete-parent-record-bound-before-child-publication',()=>{
  const limits={...annotationNativeLoadLimits,maxCaptureBytes:12000},raw=fixture(api,invoke,{readLimits:limits}),request=raw.request();request.origin.operation='b'.repeat(7000);
  assert.equal(api.admitInvocationAnnotationRead(raw.owner,request,raw.g()).status,'admitted');
  const owned=fixture(api,invoke,{readLimits:limits}),before=copy(owned.frame()),ownedRequest=owned.request();ownedRequest.origin.operation='b'.repeat(7000);assert.deepEqual(owned.invoke(ownedRequest),no);assert.deepEqual(owned.frame(),before);
 });
 check('strict-current-release-separate-from-fresh-admission',()=>{
  for(const existing of [false,true]){const x=fixture(api,invoke),r=x.request();if(existing)assert.equal(x.invoke(r).status,'admitted');
   for(const key of release)for(const value of [false,1,undefined]){const before=copy(x.frame());assert.deepEqual(x.invoke(r,'dynamic',x.context,x.g({[key]:value})),no);assert.deepEqual(x.frame(),before);denials++;}
  }
 });
 check('changed-intent-and-retired-origin-never-recover-or-readmit',()=>{
  const x=fixture(api,invoke),r=x.request(),a=x.invoke(r);assert.equal(a.status,'admitted');
  for(const key of ['expectedSource','expectedAnnotationSet','expectedPair','expectedBinding','expectedRegistration']){const before=copy(x.frame());assert.deepEqual(x.invoke({...r,[key]:r[key]+1}),no);assert.deepEqual(x.frame(),before);denials++;}
  assert.equal(api.controlInvocationAnnotationRead(x.owner,a.task,'cancel',x.g()).status,'stopped');
  assert.equal(api.retireInvocationAnnotationRead(x.owner,a.task,x.g()).status,'retired');const before=copy(x.frame());assert.deepEqual(x.invoke(r),no);assert.deepEqual(x.frame(),before);assert.equal(x.usage().issued,1);
 });
 check('ordinary-fence-retains-matching-lookup-not-new-admission',()=>{
  const x=fixture(api,invoke),r=x.request(),a=x.invoke(r);assert.equal(a.status,'admitted');
  assert(api.sealInvocationOrdinaryAdmission(x.domain,x.root(),x.context,{ownerControlAllowed:true,controlContextCurrent:true,controlScopeValid:true,protectedControlReady:true}));
  const before=copy(x.frame()),g=x.g({providerAvailable:false,launchAllowed:false,identityAllowed:false,bindingCoverageComplete:false});
  assert.equal(x.invoke(r,'dynamic',x.context,g).task,a.task);assert.deepEqual(x.invoke({...r,origin:{...r.origin,operation:'new-read'}}),no);assert.deepEqual(x.frame(),before);
 });
 check('retired-origin-history-remains-funded-in-actual-parent',()=>{
  const limits={...annotationNativeLoadLimits,maxTasks:1,maxOutstanding:1,maxIssued:128,maxHeldBytes:760000},x=fixture(api,invoke,{readLimits:limits});let bytes=0;
  const reserve=()=>api.invocationOwnerAttachmentInventory(x.domain,x.context,{ownerControlAllowed:true}).entries.find(e=>e.owner===x.owner).additionalChildReservationBytes;
  assert.equal(reserve(),limits.maxHeldBytes+2*limits.maxIssued*limits.maxCaptureBytes);
  for(let i=0;i<limits.maxIssued;i++){const r={...x.request(),origin:{operation:String(i).padStart(4,'0')+'x'.repeat(7000),effect:'read',attempt:'1'}},a=x.invoke(r);assert.equal(a.status,'admitted');assert.equal(api.controlInvocationAnnotationRead(x.owner,a.task,'cancel',x.g()).status,'stopped');assert.equal(api.retireInvocationAnnotationRead(x.owner,a.task,x.g()).status,'retired');bytes+=Buffer.byteLength(JSON.stringify(Object.values(r.origin)));}
  assert(bytes>limits.maxHeldBytes);assert(bytes<reserve()-limits.maxHeldBytes);assert.equal(x.usage().heldBytes,0);assert.equal(x.usage().issued,128);
 });
 check('malformed-and-overflowing-lifetime-bounds-refuse-cleanly',()=>{
  const x=fixture(api,invoke,{omitOwner:true}),before=copy(x.frame());const cases=[];
  for(const key of ['maxIssued','maxCaptureBytes','maxHeldBytes'])for(const value of [Symbol('invalid'),1n,NaN,Infinity,-1,undefined])cases.push({...annotationNativeLoadLimits,[key]:value});
  cases.push({...annotationNativeLoadLimits,maxIssued:Number.MAX_SAFE_INTEGER},{...annotationNativeLoadLimits,maxHeldBytes:Number.MAX_SAFE_INTEGER});
  for(const limits of cases){assert.doesNotThrow(()=>assert.equal(api.newInvocationAnnotationReadRegistry(x.domain,limits,annotationArtifactProfile.snapshot,x.g()),null));assert.deepEqual(x.frame(),before);}
 });
 check('closed-route-product-refuses-without-new-native-work',()=>{
  const x=fixture(api,invoke),r=x.request(),e=x.envelope(r,'typed');
  for(const [request,route]of [[{...e,version:2},'typed'],[{...e,variant:'source-load-candidate'},'typed'],[{...e,extra:true},'typed'],[{...e,family:0},'typed'],[e,'unknown'],[{...e,arguments:{...r,extra:true}},'typed'],[{...e,arguments:{...r,origin:{...r.origin,extra:true}}},'typed']]){const before=copy(x.frame());assert.deepEqual(invoke(x.owner,request,route,x.context,x.g()),no);assert.deepEqual(x.frame(),before);denials++;}
 });
 return {scope:'original-native-annotation-load-admission-and-retry-only',controls,routeDeliveries,denials,actualNativeTaskAndChangedSourceBasisChecked:true,actualParentOperationAdmissionAndSingleAttemptPolicyChecked:true,spentOriginLifetimeParentFundingChecked:true,newIssuerOwnerOrExecutionMode:false,candidateApplicabilityDispatchReceiptOrWholeFacadeProved:false,physicalConcurrentHostileDecoderOrIndependentProof:false};
}
function moduleUrl(raw,name,overrides={}){
 raw=raw.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,import.meta.url).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,import.meta.url).href)+',import.meta.url)');
 return 'data:text/javascript;base64,'+Buffer.from(raw).toString('base64')+'#'+name;
}
export async function checkAnnotationLoadRequestMutations(){
 const files=Object.fromEntries([['native','annotation-native-load.mjs'],['read','annotation-resource-read.mjs'],['front','source-load-request.mjs']].map(([key,file])=>[key,fs.readFileSync(new URL(file,import.meta.url),'utf8')]));
 const edits=[
  ['annotation-load-private-input-capture-omitted','native','request=copy(request);c=copy(c);',''],
  ['annotation-load-route-reads-grant-before-capture','front',"decoded.variant!=='annotation-load-candidate'||\n    planRoute(decoded,{schemaValid:true,enabled:true,grant:true,", "decoded.variant!=='annotation-load-candidate'||\n    planRoute(decoded,{schemaValid:true,enabled:true,grant:g?.annotationReadable===true&&g?.readAllowed===true,"],
  ['original-annotation-scope-ignored','native',[['if(!ledger||!equal(c,invocationAuthorityContext(o.domain))||\n','if(!ledger||false||\n'],['!equal(t.context,c)','false']]],
  ['annotation-current-release-not-strict','native','allowed=(g,keys)=>keys.every(k=>g?.[k]===true)','allowed=(g,keys)=>keys.every(k=>g?.[k])'],
  ['original-annotation-intent-ignored','native','return equal(t.request,request)?','return true?'],
  ['original-annotation-task-copied','native',"{status:'observe-existing',task}:unavailable()","{status:'observe-existing',task:{...task}}:unavailable()"],
  ['annotation-matching-retry-readmitted','native','if(ledger.parentOperations.has(request.origin.operation)){','if(false){'],
  ['annotation-retirement-restores-eligibility','read',"o.tasks.delete(task);return {status:'retired'};","o.tasks.delete(task);o.parentOperations?.delete(t.request.origin.operation);o.used.delete(originKey(t.request.origin));return {status:'retired'};"],
  ['annotation-spent-origin-parent-reservation-omitted','native',"beginInvocationOwnerAttachment(domain,'annotation-read',units,","beginInvocationOwnerAttachment(domain,'annotation-read',limits?.maxHeldBytes,"],
  ['annotation-replay-bound-operands-not-validated','native','if(!natural(limits?.maxIssued)||!natural(limits?.maxCaptureBytes)||!natural(limits?.maxHeldBytes))return null;',''],
  ['wrong-annotation-variant-admitted','front',"decoded.variant!=='annotation-load-candidate'",'false'],
  ['annotation-matching-retry-submits-producer','native',"return equal(t.request,request)?{status:'observe-existing',task}:unavailable();","{submitInvocationAnnotationRead(owner,task,g);return equal(t.request,request)?{status:'observe-existing',task}:unavailable();}"],
  ['raw-annotation-attempt-bypasses-parent','read','if(o.parentOperations?.has(r.origin.operation))return unavailable();',''],
  ['raw-annotation-history-adopted-as-parent','read','[...o.used].some(key=>JSON.parse(key)[0]===r.origin.operation)','false'],
  ['annotation-parent-record-reservation-omitted','native','lifetimeBytes=2*replayBytes','lifetimeBytes=replayBytes'],
  ['annotation-parent-capsule-bound-omitted','read','if(parent&&size([r.origin.operation,parent])>o.limits.maxCaptureBytes)return unavailable();','']
 ];
 async function load(edit=null){const raw={...files};if(edit)for(const [before,after]of Array.isArray(edit[2])?edit[2]:[[edit[2],edit[3]]]){assert.equal(raw[edit[1]].split(before).length,2,edit[0]+': exact anchor');raw[edit[1]]=raw[edit[1]].replace(before,after);}const tag=edit?.[0]??'annotation-request-baseline',ru=moduleUrl(raw.read,tag+'-read'),nu=moduleUrl(raw.native,tag+'-native',{'./annotation-resource-read.mjs':ru}),fu=moduleUrl(raw.front,tag+'-front',{'./annotation-native-load.mjs':nu});return {api:{...annotationNativeLoadApi,...await import(ru),...await import(nu)},invoke:(await import(fu)).invokeAnnotationLoadRequest};}
 const b=await load();checkAnnotationLoadRequest(b.api,b.invoke);const killed=[];
 for(const edit of edits){const a=await load(edit);let caught=false;try{checkAnnotationLoadRequest(a.api,a.invoke);}catch(e){assert.equal(e.code,'ERR_ASSERTION',edit[0]+': '+String(e.message).slice(0,160));caught=true;}assert(caught,edit[0]);killed.push(edit[0]);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{
 const baseline=checkAnnotationLoadRequest();console.log(JSON.stringify(process.argv.includes('--reference-only')?baseline:{baseline,mutations:await checkAnnotationLoadRequestMutations(),coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,wholeFacadeOrIndependentProof:false}));
}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
