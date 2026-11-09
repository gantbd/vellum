import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {engineSettingsReadFixture as fixture,engineSettingsReadGates as gates} from './engine-settings-read-checks.mjs';
import {normalizeSettingsQueryArguments as normalize,sameSettingsQueryArguments as same,settingsQueryProducerEnvelope as producer,captureSettingsQueryArgumentLimits as captureLimits,settingsQueryArgumentInputForms as inputForms,settingsQueryArgumentClassificationBytes as classificationBytes} from './settings-query-arguments.mjs';
const limits={maxRequestBytes:65536,maxInputNodes:4096,maxInputDepth:128},maximum=1048576,operations=['effective-configuration','permitted-origin','capabilities'],copy=structuredClone;
const size=v=>Buffer.byteLength(JSON.stringify(v)),nodes=v=>1+(v&&typeof v==='object'?Object.values(v).reduce((n,x)=>n+nodes(x),0):0),depth=v=>v&&typeof v==='object'&&Object.keys(v).length?1+Math.max(...Object.values(v).map(depth)):0;
function setup(operation='effective-configuration',key='scalar'){
 const z=fixture(),ctx=z.acquire();assert.equal(ctx.status,'context');const envelope=(route='typed')=>z.envelope(ctx,operation,key,route),n=(e=envelope(),c=z.c,p=limits,m=maximum)=>normalize(e,c,p,m),query=(e=envelope(),c=z.c,g=gates)=>{const packet=z.cut(z.facts,c);try{return kernel.queryInvocationSettingsRead(z.domain,z.pool,e,c,packet,g);}finally{kernel.discardInvocationSettingsReadCut(z.domain,z.pool,packet,gates);}};return {z,ctx,envelope,n,query};
}
export function checkSettingsQueryArguments(){
 const controls=[],check=(name,f)=>{f();controls.push(name);};let forms=0,nativeCases=0,denials=0,boundaries=0,opaqueRoleCases=0;
 check('guard-owned-three-forms-and-exact-captured-limits',()=>{
  const rows=inputForms();assert(Object.isFrozen(rows));assert.deepEqual(rows.map(f=>f.operation),operations);for(const f of rows){assert(Object.isFrozen(f));assert(Object.isFrozen(f.fields));assert.equal(f.family,'F07');assert.equal(f.contract,'configuration-query/1');assert.equal(f.variant,f.operation);assert.deepEqual(f.fields,['contract','operation','context','target','profile',...(f.operation==='capabilities'?[]:['key'])]);}
  assert.deepEqual(captureLimits(limits),limits);assert(Object.isFrozen(captureLimits(limits)));assert.equal(classificationBytes,size(Object.fromEntries(Object.keys(limits).map(k=>[k,Number.MAX_SAFE_INTEGER]))));for(const p of [null,{...limits,extra:1},{...limits,maxInputNodes:0},{...limits,maxRequestBytes:-1},{...limits,maxInputDepth:1.5}])assert.equal(captureLimits(p),null);assert.equal(producer(null),null);
 });
 check('typed-dynamic-meaning-keeps-context-target-and-values-origins-declarations',()=>{
  for(const op of operations)for(const key of op==='capabilities'?['scalar']:['scalar','list','map']){
   const x=setup(op,key),a=x.n(),b=x.n(x.envelope('dynamic'));assert(a&&b);assert.equal(same(a,b),true);assert.equal(a.context,x.ctx.context);assert.equal(a.target,x.ctx.target);assert.equal(a.record.request.context,null);assert.equal(a.record.request.target,null);assert.deepEqual(producer(a),x.envelope());assert.deepEqual(producer(b),x.envelope('dynamic'));const root=x.z.root(),usage=x.z.parentUsage();
   for(const n of [a,b]){const r=x.query(producer(n));assert.equal(r.status,op==='capabilities'?'capabilities':op==='permitted-origin'?'origins':'value');assert.equal(r.context,x.ctx.context);assert.equal(r.target,x.ctx.target);assert.equal(x.z.root(),root);assert.deepEqual(x.z.parentUsage(),usage);nativeCases++;}forms++;
  }
 });
 check('closed-envelope-tuples-and-value-domains-refuse-without-side-effects',()=>{
  for(const op of operations){const x=setup(op),e=x.envelope(),bad=[null,{},[],{...e,route:'wire'},{...e,extra:true},{route:'typed',request:{...e.request,family:5}},{route:'typed',request:{...e.request,version:2}},{route:'typed',request:{...e.request,extra:true}},{route:'typed',request:{...e.request,variant:op==='capabilities'?'effective-configuration':'capabilities'}},{route:'dynamic',request:{...x.envelope('dynamic').request,name:'read-document'}}];
   for(const patch of [{contract:'configuration-query/2'},{operation:'activate'},{context:null},{context:1},{context:[]},{target:null},{target:1},{target:[]},{profile:''},{profile:1},{profile:'\ud800'},{extra:true},...(op==='capabilities'?[{key:'scalar'}]:[{key:''},{key:1},{key:[]},{key:'\ud800'}])])bad.push({...e,request:{...e.request,arguments:{...e.request.arguments,...patch}}});
   for(const key of Object.keys(e.request.arguments)){const a={...e.request.arguments};delete a[key];bad.push({...e,request:{...e.request,arguments:a}});}
   const root=x.z.root(),usage=x.z.usage();for(const v of bad){assert.doesNotThrow(()=>assert.equal(x.n(v),null));denials++;}assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),usage);
  }
 });
 check('accessors-symbols-hidden-members-cycles-and-foreign-prototypes-refuse',()=>{
  const x=setup();let calls=0;
  for(const target of ['envelope','tuple','arguments','authority','limits']){const e=copy(x.envelope()),c={...x.z.c},p={...limits},value={envelope:e,tuple:e.request,arguments:e.request.arguments,authority:c,limits:p}[target],key={envelope:'route',tuple:'variant',arguments:'key',authority:'policy',limits:'maxRequestBytes'}[target];Object.defineProperty(value,key,{enumerable:true,get(){calls++;throw Error('getter must not run');}});assert.doesNotThrow(()=>assert.equal(x.n(e,c,p),null));}
  assert.equal(calls,0);for(const modify of [e=>e[Symbol('extra')]=1,e=>Object.defineProperty(e.request.arguments,'key',{value:'scalar',enumerable:false}),e=>Object.setPrototypeOf(e.request.arguments,{inherited:true}),e=>e.request.arguments.key=e]){const e=copy(x.envelope());modify(e);assert.equal(x.n(e),null);}
  for(const c of [{...x.z.c,extra:'injected'},{...x.z.c,policy:1},{...x.z.c,continuity:''},{...x.z.c,recipient:'\ud800'}])assert.equal(x.n(x.envelope(),c),null);
  for(const key of Object.keys(x.z.c)){const c={...x.z.c};delete c[key];assert.equal(x.n(x.envelope(),c),null);}
 });
 check('opaque-roles-are-not-traversed-cloned-swapped-or-authenticated',()=>{
  for(const role of ['context','target']){const x=setup(),e=x.envelope(),handle={};let calls=0;Object.defineProperty(handle,'kind',{enumerable:true,get(){calls++;throw Error('opaque getter must not run');}});e.request.arguments[role]=handle;const n=x.n(e);assert(n);assert.equal(n[role],handle);assert.equal(producer(n).request.arguments[role],handle);assert.equal(x.query(producer(n)).status,'unavailable');assert.equal(calls,0);assert.equal(same(n,x.n()),false);opaqueRoleCases++;}
  const x=setup(),e=x.envelope(),n=x.n();for(const patch of [{context:copy(x.ctx.context)},{target:copy(x.ctx.target)},{context:x.ctx.target,target:x.ctx.context},{context:{},target:{}}]){const candidate=x.n({...e,request:{...e.request,arguments:{...e.request.arguments,...patch}}});assert(candidate);assert.equal(same(n,candidate),false);assert.equal(x.query(producer(candidate)).status,'unavailable');opaqueRoleCases++;}assert.equal(x.query(producer(n)).status,'value');
 });
 check('unsupported-profile-and-unknown-key-are-not-repaired',()=>{
  for(const patch of [{profile:'other'},{key:'unknown'}]){const x=setup(),e=x.envelope();e.request.arguments={...e.request.arguments,...patch};const n=x.n(e);assert(n);assert.equal(x.query(producer(n)).status,'unavailable');assert.equal(x.query().status,'value');}
  const x=setup(),a=x.n();assert.equal(same(a,x.n(x.envelope(),{...x.z.c,policy:'other'})),false);
 });
 check('captured-values-and-authority-do-not-alias-caller-data',()=>{
  const x=setup(),e=x.envelope(),c={...x.z.c},p={...limits},n=x.n(e,c,p);assert(n);e.request.arguments.key='changed';c.policy='changed';p.maxRequestBytes=0;assert.equal(n.record.request.key,'scalar');assert.equal(n.record.context.policy,x.z.c.policy);assert.equal(n.record.limits.maxRequestBytes,limits.maxRequestBytes);for(const v of [n.record,n.record.request,n.record.context,n.record.limits])assert(Object.isFrozen(v));const out=producer(n);out.request.arguments.key='mutated';assert.equal(producer(n).request.arguments.key,'scalar');
 });
 check('four-independent-capture-envelopes-have-exact-boundaries',()=>{
  for(const op of operations){const x=setup(op),e=x.envelope(),shadow={...e.request.arguments,context:null,target:null},whole={...e,request:{...e.request,arguments:shadow}};let p={...limits};for(let i=0;i<6;i++){const n=x.n(e,x.z.c,p);assert(n);p={...p,maxRequestBytes:Math.max(size(n.record),size(whole),size(x.z.c))};}
   for(const delta of [-1,0,1]){assert.equal(x.n(e,x.z.c,{...p,maxRequestBytes:p.maxRequestBytes+delta})!==null,delta>=0);boundaries++;}
   const base=x.n();for(const [field,min]of [['maximum',Math.max(size(base.record),size(whole),size(x.z.c))],['maxInputNodes',Math.max(nodes(base.record),nodes(whole),nodes(x.z.c))],['maxInputDepth',Math.max(depth(base.record),depth(whole),depth(x.z.c))]])for(const delta of [-1,0,1]){assert.equal((field==='maximum'?x.n(e,x.z.c,limits,min+delta):x.n(e,x.z.c,{...limits,[field]:min+delta}))!==null,delta>=0,field);boundaries++;}
   for(const m of [-1,0,1,1.5,Infinity])assert.equal(x.n(e,x.z.c,limits,m),null);
  }
 });
 check('native-observed-loss-remains-absorbing-not-normalizer-authority',()=>{
  for(const op of operations){const x=setup(op),n=x.n();assert(n);assert.equal(x.query(producer(n),x.z.c,{...gates,continuous:false}).status,'unavailable');assert.equal(x.query(producer(n)).status,'unavailable');assert.equal(x.z.usage().invalidated,1);}
 });
 return {scope:'isolated-F07-settings-query-argument-normalization',forms,nativeCases,denials,boundaries,opaqueRoleCases,controls,originalContextTargetAndTypedDynamicMeaningChecked:true,nativeGatewayChargingResultCodecOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSettingsQueryArguments()));
