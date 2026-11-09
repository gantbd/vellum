// Finite metadata about existing F01 forms, not a context issuer or evaluator.
import {isDeepStrictEqual as equal} from 'node:util';
import {nativeDiscoveryQueryInputForms} from './native-discovery.mjs';
import {nativeDiscoveryResultDeclarations} from './discovery-results.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const variants=['list','inspect','advisory-enablement'],contract='contract-discovery/1',opaque={type:'object'},text={type:'string',minLength:1},natural={type:'integer',minimum:1,maximum:Number.MAX_SAFE_INTEGER};
const record=(required,properties)=>({type:'object',properties,required:[...required],additionalProperties:false});
function declarations(){
 const inputs=nativeDiscoveryQueryInputForms(),results=nativeDiscoveryResultDeclarations();
 const expectedForms={page:['status','context','coverage','entries','continuation'],descriptor:['status','context','descriptor'],advice:['status','context','target','entry','variant','enabled'],unavailable:['status']};
 if(inputs.length!==3||!equal(inputs.map(f=>f.variant),variants)||inputs.some(f=>f.family!=='F01'||f.contract!==contract||f.operation!==f.variant)||!equal(results.forms,expectedForms)||!equal(results.entryFields,['reference','version','kind'])||!equal(results.descriptorFields,['reference','version','kind','variants'])||!equal(results.facets,['input','outcomes','requirements','limits','cancellation'])||!equal(results.kinds,['query','localCommand','effectRequest','effectObservation'])||!equal(results.presentation,['title','description','category','aliases','bindings'])||results.coverage!=='captured-permitted')return null;
 return {inputs,results};
}
function input(id){const d=declarations(),f=d?.inputs.find(f=>f.variant===id);if(!f)return null;
 const properties={contract:{const:contract},operation:{const:id},context:opaque,profile:text,limit:natural,continuation:{oneOf:[{type:'null'},opaque]},entry:opaque,variant:opaque,target:opaque};
 if(new Set(f.fields).size!==f.fields.length||f.fields.some(k=>!Object.hasOwn(properties,k)))return null;
 return freeze({$schema:inputSchemaDialect,...record(f.fields,Object.fromEntries(f.fields.map(k=>[k,properties[k]])))});
}
function result(id){
 const d=declarations();if(!d||!variants.includes(id))return null;const r=d.results,kind={enum:[...r.kinds]},version=natural;
 const entry=record(r.entryFields,{reference:opaque,version,kind}),hidden=record(['reference'],{reference:opaque}),visible=record(['reference',...r.facets],Object.fromEntries(['reference',...r.facets].map(k=>[k,k==='reference'?opaque:{}])));
 // Metadata slots are inert JSON. Their UTF-8/node/depth/byte constraints and
 // original reference correlations remain native; do not expand self schemas.
 const descriptor=record(r.descriptorFields,{reference:opaque,version,kind,variants:{type:'array',items:{oneOf:[hidden,visible]}},presentation:record([],Object.fromEntries(r.presentation.map(k=>[k,{}])))});
 const status=id==='list'?'page':id==='inspect'?'descriptor':'advice',properties={status:{const:status},context:opaque,...(id==='list'?{coverage:{const:r.coverage},entries:{type:'array',items:entry},continuation:{oneOf:[{type:'null'},opaque]}}:id==='inspect'?{descriptor}:{target:opaque,entry:opaque,variant:opaque,enabled:{type:'boolean'}})};
 return freeze({$schema:inputSchemaDialect,oneOf:[record(r.forms.unavailable,{status:{const:'unavailable'}}),record(r.forms[status],properties)]});
}
function outcomes(id){const schema=result(id);return schema?freeze({forms:[id==='list'?'page':id==='inspect'?'descriptor':'advice','unavailable'],result:{coverage:'existing-native-structure-with-native-inert-metadata-bounds',schema},effect:'none',execution:'synchronous-bounded-current-release',notEstablished:['original-selector-authority-or-context-acquisition','semantic-enablement-or-handler-registration-truth','metadata-as-code-schema-evaluator-or-transport','complete-all-family-products-or-asynchronous-events']}):null;}
export function discoverySelfDescriptorInputForms(){return declarations()?.inputs??null;}
export function discoverySelfInputSchemaValid(value,c,id){const expected=c===contract?input(id):null;return expected!==null&&equal(value,expected);}
export function discoverySelfOutcomesValid(value,c,id){const expected=c===contract?outcomes(id):null;return expected!==null&&equal(value,expected);}
export function discoverySelfEngineDescriptor(){
 const rows=variants.map(id=>({id,input:{contract,variant:id,coverage:inputSchemaCoverage,schema:input(id)},outcomes:outcomes(id),requirements:{inputCoverage:inputSchemaCoverage,currentPredicates:'original-owned-discovery-context-and-current-recipient-captured-cut-release',opaqueRoles:id==='list'?['original-context','original-or-null-continuation']:id==='inspect'?['original-context','original-entry']:['original-context','original-entry','original-variant','original-target'],notEstablished:['request-dependent-reference-equality-or-row-cardinality-from-schema','metadata-budget-or-original-owner-authenticity-from-shape','context-acquisition-or-public-handle-codec']},limits:{values:'existing-parent-discovery-query-capture-reply-selector-and-metadata-profiles'},cancellation:'protected-observer-stop-and-retirement; never-producer-cancellation'}));
 return rows.every(r=>r.input.schema&&r.outcomes)?freeze({family:'F01',contract,version:1,kind:'query',variants:rows,presentation:{}}):null;
}
