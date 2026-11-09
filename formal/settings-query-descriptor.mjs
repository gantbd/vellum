// Fixed F07 discovery data. Local references describe the existing type grammar;
// this module is not an engine schema evaluator, loader or authority mechanism.
import {isDeepStrictEqual as equal} from 'node:util';
import {settingsQueryInputForms} from './settings-query-declarations.mjs';
import {settingsQueryResultDeclarations} from './settings-query-results.mjs';
import {engineSettingsTypeDeclarations} from './engine-settings-state.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const contract='configuration-query/1',operations=['effective-configuration','permitted-origin','capabilities'],text={type:'string',minLength:1},opaque={type:'object'},safeInteger={type:'integer',minimum:Number.MIN_SAFE_INTEGER,maximum:Number.MAX_SAFE_INTEGER};
const record=(fields,properties)=>Array.isArray(fields)&&new Set(fields).size===fields.length&&fields.every(k=>Object.hasOwn(properties,k)&&properties[k]!==null)?{type:'object',properties:Object.fromEntries(fields.map(k=>[k,properties[k]])),required:[...fields],additionalProperties:false}:null;
function declarations(){
 const d=settingsQueryResultDeclarations(),t=engineSettingsTypeDeclarations();
 return equal(d.forms,{value:['status','context','target','key','value'],origins:['status','context','target','key','coverage','origins'],capabilities:['status','context','target','coverage','capabilities'],unavailable:['status']})&&equal(d.declarationFields,['key','type','domain','scopes','composition','sensitivity'])&&equal(t.forms,{boolean:['kind'],text:['kind'],integer:['kind','min','max'],enum:['kind','values'],list:['kind','element'],map:['kind','fields']})&&equal(t.domains,['source','serialization','analysis'])&&equal(t.scopes,['host','language','workspace','document','view'])&&equal(t.compositions,['replace','merge-map'])?{...d,types:t}:null;
}
function inputSchema(operation){
 const forms=settingsQueryInputForms().filter(f=>f.operation===operation);if(!operations.includes(operation)||forms.length!==1)return null;const f=forms[0];if(f.family!=='F07'||f.contract!==contract||f.variant!==operation)return null;
 const body=record(f.fields,{contract:{const:contract},operation:{const:operation},context:opaque,target:opaque,profile:text,key:text});return body&&freeze({$schema:inputSchemaDialect,...body});
}
function typeDefinitions(t){
 const ref={$ref:'#/$defs/settingType'},forms=Object.entries(t.forms).map(([kind,fields])=>record(fields,{kind:{const:kind},min:safeInteger,max:safeInteger,values:{type:'array',minItems:1,uniqueItems:true,items:{}},element:ref,fields:{type:'object',propertyNames:text,additionalProperties:ref}}));
 return forms.every(Boolean)?{settingType:{oneOf:forms}}:null;
}
const bindings=Object.freeze(['actual-owned-context-target-and-context-local-origin-identities','current-captured-policy-visibility-and-permitted-subset','current-key-type-value-applicability-and-declaration-order','native-schema-budget-integer-bound-order-and-declaration-key-uniqueness','actual-parent-decoder-reservation-and-release-before-disclosure']);
const limits=Object.freeze(['opaque-authenticity-or-identity-from-json-shadows','current-policy-or-complete-registry-from-permitted-only','integer-bound-order-or-distinct-declaration-keys-from-schema-shape','utf8-validity-or-runtime-byte-node-depth-budgets','generic-schema-evaluator-remote-reference-loading-or-wire-encoding','all-products-host-physical-global-or-independent-refinement']);
function resultMetadata(operation){
 const d=declarations();if(!d||!operations.includes(operation))return null;let success,defs;
 if(operation==='effective-configuration')success=record(d.forms.value,{status:{const:'value'},context:opaque,target:opaque,key:text,value:{}});
 else if(operation==='permitted-origin'){
  const origins={type:'array',items:opaque};success=record(d.forms.origins,{status:{const:'origins'},context:opaque,target:opaque,key:text,coverage:{const:'permitted-only'},origins:{oneOf:[origins,{type:'object',propertyNames:text,additionalProperties:origins}]}});
 }else{
  defs=typeDefinitions(d.types);if(!defs)return null;
  const declaration=record(d.declarationFields,{key:text,type:{$ref:'#/$defs/settingType'},domain:{enum:[...d.types.domains]},scopes:{type:'array',minItems:1,uniqueItems:true,items:{enum:[...d.types.scopes]}},composition:{enum:[...d.types.compositions]},sensitivity:{type:'boolean'}});if(!declaration)return null;
  declaration.allOf=[{if:{properties:{composition:{const:'merge-map'}},required:['composition']},then:{properties:{type:{properties:{kind:{const:'map'}},required:['kind']}}}}];
  success=record(d.forms.capabilities,{status:{const:'capabilities'},context:opaque,target:opaque,coverage:{const:'permitted-only'},capabilities:{type:'array',items:declaration}});
 }
 const unavailable=record(d.forms.unavailable,{status:{const:'unavailable'}});return success&&unavailable?freeze({coverage:'native-value-structure-only',scope:'native-operation-reply-only',schema:{$schema:inputSchemaDialect,...(defs?{$defs:defs}:{}),oneOf:[success,unavailable]},runtimeBindings:[...bindings],notEstablished:[...limits]}):null;
}
function outcomes(operation){const result=resultMetadata(operation);return result&&freeze({statusVocabulary:[operation==='effective-configuration'?'value':operation==='permitted-origin'?'origins':'capabilities','unavailable'],result,effect:'no-authored-or-provider-effect',retention:'owned-native-settings-context-and-context-local-origin-retention',execution:'synchronous-bounded-native-settings-query',events:'no-asynchronous-events-in-this-synchronous-profile',release:'actual-native-settings-reader-current-policy-and-captured-ceilings; known-loss-is-absorbing'});}
export function settingsQueryInputSchemaValid(value,c,operation){const expected=c===contract?inputSchema(operation):null;return expected!==null&&equal(value,expected);}
export function settingsQueryOutcomesValid(value,c,operation){const expected=c===contract?outcomes(operation):null;return expected!==null&&equal(value,expected);}
export function settingsQueryEngineDescriptor(){
 const variants=operations.map(id=>({id,input:{contract,variant:id,coverage:inputSchemaCoverage,schema:inputSchema(id)},outcomes:outcomes(id),requirements:{inputCoverage:inputSchemaCoverage,currentPredicates:'actual-owned-settings-context-original-target-current-permitted-capture-and-release',resources:'actual-parent-and-pool-reply-capture-and-explicit-decoder-workspace-reservations',notEstablished:['metadata-as-context-or-origin-authority','permitted-only-as-complete-registry','schema-shape-as-current-configuration-truth']},limits:{values:'actual-native-settings-profile-byte-node-depth-and-current-configuration-limits'},cancellation:'separate-owner-stop-or-expiry-ends-reading-only; never-settings-publication'}));
 return variants.every(v=>v.input.schema&&v.outcomes)?freeze({family:'F07',contract,version:1,kind:'query',variants,presentation:{}}):null;
}
