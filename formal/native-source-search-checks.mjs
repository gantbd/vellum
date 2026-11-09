import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {nativeSourceContextFixture,nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
import {invocationCompoundJob} from './invocation-publication-checks.mjs';
const profile={limits:{maxRootBytes:1024,maxScopes:8,maxInputBytes:1024,maxQueryBytes:128,maxComparisons:8192,maxMatches:8,maxReplyBytes:4096,maxReplacementBytes:1024,maxEditBytes:8192,chunkBytes:1},maxResults:2,maxIssued:8,maxRequestBytes:8192,maxCaptureBytes:4096,maxSelectorBytes:256,maxInputNodes:16384,maxInputDepth:64,maxHeldBytes:262144};
const clone=structuredClone,flags=['grant','scope','continuity','capacity','retained','notStopped'];
function current(r){return {request:{...r,ref:{...r.ref},scopes:r.scopes.map(s=>({...s})),navigation:r.navigation===null?null:{...r.navigation}},...Object.fromEntries(flags.map(k=>[k,true]))};}
function fixture(api=kernel,patch={},parent=undefined){
 const x=nativeSourceContextFixture(api,{},undefined,parent),reference=x.acquire().context;assert(reference);const before=x.z.usage(),p={...profile,...patch},handle=api.openInvocationSourceSearch(x.z.domain,x.handle,p,facts);assert(handle);
 const request=(operation='literal-matches')=>({contract:'literal-search/1',operation,context:reference,ref:{authority:'a',document:'A',incarnation:1,revision:1},query:'a',profile:'literal-exact/1',scopes:[{lo:0,hi:3}],generation:1,navigation:operation==='literal-matches'?null:{start:operation==='next-match'?0:3,current:null,wrap:false}});
 const query=(r=request(),q=current(r),c=x.c,g=facts)=>api.queryInvocationSourceSearch(x.z.domain,handle,r,c,q,g);
 const release=(result,q,c=x.c,g=facts)=>api.releaseInvocationSourceSearchResult(x.z.domain,handle,result,c,q,g);
 const retire=(result,g=facts)=>api.retireInvocationSourceSearchResult(x.z.domain,handle,result,g);
 const usage=()=>api.invocationSourceSearchUsage(x.z.domain);return {api,x,reference,p,handle,before,request,query,release,retire,usage};
}
export function checkNativeSourceSearch(api=kernel){const controls=[],check=(name,f)=>{f();controls.push(name);};let nativeCases=0,denialCases=0;
 check('actual-source-and-parent-reservation-support-all-native-query-variants',()=>{
  for(const operation of ['literal-matches','next-match','previous-match']){const z=fixture(api),r=z.request(operation),q=current(r),root=z.x.z.root(),out=z.query(r,q);assert.equal(out.status,'complete');assert.equal(out.context,z.reference);assert.equal(out.total,1);assert.equal(z.release(out,q),out);assert.equal(z.usage().results,1);assert.equal(z.x.usage().holds,1);assert.equal(z.x.z.usage().declaredHeldBytes-z.before.declaredHeldBytes,z.usage().declaredHeldBytes+parentFundedPoolAttachmentBytes(api,z.x.z.domain,z.handle));assert.equal(z.x.z.root(),root);nativeCases++;}
 });
 check('separate-source-and-query-facts-are-exact-true-without-grant-promotion',()=>{
  for(const field of flags)for(const value of [false,undefined,1]){const z=fixture(api),r=z.request(),q={...current(r),[field]:value},before=z.x.usage();assert.equal(z.query(r,q).status,'unavailable');assert.equal(z.usage().results,0);assert.equal(z.x.usage().holds,0);assert.equal(z.x.usage().issued,before.issued);denialCases++;}
  const z=fixture(api),r=z.request();assert.equal(z.query(r,current(r),z.x.c,{...facts,sourceReadable:false}).status,'unavailable');assert.equal(z.x.usage().invalidated,1);
 });
 check('source-stop-retains-child-until-protected-result-retirement',()=>{
  const z=fixture(api),r=z.request(),q=current(r),out=z.query(r,q),before=z.usage(),source=z.x.usage();assert.equal(out.status,'complete');assert.equal(api.retireInvocationSourceSearch(z.x.z.domain,z.handle,facts),false);assert.equal(z.x.stop(z.reference,z.x.c,{ownerControlAllowed:true}).status,'stopped');assert.equal(z.release(out,q).status,'unavailable');assert.equal(z.x.usage().retainedRoots,1);assert.equal(api.retireInvocationSourceContext(z.x.z.domain,z.x.handle,z.reference,facts),false);
  for(const field of ['ownerControlAllowed','retentionReleased','deliveryDrained'])assert.equal(z.retire(out,{...facts,[field]:false}),false);
  assert.equal(z.retire(clone(out)),false);assert.equal(z.retire(out),true);assert.equal(z.retire(out),false);assert.equal(z.x.usage().holds,0);assert.equal(z.x.usage().retainedRoots,0);assert.equal(z.x.usage().issued,source.issued);assert.equal(z.usage().issued,before.issued);assert.equal(z.usage().declaredHeldBytes,before.declaredHeldBytes);
  assert.equal(api.retireInvocationSourceContext(z.x.z.domain,z.x.handle,z.reference,facts),true);assert.equal(api.retireInvocationSourceContexts(z.x.z.domain,z.x.handle,facts),true);assert.equal(api.retireInvocationSourceSearch(z.x.z.domain,z.handle,facts),true);assert.equal(z.x.z.usage().declaredHeldBytes,z.x.prior.declaredHeldBytes);
 });
 check('query-loss-is-absorbing-without-closing-independent-source-read',()=>{
  for(const field of ['grant','scope','continuity','retained','notStopped']){const z=fixture(api),r=z.request(),q=current(r),out=z.query(r,q),before=z.x.usage();assert.equal(z.release(out,{...q,[field]:false}).status,'unavailable');assert.equal(z.usage().invalidated,1);assert.equal(z.release(out,q).status,'unavailable');assert.deepEqual(z.x.usage(),before);assert.equal(z.x.query(z.reference).status,'content');}
 });
 check('unknown-and-transient-query-facts-do-not-invalidate',()=>{
  for(const field of flags)for(const value of field==='capacity'?[false,undefined,1]:[undefined,1]){const z=fixture(api),r=z.request(),q=current(r),out=z.query(r,q),before=z.usage();assert.equal(z.release(out,{...q,[field]:value}).status,'unavailable');assert.deepEqual(z.usage(),before);assert.equal(z.release(out,q),out);}
 });
 check('foreign-results-context-copies-scopes-and-changed-current-requests-do-not-poison',()=>{
  const z=fixture(api),r=z.request(),q=current(r),out=z.query(r,q),before=z.usage();assert.equal(z.release(clone(out),q).status,'unavailable');assert.equal(z.release(out,{...q,request:{...q.request,context:clone(z.reference)},scope:false}).status,'unavailable');assert.equal(z.release(out,{...q,request:{...q.request,generation:2},scope:false}).status,'unavailable');assert.equal(z.release(out,{...q,scope:false},{...z.x.c,recipient:'foreign'}).status,'unavailable');assert.deepEqual(z.usage(),before);assert.equal(z.release(out,q),out);
  assert.equal(z.query({...r,context:clone(z.reference)},current({...r,context:clone(z.reference)})).status,'unavailable');
 });
 check('result-count-and-spent-issuance-are-separate',()=>{
  const z=fixture(api,{maxResults:1,maxIssued:1}),r=z.request(),q=current(r),out=z.query(r,q);assert.equal(out.status,'complete');assert.equal(z.query(r,q).status,'unavailable');assert.equal(z.retire(out),true);assert.equal(z.usage().results,0);assert.equal(z.usage().issued,1);assert.equal(z.query(r,q).status,'unavailable');assert.equal(api.retireInvocationSourceSearch(z.x.z.domain,z.handle,facts),true);assert.equal(api.openInvocationSourceSearch(z.x.z.domain,z.x.handle,profile,facts),null);
 });
 check('known-unpublished-refusal-releases-hold-not-spent-issuance',()=>{
  const z=fixture(api),r={...z.request(),profile:'unsupported'},before=z.x.usage();assert.equal(z.query(r,current(r)).status,'unsupported');assert.equal(z.x.usage().holds,0);assert.equal(z.x.usage().issued,before.issued+1);assert.equal(z.usage().results,0);assert.equal(z.usage().issued,1);
 });
 check('new-source-publication-does-not-refresh-the-captured-query-root',()=>{
  const z=fixture(api),r=z.request(),q=current(r),before=z.x.z.root();assert.equal(z.x.z.submit(invocationCompoundJob(z.x.z.reference,before)).status,'accepted');assert.equal(z.x.z.run().status,'settled');const out=z.query(r,q);assert.equal(out.status,'complete');assert.equal(out.ref.revision,1);assert.equal(out.total,1);assert.equal(z.x.z.root().documents[0].revision,2);
 });
 check('source-loss-is-not-masked-by-query-capacity-refusal',()=>{
  const z=fixture(api),r=z.request(),q=current(r),out=z.query(r,q);assert.equal(z.release(out,{...q,capacity:false},z.x.c,{...facts,sourceReadable:false}).status,'unavailable');assert.equal(z.x.usage().invalidated,1);assert.equal(z.release(out,q).status,'unavailable');
 });
 check('malformed-data-and-accessors-have-no-source-or-result-side-effects',()=>{
  const z=fixture(api),r=z.request(),before=z.x.usage(),usage=z.usage();
  for(const bad of [null,[],{...r,extra:true},{...r,operation:'execute'},{...r,scopes:[null,null]},{...r,scopes:[{lo:{valueOf:1,toString:1},hi:3}]},{...r,ref:{...r.ref,document:'foreign'}}]){const q=bad&&typeof bad==='object'&&!Array.isArray(bad)&&Array.isArray(bad.scopes)&&bad.scopes.every(s=>s!==null)?current(bad):current(r);assert.doesNotThrow(()=>assert.equal(z.query(bad,q,z.x.c,{...facts,scope:false}).status,'unavailable'));}
  let calls=0;const bad={...r};Object.defineProperty(bad,'query',{enumerable:true,get(){calls++;throw Error('getter');}});assert.doesNotThrow(()=>assert.equal(z.query(bad,current(r)).status,'unavailable'));assert.equal(calls,0);assert.deepEqual(z.x.usage(),before);assert.deepEqual(z.usage(),usage);
 });
 check('pool-local-capture-selector-and-actual-parent-bounds-are-independent',()=>{
  for(const patch of [{maxHeldBytes:0},{maxResults:100,maxHeldBytes:100000000},{maxInputNodes:0},{maxInputDepth:0}]){const x=nativeSourceContextFixture(api),before=x.z.usage();assert.equal(api.openInvocationSourceSearch(x.z.domain,x.handle,{...profile,...patch},facts),null);assert.deepEqual(x.z.usage(),before);assert.equal(api.invocationSourceSearchUsage(x.z.domain),null);}
  for(const patch of [{maxResults:0},{maxCaptureBytes:0},{maxSelectorBytes:0}]){const z=fixture(api,patch),r=z.request(),before=z.x.usage();assert.equal(z.query(r,current(r)).status,'unavailable');assert.equal(z.usage().results,0);assert.equal(z.x.usage().holds,0);assert.equal(z.x.usage().issued,before.issued);}
 });
 return {scope:'isolated-native-source-search-child-owner',nativeCases,denialCases,controls,actualSourceLeaseAndParentChargeChecked:true,asynchronousSchedulingF11F12CurrentUseOrPublicProtocolProved:false,hostPolicyPhysicalConcurrentCrashOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeSourceSearch()));
export {fixture as nativeSourceSearchFixture,profile as nativeSourceSearchProfile,current as nativeSourceSearchCurrent};
