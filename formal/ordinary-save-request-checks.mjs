import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import * as writer from './source-save.mjs';
import * as destinations from './native-source-destinations.mjs';
import * as facade from './ordinary-save-request.mjs';
import {nativeSourceReadApi} from './native-source-read-checks.mjs';
import {nativeSourceSaveFixture} from './native-source-save-checks.mjs';
import {ordinaryAnnotationSaveFixture,ordinaryAnnotationSaveApi} from './ordinary-annotation-save-checks.mjs';
import {annotationArtifactContext,annotationArtifactProfile} from './annotation-artifact-checks.mjs';
import {sourceSaveLimits,sourceSaveCodec} from './source-save-checks.mjs';
const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url))),family=families.find(f=>f.id==='F21'),copy=structuredClone,no={status:'unavailable'};
const envelope=(variant,args,route)=>({[route==='typed'?'family':'name']:route==='typed'?families.indexOf(family):family.name,variant,version:1,arguments:args});
function source(api={...nativeSourceReadApi,...kernel,...writer,...destinations},f=facade){
 const z=nativeSourceSaveFixture(api),context=api.invocationAuthorityContext(z.domain),g=(extra={})=>z.g({retained:true,transferReady:true,...extra});
 return {...z,api,context,g,args:()=>z.request(),invoke:(r,route='typed',c=context,h=g(),profile=z.profile)=>f.invokeOrdinarySourceSaveRequest(z.writer,z.domain,profile,envelope('captured-source-save',r,route),route,c,h)};
}
function annotation(api=ordinaryAnnotationSaveApi, f=facade){
 const z=ordinaryAnnotationSaveFixture(api),context=api.invocationAuthorityContext(z.z.domain),g=(extra={})=>z.g({authorized:true,scope:true,continuous:true,live:true,notStopped:true,writeAllowed:true,dispositionReadable:true,annotationReadable:true,retained:true,replyReady:true,transferReady:true,resourcesReady:true,...extra});
 return {...z,api,context,g,args:()=>({request:z.request(),context:copy(annotationArtifactContext),binding:z.z.binding()}),
  invoke:(a,route='typed',c=context,h=g(),pool=z.z.pool,profile=annotationArtifactProfile)=>f.invokeOrdinaryAnnotationSaveRequest(z.x.owner,z.z.domain,pool,profile,envelope('captured-annotation-save',a,route),route,c,h)};
}
export {source as sourceSaveRequestFixture,annotation as annotationSaveRequestFixture};
export function checkOrdinarySaveRequests(sourceApi={...nativeSourceReadApi,...kernel,...writer,...destinations},annotationApi=ordinaryAnnotationSaveApi,facadeApi=facade){
 const S=()=>source(sourceApi,facadeApi),A=()=>annotation(annotationApi,facadeApi);
 const controls=[];let routes=0,denials=0;const check=(name,f)=>{f();controls.push(name);};
 check('source-original-parent-and-child-across-both-routes',()=>{
  const z=S(),r=z.args(),before=z.root(),a=z.invoke(r);assert.equal(a.status,'admitted');assert.equal(z.root(),before);
  const u=copy(z.usage());for(const route of ['typed','dynamic']){const b=z.invoke(r,route);assert.equal(b.status,'observe-existing');assert.equal(b.task,a.task);assert.deepEqual(z.usage(),u);routes++;}
  assert.equal(z.api.inspectSourceSave(z.writer,a.task,z.g()).phase,'queued');assert.equal(z.submitted.size,0);
  z.edit();const edited=z.root(),after=copy(z.usage());assert.equal(z.invoke(r,'dynamic').task,a.task);assert.deepEqual(z.invoke(z.args()),no);assert.equal(z.root(),edited);assert.deepEqual(z.usage(),after);
 });
 check('annotation-original-parent-retains-full-serialization-association',()=>{
  const z=A(),args=z.args(),original=copy(args),root=z.z.current(),a=z.invoke(args);assert.equal(a.status,'admitted');assert.equal(z.z.current(),root);
  const u=copy(z.usage());for(const route of ['typed','dynamic']){const b=z.invoke(original,route);assert.equal(b.status,'observe-existing');assert.equal(b.task,a.task);assert.deepEqual(z.usage(),u);routes++;}
  args.binding={...args.binding,unexpected:'different-association'};assert.deepEqual(z.invoke(args),no);
  const changed=copy(original);changed.context.purpose+='-changed';assert.deepEqual(z.invoke(changed),no);assert.deepEqual(z.usage(),u);
  // Technical work profiles and replacement pool cannot recapture or migrate an
  // already-owned child. They are not semantic intent or authority substitutes.
  assert.equal(z.invoke(original,'dynamic',z.context,z.g(),{},{}).task,a.task);assert.deepEqual(z.usage(),u);
 });
 check('normalized-intent-and-scope-captured-before-current-fact-reads',()=>{
  for(const fact of ['authorized','writeAllowed'])for(const kind of ['source','annotation']){
   const z=kind==='source'?S():A(),args=z.args(),original=copy(args),context=copy(z.context),originalContext=copy(context);let changed=false;
   const g={...z.g(),get [fact](){if(!changed){changed=true;const r=kind==='source'?args:args.request;r.expectedSource++;context.recipient+='-changed';if(kind==='annotation')args.binding={different:'association'};}return true;}};
   const a=z.invoke(args,'typed',context,g);assert.equal(a.status,'admitted');assert(changed);
   assert.equal(z.invoke(original,'dynamic',originalContext,z.g()).task,a.task);assert.deepEqual(z.invoke(args,'typed',originalContext,z.g()),no);
  }
 });
 check('changed-effect-attempt-and-raw-path-cannot-add-a-child',()=>{
  for(const kind of ['source','annotation']){
   const z=kind==='source'?S():A(),args=z.args(),request=kind==='source'?args:args.request,a=z.invoke(args);assert.equal(a.status,'admitted');const u=copy(z.usage());
   for(const patch of [{attempt:'2'},{effect:'other'}]){
    const changed=copy(args),r=kind==='source'?changed:changed.request;r.origin={...r.origin,...patch};assert.deepEqual(z.invoke(changed),no);
    assert.deepEqual(kind==='source'?z.run('ordinary-admit',r,z.g()):z.admit(r,z.g()),no);
    if(kind==='source')assert.deepEqual(z.api.admitSourceSave(z.writer,z.root(),r,z.g()),no);
    assert.deepEqual(z.usage(),u);denials++;
   }
   assert.equal(request.origin.attempt,'1');
  }
 });
 check('raw-work-cannot-be-retroactively-adopted',()=>{
  const s=S(),sr=s.args();s.admit(sr,s.g());assert.deepEqual(s.invoke(sr),no);sr.origin.attempt='2';assert.deepEqual(s.invoke(sr),no);
  const a=A(),ar=a.args();assert.equal(a.admit(ar.request,a.g()).status,'admitted');assert.deepEqual(a.invoke(ar),no);ar.request.origin.attempt='2';assert.deepEqual(a.invoke(ar),no);
 });
 check('complete-original-scope-and-current-release-before-retry',()=>{
  for(const kind of ['source','annotation']){
   const z=kind==='source'?S():A(),r=z.args();assert.equal(z.invoke(r).status,'admitted');
   const keys=['authorized','scope','continuous','live','notStopped','writeAllowed','dispositionReadable','retained','replyReady','transferReady','resourcesReady',kind==='source'?'sourceReadable':'annotationReadable'];
   for(const key of keys)for(const value of [false,1,undefined]){const u=copy(z.usage());assert.deepEqual(z.invoke(r,'dynamic',z.context,z.g({[key]:value})),no);assert.deepEqual(z.usage(),u);denials++;}
   for(const key of Object.keys(z.context)){const u=copy(z.usage());assert.deepEqual(z.invoke(r,'typed',{...z.context,[key]:z.context[key]+'-foreign'}),no);assert.deepEqual(z.usage(),u);denials++;}
  }
 });
 check('matching-retained-child-survives-fence-without-launch-or-capture',()=>{
  for(const kind of ['source','annotation']){
   const z=kind==='source'?S():A(),r=z.args(),a=z.invoke(r),domain=kind==='source'?z.domain:z.z.domain,root=kind==='source'?z.root():z.z.current();
   assert(z.api.sealInvocationOrdinaryAdmission(domain,root,z.context,{ownerControlAllowed:true,controlContextCurrent:true,controlScopeValid:true,protectedControlReady:true}));
   const u=copy(z.usage()),q=z.invoke(r,'dynamic',z.context,z.g({providerAvailable:false,launchAllowed:false,preparationReserved:false}));assert.equal(q.task,a.task);assert.equal(q.status,'observe-existing');assert.deepEqual(z.usage(),u);
   const next=copy(r);(kind==='source'?next:next.request).origin.operation='new-after-fence';assert.deepEqual(z.invoke(next),no);assert.deepEqual(z.usage(),u);
  }
 });
 check('stop-and-retirement-preserve-spent-parent',()=>{
  for(const kind of ['source','annotation']){
   const z=kind==='source'?S():A(),args=z.args(),a=z.invoke(args),owner=kind==='source'?z.writer:z.x.owner,root=()=>kind==='source'?z.root():z.z.current();
   assert.equal(z.api.controlSourceSave(owner,a.task,'cancel',{controlAllowed:true}).status,'stopped');assert.equal(z.invoke(args).task,a.task);
   // Annotation reservation claims are actual retained duties, not a boolean
   // retentionReleased override. A retained claim may prevent retirement.
   const retired=z.api.retireSourceSave(owner,a.task,root(),{retirementAllowed:true,retentionReleased:true,deliveryDrained:true});
   if(kind==='source')assert.equal(retired.status,'retired');else assert.equal(retired.status,'unavailable');
   const changed=copy(args);(kind==='source'?changed:changed.request).origin.attempt='after-stop';assert.deepEqual(z.invoke(changed),no);
   if(retired.status==='retired'){assert.deepEqual(z.invoke(args),no);assert.deepEqual(z.api.admitSourceSave(owner,root(),changed,z.g()),no);}else{
    assert.equal(z.invoke(args).task,a.task);
    const claim=z.api.claimAnnotationWriteReservation(owner,a.task,z.g()).claim,reservation=z.api.readAnnotationWriteClaim(claim).link.reservation;
    assert.equal(z.z.release(reservation,z.g()).status,'released');
    assert.equal(z.api.retireSourceSave(owner,a.task,root(),{retirementAllowed:true,retentionReleased:true,deliveryDrained:true}).status,'retired');
    assert.deepEqual(z.invoke(args),no);assert.deepEqual(z.invoke(changed),no);assert.deepEqual(z.admit(changed.request,z.g()),no);
   }
  }
 });
 check('admission-and-matching-observation-do-not-attest-storage',()=>{
  const z=S(),r=z.args(),a=z.invoke(r);assert.equal(z.dispatch(a.task).status,'dispatched');const root=z.root(),u=copy(z.usage());
  assert.equal(z.invoke(r,'dynamic').task,a.task);const evidence=z.api.inspectSourceSave(z.writer,a.task,z.g());
  assert.equal(evidence.knowledge.application,'unknown');assert.equal(evidence.requestedStrengthSatisfied,false);assert.equal(z.root(),root);assert.deepEqual(z.usage(),u);
  assert.equal(z.written(a.task).status,'recorded');assert.equal(z.invoke(r).task,a.task);assert.equal(z.root(),root);
  assert.equal(z.api.inspectSourceSave(z.writer,a.task,z.g()).requestedStrengthSatisfied,true);
 });
 check('parent-record-capsule-refuses-before-any-admission-or-claim',()=>{
  for(const kind of ['source','annotation']){
   const z=kind==='source'?S():A(),args=z.args(),request=kind==='source'?args:args.request;request.origin.operation='p'.repeat(17000);
   const before=copy(z.usage());assert.deepEqual(z.invoke(args),no);assert.deepEqual(z.usage(),before);
   // The raw request fits: refusal above is the aggregate parent record, not
   // a malformed request, unavailable provider or a blanket large-input ban.
   assert.equal((kind==='source'?z.run('ordinary-admit',request,z.g()):z.admit(request,z.g())).status,'admitted');
  }
 });
 check('shared-native-writer-separately-funds-lifetime-parent-records',()=>{
  const z=S(),maximum=Number.MAX_SAFE_INTEGER,domain={kind:'invocation-domain/1'},lease={kind:'native-owner-attachment/1'},size=v=>Buffer.byteLength(JSON.stringify(v));
  const frame=size({sponsor:domain,lease,limits:sourceSaveLimits,codecLimits:sourceSaveCodec,charge:maximum}),faultKey=12*sourceSaveLimits.maxTagBytes+7;
  const expected=sourceSaveLimits.maxHeldBytes+sourceSaveCodec.maxHeldBytes+sourceSaveLimits.maxIssued*(2*sourceSaveLimits.maxCaptureBytes+faultKey)+frame;
  assert.equal(z.api.sourceWriterAttachmentUsage(z.writer).nodeBytes,expected);
  const args=z.args(),a=z.invoke(args);assert.equal(a.status,'admitted');assert.equal(z.api.controlSourceSave(z.writer,a.task,'cancel',{controlAllowed:true}).status,'stopped');
  assert.equal(z.api.retireSourceSave(z.writer,a.task,z.root(),{retirementAllowed:true,retentionReleased:true,deliveryDrained:true}).status,'retired');
  assert.equal(z.api.sourceSaveUsage(z.writer).heldBytes,0);assert.equal(z.api.sourceWriterAttachmentUsage(z.writer).nodeBytes,expected);assert.deepEqual(z.invoke(args),no);
 });
 return {scope:'owned-ordinary-save-parent-request-reference-only',controls,routes,denials,actualSourceAndAnnotationParentTaskBindingChecked:true,originalAnnotationBindingAndContextRetained:true,matchingDoesNotDispatchOrAttestPersistence:true,fullFacadeDiscoveryReceiptEventOrShutdownProved:false,hostPhysicalConcurrentCrashOrIndependentProof:false};
}
function moduleURL(raw,tag,overrides={}){
 raw=raw.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,import.meta.url).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,import.meta.url).href)+',import.meta.url)');
 return 'data:text/javascript;base64,'+Buffer.from(raw).toString('base64')+'#'+tag;
}
export async function checkOrdinarySaveRequestMutations(){
 const names=['source-destination-state.mjs','retained-lifecycle-state.mjs','source-read-state.mjs','invocation-publication.mjs','annotation-artifact.mjs','source-codec.mjs','resource-read.mjs','source-save.mjs','annotation-restoration.mjs','annotation-native-load.mjs','lifecycle-products.mjs','native-retained-lifecycle.mjs','native-source-read.mjs','native-source-destinations.mjs'];
 const originalFiles=Object.fromEntries(names.map(n=>[n,fs.readFileSync(new URL(n,import.meta.url),'utf8')]));
 const checker=fs.readFileSync(new URL('./native-source-save-checks.mjs',import.meta.url),'utf8'),start=checker.indexOf(' const load=async m=>'),end=checker.indexOf('\n checkNativeSourceSave(await load()',start);
 assert(start>=0&&end>start);const body=checker.slice(start,end);assert.equal(body.split('return Object.assign({},original,...mods);').length,2);
 const AsyncFunction=Object.getPrototypeOf(async function(){}).constructor;
 const graph=new AsyncFunction('files','names','original','moduleURL','assert',body.replace('return Object.assign({},original,...mods);','return {...Object.assign({},original,...mods),moduleUrls:urls};')+';return await load(null);');
 const edits=[
  ['save-original-scope-ignored','source-save.mjs',[['equal(c,invocationAuthorityContext(domain))','true'],['equal(parent.context,c)','true']]],
  ['save-current-release-truthy','source-save.mjs',[["kind==='ordinary-source-save'?'sourceReadable':'annotationReadable'].every(k=>g?.[k]===true)","kind==='ordinary-source-save'?'sourceReadable':'annotationReadable'].every(k=>g?.[k])"]]],
  ['save-original-intent-ignored','source-save.mjs',[['equal(parent.intent,intent)','true']]],
  ['save-original-child-copied','source-save.mjs',[["{status:'observe-existing',task:parent.task}","{status:'observe-existing',task:{...parent.task}}"]]],
  ['source-parent-not-installed','source-destination-state.mjs',[['if(p.plan.parentOperation)p.writer.parentOperations.set(p.plan.capture.request.origin.operation,{...copy(p.plan.parentOperation),domain:p.domain,task:p.task});','']]],
  ['annotation-parent-not-installed','source-save.mjs',[['if(parent)o.parentOperations.set(r.origin.operation,{...parent,domain,task});','']]],
  ['raw-source-attempt-bypasses-parent','source-save.mjs',[['if(!d||o.parentOperations.has(r.origin.operation))return null;','if(!d)return null;']]],
  ['raw-annotation-attempt-bypasses-parent','source-save.mjs',[['if(o.parentOperations.has(r.origin.operation))return fail();','']]],
  ['raw-history-retroactively-adopted','source-save.mjs',[['[...o.used].some(key=>JSON.parse(key)[0]===intent.request.origin.operation)','false']]],
  ['save-parent-capsule-bound-omitted','source-save.mjs',[["return size([intent.request.origin.operation,{...parent,domain:{kind:'invocation-domain/1'},task:{kind:'source-save-task/1'}}])<=o.limits.maxCaptureBytes?parent:null;","return parent;"]]],
  ['save-parent-record-unfunded','source-save.mjs',[['2*o.limits.maxCaptureBytes+faultKey','o.limits.maxCaptureBytes+faultKey']]],
  ['save-retirement-restores-parent-eligibility','source-save.mjs',[['if(parent?.task===task)o.parentOperations.set(t.capture.request.origin.operation,parent.retirement);','if(parent?.task===task)o.parentOperations.delete(t.capture.request.origin.operation);']]]
  ,['source-original-intent-not-captured','source-save.mjs',[['const intent=copy({request}),context=copy(c),prior=','const intent={request},context=c,prior=']]]
  ,['annotation-original-intent-not-captured','source-save.mjs',[['const intent=copy(input),scope=copy(operationContext),prior=','const intent=input,scope=operationContext,prior=']]]
  ,['outer-route-reads-grant-before-capture','ordinary-save-request.mjs',[['grant:true,current:true','grant:g?.writeAllowed===true,current:true']]]
 ];
 async function load(edit=null){
  const files={...originalFiles,'ordinary-save-request.mjs':fs.readFileSync(new URL('./ordinary-save-request.mjs',import.meta.url),'utf8')};if(edit)for(const [before,after]of edit[2]){assert.equal(files[edit[1]].split(before).length,2,edit[0]+': exact mutation anchor');files[edit[1]]=files[edit[1]].replace(before,after);}
  const api=await graph(files,names,nativeSourceReadApi,moduleURL,assert),url=moduleURL(files['ordinary-save-request.mjs'],edit?.[0]??'save-request-baseline',{'./source-save.mjs':api.moduleUrls['source-save.mjs'],'./native-source-destinations.mjs':api.moduleUrls['native-source-destinations.mjs']});
  return {api,f:await import(url)};
 }
 const baseline=await load();checkOrdinarySaveRequests(baseline.api,{...ordinaryAnnotationSaveApi,...baseline.api},baseline.f);
 const killed=[];for(const edit of edits){const a=await load(edit);let caught=false;try{checkOrdinarySaveRequests(a.api,{...ordinaryAnnotationSaveApi,...a.api},a.f);}catch(e){assert.equal(e.code,'ERR_ASSERTION',edit[0]+': '+String(e.message).slice(0,160));caught=true;}assert(caught,edit[0]);killed.push(edit[0]);}
 return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{const baseline=checkOrdinarySaveRequests();console.log(JSON.stringify(process.argv.includes('--reference-only')?baseline:{baseline,mutations:await checkOrdinarySaveRequestMutations(),coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,wholeFacadeOrIndependentProof:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
