// Fixed first-party outer declaration, not a decoder or enablement oracle.
import {isDeepStrictEqual as equal} from 'node:util';
import {selectionInputForms} from './selection-products.mjs';
import {lineInputForm} from './line-edits.mjs';
import {boundaryInputForms} from './boundary-commands.mjs';
import {configuredLineInputForm} from './configured-line-state.mjs';
import {invocationTerminalForms} from './invocation-publication.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
const contract='engine-ordinary-edit-declaration/1',variant='ordinary-edit';
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const object={type:'object'},text={type:'string',minLength:1},natural={type:'integer',minimum:0,maximum:Number.MAX_SAFE_INTEGER};
const fields={contract:text,operation:text,action:text,invocation:text,view:object,document:object,sourceRevision:natural,expectedSelection:natural,profile:text,text:{type:'string'},selectionAfter:text,settings:object,boundary:object,unit:text,direction:text,count:{type:'integer',minimum:1,maximum:Number.MAX_SAFE_INTEGER},extend:{type:'boolean'},configuration:object};
export function ordinaryEditInputForms(){return Object.freeze([...selectionInputForms().filter(f=>f.family==='F11'&&f.variant===variant),lineInputForm(),...boundaryInputForms().filter(f=>f.family==='F11'&&f.variant===variant),configuredLineInputForm()]);}
function schema(){const forms=ordinaryEditInputForms();if(forms.length!==4||new Set(forms.map(f=>f.contract)).size!==forms.length||forms.some(f=>f.family!=='F11'||f.operation!==variant||f.variant!==variant||f.fields.some(k=>!Object.hasOwn(fields,k))))return null;
 return freeze({$schema:inputSchemaDialect,oneOf:forms.map(f=>({type:'object',properties:Object.fromEntries(f.fields.map(k=>[k,k==='contract'||k==='operation'?{const:f[k]}:k==='profile'?{const:f.profileConstant}:{...fields[k]}])),required:[...f.fields],additionalProperties:false}))});
}
export function ordinaryEditSchemaValid(value){const expected=schema();return expected!==null&&equal(value,expected);}
export function ordinaryEditEngineDescriptor(){const input=schema();if(!input)return null;return freeze({family:'F11',contract,version:1,kind:'localCommand',variants:[{id:variant,input:{contract,variant,coverage:inputSchemaCoverage,schema:input},outcomes:{qualifiedLocalTerminalForms:invocationTerminalForms(),effect:'none',release:'captured-current-recipient-intersection; not guaranteed disclosure'},requirements:{currentPredicates:'owning-profile-target-source-selection-authority-and-companions',inputCoverage:inputSchemaCoverage,notEstablished:['nested-value-codec','boundary-reference-authenticity','semantic-enablement','complete-family-dispatch','public-wire']},limits:{values:'owning-request-and-preparation-profiles'},cancellation:'owned-prepublication-stop; accepted-publication-is-not-retracted'}],presentation:{}});}
