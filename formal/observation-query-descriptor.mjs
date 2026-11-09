// Fixed selected F06 guard declarations. No evaluator, token codec or event transport.
import {isDeepStrictEqual as equal} from 'node:util';
import {invocationObservationArgumentInputForms} from './invocation-observation-arguments.mjs';
import {invocationObservationResultDeclarations} from './invocation-observation-results.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;},contract='execution-observation/1',operations=['status','bounded-observation-wait','retained-outcome','minimal-control-status'],opaque={type:'object'},text={type:'string',minLength:1};
const record=(fields,properties)=>Array.isArray(fields)&&new Set(fields).size===fields.length&&fields.every(k=>Object.hasOwn(properties,k)&&properties[k]!==null)?{type:'object',properties:Object.fromEntries(fields.map(k=>[k,properties[k]])),required:[...fields],additionalProperties:false}:null;
function declarations(){
 const d=invocationObservationResultDeclarations(),expected={execution:['status','phase','observation'],outcome:['status','observation','outcome'],waiting:['status','wait'],observed:['status','result'],'observation-ended':['status'],unavailable:['status']};
 return equal(d.forms,expected)&&equal(d.bounds,{reasons:2,participants:2,recovery:1})&&equal(d.vocabulary.phases,['pending','terminal'])?d:null;
}
function inputSchema(operation){
 const forms=invocationObservationArgumentInputForms().filter(f=>f.operation===operation);if(!operations.includes(operation)||forms.length!==1)return null;const f=forms[0];if(f.family!=='F06'||f.contract!==contract||f.variant!==operation)return null;
 const body=record(f.fields,{contract:{const:contract},operation:{const:operation},invocation:text,context:opaque,profile:text,after:opaque,budget:{type:'integer',minimum:1,maximum:Number.MAX_SAFE_INTEGER}});return body&&freeze({$schema:inputSchemaDialect,...body});
}
function outcomeSchema(d){
 const reasons=[['unavailable'],...d.reasons.map(r=>[r])];for(let i=0;i<d.reasons.length;i++)for(let j=i+1;j<d.reasons.length;j++)reasons.push([d.reasons[i],d.reasons[j]]);
 const body=record(d.outcomeFields,{observation:{const:'available'},publication:{enum:[...d.vocabulary.publication]},control:{enum:[...d.vocabulary.control]},effect:{enum:[...d.vocabulary.effect]},reasons:{type:'array',minItems:1,maxItems:d.bounds.reasons,enum:reasons},participants:{type:'array',maxItems:d.bounds.participants,items:{type:'string'}},count:{type:['integer','null'],minimum:0,maximum:d.bounds.participants},correlation:{type:['string','null']},recovery:{type:'array',maxItems:d.bounds.recovery,items:{enum:[...d.recovery]}}});if(!body)return null;
 const details=[{properties:{count:{const:null},participants:{maxItems:0},correlation:{const:null}}}];for(let n=0;n<=d.bounds.participants;n++)details.push({properties:{count:{const:n},participants:{minItems:n,maxItems:n},correlation:{type:'string'}}});
 return {...body,allOf:[{not:{properties:{publication:{const:'applied'},control:{const:'rejected'}},required:['publication','control']}},{oneOf:details}]};
}
function executionSchema(d,rich){
 const plain=record(d.forms.execution,{status:{const:'execution'},phase:{enum:[...d.vocabulary.phases]},observation:opaque});if(!plain||!rich)return plain;
 const outcome=outcomeSchema(d),full=record([...d.forms.execution,'outcome'],{status:{const:'execution'},phase:{const:'terminal'},observation:opaque,outcome});return full?{oneOf:[plain,full]}:null;
}
const bindings=Object.freeze(['actual-invocation-observer-context-and-issued-role-identities','original-captured-current-policy-and-release','projected-phase-outcome-and-reference-signature','protected-control-reservation-and-retained-lifetime']),limits=Object.freeze(['opaque-authenticity-or-identity-from-json-shadows','actual-policy-retention-clock-or-origin-truth','shaped-recovery-as-retry-authority','public-explanation-codes-or-wire-representation','public-wait-acquisition-or-event-transport','utf8-from-standard-string-shape','all-products-physical-global-or-independent-refinement']);
function resultMetadata(operation,continuation=false){
 const d=declarations();if(!d||!operations.includes(operation)||continuation&&operation!=='bounded-observation-wait')return null;
 const unavailable=record(d.forms.unavailable,{status:{const:'unavailable'}}),waiting=record(d.forms.waiting,{status:{const:'waiting'},wait:opaque});let branches;
 if(continuation){const observed=record(d.forms.observed,{status:{const:'observed'},result:executionSchema(d,true)}),ended=record(d.forms['observation-ended'],{status:{const:'observation-ended'}});branches=[waiting,observed,ended,unavailable];}
 else if(operation==='bounded-observation-wait')branches=[waiting,unavailable];
 else if(operation==='retained-outcome')branches=[record(d.forms.outcome,{status:{const:'outcome'},observation:opaque,outcome:outcomeSchema(d)}),unavailable];
 else branches=[executionSchema(d,operation==='status'),unavailable];
 return branches.every(Boolean)?freeze({coverage:'native-value-structure-only',scope:continuation?'private-owned-wait-transition-only':'native-operation-reply-only',schema:{$schema:inputSchemaDialect,oneOf:branches},runtimeBindings:[...bindings,...(operation==='bounded-observation-wait'?['wait-budget-clock-capacity-drain-and-producer-separation']:[])],notEstablished:[...limits]}):null;
}
function outcomes(operation){
 const result=resultMetadata(operation);if(!result)return null;const waiting=operation==='bounded-observation-wait',continuation=waiting?resultMetadata(operation,true):null;if(waiting&&!continuation)return null;
 return freeze({statusVocabulary:[waiting?'waiting':operation==='retained-outcome'?'outcome':'execution','unavailable'],result,...(waiting?{continuation}:{}),effect:'no-authored-or-provider-effect',retention:waiting?'owned-wait-retained-until-protected-drain':'native-context-and-reference-retention',execution:waiting?'synchronous-owned-wait-start; separate-private-bounded-advance':'synchronous-bounded-native-observation',events:waiting?'private-owned-wait-transitions-not-public-event-transport':'no-asynchronous-events-in-this-synchronous-profile',release:'actual-native-observer-current-policy-and-retention; known-loss-is-absorbing'});
}
export function observationQueryInputSchemaValid(value,c,operation){const expected=c===contract?inputSchema(operation):null;return expected!==null&&equal(value,expected);}
export function observationQueryOutcomesValid(value,c,operation){const expected=c===contract?outcomes(operation):null;return expected!==null&&equal(value,expected);}
export function observationQueryEngineDescriptor(){
 const variants=operations.map(id=>({id,input:{contract,variant:id,coverage:inputSchemaCoverage,schema:inputSchema(id)},outcomes:outcomes(id),requirements:{inputCoverage:inputSchemaCoverage,currentPredicates:'actual-owned-observer-original-invocation-and-issued-roles-with-current-release',resources:id==='minimal-control-status'?'protected-control-independent-of-ordinary-readiness-and-issuance':id==='bounded-observation-wait'?'actual-ordinary-control-wait-budget-and-capacity':'actual-ordinary-release-and-reply-capacity',notEstablished:['native-acquisition-or-decoder-capability-from-metadata','observation-advice-as-producer-control','public-event-or-wire-completeness']},limits:{values:'actual-native-observer-and-shared-reservation; explicit-prepaid-decoder-profile-if-supported'},cancellation:'separate-owner-stop-or-expiry-ends-observation-only; never-producer-cancellation'}));
 return variants.every(v=>v.input.schema&&v.outcomes)?freeze({family:'F06',contract,version:1,kind:'query',variants,presentation:{}}):null;
}
