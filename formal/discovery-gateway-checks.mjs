import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as native from './native-discovery.mjs';
import * as kernel from './invocation-publication.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {discoveryArgumentClassificationBytes as classification} from './discovery-arguments.mjs';
const limits={maxRequestBytes:32768,maxInputNodes:4096,maxInputDepth:128},operations=['list','inspect','advisory-enablement'],factFields=['authorized','scope','continuous','live','notStopped','discoveryAllowed','retained','transferReady','replyReady','resourcesReady','advisoryAdviceAttested','advisoryTargetCurrent','advisoryTargetEqualityObservable','discoveryInterpretationCurrent','outsidePublicationGuard'];
const size=v=>Buffer.byteLength(JSON.stringify(v)),nodes=v=>1+(v&&typeof v==='object'?Object.values(v).reduce((n,x)=>n+nodes(x),0):0),depth=v=>v&&typeof v==='object'&&Object.keys(v).length?1+Math.max(...Object.values(v).map(depth)):0;
export function discoveryGatewayFixture(operation='list',route='typed',profile=10,options={},api=native){
 const x=nativeDiscoveryFixture(api,undefined,{...options,limits:{maxPageRows:4,maxMetadataDepth:64,...options.limits}}),c={...x.scope,profile:'owned-contract-discovery/'+profile},rows=api.nativeBuiltinDiscoveryRows(x.owner,c,x.g());assert(Array.isArray(rows));
 const cut=()=>api.prepareNativeBuiltinDiscoveryCut(x.owner,c,x.visibility(rows),x.target(),x.advice(rows),x.g()),p=cut();assert(p);const a=api.acquireNativeDiscovery(x.owner,c,p,x.g());assert.equal(a.status,'context');api.retireNativeDiscoveryCut(x.owner,p,x.g());
 const base={contract:'contract-discovery/1',profile:c.profile,context:a.context},direct=(request,g=x.g(),scope=c)=>{const p=cut();assert(p);const r=api.queryNativeDiscovery(x.owner,request,scope,p,g);api.retireNativeDiscoveryCut(x.owner,p,x.g());return r;},page=direct({...base,operation:'list',limit:1,continuation:null});assert.equal(page.status,'page');const entry=page.entries[0].reference,d=direct({...base,operation:'inspect',entry});assert.equal(d.status,'descriptor');
 const request={...base,operation,...(operation==='list'?{limit:1,continuation:null}:operation==='inspect'?{entry}:{entry,variant:d.descriptor.variants[0].reference,target:a.target})},envelope=(r=request)=>({route,request:{...(route==='typed'?{family:0}:{name:'discover-contracts'}),variant:r.operation,version:1,arguments:r}});let last;
 const query=(e=envelope(),p=limits,scope=c,g=x.g(),owner=x.owner,provided)=>{const made=provided===undefined,packet=made?cut():provided;if(made)assert(packet);last={root:x.z.root(),parent:x.z.usage(),usage:x.usage(),made,packet};try{const result=api.queryDecodedNativeDiscovery(owner,e,p,scope,packet,g);last.afterQuery=api.nativeDiscoveryUsage(x.owner);return result;}finally{api.retireNativeDiscoveryCut(x.owner,packet,x.g());}};
 return {x,c,a,request,page,entry,cut,envelope,query,direct,last:()=>last,usage:x.usage,decodeUsage:()=>api.nativeDiscoveryDecodeUsage(x.owner),root:()=>x.z.root(),retire:()=>{api.stopNativeDiscovery(x.owner,a.context,c,x.g());return api.retireNativeDiscovery(x.owner,a.context,x.g());}};
}
export function checkDiscoveryGateway(api=native,factory=discoveryGatewayFixture){
 const controls=[],check=(name,f)=>{f();controls.push(name);},make=(op='list',route='typed',profile=10,options={})=>factory(op,route,profile,options,api);let nativeCases=0,denials=0,boundaries=0,lossCases=0,opaqueBookkeeping=0;
 const frame=x=>{const b=x.last();assert.equal(x.root(),b.root);assert.deepEqual(x.x.z.usage(),b.parent);assert.deepEqual(x.usage(),{...b.usage,preparations:b.usage.preparations-(b.made?1:0)});assert.deepEqual(x.decodeUsage(),{busy:false,declaredWorkspaceBytes:0,faulted:false});};
 check('actual-query-fact-declaration-is-fixed-and-not-host-bookkeeping',()=>{const f=api.nativeDiscoveryQueryCurrentFactFields();assert(Object.isFrozen(f));assert.deepEqual([...f].sort(),[...factFields].sort());assert.equal(new Set(f).size,15);});
 check('actual-owner-dispatches-all-ten-profiles-three-forms-and-both-routes',()=>{
  for(let profile=1;profile<=10;profile++)for(const op of operations)for(const route of ['typed','dynamic']){const x=make(op,route,profile),r=x.query();assert.equal(r.status,{list:'page',inspect:'descriptor','advisory-enablement':'advice'}[op]);assert.equal(r.context,x.a.context);frame(x);nativeCases++;}
 });
 check('local-parent-headroom-includes-classification-and-seven-complete-input-frames',()=>{
  const base=make().usage().declaredHeldBytes,needed=classification+7*limits.maxRequestBytes;
  for(const delta of [-1,0,1]){const x=make('list','typed',10,{limits:{maxHeldBytes:base+needed+delta}});assert.equal(x.query().status,delta<0?'unavailable':'page');frame(x);boundaries++;}
 });
 check('classification-headroom-precedes-input-limit-inspection',()=>{
  const base=make().usage().declaredHeldBytes;
  for(const delta of [-1,0,1]){const x=make('list','typed',10,{limits:{maxHeldBytes:base+classification+delta}});let reads=0;const observed=new Proxy({...limits},{ownKeys(target){reads++;return Reflect.ownKeys(target);}});assert.equal(x.query(x.envelope(),observed).status,'unavailable');assert.equal(reads,delta<0?0:1);frame(x);boundaries++;}
 });
 check('selected-current-facts-have-independent-byte-node-and-depth-boundaries',()=>{
  const project=g=>({facts:Object.fromEntries(factFields.filter(k=>Object.hasOwn(g,k)&&g[k]!==undefined).map(k=>[k,g[k]]))});
  for(const dimension of ['bytes','nodes','depth'])for(const delta of [-1,0,1]){
   const x=make(),p={...limits,...(dimension==='nodes'?{maxInputNodes:128}:dimension==='depth'?{maxInputDepth:8}:{})},g=x.x.g({advisoryAdviceAttested:dimension==='bytes'?'':dimension==='nodes'?[]:null});
   if(dimension==='bytes'){g.advisoryAdviceAttested='x'.repeat(p.maxRequestBytes-size(project(g))+delta);assert.equal(size(project(g)),p.maxRequestBytes+delta);}
   else if(dimension==='nodes'){g.advisoryAdviceAttested=Array(p.maxInputNodes-nodes(project(g))+delta).fill(null);assert.equal(nodes(project(g)),p.maxInputNodes+delta);}
   else{for(let i=0;i<p.maxInputDepth+delta-2;i++)g.advisoryAdviceAttested={child:g.advisoryAdviceAttested};assert.equal(depth(project(g)),p.maxInputDepth+delta);}
   assert.equal(x.query(x.envelope(),p,x.c,g).status,delta<=0?'page':'unavailable',dimension);frame(x);boundaries++;
  }
 });
 check('foreign-context-owner-recipient-refuse-before-loss-but-bound-aliases-observe-loss',()=>{
  for(const op of operations){const x=make(op);for(const role of ['context',...(op==='list'?['continuation']:op==='inspect'?['entry']:['entry','variant','target'])]){const e=x.envelope({...x.request,[role]:{}});assert.equal(x.query(e,limits,x.c,x.x.g({discoveryAllowed:false})).status,'unavailable');if(role==='context')frame(x);else{assert.equal(x.usage().stopped,1);break;}denials++;} }
  const x=make();for(const scope of [{...x.c,authority:'foreign'},{...x.c,engine:'foreign'},{...x.c,recipient:'foreign'},{...x.c,policy:'foreign'},{...x.c,extra:'injected'}]){assert.equal(x.query(x.envelope(),limits,scope,x.x.g({discoveryAllowed:false})).status,'unavailable');frame(x);denials++;}
  for(const owner of [{},structuredClone(x.x.owner)]){assert.equal(x.query(x.envelope(),limits,x.c,x.x.g({discoveryAllowed:false}),owner).status,'unavailable');frame(x);denials++;}
  assert.equal(x.query(x.envelope(),limits,x.c,x.x.g(),x.x.owner,{}).status,'unavailable');frame(x);denials++;assert.equal(x.query().status,'page');frame(x);
 });
 check('unknown-profile-limit-and-aliases-are-native-refusals-not-repaired-targets',()=>{
  for(const op of operations){const x=make(op);for(const patch of [{profile:'unsupported'},{...(op==='list'?{limit:5}:op==='inspect'?{entry:{}}:{target:{}})}]){assert.equal(x.query(x.envelope({...x.request,...patch})).status,'unavailable');frame(x);denials++;}assert.equal(x.query().status,{list:'page',inspect:'descriptor','advisory-enablement':'advice'}[op]);frame(x);}
 });
 check('known-loss-absorbs-unknown-withholds-and-protected-cleanup-retains-spent-issuance',()=>{
  for(const field of ['authorized','scope','continuous','live','notStopped','discoveryAllowed','retained','discoveryInterpretationCurrent'])for(const value of [false,undefined,1,null]){
   const x=make();assert.equal(x.query(x.envelope(),limits,x.c,x.x.g({[field]:value})).status,'unavailable');assert.deepEqual(x.decodeUsage(),{busy:false,declaredWorkspaceBytes:0,faulted:false});const b=x.last();assert.equal(x.root(),b.root);assert.deepEqual(x.x.z.usage(),b.parent);assert.equal(x.usage().issued,b.usage.issued);assert.equal(x.usage().declaredHeldBytes,b.usage.declaredHeldBytes);assert.equal(x.usage().stopped,value===false?1:0);assert.equal(x.query().status,value===false?'unavailable':'page');if(value===false){const issued=x.usage().issued;assert.equal(x.retire().status,'retired');assert.equal(x.usage().issued,issued);lossCases++;}denials++;
  }
 });
 check('transient-release-pressure-and-advice-grants-do-not-close-or-fault-owner',()=>{
  for(const field of ['resourcesReady','replyReady','transferReady','outsidePublicationGuard'])for(const value of [false,undefined,1,null]){const x=make();assert.equal(x.query(x.envelope(),limits,x.c,x.x.g({[field]:value})).status,'unavailable');frame(x);assert.equal(x.query().status,'page');frame(x);denials++;}
  const x=make('advisory-enablement');for(const field of ['advisoryAdviceAttested','advisoryTargetCurrent','advisoryTargetEqualityObservable']){assert.equal(x.query(x.envelope(),limits,x.c,x.x.g({[field]:false})).status,'unavailable');frame(x);denials++;}assert.equal(x.query().status,'advice');frame(x);
 });
 check('selected-data-accessors-refuse-and-opaque-host-bookkeeping-is-not-cloned',()=>{
  for(const target of ['envelope','arguments','limits','context','facts']){const x=make(),e=x.envelope(),p={...limits},c={...x.c},g=x.x.g();e.request.arguments={...x.request};let calls=0;const value={envelope:e,arguments:e.request.arguments,limits:p,context:c,facts:g}[target],key={envelope:'route',arguments:'profile',limits:'maxRequestBytes',context:'policy',facts:'authorized'}[target];Object.defineProperty(value,key,{enumerable:true,get(){calls++;throw Error('unexpected data getter');}});assert.doesNotThrow(()=>assert.equal(x.query(e,p,c,g).status,'unavailable'));assert.equal(calls,0);frame(x);denials++;}
  const x=make(),g=x.x.g(),opaque=new WeakMap();g.testOnlyOpaque=opaque;let calls=0;Object.defineProperty(g,'unusedHostGetter',{enumerable:true,get(){calls++;throw Error('unrelated host bookkeeping');}});assert.equal(x.query(x.envelope(),limits,x.c,g).status,'page');assert.equal(g.testOnlyOpaque,opaque);assert.equal(calls,0);frame(x);opaqueBookkeeping++;
  // A verifier-owned probe counts named reads; this is not proxy isolation.
  let reads=0;const observed=new Proxy(x.x.g(),{ownKeys(){throw Error('must not enumerate host bookkeeping');},getOwnPropertyDescriptor(target,key){assert(factFields.includes(key));reads++;return Object.getOwnPropertyDescriptor(target,key);}});assert.equal(x.query(x.envelope(),limits,x.c,observed).status,'page');assert.equal(reads,15);frame(x);opaqueBookkeeping++;
  const h={};Object.defineProperty(h,'kind',{get(){calls++;throw Error('opaque role getter');}});assert.equal(x.query(x.envelope({...x.request,context:h})).status,'unavailable');assert.equal(calls,0);frame(x);denials++;
 });
 check('malformed-limits-and-stopped-contexts-do-not-acquire-or-refund-native-retention',()=>{
  const x=make();for(const p of [null,{...limits,maxRequestBytes:0},{...limits,maxInputNodes:0},{...limits,maxInputDepth:0},{...limits,maxRequestBytes:Number.MAX_SAFE_INTEGER},{...limits,extra:1}]){assert.equal(x.query(x.envelope(),p).status,'unavailable');frame(x);denials++;}
  const issued=x.usage().issued;assert.equal(api.stopNativeDiscovery(x.x.owner,x.a.context,x.c,x.x.g()).status,'stopped');assert.equal(x.query().status,'unavailable');assert.equal(x.usage().issued,issued);assert.equal(x.decodeUsage().declaredWorkspaceBytes,0);assert.equal(x.retire().status,'retired');
 });
 return {scope:'actual-local-owner-decoded-F01-query-gateway',controls,nativeCases,denials,boundaries,lossCases,opaqueBookkeeping,originalRolesAndLocalOwnerWorkspaceChecked:true,invocationGlobalOrResultWorkspaceOrPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkDiscoveryGateway()));
export {limits as discoveryGatewayLimits};
