// Describes existing F10 checks. It grants no execution or observation authority.
import {isDeepStrictEqual as equal} from 'node:util';
import {proposalValidationDeclaration} from './invocation-publication.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
function declaration(){
 const d=proposalValidationDeclaration();return d?.family==='F10'&&d.contract==='proposal-validation/1'&&d.operation==='advisory-validation'&&d.profile==='native-compound-current-checks/1'&&equal(d.requestFields,['contract','operation','plan'])&&equal(d.basisFields,['root','context','request'])&&equal(d.replyFields,['status','profile','checksSatisfied'])?d:null;
}
const record=(fields,properties)=>({type:'object',properties,required:[...fields],additionalProperties:false});
function input(){const d=declaration();return d?freeze({$schema:inputSchemaDialect,...record(d.requestFields,{contract:{const:d.contract},operation:{const:d.operation},plan:{type:'object'}})}):null;}
function result(){const d=declaration();return d?freeze({$schema:inputSchemaDialect,oneOf:[record(['status'],{status:{const:'unavailable'}}),record(d.replyFields,{status:{const:'advice'},profile:{const:d.profile},checksSatisfied:{type:'boolean'}})]}):null;}
function outcomes(){const schema=result();return schema?freeze({forms:['advice','unavailable'],result:{coverage:'existing-native-value-structure',schema},effect:'none',execution:'synchronous-current-check-advice-only',notEstablished:['execution-permission-or-reservation','future-currentness-or-success','complete-job-or-semantic-evidence-coverage','public-wire-or-asynchronous-events']}):null;}
export function advisoryDescriptorInputForms(){const d=declaration();return d?freeze([{family:d.family,contract:d.contract,operation:d.operation,variant:d.operation,fields:[...d.requestFields]}]):null;}
export function advisoryInputSchemaValid(value,contract,variant){const d=declaration(),schema=input();return Boolean(d&&schema&&contract===d.contract&&variant===d.operation&&equal(value,schema));}
export function advisoryOutcomesValid(value,contract,variant){const d=declaration(),expected=outcomes();return Boolean(d&&expected&&contract===d.contract&&variant===d.operation&&equal(value,expected));}
export function advisoryEngineDescriptor(){
 const d=declaration(),schema=input(),meaning=outcomes();return d&&schema&&meaning?freeze({family:d.family,contract:d.contract,version:1,kind:'query',variants:[{id:d.operation,input:{contract:d.contract,variant:d.operation,coverage:inputSchemaCoverage,schema},outcomes:meaning,requirements:{inputCoverage:inputSchemaCoverage,currentPredicates:'original-domain-current-context-and-exact-root-request-observation-basis',nativeChecks:'existing-compound-source-annotation-view-and-declared-read-dependency-rules',notEstablished:['nested-plan-value-validation','original-identity-or-current-authority-from-schema','advice-as-permission-or-held-capacity','complete-all-job-or-provider-semantic-coverage']},limits:{values:'actual-current-owner-intent-root-reply-and-candidate-work-profiles'},cancellation:'no-command-or-effect-is-started; current-notStopped-is-still-required'}],presentation:{}}):null;
}
