import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as descriptorApi from './lifecycle-engine-descriptor.mjs';
import * as kernel from './invocation-publication.mjs';
import {nativeLifecycleResultValid} from './lifecycle-results.mjs';
import {activationEnvelopeFixture,activationEnvelopeIds} from './lifecycle-activation-envelope-checks.mjs';
import {lifecycleEnvelopeFixture} from './lifecycle-envelope-checks.mjs';
import {loadInvocationFixture} from './load-invocation-checks.mjs';
import {annotationLoadInvocationFixture} from './annotation-load-invocation-checks.mjs';
import {annotationNativeLoadDonor,annotationNativeLoadPack} from './annotation-native-load-checks.mjs';
import {lifecycleInvocationFixture} from './lifecycle-invocation-checks.mjs';
import {validateStandardSchemas} from './standard-schema-validator.mjs';
const copy=structuredClone,ids=['create-scratch','attach-view','detach-view','membership','dispose',...activationEnvelopeIds];
export function lifecycleMetadataFixture(id='source-load',route='typed'){
 const activation=activationEnvelopeIds.includes(id),x=activation?activationEnvelopeFixture(id,route):lifecycleEnvelopeFixture(id,route),kind=id.includes('-container-')?'relation':'lifecycle';
 const variant=id==='attach-view'||id==='detach-view'?'attach-detach-view':id==='source-load'||id.startsWith('annotation-')?'activate-loaded-candidate':id==='source-saveas'||id==='source-rebind'?'rebind':id.includes('-container-')?id.startsWith('view-')?'view-container':'workspace-container':id;
 const read=()=>kind==='relation'?kernel.observeRelationResult(x.domain,x.ticket,x.controlContext,x.controlPolicy,{...x.facts(),relationResultReadable:true,relationResultContextCurrent:true,historicalTargetReadable:true}):kernel.observeLifecycleResult(x.domain,x.ticket,x.controlContext,x.controlPolicy,{...x.facts(),lifecycleResultReadable:true,lifecycleResultContextCurrent:true,historicalDocumentReadable:true});
 const execute=()=>{const prepared=x.prepare();assert(prepared,id);const p=activation?prepared:{job:{kind,plan:prepared},route};assert.equal(kernel.submitInvocation(x.domain,x.ticket,p.job,p.route,x.controlContext,x.controlPolicy,x.facts()).status,'accepted');assert.equal(kernel.runInvocation(x.domain,x.ticket,x.controlContext,x.facts(),x.host).status,'settled');return read();};
 return {x,id,route,kind,variant,request:x.request,execute,read};
}
export function checkLifecycleDiscoveryProducts(api=descriptorApi){
 const descriptor=api.lifecycleEngineDescriptor();assert(descriptor);const variants=['create-scratch','activate-loaded-candidate','attach-detach-view','membership','rebind','dispose','view-container','workspace-container'],forms=api.lifecycleDescriptorInputForms(),row=id=>descriptor.variants.find(v=>v.id===id),cases=[],controls=[],check=(n,f)=>{f();controls.push(n);},add=(schema,value,valid)=>cases.push({schema,value:JSON.parse(JSON.stringify(value)),valid});
 let nativeProducts=0,syntaxDenials=0,recognitionDenials=0,runtimeCounterexamples=0,disjointPairs=0;
 check('eight-existing-variants-fourteen-handler-forms-and-no-public-private-discharge',()=>{
  assert.equal(descriptor.family,'F17');assert.equal(descriptor.contract,'engine-lifecycle-declaration/1');assert.equal(descriptor.kind,'localCommand');assert.equal(descriptor.version,1);assert.deepEqual(descriptor.variants.map(v=>v.id),variants);assert.deepEqual(forms.map(f=>f.id),ids);assert(Object.isFrozen(descriptor)&&Object.isFrozen(forms));
  assert.deepEqual(descriptor.variants.map(v=>v.input.schema.oneOf.length),[1,3,2,1,2,1,2,2]);assert.deepEqual(descriptor.variants.map(v=>v.outcomes.result.schema.oneOf[0].properties.result.oneOf.length),[1,7,2,1,2,1,2,2]);
  for(const v of descriptor.variants){assert.equal(v.input.coverage,'outer-structure-only');assert.equal(v.outcomes.result.coverage,'native-value-structure-only');assert.equal(v.outcomes.result.scope,'existing-native-operation-result-facet-only');assert(api.lifecycleInputSchemaValid(v.input.schema,descriptor.contract,v.id));assert(api.lifecycleOutcomesValid(v.outcomes,descriptor.contract,v.id));assert(!JSON.stringify(v.input.schema).includes('release-provenance'));assert(!JSON.stringify(v.outcomes.result.schema).includes('release-binding'));assert(Object.isFrozen(v.outcomes.result.schema));}
  const fixed=f=>new Map([['header.variant',f.variant],['arguments.contract',f.contract],['arguments.operation',f.operation],['arguments.profile',f.profile],...Object.entries(f.matches).map(([k,v])=>['arguments.'+k,v]),...Object.entries(f.nested?.matches??{}).map(([k,v])=>['arguments.request.'+k,v])]);
  for(let i=0;i<forms.length;i++)for(let j=i+1;j<forms.length;j++){const a=fixed(forms[i]),b=fixed(forms[j]);assert([...a].some(([k,v])=>b.has(k)&&b.get(k)!==v));disjointPairs++;}assert.equal(disjointPairs,91);
 });
 check('both-routes-actual-owner-publication-and-current-result-have-standard-syntax',()=>{
  for(const id of ids)for(const route of ['typed','dynamic']){
   const z=lifecycleMetadataFixture(id,route),v=row(z.variant),r=z.execute();assert.equal(r.status,'result');assert(nativeLifecycleResultValid(r,z.kind));add(v.input.schema,z.request,true);add(v.outcomes.result.schema,r,true);add(v.outcomes.result.schema,{status:'unavailable'},true);nativeProducts++;
   for(const field of Object.keys(z.request)){const bad=copy(z.request);delete bad[field];add(v.input.schema,bad,false);syntaxDenials++;}
   add(v.input.schema,{...copy(z.request),extra:true},false);syntaxDenials++;
   if(z.request.request){for(const field of Object.keys(z.request.request)){const bad=copy(z.request);delete bad.request[field];add(v.input.schema,bad,false);syntaxDenials++;}const bad=copy(z.request);bad.request.extra=true;add(v.input.schema,bad,false);syntaxDenials++;}
   for(const field of Object.keys(r.result)){const bad=copy(r);delete bad.result[field];add(v.outcomes.result.schema,bad,false);syntaxDenials++;}
   for(const bad of [{...copy(r),extra:true},{status:'result',result:{...copy(r.result),extra:true}},{status:'unavailable',result:copy(r.result)},{status:'result'},{status:'surprise'}]){add(v.outcomes.result.schema,bad,false);syntaxDenials++;}
   for(const other of variants.filter(x=>x!==z.variant)){add(row(other).input.schema,z.request,false);add(row(other).outcomes.result.schema,r,false);syntaxDenials+=2;}
  }
 });
 check('source-activation-and-annotation-phase-status-branches-have-native-witnesses',()=>{
  for(const mode of ['created','joined','already-activated'])for(const route of ['typed','dynamic']){const x=loadInvocationFixture(undefined,mode);assert.equal(x.submit(route).status,'accepted');assert.equal(x.run().status,'settled');const r=x.read();assert.equal(r.result.activation,mode);add(row('activate-loaded-candidate').outcomes.result.schema,r,true);const bad=copy(r);bad.result.status=r.result.status==='applied'?'unchanged':'applied';add(row('activate-loaded-candidate').outcomes.result.schema,bad,false);syntaxDenials++;nativeProducts++;}
  for(const mode of ['restore','adopt-baseline','current'])for(const route of ['typed','dynamic']){const x=annotationLoadInvocationFixture(undefined,mode);assert.equal(x.submit(route).status,'accepted');assert.equal(x.run().status,'settled');const r=x.result();add(row('activate-loaded-candidate').outcomes.result.schema,r,true);for(const patch of [{status:r.result.status==='applied'?'unchanged':'applied'},{phase:r.result.phase==='restore'?'adopt-baseline':'restore'},{disposition:'surprise'}]){add(row('activate-loaded-candidate').outcomes.result.schema,{status:'result',result:{...copy(r.result),...patch}},false);syntaxDenials++;}nativeProducts++;}
  for(const route of ['typed','dynamic']){const pack=annotationNativeLoadPack(annotationNativeLoadDonor().z);pack.snapshot.records=[];pack.binding.identities=[];pack.raw=Array.from(Buffer.from(JSON.stringify(pack.snapshot)));const x=annotationLoadInvocationFixture(undefined,'restore',pack);assert.equal(x.submit(route).status,'accepted');assert.equal(x.run().status,'settled');const r=x.result();assert.equal(r.result.disposition,'unchanged');add(row('activate-loaded-candidate').outcomes.result.schema,r,true);nativeProducts++;}
  for(const kind of ['release-binding','release-annotations','release-provenance']){const x=lifecycleInvocationFixture(undefined,kind);assert.equal(x.submit().status,'accepted');assert.equal(x.run().status,'settled');const r=x.read();assert.equal(r.status,'result');for(const v of descriptor.variants){add(v.outcomes.result.schema,r,false);syntaxDenials++;}}
 });
 check('structural-acceptance-does-not-prove-native-values-identity-or-budgets',()=>{
  const x=lifecycleEnvelopeFixture(),e=x.envelope();e.request.arguments.request.source=[192];add(row('create-scratch').input.schema,e.request.arguments,true);const before=x.root();assert.equal(x.prepare(e),null);assert.equal(x.root(),before);assert(x.prepare());runtimeCounterexamples++;
  const y=lifecycleMetadataFixture(),bad=copy(y.request);bad.invocation+='shadow';add(row(y.variant).input.schema,bad,true);const f=y.x.envelope();f.request.arguments=bad;assert.equal(y.x.prepare(f),null);assert(y.x.prepare());runtimeCounterexamples++;
  const z=lifecycleEnvelopeFixture(),request=z.envelope().request.arguments;add(row('create-scratch').input.schema,request,true);assert.equal(z.prepare(z.envelope(),{maxRequestBytes:0,maxInputNodes:4096,maxInputDepth:128}),null);assert(z.prepare());runtimeCounterexamples++;
  const a=lifecycleMetadataFixture(),r=a.execute(),forged=copy(r);forged.result.document.id='\ud800';add(row(a.variant).outcomes.result.schema,forged,true);assert.equal(nativeLifecycleResultValid(forged,'lifecycle'),false);runtimeCounterexamples++;
  add(row(a.variant).outcomes.result.schema,copy(r),true);assert.equal(kernel.observeLifecycleResult(a.x.domain,copy(a.x.ticket),a.x.controlContext,a.x.controlPolicy,a.x.facts()).status,'unavailable');runtimeCounterexamples++;
 });
 check('metadata-recognizers-refuse-widening-loaders-false-authority-and-event-completeness',()=>{
  for(const v of descriptor.variants){
   for(const schema of [{...copy(v.input.schema),oneOf:[]},{...copy(v.input.schema),$ref:'https://not-loaded.invalid/schema'},{...copy(v.input.schema),execute:()=>{throw Error('must not run');}},{...copy(v.input.schema),additionalProperties:true}]){assert.equal(api.lifecycleInputSchemaValid(schema,descriptor.contract,v.id),false);recognitionDenials++;}
   for(const out of [{...copy(v.outcomes),events:'complete-public-lifecycle-events'},{...copy(v.outcomes),effect:'performs-provider-write'},{...copy(v.outcomes),result:{...copy(v.outcomes.result),runtimeBindings:[]}},{...copy(v.outcomes),result:{...copy(v.outcomes.result),notEstablished:[]}},{...copy(v.outcomes),result:{...copy(v.outcomes.result),scope:'new-public-query'}}]){assert.equal(api.lifecycleOutcomesValid(out,descriptor.contract,v.id),false);recognitionDenials++;}
  }
 });
 try{validateStandardSchemas(cases);}catch(error){const index=Number(/value case (\d+)/.exec(error.message)?.[1]);if(Number.isSafeInteger(index))console.error(JSON.stringify({schemaCase:index,...cases[index]}));throw error;}
 return {scope:'handler-bound-F17-lifecycle-discovery-products',controls,variants:8,forms:14,disjointPairs,validatorCases:cases.length,nativeProducts,syntaxDenials,recognitionDenials,runtimeCounterexamples,noPublicResultQueryOrWireSelected:true,shapeDoesNotGrantNativeAuthority:true,fullF17EventsOrHostPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkLifecycleDiscoveryProducts()));
