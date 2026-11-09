import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {settingsQueryEngineDescriptor,settingsQueryInputSchemaValid,settingsQueryOutcomesValid} from './settings-query-descriptor.mjs';
import {settingsQueryInputForms} from './settings-query-declarations.mjs';
import {engineSettingsTypeDeclarations,compileEngineSettings} from './engine-settings-state.mjs';
import {settingsQueryResultValid} from './settings-query-results.mjs';
import {settingsGatewayFixture} from './settings-query-gateway-checks.mjs';
import {engineSettingsDefinition,engineSettingsProfile} from './engine-settings-checks.mjs';
import {validateStandardSchemas} from './standard-schema-validator.mjs';
const copy=structuredClone,operations=['effective-configuration','permitted-origin','capabilities'],limits={maxRequestBytes:65536,maxInputNodes:4096,maxInputDepth:128};
export function checkSettingsDiscoveryProducts(){
 const descriptor=settingsQueryEngineDescriptor();assert(descriptor);const variant=op=>descriptor.variants.find(v=>v.id===op),cases=[],controls=[],check=(name,f)=>{f();controls.push(name);},add=(schema,value,valid)=>cases.push({schema,value:JSON.parse(JSON.stringify(value)),valid});
 let nativeProducts=0,typeProducts=0,syntaxDenials=0,recognitionDenials=0,runtimeCounterexamples=0;
 const capSchema=variant('capabilities').outcomes.result.schema,baseDeclaration={key:'prose',type:{kind:'text'},domain:'source',scopes:['document'],composition:'replace',sensitivity:true},cap=declarations=>({status:'capabilities',context:{},target:{},coverage:'permitted-only',capabilities:declarations});
 const compileType=(type,value,patch={})=>{const d=engineSettingsDefinition();d.schemas=[{...baseDeclaration,type,default:value,...patch}];d.restrictions={};return compileEngineSettings(d,'a',engineSettingsProfile);};
 check('three-existing-operations-and-immutable-guard-bound-declarations',()=>{
  assert.equal(descriptor.family,'F07');assert.equal(descriptor.contract,'configuration-query/1');assert.equal(descriptor.kind,'query');assert.equal(descriptor.version,1);assert.deepEqual(descriptor.variants.map(v=>v.id),operations);
  const types=engineSettingsTypeDeclarations();assert(Object.isFrozen(types));for(const v of Object.values(types)){assert(Object.isFrozen(v));for(const a of Object.values(v))if(Array.isArray(a))assert(Object.isFrozen(a));}
  for(const f of settingsQueryInputForms()){const v=variant(f.operation);assert.deepEqual(v.input.schema.required,f.fields);assert(Object.isFrozen(v.input.schema));assert(Object.isFrozen(v.outcomes.result.schema));assert.equal(v.input.coverage,'outer-structure-only');assert.equal(v.outcomes.result.coverage,'native-value-structure-only');assert.equal(v.outcomes.result.scope,'native-operation-reply-only');assert.equal(v.outcomes.events,'no-asynchronous-events-in-this-synchronous-profile');assert.equal(v.outcomes.effect,'no-authored-or-provider-effect');assert.equal(settingsQueryInputSchemaValid(v.input.schema,descriptor.contract,v.id),true);assert.equal(settingsQueryOutcomesValid(v.outcomes,descriptor.contract,v.id),true);assert.equal(Object.hasOwn(v.outcomes.result.schema,'$defs'),f.operation==='capabilities');}
  assert.deepEqual(Object.keys(capSchema.$defs),['settingType']);assert.equal(capSchema.$defs.settingType.oneOf.length,6);
  const refs=[];function walk(v){if(v&&typeof v==='object'){if(Object.hasOwn(v,'$ref'))refs.push(v.$ref);Object.values(v).forEach(walk);}}walk(capSchema);assert.deepEqual(refs,['#/$defs/settingType','#/$defs/settingType','#/$defs/settingType']);
 });
 check('actual-parent-typed-dynamic-results-match-closed-json-products',()=>{
  for(const op of operations)for(const key of op==='capabilities'?['scalar']:['scalar','list','map'])for(const route of ['typed','dynamic']){const z=settingsGatewayFixture(op,key,route),r=z.query(),v=variant(op);assert.equal(r.status,op==='effective-configuration'?'value':op==='permitted-origin'?'origins':'capabilities');add(v.input.schema,z.envelope().request.arguments,true);add(v.outcomes.result.schema,r,true);add(v.outcomes.result.schema,{status:'unavailable'},true);assert(settingsQueryResultValid(r,op,z.ctx.context,z.ctx.target,key,limits));for(const field of Object.keys(r)){const bad={...r};delete bad[field];add(v.outcomes.result.schema,bad,false);syntaxDenials++;}add(v.outcomes.result.schema,{...r,extra:1},false);syntaxDenials++;for(const field of Object.keys(z.envelope().request.arguments)){const bad={...z.envelope().request.arguments};delete bad[field];add(v.input.schema,bad,false);syntaxDenials++;}add(v.input.schema,{...z.envelope().request.arguments,extra:1},false);syntaxDenials++;nativeProducts++;}
 });
 check('individual-input-and-reply-field-domains-refuse-wrong-json-kinds',()=>{
  for(const op of operations){const z=settingsGatewayFixture(op),request=z.envelope().request.arguments,r=z.query(),v=variant(op);
   for(const [field,values]of Object.entries({contract:['foreign',null],operation:['surprise',null],context:[null,1,'shadow',[]],target:[null,1,'shadow',[]],profile:['',1],...(op==='capabilities'?{}:{key:['',1]})}))for(const value of values){add(v.input.schema,{...request,[field]:value},false);syntaxDenials++;}
   for(const [field,values]of Object.entries({status:['surprise',null],context:[null,1,'shadow',[]],target:[null,1,'shadow',[]],...(op==='capabilities'?{coverage:['complete',null],capabilities:[null,{},1]}:op==='permitted-origin'?{key:['',1],coverage:['complete',null],origins:[null,1,[1],{field:[1]}]}:{key:['',1]})}))for(const value of values){add(v.outcomes.result.schema,{...r,[field]:value},false);syntaxDenials++;}
  }
 });
 check('all-six-types-nested-prose-and-code-shaped-values-have-real-native-producers',()=>{
  const examples=[['boolean',{kind:'boolean'},false],['text',{kind:'text'},'English / Deutsch / ```mermaid\ngraph TD; A-->B\n```'],['integer',{kind:'integer',min:-8,max:8},-2],['enum',{kind:'enum',values:[null,true,1,'prose',{a:1,b:2},[1,2]]},{b:2,a:1}],['list',{kind:'list',element:{kind:'map',fields:{text:{kind:'text'}}}},[{text:'prose'},{text:'code'}]],['map',{kind:'map',fields:{prose:{kind:'text'},tokens:{kind:'list',element:{kind:'integer',min:0,max:8}}}},{prose:'two domains',tokens:[1,2]}]];
  for(const [kind,type,value]of examples){assert(compileType(type,value),kind);add(capSchema,cap([{...baseDeclaration,type}]),true);for(const route of ['typed','dynamic']){const definition=engineSettingsDefinition();definition.schemas[0]={...definition.schemas[0],type,default:value};definition.restrictions={};const z=settingsGatewayFixture('capabilities','scalar',route,{definition}),r=z.query();assert.equal(r.status,'capabilities',kind);assert.deepEqual(r.capabilities[0].type,type);add(capSchema,r,true);typeProducts++;}}
  for(const type of [{kind:'map',fields:{}},{kind:'enum',values:[0,'0',false,null,{},[]]}]){const value=type.kind==='map'?{}:0;assert(compileType(type,value));add(capSchema,cap([{...baseDeclaration,type,composition:type.kind==='map'?'merge-map':'replace'}]),true);}
 });
 check('recursive-syntax-closed-vocabulary-and-declaration-fields-refuse-independently',()=>{
  const badTypes=[{}, {kind:'surprise'}, {kind:'boolean',extra:1},{kind:'text',extra:1},{kind:'integer',min:0},{kind:'integer',min:0,max:1.5},{kind:'integer',min:Number.MIN_SAFE_INTEGER-1,max:1},{kind:'integer',min:0,max:Number.MAX_SAFE_INTEGER+1},{kind:'integer',min:0,max:1,extra:1},{kind:'enum',values:[]},{kind:'enum',values:[0,-0]},{kind:'enum',values:[{a:1,b:2},{b:2,a:1}]},{kind:'enum',values:[1],extra:1},{kind:'list'},{kind:'list',element:{kind:'surprise'}},{kind:'list',element:{kind:'text'},extra:1},{kind:'map',fields:[]},{kind:'map',fields:{'':{kind:'text'}}},{kind:'map',fields:{child:{kind:'list',element:{kind:'surprise'}}}},{kind:'map',fields:{},extra:1}];
  for(const type of badTypes){add(capSchema,cap([{...baseDeclaration,type}]),false);assert.equal(compileType(type,null),null);syntaxDenials++;}
  const badDeclarations=[{key:''},{domain:'input'},{scopes:[]},{scopes:['document','document']},{scopes:['foreign']},{scopes:'document'},{composition:'append'},{composition:'merge-map'},{sensitivity:1},{default:'not disclosed'},...Object.keys(baseDeclaration).map(k=>({remove:k}))];
  for(const patch of badDeclarations){const d={...baseDeclaration,...patch};if(Object.hasOwn(patch,'remove')){delete d[patch.remove];delete d.remove;}add(capSchema,cap([d]),false);syntaxDenials++;}
  for(const op of operations){const schema=variant(op).outcomes.result.schema;add(schema,{status:'unavailable',reason:'private'},false);for(const other of operations.filter(x=>x!==op)){const z=settingsGatewayFixture(other),r=z.query();add(schema,r,false);syntaxDenials++;}}
  add(variant('permitted-origin').outcomes.result.schema,{status:'origins',context:{},target:{},key:'map',coverage:'permitted-only',origins:{left:[{}],right:[]}},true);
 });
 check('standard-shape-deliberately-does-not-establish-runtime-meaning',()=>{
  const reversed={kind:'integer',min:2,max:1};add(capSchema,cap([{...baseDeclaration,type:reversed}]),true);assert.equal(compileType(reversed,1),null);runtimeCounterexamples++;
  const duplicate=cap([baseDeclaration,{...baseDeclaration,sensitivity:false}]);add(capSchema,duplicate,true);assert.equal(settingsQueryResultValid(duplicate,'capabilities',duplicate.context,duplicate.target,'prose',limits),false);runtimeCounterexamples++;
  const z=settingsGatewayFixture(),r=z.query();add(variant('effective-configuration').outcomes.result.schema,{...r,value:'not this integer setting'},true);assert.equal(r.value,1);runtimeCounterexamples++;
  const cloned={...r,context:copy(r.context)};add(variant('effective-configuration').outcomes.result.schema,cloned,true);assert.equal(settingsQueryResultValid(cloned,'effective-configuration',r.context,r.target,'scalar',limits),false);runtimeCounterexamples++;
  add(capSchema,cap([{...baseDeclaration,type:{kind:'text'},key:'\ud800'}]),true);assert.equal(compileType({kind:'text'},'x',{key:'\ud800'}),null);runtimeCounterexamples++;
  const deep={kind:'list',element:{kind:'list',element:{kind:'text'}}};add(capSchema,cap([{...baseDeclaration,type:deep}]),true);const definition=engineSettingsDefinition();definition.schemas[0]={...definition.schemas[0],type:deep,default:[['text']]};definition.restrictions={};assert.equal(compileEngineSettings(definition,'a',{...engineSettingsProfile,maxSchemaDepth:1}),null);runtimeCounterexamples++;
 });
 check('recognizers-reject-loaders-widening-and-authority-substitution',()=>{
  for(const op of operations){const v=variant(op);for(const schema of [{...copy(v.input.schema),additionalProperties:true},{...copy(v.input.schema),$ref:'https://not-loaded.invalid/schema'},{...copy(v.input.schema),callback:()=>{throw Error('must not run');}}]){assert.equal(settingsQueryInputSchemaValid(schema,descriptor.contract,op),false);recognitionDenials++;}
   for(const out of [{...copy(v.outcomes),events:'public-events-complete'},{...copy(v.outcomes),result:{...copy(v.outcomes.result),coverage:'permission-proved'}},{...copy(v.outcomes),result:{...copy(v.outcomes.result),notEstablished:[]}}]){assert.equal(settingsQueryOutcomesValid(out,descriptor.contract,op),false);recognitionDenials++;}}
  for(const ref of ['https://not-loaded.invalid/types','#/$defs/other']){const out=copy(variant('capabilities').outcomes);out.result.schema.$defs.settingType.oneOf[4].properties.element.$ref=ref;assert.equal(settingsQueryOutcomesValid(out,descriptor.contract,'capabilities'),false);recognitionDenials++;}
 });
 validateStandardSchemas(cases);
 return {scope:'guard-bound-F07-settings-discovery-products',operations:3,inputSchemas:3,resultSchemas:3,typeKinds:6,validatorCases:cases.length,nativeProducts,typeProducts,syntaxDenials,recognitionDenials,runtimeCounterexamples,controls,localReferencesAreFixedMetadataNotAnEngineEvaluator:true,shapeIsNotRuntimeAuthority:true,allProductsHostPhysicalGlobalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSettingsDiscoveryProducts()));
