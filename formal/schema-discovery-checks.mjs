// Selected handler-bound outer schemas and owned profile evolution, not a codec.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {validateStandardSchemas} from './standard-schema-validator.mjs';
import {pathToFileURL} from 'node:url';
import * as discovery from './native-discovery.mjs';
import * as kernel from './invocation-publication.mjs';
import {contextEngineDescriptor} from './context-engine-descriptor.mjs';
import {engineSettingsInputForms} from './engine-settings-state.mjs';
import {registrationInputForms} from './registration-invocation-state.mjs';
import {sourceSerializationInputForm,scopedSourceSerializationInputForm} from './source-serialization.mjs';
import {scopedSerializationFixture} from './scoped-serialization-checks.mjs';
import {sourceSerializationFixture} from './source-serialization-checks.mjs';
import {outerInputSchema,outerInputSchemaValid} from './input-schema-profiles.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {engineSettingsFixture} from './engine-settings-checks.mjs';
import {engineSettingsDefinitionFixture} from './engine-settings-definition-checks.mjs';
import {engineSettingsLayerFixture} from './engine-settings-layer-checks.mjs';
import {scopedSettingsFixture} from './scoped-settings-checks.mjs';
import {scopedDefinitionFixture} from './scoped-definition-checks.mjs';
import {scopedLifecycleFixture} from './scoped-lifecycle-checks.mjs';
import {basisLifecycleFixture} from './basis-lifecycle-checks.mjs';
import {registrationInvocationFixture} from './registration-invocation-checks.mjs';
import {scopedRegistrationInvocationFixture} from './registration-scoped-checks.mjs';
const copy=structuredClone,no={status:'unavailable'},profile='owned-contract-discovery/2';
// Independent expectations: never derive the expected field list from a schema.
const forms=[
 {variant:'semantic-settings',factory:engineSettingsFixture,fields:['contract','operation','invocation','profile','changes','dependencies'],route:'settings'},
 {variant:'semantic-settings',factory:engineSettingsLayerFixture,fields:['contract','operation','invocation','profile','expected','create','remove','targets','dependencies'],route:'settings'},
 {variant:'semantic-settings',factory:scopedSettingsFixture,fields:['contract','operation','invocation','profile','changes','dependencies'],route:'scopedSettings'},
 {variant:'supported-registry',factory:engineSettingsDefinitionFixture,fields:['contract','operation','invocation','profile','expected','definition','dependencies'],route:'settings'},
 {variant:'supported-registry',factory:registrationInvocationFixture,fields:['contract','operation','invocation','profile','request'],route:'native'},
 {variant:'supported-registry',factory:scopedRegistrationInvocationFixture,fields:['contract','operation','invocation','profile','request'],route:'scoped'},
 {variant:'semantic-settings',factory:sourceSerializationFixture,fields:['contract','operation','invocation','profile','document','expectedSerialization','serialization','dependencies'],route:'serialization'},
 {variant:'supported-registry',factory:scopedDefinitionFixture,fields:['contract','operation','invocation','profile','definition','dependencies'],route:'scopedDefinition'},
 {variant:'semantic-settings',factory:scopedLifecycleFixture,fields:['contract','operation','invocation','profile','create','remove','targets','dependencies'],route:'scopedLifecycle'},
 {variant:'semantic-settings',factory:scopedSerializationFixture,fields:['contract','operation','invocation','profile','serialization','dependencies'],route:'scopedSerialization'},
 {variant:'semantic-settings',factory:basisLifecycleFixture,fields:['contract','operation','invocation','profile','create','remove','targets','dependencies'],route:'basisLifecycle'}
];
function prepare(f,edit){
 const x=f.factory(kernel),h=typeof x.host==='function'?x.host():x.g(),c=x.controlContext;
 const issued=kernel.issueInvocation(x.domain,c,h);assert(issued);
 const r={...copy(x.request),invocation:issued.reference};if(edit)edit(r);
 const before=x.root();let plan;
 if(f.route==='settings')plan=kernel.prepareInvocationSemanticSettings(x.domain,issued.ticket,{route:'typed',request:{family:15,variant:f.variant,version:1,arguments:r}},x.p,c,h);
 else if(f.route==='scopedSettings')plan=kernel.prepareInvocationScopedSemanticSettings(x.domain,issued.ticket,{route:'typed',request:{family:15,variant:f.variant,version:1,arguments:r}},x.refs,x.p,c,h);
 else if(f.route==='native')plan=kernel.prepareInvocationRegistrationIntent(x.domain,issued.ticket,r,x.profile,c,h);
 else if(f.route==='serialization')plan=kernel.prepareInvocationSourceSerialization(x.domain,issued.ticket,{route:'typed',request:{family:15,variant:f.variant,version:1,arguments:r}},x.p,c,h);
 else if(f.route==='scopedDefinition')plan=kernel.prepareInvocationScopedSettingsDefinition(x.domain,issued.ticket,{route:'typed',request:{family:15,variant:f.variant,version:1,arguments:r}},x.basis,x.p,c,h);
 else if(f.route==='scopedLifecycle')plan=kernel.prepareInvocationScopedSettingsLifecycle(x.domain,issued.ticket,{route:'typed',request:{family:15,variant:f.variant,version:1,arguments:r}},x.basis,x.p,c,h);
 else if(f.route==='scopedSerialization')plan=kernel.prepareInvocationScopedSourceSerialization(x.domain,issued.ticket,{route:'typed',request:{family:15,variant:f.variant,version:1,arguments:r}},x.basis,x.p,c,h);
 else if(f.route==='basisLifecycle')plan=kernel.prepareInvocationBasisSettingsLifecycle(x.domain,issued.ticket,{route:'typed',request:{family:15,variant:f.variant,version:1,arguments:r}},x.lifecycleBasis,[x.writeBasis],x.p,c,h);
 else plan=x.prepare(r,x.profile,x.refs,x.scope,x.g(),x.owner,issued.ticket);
 assert.equal(x.root(),before);return {request:r,accepted:Boolean(plan)};
}
function fixture(api=discovery,options={}){
 const x=nativeDiscoveryFixture(api,undefined,options),scope={...x.scope,profile},a=x.api;
 const rows=()=>a.nativeBuiltinDiscoveryRows(x.owner,scope,x.g());
 const packet=(o={})=>a.prepareNativeBuiltinDiscoveryCut(x.owner,o.scope??scope,x.visibility(rows()??[]),null,null,o.g??x.g());
 const open=(o={})=>{const p=packet(o);assert(p);const r=a.acquireNativeDiscovery(x.owner,o.scope??scope,p,x.g());a.retireNativeDiscoveryCut(x.owner,p,x.g());return r;};
 const query=(context,request,o={})=>{const p=o.packet??packet();assert(p);const r=a.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile,context,...request},o.scope??scope,p,o.g??x.g());a.retireNativeDiscoveryCut(x.owner,p,x.g());return r;};
 return {...x,scope,rows,packet,open,query};
}
function frozen(v){if(v&&typeof v==='object'){assert(Object.isFrozen(v));Object.values(v).forEach(frozen);}}
export function checkSchemaDiscovery(api=discovery,descriptor=contextEngineDescriptor){
 const controls=[],control=(name,f)=>{f();controls.push(name);};
 control('eleven-handler-forms-share-exact-frozen-declarations',()=>{
  const declarations=[...engineSettingsInputForms(),...registrationInputForms(),sourceSerializationInputForm(),scopedSourceSerializationInputForm()];assert.equal(declarations.length,11);
  const d=descriptor();frozen(d);assert.equal(d.family,'F16');assert.equal(d.variants.length,2);
  for(const f of forms){const b=prepare(f);assert(b.accepted);const decl=declarations.find(v=>v.contract===b.request.contract);assert(decl);frozen(decl);
   assert.equal(decl.variant,f.variant);assert.equal(decl.operation,b.request.operation);assert.deepEqual([...decl.fields].sort(),f.fields.toSorted());
   const v=d.variants.find(v=>v.id===f.variant);assert(v);assert.equal(v.input.coverage,'outer-structure-only');
   const branch=v.input.schema.oneOf.find(b=>b.properties.contract.const===decl.contract);assert(branch);assert.deepEqual(branch.required.toSorted(),f.fields.toSorted());assert.equal(branch.properties.operation.const,b.request.operation);
  }
  assert.notEqual(descriptor(),d);assert.deepEqual(d.variants[0].outcomes.qualifiedLocalTerminalForms,[{publication:'applied',control:'done'},{publication:'unchanged',control:'done'},{publication:'notApplied',control:'rejected'},{publication:'notApplied',control:'stopped'},{publication:'unknown',control:'faulted'}]);
 });
 control('explicit-profile-adds-f16-without-changing-legacy-source',()=>{
  const x=fixture(api),old=x.api.nativeBuiltinDiscoveryRows(x.owner,{...x.scope,profile:'owned-contract-discovery/1'},x.g());assert.equal(old.length,2);
  const rows=x.rows();assert.equal(rows.length,3);assert.deepEqual(rows.slice(0,2),old);assert.equal(rows[2].variants.length,2);assert.deepEqual(rows[2].variants,descriptor().variants);
  const r=x.open();assert.equal(r.status,'context');const page=x.query(r.context,{operation:'list',limit:3,continuation:null});assert.equal(page.status,'page');assert.equal(page.entries.length,3);
  const d=x.query(r.context,{operation:'inspect',entry:page.entries[2].reference});assert.equal(d.status,'descriptor');
  assert.deepEqual(d.descriptor.variants.map(v=>v.input),rows[2].variants.map(v=>v.input));
 });
 control('profile-mismatch-cannot-consume-or-poison-a-captured-context',()=>{
  const x=fixture(api),r=x.open(),p=x.packet(),before=x.usage();
  assert.deepEqual(x.query(r.context,{operation:'list',limit:3,continuation:null,profile:'owned-contract-discovery/1'},{packet:p}),no);
  assert.equal(x.usage().issued,before.issued);assert.equal(x.usage().stopped,0);
  assert.equal(x.query(r.context,{operation:'list',limit:3,continuation:null}).status,'page');
  assert.equal(x.api.nativeBuiltinDiscoveryRows(x.owner,{...x.scope,profile:'owned-contract-discovery/1099'},x.g()),null);
 });
 control('prepared-cut-stays-bound-to-every-context-field-and-origin',()=>{
  const x=fixture(api),r=x.open();
  for(const field of Object.keys(x.scope)){
   const p=x.packet(),scope={...x.scope,[field]:'foreign-'+field},before=x.usage();
   assert.deepEqual(x.api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile:scope.profile,context:r.context,operation:'list',limit:3,continuation:null},scope,p,x.g()),no);
   assert.equal(x.usage().issued,before.issued);assert.equal(x.usage().stopped,0);
   assert.equal(x.query(r.context,{operation:'list',limit:3,continuation:null},{packet:p}).status,'page');
  }
 });
 control('schema-profile-neither-bypasses-origin-nor-recipient-release',()=>{
  const x=fixture(api),r=x.open(),rows=x.rows(),p=x.api.prepareNativeDiscoveryCut(x.owner,x.scope,rows,x.visibility(rows),null,null,x.g());assert(p);
  assert.deepEqual(x.query(r.context,{operation:'list',limit:3,continuation:null},{packet:p}),no);assert.equal(x.usage().stopped,0);
  assert.equal(x.query(r.context,{operation:'list',limit:3,continuation:null}).status,'page');
  assert.deepEqual(x.query(r.context,{operation:'list',limit:3,continuation:null},{g:x.g({authorized:false})}),no);assert.equal(x.usage().stopped,1);
  assert.deepEqual(x.query(r.context,{operation:'list',limit:3,continuation:null}),no);
 });
 control('schema-descriptions-respect-independent-disclosure',()=>{
  const x=fixture(api,{schema:false}),r=x.open(),p=x.query(r.context,{operation:'list',limit:3,continuation:null});
  const d=x.query(r.context,{operation:'inspect',entry:p.entries[2].reference});assert.equal(d.status,'descriptor');
  for(const v of d.descriptor.variants)assert.deepEqual(Object.keys(v),['reference']);
 });
 control('discovery-preserves-native-state-and-advisory-unknown',()=>{
  const x=fixture(api),root=x.z.root(),usage=copy(x.z.usage()),r=x.open(),p=x.query(r.context,{operation:'list',limit:3,continuation:null}),d=x.query(r.context,{operation:'inspect',entry:p.entries[2].reference});
  assert.deepEqual(x.query(r.context,{operation:'advisory-enablement',entry:p.entries[2].reference,variant:d.descriptor.variants[0].reference,target:null}),no);
  assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),usage);assert.equal(x.usage().stopped,0);
 });
 control('stop-retirement-does-not-refund-cross-profile-lifetime-issuance',()=>{
  const x=fixture(api,{limits:{maxIssued:1}}),r=x.open();assert.equal(r.status,'context');
  assert.equal(x.api.stopNativeDiscovery(x.owner,r.context,x.scope,x.g()).status,'stopped');
  assert.equal(x.api.retireNativeDiscovery(x.owner,r.context,x.g()).status,'retired');assert.equal(x.usage().issued,1);
  const scope={...x.scope,profile:'owned-contract-discovery/1'},rows=x.api.nativeBuiltinDiscoveryRows(x.owner,scope,x.g()),p=x.api.prepareNativeBuiltinDiscoveryCut(x.owner,scope,x.visibility(rows),null,null,x.g());assert(p);
  assert.deepEqual(x.api.acquireNativeDiscovery(x.owner,scope,p,x.g()),no);assert.equal(x.usage().issued,1);
 });
 control('schema-bounds-remain-independent-and-own-no-new-issuer',()=>{
  for(const limits of [{maxEntries:2},{maxVariants:2},{maxMetadataDepth:3},{maxMetadataNodes:16},{maxMetadataBytes:32},{maxRowBytes:32},{maxPacketBytes:32}])assert.equal(fixture(api,{limits}).packet(),null,JSON.stringify(limits));
  const x=fixture(api,{limits:{maxContexts:1}}),r=x.open();assert.equal(r.status,'context');
  const oldScope={...x.scope,profile:'owned-contract-discovery/1'},oldRows=x.api.nativeBuiltinDiscoveryRows(x.owner,oldScope,x.g()),p=x.api.prepareNativeBuiltinDiscoveryCut(x.owner,oldScope,x.visibility(oldRows),null,null,x.g());assert(p);
  assert.deepEqual(x.api.acquireNativeDiscovery(x.owner,oldScope,p,x.g()),no);
 });
 control('legacy-profile-rejects-schema-inputs-and-malformed-shapes-refuse',()=>{
  const x=fixture(api),rows=x.rows(),generic=(r,c=x.scope)=>x.api.prepareNativeDiscoveryCut(x.owner,c,r,x.visibility(r),null,null,x.g());
  assert.equal(generic(rows,{...x.scope,profile:'owned-contract-discovery/1'}),null);
  const edits=[v=>v.input=null,v=>v.input.schema.oneOf=[],v=>v.input.schema.$ref='https://invalid.example/schema',v=>v.input.schema.oneOf.push(copy(v.input.schema.oneOf[0])),v=>v.input.schema.oneOf[0].required.pop(),v=>v.input.schema.oneOf[0].additionalProperties=true,v=>v.input.schema.oneOf[0].properties.invocation.type='number',v=>v.input.schema.oneOf[0].properties.request={type:'object',$ref:'#/remote'},v=>v.input.coverage='complete-value-codec',v=>v.input.schema.$schema='https://invalid.example/dialect'];
  for(const edit of edits){const bad=copy(rows);edit(bad[2].variants[1]);assert.equal(generic(bad),null);}
  assert.equal(x.usage().preparations,0);assert.equal(x.usage().issued,0);
 });
 control('schema-recognition-is-a-closed-metadata-profile-not-a-loader',()=>{
  for(const v of descriptor().variants)assert(outerInputSchemaValid(v.input.schema));
  for(const schema of [null,{},{$schema:'https://json-schema.org/draft/2020-12/schema',oneOf:[]}])assert.equal(outerInputSchemaValid(schema),false);
  assert.equal(outerInputSchema([]),null);assert.equal(outerInputSchema([{contract:'x',operation:'x',fields:['contract','operation','not-a-field']}]),null);
 });
 return {scope:'selected-handler-bound-outer-schema-discovery-only',controls,handlerForms:11,variants:2,profileVersioningChecked:true,ownedSchemaReleaseChecked:true,semanticEnablementOrCompleteCodecProved:false,publicWireOrAllFamilyDispatchProved:false,physicalHostOrIndependentRefinementProved:false};
}
export function checkSchemaHandlerProducts(){
 const baseline=forms.map(f=>prepare(f)),cases=[];let omissions=0,extras=0,contextCounterexamples=0;
 for(let i=0;i<forms.length;i++){const f=forms[i];assert(baseline[i].accepted);cases.push({variant:f.variant,request:baseline[i].request,expected:true});
  for(const field of f.fields){const b=prepare(f,r=>delete r[field]);assert.equal(b.accepted,false);cases.push({variant:f.variant,request:b.request,expected:false});omissions++;}
  const b=prepare(f,r=>r.keymap='ui');assert.equal(b.accepted,false);cases.push({variant:f.variant,request:b.request,expected:false});extras++;
 }
 for(const f of forms.filter(f=>['settings','scopedSettings','serialization','scopedDefinition','scopedLifecycle','scopedSerialization','basisLifecycle'].includes(f.route))){const b=prepare(f,r=>r.profile='another-execution-context');assert.equal(b.accepted,false);cases.push({variant:f.variant,request:b.request,expected:true});contextCounterexamples++;}
 const schemas=Object.fromEntries(contextEngineDescriptor().variants.map(v=>[v.id,v.input.schema]));
 assert.equal(Object.keys(schemas).length,2);assert.equal(validateStandardSchemas(cases.map(c=>({schema:schemas[c.variant],value:c.request,valid:c.expected}))),105);
 return {valid:baseline.length,omissions,extras,contextCounterexamples,standardValidatorCases:cases.length};
}
const mutations=[
 ['legacy-schema-downgrade','native-discovery.mjs','[catalog.schemaProfile,catalog.editingSchemaProfile,catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(profile)&&exact(input,','true&&exact(input,'],
 ['ignore-query-profile','native-discovery.mjs','request.profile!==c?.profile','false'],
 ['admit-arbitrary-schema','native-discovery.mjs','outerInputSchemaValid(input.schema)','true'],
 ['omit-f16-source','native-discovery.mjs','[catalog.schemaProfile,catalog.editingSchemaProfile,catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)?[contextEngineDescriptor()]:[]','[catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)?[contextEngineDescriptor()]:[]'],
 ['descriptor-field-removed','context-engine-descriptor.mjs','const input=outerInputSchema(forms.filter(f=>f.variant===id))','const input=outerInputSchema(forms.filter(f=>f.variant===id).map(f=>({...f,fields:f.fields.filter(k=>k!=="invocation")})))'],
 ['descriptor-variant-substituted','context-engine-descriptor.mjs','rows.push({id,input:','rows.push({id:id==="semantic-settings"?"supported-registry":id,input:'],
 ['descriptor-schema-coverage-inflated','context-engine-descriptor.mjs','coverage:inputSchemaCoverage,schema:input','coverage:"complete-value-codec",schema:input']
];
export async function checkSchemaDiscoveryMutations(){
 const directory=new URL('.',import.meta.url),source=name=>fs.readFileSync(new URL(name,directory),'utf8'),url=(name,s)=>'data:text/javascript;base64,'+Buffer.from(s.replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,new URL(name,directory)).href+q)).toString('base64');
 const load=async m=>{let ds=source('context-engine-descriptor.mjs'),ns=source('native-discovery.mjs');if(m){const current=m[1]==='native-discovery.mjs'?ns:ds;assert(current.includes(m[2]),m[0]);if(m[1]==='native-discovery.mjs')ns=ns.replace(m[2],m[3]);else ds=ds.replace(m[2],m[3]);}
  const du=url('context-engine-descriptor.mjs',ds),d=await import(du);ns=ns.replace("'./context-engine-descriptor.mjs'",JSON.stringify(du));return {api:await import(url('native-discovery.mjs',ns)),descriptor:d.contextEngineDescriptor};};
 const baseline=await load();checkSchemaDiscovery(baseline.api,baseline.descriptor);const killed=[];
 for(const m of mutations){const x=await load(m);let failed=false;try{checkSchemaDiscovery(x.api,x.descriptor);}catch(e){assert.equal(e.code,'ERR_ASSERTION',m[0]+': '+String(e.message).slice(0,120));failed=true;}assert(failed,m[0]);killed.push(m[0]);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify({...checkSchemaDiscovery(),...checkSchemaHandlerProducts(),mutationBaselineChecked:true,mutations:await checkSchemaDiscoveryMutations()}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
