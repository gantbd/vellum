import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {invocationPublicationFixture,invocationCompoundJob} from './invocation-publication-checks.mjs';
const copy=structuredClone,no={status:'unavailable'};
const profile={sourceProfile:{maxRootBytes:1024,maxInputBytes:1024,maxContentBytes:1024,maxReplyBytes:8192},maxContexts:2,maxHolds:2,maxIssued:16,maxRootBytes:262144,maxCaptureBytes:8192,maxContextBytes:2048,maxRequestBytes:8192,maxReplyBytes:8192,maxSelectorBytes:512,maxInputNodes:16384,maxInputDepth:64,maxHeldBytes:262144};
const good=Object.fromEntries(['authorized','scope','continuous','live','notStopped','sourceReadable','outsidePublicationGuard','resourcesReady','preparationReserved','retentionReady','retained','replyReady','transferReady','ownerControlAllowed','retentionReleased','deliveryDrained'].map(k=>[k,true]));
function fixture(api=kernel,patch={},state=undefined,parent=undefined){
 const z=invocationPublicationFixture(api,state,parent),p={...profile,...patch},document=z.root().documents[0].document,c={document,recipient:'reader',purpose:'query',destination:'client',policy:'policy-1',continuity:'c1',profile:'source-read'},prior=copy(z.usage()),handle=api.openInvocationSourceContexts(z.domain,p,good);assert(handle);
 const request={contract:'read-context/1',operation:'snapshot-handoff',document,selection:{kind:'capture-current'},data:['source'],profile:c.profile};
 const acquire=(r=request,claim=c,g=good)=>api.acquireInvocationSourceContext(z.domain,handle,r,claim,g);
 const query=(ref,change={},claim=c,g=good)=>api.queryInvocationSourceContext(z.domain,handle,{contract:'source-query/1',operation:'bounded-range',snapshot:ref,document,profile:c.profile,lo:0,hi:3,...change},claim,g);
 const stop=(ref,claim=c,g=good)=>api.stopInvocationSourceContext(z.domain,handle,{contract:'read-context/1',operation:'stop-observation',context:ref},claim,g);
 const usage=()=>api.invocationSourceContextUsage(z.domain);
 return {api,z,p,c,document,request,prior,handle,acquire,query,stop,usage};
}
export function checkNativeSourceContexts(api=kernel){const controls=[],check=(n,f)=>{f();controls.push(n);};let lossCases=0,unknownCases=0,transientCases=0;
 check('recursive-hold-discharge-cannot-decrement-a-retired-original-hold',()=>{const x=fixture(api),context=x.acquire().context,hold=api.holdInvocationSourceContext(x.z.domain,x.handle,context,good),g={...good};assert(hold);Object.defineProperty(g,'ownerControlAllowed',{get(){assert.equal(api.releaseInvocationSourceContextHold(x.z.domain,x.handle,hold,good),true);return true;}});assert.equal(api.releaseInvocationSourceContextHold(x.z.domain,x.handle,hold,g),false);assert.equal(x.usage().holds,0);assert.equal(x.stop(context).status,'stopped');assert.equal(api.retireInvocationSourceContext(x.z.domain,x.handle,context,good),true);assert.equal(api.retireInvocationSourceContexts(x.z.domain,x.handle,good),true);assert.equal(x.z.usage().declaredHeldBytes,x.prior.declaredHeldBytes);});
 check('actual-parent-charge-and-owned-native-source-query',()=>{
  const x=fixture(api),root=x.z.root(),u=copy(x.usage());assert.equal(x.z.usage().declaredHeldBytes-x.prior.declaredHeldBytes,u.declaredHeldBytes+parentFundedPoolAttachmentBytes(api,x.z.domain,x.handle));assert(u.declaredHeldBytes>0);
  const a=x.acquire();assert.equal(a.status,'context');assert.deepEqual(a.data,['source']);assert(Object.isFrozen(a.context));
  const r=x.query(a.context);assert.equal(r.status,'content');assert.equal(r.context,a.context);assert.deepEqual(r.source,[97,98,99]);assert.equal(r.ref.revision,1);
  assert.equal(x.query(a.context,{operation:'line',line:0,lo:undefined,hi:undefined}).status,'unavailable');
  const line=api.queryInvocationSourceContext(x.z.domain,x.handle,{contract:'source-query/1',operation:'line',snapshot:a.context,document:x.document,profile:x.c.profile,line:0},x.c,good);assert.equal(line.status,'line');assert.equal(line.context,a.context);assert.deepEqual(line.body.source,[97,98,99]);
  r.source[0]=0;assert.deepEqual(x.query(a.context).source,[97,98,99]);assert.equal(x.z.root(),root);assert.equal(x.usage().declaredHeldBytes,u.declaredHeldBytes);
 });
 check('new-native-source-publication-does-not-refresh-retained-capture',()=>{
  const x=fixture(api),old=x.acquire().context;assert.equal(x.z.submit(invocationCompoundJob(x.z.reference,x.z.root())).status,'accepted');assert.equal(x.z.run().status,'settled');assert.equal(x.z.root().documents[0].revision,2);
  assert.deepEqual(x.query(old).source,[97,98,99]);assert.equal(x.query(old).ref.revision,1);const fresh=x.acquire().context;assert.notEqual(fresh,old);assert.deepEqual(x.query(fresh).source,[100,98,99]);assert.equal(x.query(fresh).ref.revision,2);
 });
 check('cloned-foreign-handles-scopes-and-malformed-requests-cannot-poison',()=>{
  const x=fixture(api),a=x.acquire().context,before=copy(x.usage()),denying={...good,scope:false};
  for(const ref of [copy(a),{},null])assert.deepEqual(x.query(ref,{},x.c,denying),no);
  const other=fixture(api);assert.deepEqual(api.queryInvocationSourceContext(other.z.domain,x.handle,{contract:'source-query/1',operation:'bounded-range',snapshot:a,document:x.document,profile:x.c.profile,lo:0,hi:3},x.c,denying),no);
  for(const field of ['recipient','purpose','destination','policy','continuity','profile'])assert.deepEqual(x.query(a,{}, {...x.c,[field]:'foreign'},denying),no);
  for(const patch of [{extra:true},{contract:'wrong'},{operation:'execute'},{operation:{toString:1,valueOf:1}},{operation:null},{operation:[]},{lo:-1},{lo:2,hi:1},{lo:0,hi:4},{operation:'line',line:-1},{document:{...x.document,incarnation:2}},{profile:'foreign'}])assert.doesNotThrow(()=>assert.deepEqual(x.query(a,patch,x.c,denying),no));
  let called=0;const bad={contract:'source-query/1',snapshot:a,document:x.document,profile:x.c.profile,lo:0,hi:3};Object.defineProperty(bad,'operation',{enumerable:true,get(){called++;throw Error('getter');}});assert.doesNotThrow(()=>assert.deepEqual(api.queryInvocationSourceContext(x.z.domain,x.handle,bad,x.c,denying),no));assert.equal(called,0);
  assert.deepEqual(x.usage(),before);assert.equal(x.query(a).status,'content');
 });
 check('observed-bound-loss-is-absorbing-and-does-not-refund',()=>{
  for(const key of ['authorized','scope','continuous','live','retained','sourceReadable']){const x=fixture(api),a=x.acquire().context;assert(api.holdInvocationSourceContext(x.z.domain,x.handle,a,good));const before=copy(x.usage());assert.deepEqual(x.query(a,{},x.c,{...good,[key]:false}),no);assert.equal(x.usage().invalidated,1);assert.equal(x.usage().contexts,1);assert.equal(x.usage().issued,before.issued);assert.equal(x.usage().declaredHeldBytes,before.declaredHeldBytes);assert.deepEqual(x.query(a),no);lossCases++;}
 });
 check('unknown-facts-and-transient-resource-refusal-do-not-invalidate',()=>{
  for(const key of ['authorized','scope','continuous','live','retained','sourceReadable'])for(const value of [undefined,1]){const x=fixture(api),a=x.acquire().context,before=copy(x.usage());assert.deepEqual(x.query(a,{},x.c,{...good,[key]:value}),no);assert.deepEqual(x.usage(),before);assert.equal(x.query(a).status,'content');unknownCases++;}
  for(const key of ['resourcesReady','replyReady','transferReady','outsidePublicationGuard']){const x=fixture(api),a=x.acquire().context,before=copy(x.usage());assert.deepEqual(x.query(a,{},x.c,{...good,[key]:false}),no);assert.deepEqual(x.usage(),before);assert.equal(x.query(a).status,'content');transientCases++;}
 });
 check('all-acquisition-facts-are-strict-and-resource-refusal-does-not-mask-bound-loss',()=>{
  for(const field of ['authorized','scope','continuous','live','notStopped','sourceReadable','outsidePublicationGuard','resourcesReady','preparationReserved','retentionReady','replyReady','transferReady'])for(const value of [false,undefined,1]){const x=fixture(api),before=copy(x.usage());assert.deepEqual(x.acquire(x.request,x.c,{...good,[field]:value}),no);assert.deepEqual(x.usage(),before);}
  const x=fixture(api),a=x.acquire().context;assert.deepEqual(x.query(a,{},x.c,{...good,scope:false,resourcesReady:false,replyReady:false}),no);assert.equal(x.usage().invalidated,1);assert.deepEqual(x.query(a),no);
 });
 check('stop-with-held-root-requires-separate-protected-discharge',()=>{
  const x=fixture(api),a=x.acquire().context,hold=api.holdInvocationSourceContext(x.z.domain,x.handle,a,good);assert(hold);const before=copy(x.usage());
  assert.equal(x.stop(a,x.c,{ownerControlAllowed:true}).status,'stopped');assert.equal(x.usage().retainedRoots,1);assert.equal(api.retireInvocationSourceContext(x.z.domain,x.handle,a,good),false);assert.deepEqual(x.query(a),no);
  assert.equal(api.releaseInvocationSourceContextHold(x.z.domain,x.handle,copy(hold),good),false);
  for(const key of ['ownerControlAllowed','retentionReleased','deliveryDrained'])assert.equal(api.releaseInvocationSourceContextHold(x.z.domain,x.handle,hold,{...good,[key]:false}),false);
  assert.equal(api.releaseInvocationSourceContextHold(x.z.domain,x.handle,hold,good),true);assert.equal(x.usage().retainedRoots,0);assert.equal(api.releaseInvocationSourceContextHold(x.z.domain,x.handle,hold,good),false);
  assert.equal(api.retireInvocationSourceContext(x.z.domain,x.handle,a,good),true);assert.equal(x.usage().issued,before.issued);assert.equal(x.usage().declaredHeldBytes,before.declaredHeldBytes);
  assert.equal(api.retireInvocationSourceContexts(x.z.domain,x.handle,good),true);assert.equal(x.z.usage().declaredHeldBytes,x.prior.declaredHeldBytes);assert.equal(api.retireInvocationSourceContexts(x.z.domain,x.handle,good),false);assert.equal(api.openInvocationSourceContexts(x.z.domain,profile,good),null);
 });
 check('unsupported-acquisition-never-becomes-a-narrower-success',()=>{
  const x=fixture(api),before=copy(x.usage()),parent=copy(x.z.usage());for(const r of [{...x.request,data:['annotations']},{...x.request,data:['source','annotations']},{...x.request,selection:{kind:'retained',reference:'old'}},{...x.request,extra:true}]){assert.deepEqual(x.acquire(r),no);assert.deepEqual(x.usage(),before);assert.deepEqual(x.z.usage(),parent);}assert.equal(x.acquire().status,'context');
 });
 check('context-hold-and-lifetime-issuance-bounds-are-independent',()=>{
  {const x=fixture(api,{maxContexts:0});assert.deepEqual(x.acquire(),no);assert(x.usage().declaredHeldBytes>=2*profile.maxRequestBytes+profile.maxCaptureBytes+2*profile.maxContextBytes+2*profile.maxReplyBytes);}
  for(const patch of [{maxHolds:0},{maxIssued:1}]){const x=fixture(api,patch),a=x.acquire().context;assert(a);assert.equal(api.holdInvocationSourceContext(x.z.domain,x.handle,a,good),null);}
  const x=fixture(api,{maxContexts:1,maxHolds:1,maxIssued:2}),a=x.acquire().context;assert(a);assert.deepEqual(x.acquire(),no);const h=api.holdInvocationSourceContext(x.z.domain,x.handle,a,good);assert(h);assert.equal(api.holdInvocationSourceContext(x.z.domain,x.handle,a,good),null);x.stop(a);assert.equal(api.releaseInvocationSourceContextHold(x.z.domain,x.handle,h,good),true);assert.equal(api.retireInvocationSourceContext(x.z.domain,x.handle,a,good),true);assert.deepEqual(x.acquire(),no);assert.equal(x.usage().issued,2);
 });
 check('actual-parent-headroom-and-pool-envelope-refuse-before-charge',()=>{
  {const z=invocationPublicationFixture(api),nested={...profile.sourceProfile},before=copy(z.usage());let called=0;Object.defineProperty(nested,'maxRootBytes',{enumerable:true,get(){called++;throw Error('profile getter');}});assert.doesNotThrow(()=>assert.equal(api.openInvocationSourceContexts(z.domain,{...profile,sourceProfile:nested},good),null));assert.equal(called,0);assert.deepEqual(z.usage(),before);}
  for(const patch of [{maxContexts:100,maxHeldBytes:100000000},{maxHeldBytes:0},{maxInputNodes:0},{maxInputDepth:0},{extra:true}]){const z=invocationPublicationFixture(api),before=copy(z.usage());assert.equal(api.openInvocationSourceContexts(z.domain,{...profile,...patch},good),null);assert.deepEqual(z.usage(),before);assert.equal(api.invocationSourceContextUsage(z.domain),null);}
 });
 check('capture-root-scope-selector-and-reply-bounds-refuse-without-issuance',()=>{
  for(const patch of [{maxRootBytes:0},{maxCaptureBytes:0},{maxContextBytes:0},{maxSelectorBytes:0},{maxReplyBytes:0},{sourceProfile:{...profile.sourceProfile,maxRootBytes:0}},{sourceProfile:{...profile.sourceProfile,maxInputBytes:0}},{sourceProfile:{...profile.sourceProfile,maxReplyBytes:0}}]){const x=fixture(api,patch),before=copy(x.usage());assert.deepEqual(x.acquire(),no);assert.deepEqual(x.usage(),before);}
 });
 check('actual-owner-fault-invalidates-bound-delivery-but-not-protected-discharge',()=>{
  const x=fixture(api),a=x.acquire().context;
  const bad={kind:'compound',plan:{contract:'owned-annotation-plan/1',invocation:x.z.reference,descriptor:'fault',sources:[],annotations:[{kind:'body',document:null,id:'note',incarnation:1,expectedBody:1,body:'invalid'}]}};
  assert.equal(x.z.submit(bad).status,'accepted');assert.equal(x.z.run().status,'faulted');assert.equal(x.z.usage().faulted,true);assert.deepEqual(x.query(a),no);assert.equal(x.usage().invalidated,1);assert.deepEqual(x.acquire(),no);assert.equal(x.stop(a,x.c,{ownerControlAllowed:true}).status,'stopped');assert.equal(api.retireInvocationSourceContext(x.z.domain,x.handle,a,good),true);assert.equal(api.retireInvocationSourceContexts(x.z.domain,x.handle,good),true);
 });
 check('empty-unicode-mixed-endings-and-invalid-boundaries-use-native-source-products',()=>{
  for(const text of ['', 'é\r\nprose\n```code```\r', '\ufeffa']){const seed=invocationPublicationFixture(api),state=copy(seed.root());state.documents[0].source=Array.from(Buffer.from(text));const x=fixture(api,{},state),a=x.acquire().context;assert(a);assert.deepEqual(x.query(a,{hi:state.documents[0].source.length}).source,state.documents[0].source);}
  const seed=invocationPublicationFixture(api),state=copy(seed.root());state.documents[0].source=[0xc3,0xa9];const x=fixture(api,{},state),a=x.acquire().context,before=copy(x.usage());assert(a);assert.deepEqual(x.query(a,{lo:1,hi:2},x.c,{...good,scope:false}),no);assert.deepEqual(x.usage(),before);assert.deepEqual(x.query(a,{lo:0,hi:2}).source,[0xc3,0xa9]);
 });
 return {scope:'isolated-native-current-source-context-owner',controls,lossCases,unknownCases,transientCases,actualParentChargeAndSourceCaptureChecked:true,nativeF05ResultLifetimeOrAllF02ProfilesProved:false,actualPolicyPhysicalConcurrentCrashPublicOrIndependentTruthProved:false};}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeSourceContexts()));
export {profile as nativeSourceContextProfile,good as nativeSourceContextFacts,fixture as nativeSourceContextFixture};
