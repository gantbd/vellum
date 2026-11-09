import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as reader from './resource-read.mjs';
import * as facade from './source-load-request.mjs';
import {nativeSourceReadApi,nativeSourceReadFixture} from './native-source-read-checks.mjs';
import {resourceReadLimits,resourceReadCodecLimits} from './resource-read-checks.mjs';
const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url))),family=families.find(f=>f.id==='F20'),copy=structuredClone,no={status:'unavailable'};
const keys=['authorized','scope','continuous','live','notStopped','readAllowed','retained','replyReady','transferReady','resourcesReady'];
function fixture(r=reader,f=facade,options={}){
 const api={...nativeSourceReadApi,...r},z=nativeSourceReadFixture(api,undefined,options),context=api.invocationAuthorityContext(z.domain);
 const grants=(more={})=>z.g({retained:true,replyReady:true,transferReady:true,...more});
 const envelope=(request,route)=>({[route==='typed'?'family':'name']:route==='typed'?families.indexOf(family):family.name,variant:'source-load-candidate',version:1,arguments:request});
 const invoke=(request,route='typed',c=context,g=grants())=>f.invokeSourceLoadRequest(z.owner,envelope(request,route),route,c,g);
 return {...z,context,grants,envelope,invoke,r:api,f};
}
export {fixture as sourceLoadRequestFixture};
export function checkSourceLoadRequest(r=reader,f=facade){
 const controls=[];let routes=0,denials=0;const check=(name,body)=>{body();controls.push(name);};
 check('private-request-and-scope-precede-current-fact-reads',()=>{
  for(const route of ['typed','dynamic','native'])for(const key of ['authorized','readAllowed'])for(const existing of [false,true]){
   const z=fixture(r,f),request=z.request(),original=copy(request),context=copy(z.context),scope=copy(context);
   const first=existing?z.invoke(original):null;if(first)assert.equal(first.status,'admitted');
   const g=z.grants();Object.defineProperty(g,key,{get(){request.origin.attempt='retargeted';context[Object.keys(context)[0]]='foreign-scope';return true;}});
   const a=route==='native'?z.r.requestInvocationSourceRead(z.owner,request,context,g):z.invoke(request,route,context,g);
   assert.equal(a.status,existing?'observe-existing':'admitted');if(first)assert.equal(a.task,first.task);
   const repeated=z.invoke(original,'typed',scope);assert.equal(repeated.status,'observe-existing');assert.equal(repeated.task,a.task);
   assert.deepEqual(z.invoke(request,'typed',scope),no);
  }
 });
 check('same-original-task-across-routes-and-native-phases',()=>{
  for(const route of ['typed','dynamic']){
   const z=fixture(r,f),request=z.request(),original=copy(request),root=z.root(),a=z.invoke(request,route);
   assert.equal(a.status,'admitted');assert.equal(z.r.resourceReadStatus(z.owner,a.task).phase,'ready');assert.equal(z.root(),root);
   const held=copy(z.usage());request.requiredRead='atomic-resource-snapshot';
   assert.deepEqual(z.invoke(request,route),no);assert.deepEqual(z.usage(),held);
   for(const other of ['typed','dynamic']){const q=z.invoke(original,other);assert.equal(q.status,'observe-existing');assert.equal(q.task,a.task);assert.deepEqual(z.usage(),held);routes++;}
   z.events.set(a.task,original);assert.equal(z.submit(a.task).status,'submitted');z.complete(a.task);
   const completed=copy(z.usage());assert.equal(z.invoke(original,route).task,a.task);assert.deepEqual(z.usage(),completed);
   assert.equal(z.r.controlResourceRead(z.owner,a.task,'cancel',{controlAllowed:true}).status,'stopped');
   assert.equal(z.invoke(original,route).task,a.task);assert(z.r.resourceReadStatus(z.owner,a.task).stopped);assert.equal(z.root(),root);
  }
 });
 check('whole-original-context-and-actual-owner-required',()=>{
  const z=fixture(r,f),request=z.request(),a=z.invoke(request);assert.equal(a.status,'admitted');
  for(const key of Object.keys(z.context))for(const value of [undefined,z.context[key]+'-foreign']){const usage=copy(z.usage());assert.deepEqual(z.invoke(request,'typed',{...z.context,[key]:value}),no);assert.deepEqual(z.usage(),usage);denials++;}
  for(const owner of [{},{...z.owner}]){assert.deepEqual(z.f.invokeSourceLoadRequest(owner,z.envelope(request,'typed'),'typed',z.context,z.grants()),no);denials++;}
 });
 check('owned-parent-prevents-attempt-relabel-and-raw-bypass',()=>{
  const z=fixture(r,f),request=z.request(),a=z.invoke(request);assert.equal(a.status,'admitted');const usage=copy(z.usage());
  for(const patch of [{attempt:'another-attempt'},{effect:'another-effect'}]){
   const changed={...request,origin:{...request.origin,...patch}};
   assert.deepEqual(z.invoke(changed,'dynamic'),no);
   assert.deepEqual(z.r.admitInvocationSourceRead(z.owner,changed,z.grants()),no);
   assert.deepEqual(z.usage(),usage);
  }
  assert.equal(z.r.controlResourceRead(z.owner,a.task,'cancel',{controlAllowed:true}).status,'stopped');
  assert.equal(z.r.retireInvocationSourceRead(z.owner,a.task,{retirementAllowed:true,retentionReleased:true,deliveryDrained:true}).status,'retired');
  const retired=copy(z.usage()),changed={...request,origin:{...request.origin,attempt:'after-retirement'}};
  assert.deepEqual(z.invoke(changed),no);assert.deepEqual(z.r.admitInvocationSourceRead(z.owner,changed,z.grants()),no);assert.deepEqual(z.usage(),retired);
 });
 check('raw-attempt-is-not-retroactive-parent-admission',()=>{
  const z=fixture(r,f),request=z.request(),a=z.r.admitInvocationSourceRead(z.owner,request,z.grants());assert.equal(a.status,'admitted');
  const changed={...request,origin:{...request.origin,attempt:'not-a-parent'}};
  assert.deepEqual(z.invoke(request),no);assert.deepEqual(z.invoke(changed),no);
  assert.equal(z.r.controlResourceRead(z.owner,a.task,'cancel',{controlAllowed:true}).status,'stopped');
  assert.equal(z.r.retireInvocationSourceRead(z.owner,a.task,{retirementAllowed:true,retentionReleased:true,deliveryDrained:true}).status,'retired');
  const retired=copy(z.usage());assert.deepEqual(z.invoke(changed),no);assert.deepEqual(z.usage(),retired);
 });
 check('complete-parent-capsule-prepaid-before-child-admission',()=>{
  const limits={...resourceReadLimits,maxCaptureBytes:5000},raw=fixture(r,f,{limits}),request=raw.request('b'.repeat(3000));
  assert.equal(raw.r.admitInvocationSourceRead(raw.owner,request,raw.grants()).status,'admitted');
  const owned=fixture(r,f,{limits}),before=copy(owned.usage());assert.deepEqual(owned.invoke(owned.request('b'.repeat(3000))),no);assert.deepEqual(owned.usage(),before);
 });
 check('current-release-facts-strict-for-first-and-matching-requests',()=>{
  for(const existing of [false,true]){const z=fixture(r,f),request=z.request();if(existing)assert.equal(z.invoke(request).status,'admitted');
   for(const key of keys)for(const value of [false,1,undefined]){const usage=copy(z.usage());assert.deepEqual(z.invoke(request,'dynamic',z.context,z.grants({[key]:value})),no);assert.deepEqual(z.usage(),usage);denials++;}
  }
 });
 check('changed-semantic-intent-cannot-recover-original',()=>{
  const z=fixture(r,f),request=z.request();assert.equal(z.invoke(request).status,'admitted');
  for(const patch of [{provider:{...request.provider,id:'other'}},{resource:{...request.resource,id:'other'}},{expectedRegistration:request.expectedRegistration+1},{requiredRead:'atomic-resource-snapshot'},{profile:'unknown'}]){const usage=copy(z.usage());assert.deepEqual(z.invoke({...request,...patch}),no);assert.deepEqual(z.usage(),usage);denials++;}
 });
 check('retired-identity-is-never-readmitted',()=>{
  const z=fixture(r,f,{limits:{...resourceReadLimits,maxTasks:1,maxIssued:1}}),request=z.request(),a=z.invoke(request);assert.equal(a.status,'admitted');
  assert.equal(z.r.controlResourceRead(z.owner,a.task,'cancel',{controlAllowed:true}).status,'stopped');
  assert.equal(z.r.retireInvocationSourceRead(z.owner,a.task,{retirementAllowed:true,retentionReleased:true,deliveryDrained:true}).status,'retired');
  const usage=copy(z.usage());assert.deepEqual(z.invoke(request),no);assert.deepEqual(z.invoke(z.request('fresh')),no);assert.deepEqual(z.usage(),usage);assert.equal(usage.reader.issued,1);
 });
 check('matching-retry-survives-ordinary-fence-without-relaunch',()=>{
  const z=fixture(r,f),request=z.request(),a=z.invoke(request);assert.equal(a.status,'admitted');
  assert(z.r.sealInvocationOrdinaryAdmission(z.domain,z.root(),z.context,{ownerControlAllowed:true,controlContextCurrent:true,controlScopeValid:true,protectedControlReady:true}));
  const usage=copy(z.usage()),g=z.grants({providerAvailable:false,launchAllowed:false,identityAllowed:false,bookkeepingReserved:false});
  const q=z.invoke(request,'dynamic',z.context,g);assert.equal(q.status,'observe-existing');assert.equal(q.task,a.task);
  assert.deepEqual(z.invoke(z.request('new')),no);assert.deepEqual(z.usage(),usage);assert.equal(z.r.resourceReadStatus(z.owner,a.task).phase,'ready');
 });
 check('lifetime-spent-origins-have-independent-parent-reservation',()=>{
  const limits={...resourceReadLimits,maxTasks:1,maxOutstanding:1,maxHeldBytes:350000},z=fixture(r,f,{limits});let retainedBytes=0;
  const reserved=()=>z.r.invocationOwnerAttachmentInventory(z.domain,z.context,{ownerControlAllowed:true}).entries.find(e=>e.owner===z.owner).additionalChildReservationBytes;
  assert.equal(reserved(),limits.maxHeldBytes+2*limits.maxIssued*limits.maxCaptureBytes);
  for(let i=0;i<limits.maxIssued;i++){
   const request=z.request(String(i).padStart(4,'0')+'x'.repeat(3000)),a=z.invoke(request);assert.equal(a.status,'admitted');
   assert.equal(z.r.controlResourceRead(z.owner,a.task,'cancel',{controlAllowed:true}).status,'stopped');
   assert.equal(z.r.retireInvocationSourceRead(z.owner,a.task,{retirementAllowed:true,retentionReleased:true,deliveryDrained:true}).status,'retired');
   retainedBytes+=Buffer.byteLength(JSON.stringify(Object.values(request.origin)));
  }
  assert(retainedBytes>limits.maxHeldBytes);assert(retainedBytes<reserved()-limits.maxHeldBytes);
  assert.equal(reserved(),limits.maxHeldBytes+2*limits.maxIssued*limits.maxCaptureBytes);
  assert.equal(z.r.resourceReadUsage(z.owner).heldBytes,0);assert.equal(z.r.resourceReadUsage(z.owner).issued,128);
 });
 check('invalid-and-overflowing-replay-bounds-refuse-without-parent-mutation',()=>{
  const z=fixture(r,f),usage=copy(z.usage()),inventory=z.r.invocationOwnerAttachmentInventory(z.domain,z.context,{ownerControlAllowed:true});
  const cases=[];for(const key of ['maxIssued','maxCaptureBytes','maxHeldBytes'])for(const value of [Symbol('invalid'),1n,NaN,Infinity,-1,undefined])cases.push({...resourceReadLimits,[key]:value});
  cases.push({...resourceReadLimits,maxIssued:Number.MAX_SAFE_INTEGER},{...resourceReadLimits,maxHeldBytes:Number.MAX_SAFE_INTEGER});
  for(const limits of cases){assert.doesNotThrow(()=>assert.equal(z.r.newInvocationSourceReadRegistry(z.domain,limits,resourceReadCodecLimits,z.profile,z.grants()),null));assert.deepEqual(z.usage(),usage);assert.deepEqual(z.r.invocationOwnerAttachmentInventory(z.domain,z.context,{ownerControlAllowed:true}),inventory);}
 });
 check('closed-product-and-catalog-route-refuse-before-admission',()=>{
  const z=fixture(r,f),request=z.request(),e=z.envelope(request,'typed');
  for(const [envelope,route]of [[{...e,version:2},'typed'],[{...e,variant:'annotation-load-candidate'},'typed'],[{...e,extra:true},'typed'],[{...e,family:0},'typed'],[e,'unknown'],[{...e,arguments:{...request,extra:true}},'typed'],[{...e,arguments:{...request,origin:{...request.origin,extra:true}}},'typed']]){const usage=copy(z.usage());assert.deepEqual(z.f.invokeSourceLoadRequest(z.owner,envelope,route,z.context,z.grants()),no);assert.deepEqual(z.usage(),usage);denials++;}
 });
 return {scope:'original-native-source-load-admission-and-retry-only',controls,routes,denials,actualNativeTaskAndScopeChecked:true,actualParentOperationAdmissionAndSingleAttemptPolicyChecked:true,firstAdmissionUsesExistingNativeFence:true,newIssuerOwnerOrExecutionMode:false,providerDispatchOrReceiptOrF19AssemblyProved:false,hostileDecoderPhysicalConcurrentOrIndependentProof:false};
}
function moduleUrl(raw,name,overrides={}){
 raw=raw.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,import.meta.url).href))
  .replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,import.meta.url).href)+',import.meta.url)');
 return 'data:text/javascript;base64,'+Buffer.from(raw).toString('base64')+'#'+name;
}
export async function checkSourceLoadRequestMutations(){
 const resource=fs.readFileSync(new URL('./resource-read.mjs',import.meta.url),'utf8'),front=fs.readFileSync(new URL('./source-load-request.mjs',import.meta.url),'utf8');
 // Reuse the existing native checker graph constructor, rather than mixing a
 // copied reader/writer with another module instance's private ownership maps.
 const checker=fs.readFileSync(new URL('./native-source-read-checks.mjs',import.meta.url),'utf8'),start=checker.indexOf('const load=async m=>{'),end=checker.indexOf(';assert.deepEqual(mutants.map',start);
 assert(start>=0&&end>start);const body=checker.slice(start,end);assert.equal(body.split('return {...original,').length,2);
 const names=['source-read-state.mjs','retained-lifecycle-state.mjs','invocation-publication.mjs','annotation-artifact.mjs','source-codec.mjs','resource-read.mjs','source-save.mjs','annotation-restoration.mjs','annotation-native-load.mjs','lifecycle-products.mjs','native-retained-lifecycle.mjs','native-source-read.mjs','native-source-destinations.mjs'];
 const originalFiles=Object.fromEntries(names.map(n=>[n,fs.readFileSync(new URL(n,import.meta.url),'utf8')]));
 const AsyncFunction=Object.getPrototypeOf(async function(){}).constructor;
 const graph=new AsyncFunction('files','names','original','url','assert',body.replace('return {...original,','return {readerUrl:r,...original,')+';return await load(null);');
 const edits=[
  ['load-private-input-capture-omitted','resource','request=copy(request);c=copy(c);',''],
  ['load-route-reads-grant-before-capture','front',"decoded.variant!=='source-load-candidate'||\n    planRoute(decoded,{schemaValid:true,enabled:true,grant:true,", "decoded.variant!=='source-load-candidate'||\n    planRoute(decoded,{schemaValid:true,enabled:true,grant:g?.readAllowed===true,"],
  ['original-scope-ignored','resource',[["if(!o?.nativeDomain||!equal(c,invocationAuthorityContext(o.nativeDomain))||\n",'if(!o?.nativeDomain||false||\n'],["!equal(t.context,c)",'false']]],
  ['current-release-not-strict','resource',"every(k=>g?.[k]===true)||\n    !requestValid(request,o.limits)","every(k=>g?.[k])||\n    !requestValid(request,o.limits)"],
  ['original-intent-ignored','resource','return equal(t.request,request)?','return true?'],
  ['original-task-copied','resource',"{status:'observe-existing',task}:unavailable()","{status:'observe-existing',task:{...task}}:unavailable()"],
  ['matching-retry-readmitted','resource','if(o.parentOperations.has(request.origin.operation)){','if(false){'],
  ['retirement-restores-eligibility','resource',"o.tasks.delete(task);return {status:'retired'};","o.tasks.delete(task);o.parentOperations?.delete(t.request.origin.operation);o.used.delete(originKey(t.request.origin));return {status:'retired'};"],
  ['matching-retry-submits-producer','resource',"return equal(t.request,request)?{status:'observe-existing',task}:unavailable();","{submitInvocationSourceRead(owner,task,g);return equal(t.request,request)?{status:'observe-existing',task}:unavailable();}"],
  ['wrong-variant-admitted','front',"decoded.variant!=='source-load-candidate'",'false'],
  ['spent-origin-parent-reservation-omitted','resource',"beginInvocationOwnerAttachment(domain,'source-read',units,","beginInvocationOwnerAttachment(domain,'source-read',limits?.maxHeldBytes,"],
  ['replay-bound-operands-not-validated','resource',"if(!natural(limits?.maxIssued)||!natural(limits?.maxCaptureBytes)||!natural(limits?.maxHeldBytes))return null;",''],
  ['raw-attempt-bypasses-owned-parent','resource','if(o.parentOperations?.has(r.origin.operation))return unavailable();',''],
  ['raw-history-adopted-as-parent','resource','[...o.used].some(key=>JSON.parse(key)[0]===r.origin.operation)','false'],
  ['parent-record-reservation-omitted','resource','lifetimeBytes=2*replayBytes','lifetimeBytes=replayBytes'],
  ['parent-capsule-bound-omitted','resource','if(parent&&size([r.origin.operation,parent])>o.limits.maxCaptureBytes)return unavailable();','']
 ];
 async function load(edit=null){
  const files={resource,front};if(edit)for(const [before,after]of Array.isArray(edit[2])?edit[2]:[[edit[2],edit[3]]]){assert.equal(files[edit[1]].split(before).length,2,edit[0]+': exact mutation anchor');files[edit[1]]=files[edit[1]].replace(before,after);}
  const tag=edit?.[0]??'source-load-baseline',api=await graph({...originalFiles,'resource-read.mjs':files.resource},names,nativeSourceReadApi,moduleUrl,assert),fu=moduleUrl(files.front,tag+'-facade',{'./resource-read.mjs':api.readerUrl});
  return {r:api,f:await import(fu)};
 }
 const baseline=await load();checkSourceLoadRequest(baseline.r,baseline.f);const killed=[];
 for(const edit of edits){const a=await load(edit);let caught=false;try{checkSourceLoadRequest(a.r,a.f);}catch(e){assert.equal(e.code,'ERR_ASSERTION',edit[0]+': '+String(e.message).slice(0,160));caught=true;}assert(caught,edit[0]);killed.push(edit[0]);}
 return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{
 const baseline=checkSourceLoadRequest();console.log(JSON.stringify(process.argv.includes('--reference-only')?baseline:{baseline,mutations:await checkSourceLoadRequestMutations(),copiedSelectedGraphBaselineChecked:true,constructionFailureCounted:false,wholeFacadeOrIndependentProof:false}));
}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
