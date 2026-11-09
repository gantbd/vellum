import assert from 'node:assert/strict';import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {engineSettingsReadFixture as fixture,engineSettingsReadGates as gates} from './engine-settings-read-checks.mjs';
import {settingsQueryArgumentClassificationBytes as classification} from './settings-query-arguments.mjs';
const limits={maxRequestBytes:4096,maxInputNodes:4096,maxInputDepth:128},operations=['effective-configuration','permitted-origin','capabilities'],copy=structuredClone;
const size=v=>Buffer.byteLength(JSON.stringify(v)),nodes=v=>1+(v&&typeof v==='object'?Object.values(v).reduce((n,x)=>n+nodes(x),0):0),depth=v=>v&&typeof v==='object'&&Object.keys(v).length?1+Math.max(...Object.values(v).map(depth)):0;
export function settingsGatewayFixture(operation='effective-configuration',key='scalar',route='typed',options={},api=kernel){
 const z=fixture(api,options),ctx=z.acquire();assert.equal(ctx.status,'context');let last;
 const envelope=()=>z.envelope(ctx,operation,key,route),query=(e=envelope(),p=limits,c=z.c,g=gates,handle=z.pool,domain=z.domain)=>{const packet=z.cut(z.facts,c);last={root:z.root(),parent:z.parentUsage(),pool:z.usage(),packet};try{return api.queryDecodedInvocationSettingsRead(domain,handle,e,p,c,packet,g);}finally{api.discardInvocationSettingsReadCut(z.domain,z.pool,packet,gates);}};
 return {...z,ctx,envelope,query,last:()=>last,decodeUsage:()=>api.invocationSettingsReadDecodeUsage(z.domain,z.pool)};
}
export function checkSettingsQueryGateway(api=kernel,factory=settingsGatewayFixture){
 const controls=[],check=(name,f)=>{f();controls.push(name);},make=(op='effective-configuration',key='scalar',route='typed',options={})=>factory(op,key,route,options,api);let nativeCases=0,denials=0,boundaries=0,lossCases=0;
 const frame=(x,invalidations=0)=>{const b=x.last();assert.equal(x.root(),b.root);assert.deepEqual(x.parentUsage(),b.parent);assert.deepEqual(x.usage(),{...b.pool,packets:b.pool.packets-(b.packet?1:0),invalidated:b.pool.invalidated+invalidations});assert.deepEqual(x.decodeUsage(),{busy:false,declaredWorkspaceBytes:0,faulted:false});};
 check('actual-parent-native-dispatch-preserves-both-routes-and-original-roles',()=>{
  for(const op of operations)for(const key of op==='capabilities'?['scalar']:['scalar','list','map'])for(const route of ['typed','dynamic']){const x=make(op,key,route),r=x.query();assert.equal(r.status,op==='effective-configuration'?'value':op==='permitted-origin'?'origins':'capabilities');assert.equal(r.context,x.ctx.context);assert.equal(r.target,x.ctx.target);frame(x);nativeCases++;}
 });
 check('actual-shared-parent-reserves-classification-and-seven-complete-frames',()=>{
  const base=make().parentUsage().declaredHeldBytes,needed=classification+7*limits.maxRequestBytes;
  for(const delta of [-1,0,1]){const x=make('effective-configuration','scalar','typed',{limits:{maxHeldBytes:base+needed+delta}});assert.equal(x.query().status,delta<0?'unavailable':'value');frame(x);boundaries++;}
 });
 check('current-facts-have-independent-byte-node-and-depth-ceilings',()=>{
  for(const dimension of ['bytes','nodes','depth'])for(const delta of [-1,0,1]){const x=make(),p={...limits,...(dimension==='nodes'?{maxInputNodes:128}:dimension==='depth'?{maxInputDepth:8}:{})},g={...gates,padding:dimension==='bytes'?'':dimension==='nodes'?[]:null};
   if(dimension==='bytes'){g.padding='x'.repeat(p.maxRequestBytes-size({facts:g})+delta);assert.equal(size({facts:g}),p.maxRequestBytes+delta);}else if(dimension==='nodes'){g.padding=Array(p.maxInputNodes-nodes({facts:g})+delta).fill(null);assert.equal(nodes({facts:g}),p.maxInputNodes+delta);}else{for(let i=0;i<p.maxInputDepth+delta-2;i++)g.padding={child:g.padding};assert.equal(depth({facts:g}),p.maxInputDepth+delta);}
   assert.equal(x.query(x.envelope(),p,x.c,g).status,delta<=0?'value':'unavailable',dimension);frame(x);boundaries++;
  }
 });
 check('foreign-roles-scope-pool-and-malformed-envelope-cannot-poison-a-reader',()=>{
  const x=make(),original=x.envelope();for(const patch of [{context:copy(x.ctx.context)},{target:copy(x.ctx.target)},{context:x.ctx.target,target:x.ctx.context},{profile:'foreign'},{extra:true}]){assert.equal(x.query({...original,request:{...original.request,arguments:{...original.request.arguments,...patch}}},limits,x.c,{...gates,continuous:false}).status,'unavailable');frame(x);denials++;}
  for(const c of [{...x.c,authority:'foreign'},{...x.c,engine:'foreign'},{...x.c,recipient:'foreign'},{...x.c,policy:'foreign'},{...x.c,extra:'injected'}]){assert.equal(x.query(original,limits,c,{...gates,continuous:false}).status,'unavailable');frame(x);denials++;}
  for(const handle of [{},copy(x.pool)]){assert.equal(x.query(original,limits,x.c,{...gates,continuous:false},handle).status,'unavailable');frame(x);denials++;}
  for(const p of [null,{...limits,extra:1},{...limits,maxInputNodes:0},{...limits,maxRequestBytes:-1},{...limits,maxRequestBytes:Number.MAX_SAFE_INTEGER}]){assert.equal(x.query(original,p).status,'unavailable');frame(x);denials++;}
  assert.equal(x.query().status,'value');frame(x);
 });
 check('caller-accessors-refuse-without-running-and-workspace-refunds',()=>{
  for(const target of ['envelope','arguments','limits','context','facts']){const x=make(),e=x.envelope(),p={...limits},c={...x.c},g={...gates};e.request.arguments={...e.request.arguments};const v={envelope:e,arguments:e.request.arguments,limits:p,context:c,facts:g}[target],key={envelope:'route',arguments:'key',limits:'maxRequestBytes',context:'policy',facts:'authorized'}[target];let calls=0;Object.defineProperty(v,key,{enumerable:true,get(){calls++;throw Error('unexpected getter');}});assert.doesNotThrow(()=>assert.equal(x.query(e,p,c,g).status,'unavailable'));assert.equal(calls,0);frame(x);denials++;}
 });
 check('recognized-loss-absorbs-unknown-withholds-and-transient-pressure-does-not-fault',()=>{
  for(const field of ['authorized','scope','continuous','live','retained'])for(const value of [false,undefined,1,null]){const x=make();assert.equal(x.query(x.envelope(),limits,x.c,{...gates,[field]:value}).status,'unavailable');frame(x,value===false?1:0);assert.equal(x.query().status,value===false?'unavailable':'value');if(value===false)lossCases++;denials++;}
  for(const field of ['resourcesReady','replyReady','transferReady','outsidePublicationGuard','notStopped'])for(const value of [false,undefined,1,null]){const x=make();assert.equal(x.query(x.envelope(),limits,x.c,{...gates,[field]:value}).status,'unavailable');frame(x);assert.equal(x.query().status,'value');frame(x);denials++;}
 });
 check('opaque-internals-are-not-traversed-or-repaired',()=>{
  for(const role of ['context','target']){const x=make(),e=x.envelope(),handle={};let calls=0;Object.defineProperty(handle,'kind',{get(){calls++;throw Error('opaque getter');}});e.request.arguments[role]=handle;assert.equal(x.query(e).status,'unavailable');assert.equal(calls,0);frame(x);assert.equal(x.query().status,'value');denials++;}
 });
 check('unknown-key-is-not-repaired-and-does-not-hide-a-bound-observed-loss',()=>{
  for(const lost of [false,true]){const x=make(),e=x.envelope();e.request.arguments.key='missing';assert.equal(x.query(e,limits,x.c,lost?{...gates,continuous:false}:gates).status,'unavailable');frame(x,lost?1:0);assert.equal(x.query().status,lost?'unavailable':'value');denials++;}
 });
 return {scope:'actual-parent-decoded-F07-settings-query-gateway',nativeCases,denials,boundaries,lossCases,controls,sharedParentWorkspaceAndProtectedRefundChecked:true,hostGlobalPhysicalAllProductsOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSettingsQueryGateway()));
export {limits as settingsGatewayLimits};
