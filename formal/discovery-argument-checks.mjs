import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as native from './native-discovery.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {normalizeDiscoveryArguments as normalize,sameDiscoveryArguments as same,discoveryProducerRequest as producer,captureDiscoveryArgumentLimits as captureLimits,discoveryArgumentInputForms as inputForms,discoveryArgumentClassificationBytes as classificationBytes} from './discovery-arguments.mjs';
const limits={maxRequestBytes:65536,maxInputNodes:4096,maxInputDepth:128},maximum=1048576,operations=['list','inspect','advisory-enablement'],roles={list:['context','continuation'],inspect:['context','entry'],'advisory-enablement':['context','entry','variant','target']};
const size=v=>Buffer.byteLength(JSON.stringify(v)),nodes=v=>1+(v&&typeof v==='object'?Object.values(v).reduce((n,x)=>n+nodes(x),0):0),depth=v=>v&&typeof v==='object'&&Object.keys(v).length?1+Math.max(...Object.values(v).map(depth)):0;
export function discoveryArgumentFixture(operation='list',profile=1){
 const x=nativeDiscoveryFixture(native,undefined,{limits:{maxMetadataDepth:64}}),c={...x.scope,profile:'owned-contract-discovery/'+profile},rows=native.nativeBuiltinDiscoveryRows(x.owner,c,x.g());assert(Array.isArray(rows));
 const cut=()=>native.prepareNativeBuiltinDiscoveryCut(x.owner,c,x.visibility(rows),x.target(),x.advice(rows),x.g()),p=cut();assert(p);const a=native.acquireNativeDiscovery(x.owner,c,p,x.g());assert.equal(a.status,'context');native.retireNativeDiscoveryCut(x.owner,p,x.g());
 const query=(r,g=x.g(),scope=c)=>{const p=cut();assert(p);const out=native.queryNativeDiscovery(x.owner,r,scope,p,g);native.retireNativeDiscoveryCut(x.owner,p,x.g());return out;},base={contract:'contract-discovery/1',profile:c.profile,context:a.context},page=query({...base,operation:'list',limit:1,continuation:null});assert.equal(page.status,'page');const entry=page.entries[0].reference,d=query({...base,operation:'inspect',entry});assert.equal(d.status,'descriptor');
 const r={...base,operation,...(operation==='list'?{limit:1,continuation:null}:operation==='inspect'?{entry}:{entry,variant:d.descriptor.variants[0].reference,target:a.target})},envelope=(route='typed',request=r)=>({route,request:{...(route==='typed'?{family:0}:{name:'discover-contracts'}),variant:request.operation,version:1,arguments:request}}),n=(e=envelope(),scope=c,p=limits,m=maximum)=>normalize(e,scope,p,m);
 return {x,c,a,r,page,entry,query,envelope,n};
}
export function checkDiscoveryArguments(){
 const controls=[],check=(name,f)=>{f();controls.push(name);};let nativeCases=0,denials=0,boundaries=0,opaqueDenials=0,lossCases=0;
 check('three-fixed-native-declarations-and-owned-limit-classification',()=>{
  const forms=inputForms();assert(forms);assert.equal(forms.length,3);assert.deepEqual(forms.map(f=>f.operation),operations);assert(Object.isFrozen(forms));for(const f of forms){assert.equal(f.family,'F01');assert.equal(f.contract,'contract-discovery/1');assert(Object.isFrozen(f.fields));}assert.deepEqual(captureLimits(limits),limits);assert(Object.isFrozen(captureLimits(limits)));assert.equal(classificationBytes,size(Object.fromEntries(Object.keys(limits).map(k=>[k,Number.MAX_SAFE_INTEGER]))));for(const p of [null,{},[],{...limits,extra:1},{...limits,maxInputNodes:0},{...limits,maxRequestBytes:-1},{...limits,maxInputDepth:0.5}])assert.equal(captureLimits(p),null);assert.equal(producer(null),null);
 });
 check('ten-existing-profiles-three-forms-both-routes-reuse-original-native-selectors',()=>{
  for(let profile=1;profile<=10;profile++)for(const operation of operations){
   const x=discoveryArgumentFixture(operation,profile),a=x.n(),b=x.n(x.envelope('dynamic'));assert(a&&b);assert(same(a,b));assert(Object.isFrozen(a)&&Object.isFrozen(a.opaque));assert.deepEqual(producer(a),x.r);const root=x.x.z.root(),parent=x.x.z.usage(),usage=x.x.usage();
   for(const n of [a,b]){for(const k of roles[operation]){assert.equal(n.record.request[k],null);assert.equal(n.opaque[k],x.r[k]);assert.equal(producer(n)[k],x.r[k]);}const r=x.query(producer(n));assert.equal(r.status,{list:'page',inspect:'descriptor','advisory-enablement':'advice'}[operation]);if(operation==='advisory-enablement')assert.equal(r.enabled,true);assert.equal(x.x.z.root(),root);assert.deepEqual(x.x.z.usage(),parent);assert.deepEqual(x.x.usage(),usage);nativeCases++;}
  }
 });
 check('closed-headers-fields-and-scalar-domains-refuse-without-new-query-meaning',()=>{
  for(const operation of operations){const x=discoveryArgumentFixture(operation),bad=[null,{},[],{...x.envelope(),route:'wire'},{...x.envelope(),extra:true},{route:'typed',request:{...x.envelope().request,family:1}},{route:'typed',request:{...x.envelope().request,version:2}},{route:'typed',request:{...x.envelope().request,extra:1}},{route:'dynamic',request:{...x.envelope('dynamic').request,name:'inspect-history'}},{route:'typed',request:{...x.envelope().request,variant:operation==='list'?'inspect':'list'}}];
   for(const patch of [{contract:'contract-discovery/2'},{operation:'execute'},{context:null},{context:[]},{profile:''},{profile:1},{profile:'\ud800'},{extra:true},...(operation==='list'?[{limit:0},{limit:-1},{limit:1.5},{limit:Number.MAX_SAFE_INTEGER+1},{continuation:1},{continuation:[]}]:[{entry:null},{entry:1},{entry:[]}]),...(operation==='advisory-enablement'?[{variant:null},{variant:[]},{target:null},{target:1}]:[])])bad.push(x.envelope('typed',{...x.r,...patch}));
   for(const k of Object.keys(x.r)){const r={...x.r};delete r[k];bad.push(x.envelope('typed',r));}
   for(const e of bad){assert.doesNotThrow(()=>assert.equal(x.n(e),null));denials++;}
  }
 });
 check('capture-does-not-authenticate-opaque-context-entry-cursor-variant-or-target',()=>{
  for(const operation of operations){const x=discoveryArgumentFixture(operation);
   for(const role of roles[operation].filter(k=>k!=='continuation')){
    let calls=0;const handle={};Object.defineProperty(handle,'kind',{enumerable:true,get(){calls++;throw Error('opaque role getter');}});const r={...x.r,[role]:handle},n=x.n(x.envelope('typed',r));assert(n);assert.equal(n.opaque[role],handle);assert.equal(x.query(producer(n)).status,'unavailable');assert.equal(calls,0);const shadow=x.n(x.envelope('typed',{...x.r,[role]:structuredClone(x.r[role])}));assert(shadow);assert.equal(same(n,shadow),false);assert.equal(x.query(producer(shadow)).status,'unavailable');opaqueDenials+=2;
   }
  }
  const x=discoveryArgumentFixture(),request={...x.r,continuation:x.page.continuation},n=x.n(x.envelope('typed',request));assert(n);assert.equal(x.query(producer(n)).status,'page');nativeCases++;
  for(const continuation of [{},structuredClone(x.page.continuation),x.entry]){const n=x.n(x.envelope('typed',{...x.r,continuation}));assert(n);assert.equal(x.query(producer(n)).status,'unavailable');opaqueDenials++;}
  const rebound=x.n(x.envelope('typed',{...request,limit:2}));assert(rebound);assert.equal(x.query(producer(rebound)).status,'unavailable');opaqueDenials++;
 });
 check('accessors-symbols-nondata-prototypes-and-cycles-never-enter-canonical-data',()=>{
  const x=discoveryArgumentFixture();let calls=0;
  for(const target of ['envelope','tuple','arguments','scope','limits']){const e=x.envelope(),c={...x.c},p={...limits};e.request.arguments={...x.r};const value={envelope:e,tuple:e.request,arguments:e.request.arguments,scope:c,limits:p}[target],key={envelope:'route',tuple:'variant',arguments:'profile',scope:'policy',limits:'maxRequestBytes'}[target];Object.defineProperty(value,key,{enumerable:true,get(){calls++;throw Error('data getter');}});assert.doesNotThrow(()=>assert.equal(x.n(e,c,p),null));}assert.equal(calls,0);
  for(const change of [e=>e[Symbol('extra')]=1,e=>Object.defineProperty(e.request.arguments,'profile',{value:x.c.profile,enumerable:false}),e=>Object.setPrototypeOf(e.request.arguments,{inherited:true})]){const e=x.envelope();e.request.arguments={...x.r};change(e);assert.equal(x.n(e),null);}
  const r={...x.r};r.profile=r;assert.equal(x.n(x.envelope('typed',r)),null);for(const c of [{...x.c,extra:'injected'},{...x.c,policy:1},{...x.c,continuity:''},{...x.c,recipient:'\ud800'}])assert.equal(x.n(x.envelope(),c),null);
 });
 check('captured-values-are-owned-and-equality-binds-all-original-roles',()=>{
  for(const operation of operations){const x=discoveryArgumentFixture(operation),r={...x.r},c={...x.c},p={...limits},n=x.n(x.envelope('typed',r),c,p);assert(n);r.profile='changed';c.policy='changed';p.maxRequestBytes=0;assert.equal(n.record.request.profile,x.c.profile);assert.equal(n.record.context.policy,x.c.policy);assert.equal(n.record.limits.maxRequestBytes,limits.maxRequestBytes);assert(Object.isFrozen(n.record.request)&&Object.isFrozen(n.record.context)&&Object.isFrozen(n.record.limits));const out=producer(n);out.profile='changed';assert.equal(producer(n).profile,x.c.profile);assert(same(n,x.n(x.envelope('dynamic'))));assert.equal(same(n,x.n(x.envelope(),{...x.c,policy:'other'})),false);for(const role of roles[operation]){const other={...x.r,[role]:x.r[role]===null?{}:structuredClone(x.r[role])};const m=x.n(x.envelope('typed',other));assert(m);assert.equal(same(n,m),false);}}
 });
 check('four-capture-envelopes-have-exact-independent-boundaries',()=>{
  for(const operation of operations){const x=discoveryArgumentFixture(operation),shadow={...x.r,...Object.fromEntries(roles[operation].map(k=>[k,null]))},whole={...x.envelope(),request:{...x.envelope().request,arguments:shadow}};let p={...limits};
   for(let i=0;i<6;i++){const n=x.n(x.envelope(),x.c,p);assert(n);p={...p,maxRequestBytes:Math.max(size(n.record),size(whole),size(x.c))};}assert(x.n(x.envelope(),x.c,p));for(const delta of [-1,0,1]){assert.equal(x.n(x.envelope(),x.c,{...p,maxRequestBytes:p.maxRequestBytes+delta})!==null,delta>=0);boundaries++;}
   const base=x.n();assert(base);for(const [field,min]of [['maximum',Math.max(size(base.record),size(whole),size(x.c))],['maxInputNodes',Math.max(nodes(base.record),nodes(whole),nodes(x.c))],['maxInputDepth',Math.max(depth(base.record),depth(whole),depth(x.c))]])for(const delta of [-1,0,1]){assert.equal((field==='maximum'?x.n(x.envelope(),x.c,limits,min+delta):x.n(x.envelope(),x.c,{...limits,[field]:min+delta}))!==null,delta>=0,field);boundaries++;}
   for(const m of [-1,0,1,1.5,Infinity])assert.equal(x.n(x.envelope(),x.c,limits,m),null);
  }
 });
 check('native-profile-page-bound-and-recipient-meaning-are-not-repaired-by-decoding',()=>{
  const x=discoveryArgumentFixture();for(const patch of [{profile:'unsupported'},{limit:5}]){const n=x.n(x.envelope('typed',{...x.r,...patch}));assert(n);assert.equal(x.query(producer(n)).status,'unavailable');opaqueDenials++;}
  const n=x.n(),root=x.x.z.root(),parent=x.x.z.usage(),usage=x.x.usage();assert(n);assert.equal(x.query(producer(n),x.x.g(),{...x.c,recipient:'foreign'}).status,'unavailable');assert.equal(x.x.z.root(),root);assert.deepEqual(x.x.z.usage(),parent);assert.deepEqual(x.x.usage(),usage);assert.equal(x.query(producer(n)).status,'page');opaqueDenials++;
 });
 check('normalized-data-cannot-revive-observed-loss-or-stopped-context',()=>{
  for(const operation of operations){const x=discoveryArgumentFixture(operation),n=x.n();assert(n);assert.equal(x.query(producer(n),x.x.g({discoveryAllowed:false})).status,'unavailable');assert.equal(x.query(producer(n)).status,'unavailable');assert.equal(x.x.usage().stopped,1);lossCases++;}
  const x=discoveryArgumentFixture(),n=x.n();assert(n);assert.equal(native.stopNativeDiscovery(x.x.owner,x.a.context,x.c,x.x.g()).status,'stopped');assert.equal(x.query(producer(n)).status,'unavailable');lossCases++;
 });
 return {scope:'isolated-F01-discovery-argument-normalization',controls,forms:3,profiles:10,nativeCases,denials,boundaries,opaqueDenials,lossCases,originalRolesAndTypedDynamicMeaningChecked:true,nativeGatewayParentOwnershipResultOrWireOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkDiscoveryArguments()));
