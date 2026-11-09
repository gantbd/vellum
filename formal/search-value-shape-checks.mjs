import assert from 'node:assert/strict';
import {validateStandardSchemas} from './standard-schema-validator.mjs';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {normalizeSearchArguments,searchArgumentValueFields,sameSearchArguments} from './search-arguments.mjs';
import {searchEngineDescriptor} from './search-engine-descriptor.mjs';
import {nativeSourceSearchFixture as fixture} from './native-source-search-checks.mjs';
const limits={maxRequestBytes:8192,maxInputNodes:4096,maxInputDepth:128};
export function checkSearchValueShapes(){const fields=searchArgumentValueFields();assert.deepEqual(fields,{ref:['authority','document','incarnation','revision'],scope:['lo','hi'],navigation:['start','current','wrap']});for(const value of [fields,...Object.values(fields)])assert(Object.isFrozen(value));let normalizations=0,canonicalPairs=0,runtimeCounterexamples=0;const cases=[],descriptor=searchEngineDescriptor();
 for(const operation of ['literal-matches','next-match','previous-match']){const x=fixture(),r=x.request(operation),c=api.invocationAuthorityContext(x.x.z.domain),schema=descriptor.variants.find(v=>v.id===operation).input.schema,rows=[
  [r,true,true],[{...r,scopes:[]},true,true],[{...r,context:{}},true,true],[{...r,ref:{...r.ref,incarnation:0}},true,true],
  [{...r,ref:{}},false,false],[{...r,ref:{...r.ref,extra:true}},false,false],[{...r,ref:{...r.ref,authority:null}},false,false],[{...r,ref:{...r.ref,document:''}},false,false],[{...r,ref:{...r.ref,incarnation:-1}},false,false],[{...r,ref:{...r.ref,revision:1.5}},false,false],[{...r,ref:{...r.ref,revision:Number.MAX_SAFE_INTEGER+1}},false,false],
  [{...r,scopes:null},false,false],[{...r,scopes:[null]},false,false],[{...r,scopes:[{lo:'0',hi:1}]},false,false],[{...r,scopes:[{lo:0,hi:1,extra:true}]},false,false],[{...r,scopes:[{lo:-1,hi:1}]},false,false],
  [{...r,navigation:operation==='literal-matches'?{}:null},false,false],
  [{...r,ref:{...r.ref,authority:'foreign'}},true,false],[{...r,scopes:[{lo:2,hi:1}]},true,false],[{...r,query:'\ud800'},true,false]
 ];
 for(const field of fields.ref){const ref={...r.ref};delete ref[field];rows.push([{...r,ref},false,false]);}
 for(const field of fields.scope){const scope={lo:0,hi:1};delete scope[field];rows.push([{...r,scopes:[scope]},false,false]);}
 if(operation!=='literal-matches'){
  for(const navigation of [{...r.navigation,current:-1},{...r.navigation,start:1.5},{...r.navigation,wrap:1},{...r.navigation,extra:true}])rows.push([{...r,navigation},false,false]);
  rows.push([{...r,navigation:{...r.navigation,current:100}},true,true]);for(const field of fields.navigation){const navigation={...r.navigation};delete navigation[field];rows.push([{...r,navigation},false,false]);}
 }
 for(const [value,schemaValid,normalized]of rows){const outputs=[];for(const route of ['typed','dynamic']){const out=normalizeSearchArguments({route,request:{...(route==='typed'?{family:4}:{name:'search-matches'}),variant:operation,version:1,arguments:value}},c,limits,limits.maxRequestBytes);assert.equal(out!==null,normalized,operation);outputs.push(out);normalizations++;}if(normalized){assert.equal(sameSearchArguments(...outputs),true);canonicalPairs++;}if(schemaValid&&!normalized)runtimeCounterexamples++;cases.push({schema,value,valid:schemaValid});}
 }
 assert.equal(validateStandardSchemas(cases),94);assert.equal(normalizations,188);assert.equal(canonicalPairs,14);assert.equal(runtimeCounterexamples,9);
 return {scope:'isolated-shared-search-nested-value-shapes',validatorCases:cases.length,normalizations,canonicalPairs,runtimeCounterexamples,opaqueAuthenticityScalarOrderingOrAuthorityProvedBySchema:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSearchValueShapes()));
