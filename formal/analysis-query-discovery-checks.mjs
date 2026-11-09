import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as analysis from './analysis-work.mjs';
import * as discovery from './native-discovery.mjs';
import * as kernel from './invocation-publication.mjs';
import {analysisQueryEngineDescriptor,analysisQueryInputSchema} from './analysis-query-descriptor.mjs';
import {analysisOwnerBindingFixture} from './analysis-owner-binding-checks.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {validateStandardSchemas} from './standard-schema-validator.mjs';
const copy=structuredClone,no={status:'unavailable'},limits={maxRequestBytes:4096,maxInputNodes:4096,maxInputDepth:128};
function fixture(){
 const x=analysisOwnerBindingFixture(),a=x.ready(),template=nativeDiscoveryFixture(),c=kernel.invocationAuthorityContext(x.z.domain),scope={...c,purpose:'discover-contracts',policy:'analysis-discovery-policy',profile:'owned-contract-discovery/1'},g=template.g;
 const owner=discovery.openNativeDiscoveryRegistry(x.z.domain,template.l,g());assert(owner);
 const {family,...descriptor}=analysisQueryEngineDescriptor();assert.equal(family,'F08');const rows=[{command:{authority:c.authority,id:'accepted-analysis-query-handler',incarnation:1},...descriptor}];
 const target={identity:'analysis-query-target',basis:{document:copy(x.c.document),source:x.c.sourceRevision,analysis:copy(x.c.analysis)}};
 const prepare=(options={})=>{const selected=options.rows??rows;return discovery.prepareNativeDiscoveryCut(owner,scope,selected,options.visibility??template.visibility(selected),target,template.advice(selected),options.g??g());};
 const open=(options={})=>{const p=prepare(options);assert(p);try{return discovery.acquireNativeDiscovery(owner,scope,p,g());}finally{discovery.retireNativeDiscoveryCut(owner,p,g());}};
 const query=(context,request,options={})=>{const p=prepare(options);assert(p);try{return discovery.queryNativeDiscovery(owner,{contract:'contract-discovery/1',profile:scope.profile,context,...request},scope,p,options.queryG??g());}finally{discovery.retireNativeDiscoveryCut(owner,p,g());}};
 const request=(operation='supported-analysis')=>{const r=x.query(a.snapshot);r.operation=operation;if(operation==='supplied-evidence-inspection')delete r.range;return r;};
 const run=(r=request(),route='typed',facts=x.g)=>analysis.queryDecodedInvocationAnalysis(x.owner,{route,request:{...(route==='typed'?{family:7}:{name:'read-derived-analysis'}),variant:r.operation,version:1,arguments:r}},limits,c,facts);
 return {x,a,c,scope,g,owner,rows,prepare,open,query,request,run};
}
export function checkAnalysisQueryDiscovery(){
 const controls=[],cases=[];let nativeCases=0,runtimeSchemaRefusals=0;
 const check=(name,fn)=>{fn();controls.push(name);};
 check('fixed-declaration-uses-native-fields-and-standard-shape-only',()=>{
  const d=analysisQueryEngineDescriptor(),expected={'supported-analysis':['contract','operation','snapshot','view','range'],'supplied-evidence-inspection':['contract','operation','snapshot','view']};
  assert.equal(d.family,'F08');assert.equal(d.kind,'query');assert.equal(d.version,1);assert.deepEqual(d.presentation,{});assert.equal(d.variants.length,2);
  for(const v of d.variants){assert.deepEqual(Object.keys(v.input.fields),expected[v.id]);assert.deepEqual(v.requirements.schema.required,expected[v.id]);assert(Object.isFrozen(v.requirements.schema.properties));assert.equal(v.requirements.schema.additionalProperties,false);assert.deepEqual(v.outcomes.statusVocabulary,[v.id==='supported-analysis'?'captures':'analysis-summary','unavailable']);assert.equal(v.outcomes.effect,'no-authored-or-provider-effect');assert.equal(v.outcomes.events,'no-asynchronous-event-in-this-query-profile');assert.equal(v.outcomes.receipt,'immediate-query-value-not-a-retained-command-receipt');}
  assert.equal(analysisQueryInputSchema('unknown'),null);assert.notEqual(analysisQueryEngineDescriptor(),d);
 });
 check('actual-f01-on-same-domain-releases-both-handler-declarations',()=>{
  const z=fixture(),before=z.x.root(),parent=copy(kernel.invocationUsage(z.x.z.domain)),usage=copy(analysis.analysisUsage(z.x.owner)),context=z.open();assert.equal(context.status,'context');
  const page=z.query(context.context,{operation:'list',limit:1,continuation:null});assert.equal(page.status,'page');assert.equal(page.entries.length,1);
  const out=z.query(context.context,{operation:'inspect',entry:page.entries[0].reference});assert.equal(out.status,'descriptor');assert.equal(out.descriptor.kind,'query');assert.equal(out.descriptor.variants.length,2);
  for(const variant of out.descriptor.variants){const operation=variant.input.variant,r=z.request(operation);assert.deepEqual(Object.keys(r),Object.keys(variant.input.fields));assert.deepEqual(variant.requirements.schema,analysisQueryInputSchema(operation));for(const route of ['typed','dynamic']){assert.equal(z.run(r,route).status,operation==='supported-analysis'?'captures':'analysis-summary');nativeCases++;}}
  assert.equal(z.x.root(),before);assert.deepEqual(kernel.invocationUsage(z.x.z.domain),parent);assert.deepEqual(analysis.analysisUsage(z.x.owner),usage);
 });
 check('discovery-selectors-and-positive-advice-do-not-authorize-analysis',()=>{
  const z=fixture(),context=z.open(),entry=z.query(context.context,{operation:'list',limit:1,continuation:null}).entries[0].reference,descriptor=z.query(context.context,{operation:'inspect',entry}).descriptor,variant=descriptor.variants[0];
  assert.equal(z.query(context.context,{operation:'advisory-enablement',entry,variant:variant.reference,target:context.target}).enabled,true);
  assert.deepEqual(z.run({...z.request(),snapshot:entry}),no);assert.deepEqual(z.run({...z.request(),snapshot:variant.reference}),no);assert.deepEqual(z.run(z.request(),'typed',{...z.x.g,authorized:false}),no);
  assert.deepEqual(analysis.queryDecodedInvocationAnalysis({},z.rows,limits,z.c,z.x.g),no);
 });
 check('accepted-registry-and-schema-attestations-stay-required',()=>{
  const z=fixture(),before=discovery.nativeDiscoveryUsage(z.owner);
  for(const key of ['acceptedRegistryAttested','schemaDescriptionsAttested'])for(const value of [false,1,undefined])assert.equal(z.prepare({g:z.g({[key]:value})}),null);
  assert.deepEqual(discovery.nativeDiscoveryUsage(z.owner),before);assert.equal(z.open().status,'context');
 });
 check('unadvertised-and-withheld-metadata-do-not-widen-discovery',()=>{
  const z=fixture(),empty=z.open({rows:[]});assert.equal(empty.status,'context');assert.deepEqual(z.query(empty.context,{operation:'list',limit:1,continuation:null},{rows:[]}).entries,[]);
  const hidden=z.open({visibility:z.rows.map(row=>({command:copy(row.command),visible:false,schema:false,presentation:false,advice:false,order:null,variants:row.variants.map(v=>({id:v.id,visible:false,order:null}))}))});assert.equal(hidden.status,'context');assert.deepEqual(z.query(hidden.context,{operation:'list',limit:1,continuation:null}).entries,[]);
  const visibility=z.rows.map(row=>({command:copy(row.command),visible:true,schema:false,presentation:false,advice:false,order:0,variants:row.variants.map((v,i)=>({id:v.id,visible:true,order:i}))}));
  const context=z.open({visibility}),entry=z.query(context.context,{operation:'list',limit:1,continuation:null},{visibility}).entries[0].reference;
  const out=z.query(context.context,{operation:'inspect',entry},{visibility});assert.equal(out.status,'descriptor');assert(out.descriptor.variants.every(v=>!Object.hasOwn(v,'input')));
  const lost=z.open(),old=z.query(lost.context,{operation:'list',limit:1,continuation:null}).entries[0].reference;assert.deepEqual(z.query(lost.context,{operation:'inspect',entry:old},{visibility}),no);assert.deepEqual(z.query(lost.context,{operation:'inspect',entry:old}),no);
 });
 check('standard-schema-positives-and-malformed-shapes-are-independent',()=>{
  const z=fixture();
  for(const operation of ['supported-analysis','supplied-evidence-inspection']){
   const schema=analysisQueryInputSchema(operation),r=z.request(operation),add=(value,valid)=>cases.push({schema,value,valid});add(copy(r),true);
   for(const field of Object.keys(r)){const v=copy(r);delete v[field];add(v,false);}
   for(const v of [{...copy(r),ui:'keymap'},{...copy(r),contract:'foreign'},{...copy(r),snapshot:null},{...copy(r),view:{...r.view,incarnation:-1}},{...copy(r),view:{...r.view,extra:true}}])add(v,false);
   for(const v of [{...r,view:{...r.view,incarnation:r.view.incarnation+1}},{...r,snapshot:copy(r.snapshot)},{...r,view:{...r.view,id:'foreign'}}]){add(copy(v),true);assert.deepEqual(z.run(v),no);runtimeSchemaRefusals++;}
   if(operation==='supported-analysis'){
    add({...copy(r),range:[-1,0]},false);add({...copy(r),range:[0,1,2]},false);add({...copy(r),range:[0,0]},true);
    const backward={...r,range:[1,0]};add(copy(backward),true);assert.deepEqual(z.run(backward),no);runtimeSchemaRefusals++;
   }
  }
  assert.equal(cases.length,31);assert.equal(validateStandardSchemas(cases),31);
 });
 return {scope:'selected-F08-existing-accepted-registry-discovery',controls,nativeCases,standardSchemaCases:cases.length,runtimeSchemaRefusals,actualAnalysisAndDiscoveryDomainComposed:true,newBuiltinProfileAdded:false,metadataOrAdviceAsExecutionAuthority:false,actualInstalledRegistryProviderOrF24F28OrPhysicalOrIndependentProof:false};
}
export {fixture as analysisQueryDiscoveryFixture};
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkAnalysisQueryDiscovery()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
