import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as argumentsApi from './lifecycle-activation-arguments.mjs';
import {loadInvocationFixture} from './load-invocation-checks.mjs';
import {annotationLoadInvocationFixture} from './annotation-load-invocation-checks.mjs';
import {saveAsInvocationFixture} from './saveas-invocation-checks.mjs';
import {lifecycleInvocationFixture} from './lifecycle-invocation-checks.mjs';
import {relationInvocationFixture} from './relation-invocation-checks.mjs';
const limits={maxRequestBytes:32768,maxInputNodes:8192,maxInputDepth:128};
const ids=['source-load','annotation-restore','annotation-baseline','source-saveas','source-rebind','view-container-create','view-container-retire','workspace-container-create','workspace-container-retire'];
export function lifecycleActivationArgumentFixture(id){
 const z=id==='source-load'?loadInvocationFixture():id==='annotation-restore'?annotationLoadInvocationFixture():id==='annotation-baseline'?annotationLoadInvocationFixture(undefined,'baseline'):id==='source-saveas'?saveAsInvocationFixture():id==='source-rebind'?lifecycleInvocationFixture(undefined,'source-rebind'):relationInvocationFixture(undefined,id.startsWith('view-')?'view':'workspace',id.endsWith('-create')?'create':'retire');
 const nativePrepare=(request,foreign=false)=>id==='source-load'?z.api.prepareInvocationLoadIntent(z.domain,z.ticket,request,z.profile,foreign?{}:z.authority,z.controlContext,z.g()):id.startsWith('annotation-')?z.api.prepareInvocationAnnotationLoadIntent(z.domain,z.ticket,request,z.profile,foreign?{}:z.authority,z.inspection,z.controlContext,z.g()):id==='source-saveas'?z.api.prepareInvocationSaveAsIntent(z.domain,z.ticket,request,z.profile,foreign?{}:z.authority,z.controlContext,z.g()):id==='source-rebind'?z.api.prepareInvocationLifecycleIntent(z.domain,z.ticket,request,z.profile,foreign?{}:z.inspection,z.controlContext,z.g()):z.api.prepareInvocationRelationIntent(z.domain,z.ticket,request,z.profile,foreign?{...z.controlContext,recipient:'foreign'}:z.controlContext,z.g());
 return {...z,id,nativePrepare};
}
export function checkLifecycleActivationArguments(api=argumentsApi){
 let nativeCases=0,denials=0,roleCases=0,boundaries=0,disjointPairs=0;const controls=[],check=(n,f)=>{f();controls.push(n);},forms=api.lifecycleActivationInputForms();
 const envelope=(x,route='typed')=>({route,request:{...(route==='typed'?{family:16}:{name:'change-document-lifecycle'}),variant:forms.find(f=>f.id===x.id).variant,version:1,arguments:structuredClone(x.request)}});
 const normalize=(x,e=envelope(x),c=x.controlContext,l=limits,max=limits.maxRequestBytes)=>api.normalizeLifecycleActivationArguments(e,c,l,max);
 check('nine-shared-forms-retain-four-existing-engine-variants',()=>{assert.deepEqual(forms.map(f=>f.id),ids);assert.deepEqual([...new Set(forms.map(f=>f.variant))],['activate-loaded-candidate','rebind','view-container','workspace-container']);assert(Object.isFrozen(forms));assert(forms.every(f=>f.family==='F17'&&Object.isFrozen(f)&&Object.isFrozen(f.fields)&&(!f.nested||Object.isFrozen(f.nested.fields))));});
 check('every-pair-has-an-incompatible-fixed-discriminator',()=>{
  const fixed=f=>new Map([['header.variant',f.variant],['arguments.contract',f.contract],['arguments.operation',f.operation],['arguments.profile',f.profile],...Object.entries(f.matches).map(([k,v])=>['arguments.'+k,v]),...Object.entries(f.nested?.matches??{}).map(([k,v])=>['arguments.request.'+k,v])]);
  for(let i=0;i<forms.length;i++)for(let j=i+1;j<forms.length;j++){const a=fixed(forms[i]),b=fixed(forms[j]);assert([...a].some(([k,v])=>b.has(k)&&b.get(k)!==v),forms[i].id+' overlaps '+forms[j].id);disjointPairs++;}assert.equal(disjointPairs,36);
 });
 check('both-routes-reconstruct-actual-native-intents-with-original-authority-ports',()=>{
  for(const id of ids)for(const route of ['typed','dynamic']){const x=lifecycleActivationArgumentFixture(id),before=x.root(),a=normalize(x,envelope(x,'typed')),b=normalize(x,envelope(x,'dynamic'));assert(a&&b,id);assert(api.sameLifecycleActivationArguments(a,b));const n=route==='typed'?a:b;assert.equal(n.form,id);assert.deepEqual(api.lifecycleActivationProducerRequest(n),x.request);assert.equal(x.nativePrepare(api.lifecycleActivationProducerRequest(n)),x.plan);assert.equal(x.nativePrepare(api.lifecycleActivationProducerRequest(n),true),null);roleCases++;assert.equal(x.submit(route).status,'accepted');assert.equal(x.run().status,'settled');const publication=x.outcome().outcome.publication;assert(['applied','unchanged'].includes(publication));assert.equal(x.root().publication,before.publication+(publication==='applied'?1:0));assert.equal((x.result??x.read)().status,'result');nativeCases++;}
 });
 check('closed-outer-and-nested-fields-discriminators-profiles-and-variants',()=>{
  for(const id of ids){const x=lifecycleActivationArgumentFixture(id),form=forms.find(f=>f.id===id),original=envelope(x),bad=[null,{}, {...original,extra:true},{...original,route:'network'},{...original,request:{...original.request,extra:true}},{...original,request:{...original.request,family:17}},{...original,request:{...original.request,version:2}},{...original,request:{...original.request,variant:'dispose'}}];
   bad.push({...envelope(x,'dynamic'),route:'network'});if(form.nested){const e=envelope(x);e.request.arguments.request=null;bad.push(e);}
   for(const key of form.fields){const e=envelope(x);delete e.request.arguments[key];bad.push(e);}for(const patch of [{extra:true},{contract:'foreign'},{operation:'foreign'},{profile:'foreign'},{invocation:''},{invocation:{}}])bad.push({...original,request:{...original.request,arguments:{...original.request.arguments,...patch}}});
   if(form.nested){for(const key of form.nested.fields){const e=envelope(x);delete e.request.arguments.request[key];bad.push(e);}for(const patch of [{extra:true},...Object.keys(form.nested.matches).map(key=>({[key]:'unknown'}))]){const e=envelope(x);Object.assign(e.request.arguments.request,patch);bad.push(e);}}
   for(const key of Object.keys(form.matches)){const e=envelope(x);e.request.arguments[key]='unknown';bad.push(e);}
   for(const e of bad){let out;assert.doesNotThrow(()=>{out=normalize(x,e);},id);assert.equal(out,null,id);denials++;}for(const key of Object.keys(x.controlContext)){const c={...x.controlContext};delete c[key];assert.equal(normalize(x,original,c),null);assert.equal(normalize(x,original,{...x.controlContext,[key]:''}),null);denials+=2;}assert.equal(normalize(x,original,{...x.controlContext,extra:'private'}),null);denials++;
  }
 });
 check('owned-capture-roundtrip-and-invocation-context-meaning',()=>{
  const x=lifecycleActivationArgumentFixture('source-saveas'),e=envelope(x),c={...x.controlContext},l={...limits},n=normalize(x,e,c,l),expected=structuredClone(n.record);e.request.arguments.request.probe={};c.purpose='changed';l.maxRequestBytes=0;assert.deepEqual(n.record,expected);const round=normalize(x,api.lifecycleActivationProducerEnvelope(n));assert(round);assert(api.sameLifecycleActivationArguments(n,round));const changed=normalize(x,e);assert(changed);assert(!api.sameLifecycleActivationArguments(n,changed));assert.equal(x.nativePrepare(api.lifecycleActivationProducerRequest(changed)),null);
 });
 check('request-accessors-and-nonordinary-nested-data-refuse',()=>{
  for(const target of ['envelope','header','arguments','nested','scope','limits']){const x=lifecycleActivationArgumentFixture('annotation-restore'),e=envelope(x),c={...x.controlContext},l={...limits},v={envelope:e,header:e.request,arguments:e.request.arguments,nested:e.request.arguments.request,scope:c,limits:l}[target],key={envelope:'route',header:'variant',arguments:'profile',nested:'binding',scope:'purpose',limits:'maxInputDepth'}[target];let calls=0;Object.defineProperty(v,key,{enumerable:true,get(){calls++;throw Error('unexpected getter');}});assert.equal(normalize(x,e,c,l),null);assert.equal(calls,0);denials++;}
  for(const value of [undefined,Infinity,Symbol('not-data'),(()=>{const a=[];a.length=1;return a;})(),(()=>{const a={};a.self=a;return a;})()]){const x=lifecycleActivationArgumentFixture('annotation-restore'),e=envelope(x);e.request.arguments.request.binding=value;assert.equal(normalize(x,e),null);denials++;}
 });
 check('independent-logical-envelopes-retain-exact-boundaries',()=>{
  const x=lifecycleActivationArgumentFixture('source-load');for(const [field,upper]of [['maxRequestBytes',32768],['maxInputNodes',8192],['maxInputDepth',128]]){let low=0,high=upper;assert(normalize(x,envelope(x),x.controlContext,{...limits,[field]:high},32768));while(low<high){const mid=Math.floor((low+high)/2);if(normalize(x,envelope(x),x.controlContext,{...limits,[field]:mid},32768))high=mid;else low=mid+1;}assert(low>0);for(const d of [-1,0,1]){assert.equal(normalize(x,envelope(x),x.controlContext,{...limits,[field]:low+d},32768)!==null,d>=0);boundaries++;}}
  assert.equal(normalize(x,envelope(x),x.controlContext,limits,0),null);denials++;
 });
 return {scope:'selected-F17-activation-argument-capture',controls,forms:9,engineVariants:4,disjointPairs,nativeCases,denials,roleCases,boundaries,originalAuthorityPortsRemainOutsideData:true,noProviderPortInArgumentCapture:true,actualEnvelopeGatewayOrFullF17OrPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkLifecycleActivationArguments()));
