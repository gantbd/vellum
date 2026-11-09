import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as argumentsApi from './lifecycle-arguments.mjs';
import {lifecycleInvocationFixture} from './lifecycle-invocation-checks.mjs';
const limits={maxRequestBytes:16384,maxInputNodes:4096,maxInputDepth:128},kinds=['create-scratch','attach-view','detach-view','membership','dispose'];
const variant=kind=>['attach-view','detach-view'].includes(kind)?'attach-detach-view':kind;
const envelope=(z,route='typed')=>({route,request:{...(route==='typed'?{family:16}:{name:'change-document-lifecycle'}),variant:variant(z.kind),version:1,arguments:{...structuredClone(z.request),invocation:z.reference}}});
export function checkLifecycleArguments(api=argumentsApi){
 const controls=[],check=(name,f)=>{f();controls.push(name);};let nativeCases=0,denials=0,boundaries=0,roleCases=0,runtimeCounterexamples=0;
 const normalize=(z,e=envelope(z),c=z.controlContext,l=limits,m=limits.maxRequestBytes)=>api.normalizeLifecycleArguments(e,c,l,m);
 check('same-native-field-declarations-and-no-private-cleanup-variants',()=>{
  const forms=api.lifecycleArgumentInputForms();assert.equal(forms.length,5);assert(Object.isFrozen(forms));assert.deepEqual(forms.map(f=>f.operation),kinds);assert.deepEqual([...new Set(forms.map(f=>f.variant))],['create-scratch','attach-detach-view','membership','dispose']);assert(forms.every(f=>Object.isFrozen(f)&&Object.isFrozen(f.fields)));assert(!forms.some(f=>f.operation.startsWith('release-')));
 });
 check('both-routes-preserve-original-roles-and-reuse-actual-native-intent',()=>{
  for(const kind of kinds)for(const route of ['typed','dynamic']){const z=lifecycleInvocationFixture(undefined,kind),before=z.root(),a=normalize(z,envelope(z,'typed')),b=normalize(z,envelope(z,'dynamic'));assert(a&&b,kind);assert(api.sameLifecycleArguments(a,b));const n=route==='typed'?a:b,request=api.lifecycleProducerRequest(n);assert.equal(n.invocation,z.reference);assert.equal(request.invocation,z.reference);assert.equal(n.operation,kind);assert.deepEqual(request,z.request);assert.equal(api.lifecycleProducerEnvelope(n).request.arguments.invocation,z.reference);assert(Object.isFrozen(n.record)&&Object.isFrozen(n.record.request.request));assert.equal(z.api.prepareInvocationLifecycleIntent(z.domain,z.ticket,request,z.profile,z.inspection,z.controlContext,z.g()),z.plan);assert.equal(z.submit(route).status,'accepted');assert.equal(z.run().status,'settled');assert.equal(z.outcome().outcome.publication,'applied');assert.equal(z.root().publication,before.publication+1);assert.equal(z.read().status,'result');nativeCases++;}
 });
 check('closed-wrapper-lower-fields-route-variant-and-scope',()=>{
  for(const kind of kinds){const z=lifecycleInvocationFixture(undefined,kind),original=envelope(z),form=api.lifecycleArgumentInputForms().find(f=>f.operation===kind);
   const bad=[null,{}, {...original,extra:true},{...original,route:'network'}, {...original,request:{...original.request,extra:true}},{...original,request:{...original.request,family:17}},{...original,request:{...original.request,variant:'release-provenance'}},{...original,request:{...original.request,version:2}}];
   bad.push({...original,request:{...original.request,variant:variant(kind)==='membership'?'dispose':'membership'}});
   for(const patch of [{contract:'foreign'},{operation:'foreign'},{profile:'foreign'},{invocation:null},{invocation:[]},{extra:true}])bad.push({...original,request:{...original.request,arguments:{...original.request.arguments,...patch}}});
   for(const field of form.fields){const e=envelope(z);delete e.request.arguments.request[field];bad.push(e);}
   for(const patch of [{extra:true},{operation:'release-binding'},{operation:'release-annotations'},{operation:'release-provenance'},{operation:{}},{contract:'foreign'}]){const e=envelope(z);Object.assign(e.request.arguments.request,patch);bad.push(e);}
   for(const e of bad){assert.equal(normalize(z,e),null,kind);denials++;}
   for(const field of Object.keys(z.controlContext)){const c={...z.controlContext};delete c[field];assert.equal(normalize(z,original,c),null);assert.equal(normalize(z,original,{...z.controlContext,[field]:''}),null);denials+=2;}assert.equal(normalize(z,original,{...z.controlContext,extra:'private'}),null);denials++;
  }
 });
 check('owned-data-remains-independent-of-caller-mutation',()=>{
  const z=lifecycleInvocationFixture(),e=envelope(z),c={...z.controlContext},l={...limits},n=normalize(z,e,c,l),expected=structuredClone(n.record);e.request.arguments.request.source.push(88);c.purpose='changed';l.maxRequestBytes=0;assert.deepEqual(n.record,expected);assert.equal(api.lifecycleProducerRequest(n).invocation,z.reference);
  const changed=normalize(z,e);assert(changed);assert(!api.sameLifecycleArguments(n,changed));const roundtrip=normalize(z,api.lifecycleProducerEnvelope(n));assert(roundtrip);assert(api.sameLifecycleArguments(n,roundtrip));
 });
 check('issued-invocation-value-is-preserved-not-coerced-or-repaired',()=>{
  const z=lifecycleInvocationFixture(),a=normalize(z),e=envelope(z);e.request.arguments.invocation=z.reference+'-foreign';const b=normalize(z,e);assert(b);assert(!api.sameLifecycleArguments(a,b));assert.equal(api.lifecycleProducerRequest(b).invocation,e.request.arguments.invocation);assert.equal(z.api.prepareInvocationLifecycleIntent(z.domain,z.ticket,api.lifecycleProducerRequest(b),z.profile,z.inspection,z.controlContext,z.g()),null);roleCases++;
  const opaque={};let calls=0;Object.defineProperty(opaque,'toString',{get(){calls++;throw Error('identity coercion');}});e.request.arguments.invocation=opaque;assert.equal(normalize(z,e),null);assert.equal(calls,0);roleCases++;
 });
 check('ordinary-data-accessors-cycles-and-sparse-arrays-refuse',()=>{
  for(const target of ['envelope','header','arguments','lower','scope','limits']){const z=lifecycleInvocationFixture(),e=envelope(z),c={...z.controlContext},l={...limits},v={envelope:e,header:e.request,arguments:e.request.arguments,lower:e.request.arguments.request,scope:c,limits:l}[target],key={envelope:'route',header:'variant',arguments:'profile',lower:'source',scope:'purpose',limits:'maxInputNodes'}[target];let calls=0;Object.defineProperty(v,key,{enumerable:true,get(){calls++;throw Error('ordinary getter');}});assert.equal(normalize(z,e,c,l),null);assert.equal(calls,0);denials++;}
  for(const malformed of [()=>{const a=[];a.length=1;return a;},()=>{const a=[];a.push(a);return a;},()=>[undefined],()=>[Infinity],()=>[Symbol('not-data')]]){const z=lifecycleInvocationFixture(),e=envelope(z);e.request.arguments.request.source=malformed();assert.equal(normalize(z,e),null);denials++;}
 });
 check('independent-logical-envelopes-have-exact-boundaries',()=>{
  const z=lifecycleInvocationFixture();for(const [field,upper]of [['maxRequestBytes',16384],['maxInputNodes',4096],['maxInputDepth',128]]){let low=0,high=upper;assert(normalize(z,envelope(z),z.controlContext,{...limits,[field]:high},16384));while(low<high){const middle=Math.floor((low+high)/2);if(normalize(z,envelope(z),z.controlContext,{...limits,[field]:middle},16384))high=middle;else low=middle+1;}assert(low>0);for(const delta of [-1,0,1]){assert.equal(normalize(z,envelope(z),z.controlContext,{...limits,[field]:low+delta},16384)!==null,delta>=0);boundaries++;}}
  for(const l of [null,{...limits,extra:1},{...limits,maxInputNodes:0},{...limits,maxRequestBytes:-1},{...limits,maxInputDepth:NaN}]){assert.equal(normalize(z,envelope(z),z.controlContext,l),null);denials++;}for(const max of [-1,NaN,0]){assert.equal(normalize(z,envelope(z),z.controlContext,limits,max),null);denials++;}
 });
 check('capture-shape-does-not-grant-native-validity-or-authority',()=>{
  const z=lifecycleInvocationFixture(),e=envelope(z);e.request.arguments.request.source=[999];const n=normalize(z,e);assert(n);const before=z.root(),prepared=z.api.prepareNativeRetainedLifecycleTransition(z.domain,before,api.lifecycleProducerRequest(n).request,z.profile,z.inspection,z.g());assert.deepEqual(prepared,{status:"unavailable"});assert.equal(z.root(),before);runtimeCounterexamples++;
 });
 return {scope:'selected-F17-lifecycle-argument-capture',forms:5,engineVariants:4,controls,nativeCases,denials,boundaries,roleCases,runtimeCounterexamples,noPublicCleanupVerbs:true,actualParentGatewayOrFullF17OrPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkLifecycleArguments()));
