// Fixed handler-derived F17 metadata. No dispatcher, public result query or codec.
import {isDeepStrictEqual as equal} from 'node:util';
import {documentLifecycleInputForms} from './lifecycle-products.mjs';
import {lifecycleInvocationInputForm} from './lifecycle-invocation-state.mjs';
import {lifecycleActivationInputForms} from './lifecycle-activation-declarations.mjs';
import {lifecycleReceiptDeclarations} from './lifecycle-results.mjs';
import {invocationTerminalForms} from './invocation-publication.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
const contract='engine-lifecycle-declaration/1',variants=['create-scratch','activate-loaded-candidate','attach-detach-view','membership','rebind','dispose','view-container','workspace-container'];
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const object={type:'object'},text={type:'string',minLength:1},natural={type:'integer',minimum:0,maximum:Number.MAX_SAFE_INTEGER};
const properties={document:object,view:object,workspace:object,provider:object,resource:object,target:object,expected:natural,expectedLifecycle:natural,expectedAttachment:natural,expectedWorkspace:natural,expectedSource:natural,expectedAnnotations:natural,expectedSerialization:natural,expectedBinding:natural,expectedRegistration:natural,sourceRevision:natural,selections:{type:'array',items:object},primary:natural,source:{type:'array',items:{type:'integer',minimum:0,maximum:255}},serialization:object,insertionLineBreak:{enum:['LF','CRLF','CR']},present:{type:'boolean'},sourceDisposition:{enum:['retain','discard']},annotationDisposition:{enum:['retain','discard']},probe:object,context:object,binding:object};
const record=(fields,values)=>Array.isArray(fields)&&new Set(fields).size===fields.length&&fields.every(k=>Object.hasOwn(values,k)&&values[k]!==null)?{type:'object',properties:Object.fromEntries(fields.map(k=>[k,values[k]])),required:[...fields],additionalProperties:false}:null;
// Deliberately lazy: native guards participate in the owner import graph.
export function lifecycleDescriptorInputForms(){
 const wrapper=lifecycleInvocationInputForm();
 const forms=[...documentLifecycleInputForms().map(f=>({id:f.operation,family:f.family,variant:f.variant,contract:wrapper.contract,operation:wrapper.operation,profile:wrapper.profile,fields:[...wrapper.fields],matches:{},nested:{fields:[...f.fields],matches:{contract:f.contract,operation:f.operation}}})),...lifecycleActivationInputForms()];
 return forms.length===14&&new Set(forms.map(f=>f.id)).size===14&&forms.every(f=>f.family==='F17'&&variants.includes(f.variant))?freeze(forms):null;
}
function inputBranch(f){
 let nested=null;if(f.nested){nested=record(f.nested.fields,Object.fromEntries(f.nested.fields.map(k=>[k,Object.hasOwn(f.nested.matches,k)?{const:f.nested.matches[k]}:properties[k]??null])));if(!nested)return null;}
 // Container retirement carries a generation tuple, not a scalar revision.
 const expected=f.id==='view-container-retire'?record(['attachmentGeneration','selectionGeneration','viewportGeneration'],{attachmentGeneration:natural,selectionGeneration:natural,viewportGeneration:natural}):f.id==='workspace-container-retire'?record(['generation'],{generation:natural}):properties.expected;
 return record(f.fields,Object.fromEntries(f.fields.map(k=>[k,k==='contract'||k==='operation'||k==='profile'?{const:f[k]}:Object.hasOwn(f.matches,k)?{const:f.matches[k]}:k==='invocation'?text:k==='request'?nested:k==='expected'?expected:properties[k]??null])));
}
function inputSchema(variant){
 const all=lifecycleDescriptorInputForms(),forms=all?.filter(f=>f.variant===variant);if(!variants.includes(variant)||!forms?.length)return null;
 const oneOf=forms.map(inputBranch);return oneOf.every(Boolean)?freeze({$schema:inputSchemaDialect,oneOf}):null;
}
const bindings=Object.freeze(['original-native-domain-ticket-and-issued-invocation','current-authority-profile-dependency-and-completed-provider-frames','captured-identifier-ceiling-current-recipient-summary-and-facet-rights','native-status-phase-identity-and-complete-result-envelope-validation','existing-preparation-publication-and-protected-discharge-order']);
const notEstablished=Object.freeze(['opaque-authority-or-target-ownership-from-schema-values','all-nested-value-codecs-utf8-or-runtime-resource-budgets','semantic-enablement-or-native-executability-from-structural-acceptance','public-operation-result-query-or-wire-encoding','full-lifecycle-events-or-lossless-activation-audit','host-physical-global-or-independent-refinement']);
function receiptBranches(f){
 const d=lifecycleReceiptDeclarations(),identity=record(d.identityFields,{authority:text,id:text,incarnation:natural}),base={invocation:text,status:{enum:[...d.document.statuses]},document:identity};
 if(f.id==='source-load')return Object.entries(d.load.activation).map(([activation,status])=>record(d.load.fields,{...base,operation:{const:d.load.operation},activation:{const:activation},status:{const:status}}));
 if(f.id==='annotation-restore'||f.id==='annotation-baseline'){const phase=f.nested.matches.phase;return Object.entries(d.annotations.phases[phase]).map(([disposition,status])=>record(d.annotations.fields,{...base,operation:{const:d.annotations.operation},phase:{const:phase},disposition:{const:disposition},status:{const:status}}));}
 if(f.id.includes('-container-')){const kind=f.variant==='view-container'?'view':'workspace',status=f.matches.action==='create'?'created':'retired';return [record(d.relation.fields,{invocation:text,kind:{const:kind},status:{const:status},target:identity})];}
 const operation=f.id==='source-saveas'?'source-save-as':f.id;if(!d.document.operations.includes(operation))return null;
 return [record(d.document.fields,{...base,operation:{const:operation}})];
}
function resultMetadata(variant){
 const forms=lifecycleDescriptorInputForms()?.filter(f=>f.variant===variant);if(!variants.includes(variant)||!forms?.length)return null;
 const branches=forms.map(receiptBranches);if(branches.some(b=>!b||b.some(x=>x===null)))return null;
 const result={oneOf:branches.flat()},success=record(['status','result'],{status:{const:'result'},result}),unavailable=record(['status'],{status:{const:'unavailable'}});
 return freeze({coverage:'native-value-structure-only',scope:'existing-native-operation-result-facet-only',schema:{$schema:inputSchemaDialect,oneOf:[success,unavailable]},runtimeBindings:[...bindings],notEstablished:[...notEstablished]});
}
function outcomes(variant){const result=resultMetadata(variant);return result&&freeze({qualifiedLocalTerminalForms:invocationTerminalForms(),statusVocabulary:['result','unavailable'],result,effect:'no-new-provider-work-in-local-activation',retention:'existing-owned-invocation-and-native-producer-lifetimes',execution:'bounded-local-preparation-and-owned-publication',events:'not-an-event-subscription-or-lossless-activation-audit',release:'original-ticket-context-identifier-ceiling-and-current-summary/facet'});}
export function lifecycleInputSchemaValid(value,c,variant){const expected=c===contract?inputSchema(variant):null;return expected!==null&&equal(value,expected);}
export function lifecycleOutcomesValid(value,c,variant){const expected=c===contract?outcomes(variant):null;return expected!==null&&equal(value,expected);}
export function lifecycleEngineDescriptor(){
 const rows=variants.map(id=>({id,input:{contract,variant:id,coverage:inputSchemaCoverage,schema:inputSchema(id)},outcomes:outcomes(id),requirements:{inputCoverage:inputSchemaCoverage,currentPredicates:'actual-native-owner-authority-profile-current-target-and-companions',resources:'owning-input-preparation-publication-result-and-protected-cleanup-profiles',notEstablished:['schema-as-permission-or-native-semantic-validity','private-discharge-as-public-command','complete-family-event-or-result-query-surface']},limits:{values:'current-owning-byte-node-depth-root-and-result-envelope-profiles'},cancellation:'owned-prepublication-stop; completed-provider-work-and-published-state-are-not-retracted'}));
 return rows.every(r=>r.input.schema&&r.outcomes)?freeze({family:'F17',contract,version:1,kind:'localCommand',variants:rows,presentation:{}}):null;
}
