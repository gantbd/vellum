import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {searchArgumentClassificationBytes as classification} from './search-arguments.mjs';
import {nativeSourceSearchFixture as fixture,nativeSourceSearchCurrent as current} from './native-source-search-checks.mjs';
import {nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
const limits={maxRequestBytes:4096,maxInputNodes:4096,maxInputDepth:128};
const parent={maxSlots:4,maxIssued:16,maxIntentBytes:65536,maxResultBytes:4096,maxRootBytes:262144,maxHeldBytes:2097152};
function setup(operation='literal-matches',route='typed',p=parent){
 const x=fixture(api,{},p),r=x.request(operation),c=api.invocationAuthorityContext(x.x.z.domain),e={route,request:{...(route==='typed'?{family:4}:{name:'search-matches'}),variant:operation,version:1,arguments:r}};
 const query=(envelope=e,inputLimits=limits,context=c,source=x.x.c,q=current(r),g=facts)=>api.queryInvocationDecodedSearch(x.x.z.domain,x.handle,envelope,inputLimits,context,source,q,g);
 return {x,r,c,e,query,usage:()=>api.invocationSearchDecodeUsage(x.x.z.domain)};
}
export function checkSearchGateway(){const controls=[],check=(name,f)=>{f();controls.push(name);};let forms=0,boundaries=0,denials=0;
 check('three-forms-two-routes-use-existing-result-owner-and-refund-workspace',()=>{
  for(const operation of ['literal-matches','next-match','previous-match'])for(const route of ['typed','dynamic']){const z=setup(operation,route),before=z.x.x.z.usage(),root=z.x.x.z.root(),out=z.query();assert.equal(out.status,'complete');assert.equal(out.total,1);assert.equal(out.context,z.r.context);assert.deepEqual(z.usage(),{busy:false,declaredWorkspaceBytes:0});assert.equal(z.x.x.z.usage().declaredHeldBytes,before.declaredHeldBytes);assert.equal(z.x.x.z.root(),root);assert.equal(z.x.usage().results,1);assert.equal(z.x.x.usage().holds,1);assert.equal(z.x.retire(out),true);forms++;}
 });
 check('actual-parent-workspace-has-exact-boundary-and-does-not-spend-issuance-on-refusal',()=>{
  const base=setup().x.x.z.usage().declaredHeldBytes,needed=classification+8*limits.maxRequestBytes;
  for(const delta of [-1,0,1]){const z=setup('literal-matches','typed',{...parent,maxHeldBytes:base+needed+delta}),before=z.x.x.z.usage(),source=z.x.x.usage(),out=z.query();assert.equal(out.status,delta<0?'unavailable':'complete');assert.equal(z.x.x.z.usage().declaredHeldBytes,before.declaredHeldBytes);assert.deepEqual(z.usage(),{busy:false,declaredWorkspaceBytes:0});if(delta<0){assert.deepEqual(z.x.x.usage(),source);assert.equal(z.x.usage().issued,0);}else assert.equal(z.x.retire(out),true);boundaries++;}
 });
 check('current-request-context-and-facts-are-not-repaired-from-decoded-command',()=>{
  for(const patch of [r=>({request:{...r,query:'other'}}),()=>({capacity:false}),()=>({grant:false}),()=>({scope:false}),()=>({notStopped:false}),r=>({request:{...r,context:{}}})]){const z=setup(),q={...current(z.r),...patch(z.r)},source=z.x.x.usage();assert.equal(z.query(z.e,limits,z.c,z.x.x.c,q).status,'unavailable');assert.deepEqual(z.x.x.usage(),source);assert.equal(z.x.usage().issued,0);assert.equal(z.x.query().status,'complete');denials++;}
 });
 check('recognized-source-loss-precedes-query-capacity-and-resources-refusal',()=>{
  for(const [q,g]of [[{capacity:false},{sourceReadable:false}],[{capacity:false},{sourceReadable:false,resourcesReady:false}]]){const z=setup();assert.equal(z.query(z.e,limits,z.c,z.x.x.c,{...current(z.r),...q},{...facts,...g}).status,'unavailable');assert.equal(z.x.x.usage().invalidated,1);assert.equal(z.x.query().status,'unavailable');denials++;}
 });
 check('foreign-domain-and-source-scope-refuse-without-poisoning',()=>{
  for(const [c,s]of [[{authority:'foreign'},{}],[{engine:'foreign'},{}],[{}, {recipient:'foreign'}]]){const z=setup(),source=z.x.x.usage();assert.equal(z.query(z.e,limits,{...z.c,...c},{...z.x.x.c,...s},current(z.r),{...facts,sourceReadable:false}).status,'unavailable');assert.deepEqual(z.x.x.usage(),source);assert.equal(z.x.query().status,'complete');denials++;}
  const z=setup();assert.equal(z.query(z.e,limits,{...z.c,recipient:'another-actor'}).status,'complete');
 });
 check('all-entry-data-accessors-refuse-without-running-or-faulting',()=>{
  for(const target of ['envelope','limits','authority','source','current','facts','currentRequest']){const z=setup(),e={...z.e},p={...limits},c={...z.c},s={...z.x.x.c},q=current(z.r),g={...facts};let calls=0;const v={envelope:e,limits:p,authority:c,source:s,current:q,facts:g,currentRequest:q.request}[target],key={envelope:'route',limits:'maxRequestBytes',authority:'authority',source:'recipient',current:'capacity',facts:'sourceReadable',currentRequest:'query'}[target];Object.defineProperty(v,key,{enumerable:true,get(){calls++;throw Error('getter');}});const before=z.x.x.z.usage();assert.doesNotThrow(()=>assert.equal(z.query(e,p,c,s,q,g).status,'unavailable'));assert.equal(calls,0);assert.deepEqual(z.x.x.z.usage(),before);assert.deepEqual(z.usage(),{busy:false,declaredWorkspaceBytes:0});denials++;}
 });
 check('unsupported-empty-and-stale-navigation-outcomes-stay-native',()=>{
  for(const change of [{profile:'other'},{query:''}]){const z=setup(),r={...z.r,...change},e={...z.e,request:{...z.e.request,arguments:r}},out=z.query(e,limits,z.c,z.x.x.c,current(r));assert.equal(out.status,change.profile?'unsupported':'complete');if(change.query==='')assert.equal(out.total,0);assert.deepEqual(z.usage(),{busy:false,declaredWorkspaceBytes:0});}
  const z=setup('next-match'),r={...z.r,navigation:{...z.r.navigation,current:100}},e={...z.e,request:{...z.e.request,arguments:r}};assert.equal(z.query(e,limits,z.c,z.x.x.c,current(r)).navigation.kind,'stale');
 });
 return {scope:'isolated-synchronous-F05-decoded-search-gateway',forms,boundaries,denials,controls,physicalHostileProxyAsyncPublicEventsOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSearchGateway()));
