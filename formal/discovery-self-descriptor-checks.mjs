import assert from 'node:assert/strict';import fs from 'node:fs';import {pathToFileURL} from 'node:url';
import * as original from './discovery-self-descriptor.mjs';
import {advisoryDiscoveryFixture} from './advisory-discovery-checks.mjs';
import {nativeDiscoveryResultValid} from './discovery-results.mjs';
import {validateStandardSchemas} from './standard-schema-validator.mjs';
const copy=structuredClone,operations=['list','inspect','advisory-enablement'];
// Verification-only JSON shadows. Never serialize or inspect an opaque role.
function shadowRequest(q){return Object.fromEntries(Object.entries(q).map(([k,v])=>[k,['context','entry','variant','target'].includes(k)?{}:k==='continuation'?(v===null?null:{}):v]));}
function shadowResult(r){
 if(r.status==='unavailable')return {status:r.status};
 if(r.status==='page')return {status:r.status,context:{},coverage:r.coverage,entries:r.entries.map(e=>({reference:{},version:e.version,kind:e.kind})),continuation:r.continuation===null?null:{}};
 if(r.status==='advice')return {status:r.status,context:{},target:{},entry:{},variant:{},enabled:r.enabled};
 const d=r.descriptor;return {status:r.status,context:{},descriptor:{reference:{},version:d.version,kind:d.kind,variants:d.variants.map(v=>({reference:{},...(Object.hasOwn(v,'input')?Object.fromEntries(['input','outcomes','requirements','limits','cancellation'].map(k=>[k,copy(v[k])])):{})})),...(Object.hasOwn(d,'presentation')?{presentation:copy(d.presentation)}:{})}};
}
let retained;
export function discoverySelfDescriptorCorpus(){
 if(retained)return retained;const frames=[],runtime=[],add=(operation,request,value)=>frames.push({operation,request:shadowRequest(request),value:shadowResult(value)});
 const run=(x,a,q,route=null,c=x.c)=>{const request={contract:'contract-discovery/1',context:a.context,profile:c.profile,...q},root=x.z.root(),usage=x.z.usage(),r=x.query(a.context,q,x.g(),undefined,route,c);assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),usage);assert(nativeDiscoveryResultValid(r,request,x.l));add(q.operation,request,r);return r;};
 for(let profile=1;profile<=12;profile++){
  const x=advisoryDiscoveryFixture(),c={...x.c,profile:'owned-contract-discovery/'+profile},a=x.open(c),p=run(x,a,{operation:'list',limit:13,continuation:null},null,c);assert.equal(p.status,'page');
  for(const entry of p.entries){const d=run(x,a,{operation:'inspect',entry:entry.reference},null,c);assert.equal(d.status,'descriptor');assert(d.descriptor.variants.length);assert.equal(run(x,a,{operation:'advisory-enablement',entry:entry.reference,variant:d.descriptor.variants[0].reference,target:a.target},null,c).status,'advice');}
 }
 for(const options of [{schema:false,presentation:false},{variantVisible:()=>false},{visible:()=>false}]){const x=advisoryDiscoveryFixture(options),a=x.open(),p=run(x,a,{operation:'list',limit:13,continuation:null});for(const entry of p.entries)assert.equal(run(x,a,{operation:'inspect',entry:entry.reference}).status,'descriptor');}
 for(const route of ['typed','dynamic']){const x=advisoryDiscoveryFixture(),a=x.open(),p=run(x,a,{operation:'list',limit:13,continuation:null},route),entry=p.entries[0].reference,d=run(x,a,{operation:'inspect',entry},route);run(x,a,{operation:'advisory-enablement',entry,variant:d.descriptor.variants[0].reference,target:a.target},route);}
 {const x=advisoryDiscoveryFixture(),a=x.open(),p=run(x,a,{operation:'list',limit:1,continuation:null});assert(p.continuation);run(x,a,{operation:'list',limit:1,continuation:p.continuation});}
 for(const mode of ['context','entry','profile']){
  const x=advisoryDiscoveryFixture(),a=x.open(),p=run(x,a,{operation:'list',limit:13,continuation:null}),root=x.z.root(),usage=x.z.usage(),q={operation:'inspect',entry:p.entries[0].reference,...(mode==='profile'?{profile:'foreign'}:{})};
  const bad={...q,...(mode==='entry'?{entry:copy(q.entry)}:{})},context=mode==='context'?copy(a.context):a.context,request={contract:'contract-discovery/1',profile:x.c.profile,context,...bad},r=x.query(context,bad);assert.equal(r.status,'unavailable');assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),usage);add('inspect',request,r);runtime.push(shadowRequest(request));
 }
 let opaqueReads=0;const opaque={};Object.defineProperty(opaque,'kind',{enumerable:true,get(){opaqueReads++;throw Error('opaque inspection');}});
 const q={contract:'contract-discovery/1',operation:'advisory-enablement',profile:'owned-contract-discovery/12',context:opaque,entry:opaque,variant:opaque,target:opaque},r={status:'advice',context:opaque,entry:opaque,variant:opaque,target:opaque,enabled:false};assert(nativeDiscoveryResultValid(r,q,{maxPageRows:13,maxVariants:16,maxMetadataBytes:65536,maxMetadataNodes:65536,maxMetadataDepth:64}));const opaqueShadow={operation:q.operation,request:shadowRequest(q),value:shadowResult(r)};assert.equal(opaqueReads,0);
 retained={frames,runtime,opaqueShadow,opaqueReads};return retained;
}
export function checkDiscoverySelfDescriptor(api=original,corpus=discoverySelfDescriptorCorpus()){
 const d=api.discoverySelfEngineDescriptor(),forms=api.discoverySelfDescriptorInputForms(),catalog=JSON.parse(fs.readFileSync(new URL('./contract-discovery.json',import.meta.url),'utf8')),cases=[],controls=[];let syntaxDenials=0,recognitionDenials=0;
 const check=(name,fn)=>{fn();controls.push(name);},row=id=>d.variants.find(v=>v.id===id),add=(schema,value,valid)=>cases.push({schema,value:JSON.parse(JSON.stringify(value)),valid});assert(d&&forms);
 check('three-existing-native-query-forms-not-an-issuer-or-executable-schema',()=>{
  assert.equal(d.family,'F01');assert.equal(d.contract,catalog.contract);assert.equal(d.version,1);assert.equal(d.kind,'query');assert.deepEqual(d.variants.map(v=>v.id),operations);assert.equal(forms.length,3);assert.deepEqual(Object.keys(d.presentation),[]);
  for(const id of operations){const v=row(id);assert.deepEqual(v.input.schema.required,[...catalog.common,...catalog.variants[id]]);assert.equal(v.input.coverage,'outer-structure-only');assert.equal(v.outcomes.effect,'none');assert.equal(v.outcomes.result.coverage,'existing-native-structure-with-native-inert-metadata-bounds');assert.deepEqual(v.outcomes.forms,[id==='list'?'page':id==='inspect'?'descriptor':'advice','unavailable']);for(const limitation of ['original-selector-authority-or-context-acquisition','semantic-enablement-or-handler-registration-truth','metadata-as-code-schema-evaluator-or-transport'])assert(v.outcomes.notEstablished.includes(limitation));assert(Object.isFrozen(v.input.schema));assert(Object.isFrozen(v.outcomes.result.schema));assert(api.discoverySelfInputSchemaValid(v.input.schema,d.contract,id));assert(api.discoverySelfOutcomesValid(v.outcomes,d.contract,id));}
  assert(d.variants.every(v=>v.requirements.notEstablished.includes('context-acquisition-or-public-handle-codec')));assert(!JSON.stringify(d).includes('"$ref"'));
 });
 check('all-twelve-native-profiles-and-hidden-partial-products-fit-structural-shadows',()=>{
  for(const f of corpus.frames){const v=row(f.operation);add(v.input.schema,f.request,true);add(v.outcomes.result.schema,f.value,true);}
  add(row(corpus.opaqueShadow.operation).input.schema,corpus.opaqueShadow.request,true);add(row(corpus.opaqueShadow.operation).outcomes.result.schema,corpus.opaqueShadow.value,true);assert.equal(corpus.opaqueReads,0);
 });
 check('closed-input-fields-and-scalar-types-refuse-surprises',()=>{
  for(const id of operations){const v=row(id),request=corpus.frames.find(f=>f.operation===id).request;
   for(const key of Object.keys(request)){const bad=copy(request);delete bad[key];add(v.input.schema,bad,false);syntaxDenials++;}
   for(const bad of [{...copy(request),extra:true},{...copy(request),contract:'foreign'},{...copy(request),operation:'execute'},{...copy(request),context:null},{...copy(request),profile:''}]){add(v.input.schema,bad,false);syntaxDenials++;}
   for(const key of catalog.variants[id])for(const value of key==='limit'?[0,1.5,Number.MAX_SAFE_INTEGER+1]:key==='continuation'?[1,[]]:[null,1,[]]){add(v.input.schema,{...copy(request),[key]:value},false);syntaxDenials++;}
  }
 });
 check('result-shape-does-not-prove-original-reference-equality-or-authority',()=>{
  assert.equal(corpus.runtime.length,3);for(const q of corpus.runtime)add(row('inspect').input.schema,q,true);
  for(const id of operations){const v=row(id),value=corpus.frames.find(f=>f.operation===id).value;
   for(const key of Object.keys(value)){const bad=copy(value);delete bad[key];add(v.outcomes.result.schema,bad,false);syntaxDenials++;}
   for(const bad of [{...copy(value),extra:true},{...copy(value),status:'success'},{status:'unavailable',reason:'hidden'}]){add(v.outcomes.result.schema,bad,false);syntaxDenials++;}
   add(v.outcomes.result.schema,{status:'unavailable'},true);
   for(const other of operations.filter(o=>o!==id))add(row(other).outcomes.result.schema,value,false);
  }
  const p=corpus.frames.find(f=>f.operation==='list').value,ds=corpus.frames.find(f=>f.operation==='inspect').value;
  for(const change of [v=>v.coverage='complete',v=>v.entries[0].version=0,v=>v.entries[0].kind='execute',v=>v.entries[0].extra=true]){const bad=copy(p);change(bad);add(row('list').outcomes.result.schema,bad,false);syntaxDenials++;}
  for(const version of [1.5,Number.MAX_SAFE_INTEGER+1]){const bad=copy(p);bad.entries[0].version=version;add(row('list').outcomes.result.schema,bad,false);syntaxDenials++;}
  for(const change of [v=>v.descriptor.version=0,v=>v.descriptor.variants[0].extra=true,v=>delete v.descriptor.variants[0].input,v=>v.descriptor.presentation.unknown=true]){const bad=copy(ds);change(bad);add(row('inspect').outcomes.result.schema,bad,false);syntaxDenials++;}
  for(const enabled of [0,1,'true',null]){const value=copy(corpus.frames.find(f=>f.operation==='advisory-enablement').value);value.enabled=enabled;add(row('advisory-enablement').outcomes.result.schema,value,false);syntaxDenials++;}
  // Structural witnesses for the two generic effect kinds, not native handler
  // registration, execution or producer-truth evidence.
  for(const kind of ['effectRequest','effectObservation']){const f=corpus.frames.find(f=>f.operation==='list'),q=copy(f.request),value=copy(f.value);value.context=q.context;value.entries[0].kind=kind;assert(nativeDiscoveryResultValid(value,q,{maxPageRows:13,maxVariants:64,maxMetadataBytes:65536,maxMetadataNodes:65536,maxMetadataDepth:128}));add(row('list').outcomes.result.schema,value,true);}
 });
 check('exact-recognizers-bind-contract-variant-and-complete-meaning',()=>{
  for(const id of operations){const v=row(id);for(const [c,other]of [['foreign',id],[d.contract,'execute']]){assert.equal(api.discoverySelfInputSchemaValid(v.input.schema,c,other),false);assert.equal(api.discoverySelfOutcomesValid(v.outcomes,c,other),false);recognitionDenials+=2;}assert.equal(api.discoverySelfInputSchemaValid({...copy(v.input.schema),additionalProperties:true},d.contract,id),false);assert.equal(api.discoverySelfOutcomesValid({...copy(v.outcomes),effect:'publish'},d.contract,id),false);recognitionDenials+=2;}
 });
 validateStandardSchemas(cases);return {scope:'fixed-existing-F01-self-description',controls,forms:3,variants:3,nativeFrameFixtures:corpus.frames.length,structuralKindWitnesses:2,validatorCases:cases.length,syntaxDenials,recognitionDenials,runtimeCounterexamples:corpus.runtime.length,opaqueReads:corpus.opaqueReads,discoveryRegistrationSelected:false,newVerbIssuerWireOrEvaluatorSelected:false,hostPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkDiscoverySelfDescriptor()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[audit module]'));process.exitCode=1;}
