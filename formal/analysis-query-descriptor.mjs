// Handler-owned metadata for the existing accepted-registry discovery route.
// Descriptive schemas are not a native snapshot codec or an execution grant.
import {analysisQueryArgumentForms} from './analysis-query-arguments.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const text={type:'string',minLength:1},natural={type:'integer',minimum:0,maximum:Number.MAX_SAFE_INTEGER};
export function analysisQueryInputSchema(operation){
 const form=analysisQueryArgumentForms()?.find(f=>f.operation===operation);if(!form)return null;
 const properties={contract:{const:form.contract},operation:{const:operation},snapshot:{type:'object'},
  view:{type:'object',properties:{authority:text,id:text,incarnation:natural},required:['authority','id','incarnation'],additionalProperties:false},
  range:{type:'array',minItems:2,maxItems:2,prefixItems:[natural,natural],items:false}};
 return freeze({$schema:inputSchemaDialect,type:'object',properties:Object.fromEntries(form.fields.map(k=>[k,properties[k]])),required:[...form.fields],additionalProperties:false});
}
export function analysisQueryEngineDescriptor(){
 const forms=analysisQueryArgumentForms();if(!forms)return null;
 return freeze({family:'F08',contract:'analysis-query/1',version:1,kind:'query',variants:forms.map(f=>{
  const schema=analysisQueryInputSchema(f.operation);
  return {id:f.operation,input:{contract:f.contract,variant:f.operation,fields:schema.properties},
   outcomes:{statusVocabulary:[f.operation==='supported-analysis'?'captures':'analysis-summary','unavailable'],effect:'no-authored-or-provider-effect',execution:'synchronous-bounded-native-analysis-query',events:'no-asynchronous-event-in-this-query-profile',receipt:'immediate-query-value-not-a-retained-command-receipt'},
   requirements:{inputCoverage:inputSchemaCoverage,schema,runtimeBindings:['original-owned-analysis-snapshot','actual-domain-caller-scope-and-current-permission','current-source-provider-analysis-generations-and-view-attachment','scalar-byte-boundaries-range-order-and-source-extent','actual-decoder-and-native-query-workspace-and-release-limits'],notEstablished:['opaque-identity-or-permission-from-schema','enabled-provider-or-installed-handler-from-description','F16-activation-F24-task-retry-or-F28-evidence-binding','wire-encoding-physical-or-independent-refinement']},
   limits:{names:['maxQueryBytes','maxReplyBytes','maxHeldBytes','maxRequestBytes','maxInputNodes','maxInputDepth'],values:'current-owner-and-explicit-caller-profile'},
   cancellation:'native-owned-stop-disables-query; retained-evidence-and-quiescence-remain-independent'};
 }),presentation:{}});
}
