// Fixed F12 declaration of existing semantic forms, not a dispatcher or keymap.
import {isDeepStrictEqual as equal} from 'node:util';
import {selectionInputForms} from './selection-products.mjs';
import {boundaryInputForms} from './boundary-commands.mjs';
import {invocationTerminalForms} from './invocation-publication.mjs';
import {inputSchemaDialect,inputSchemaCoverage} from './input-schema-profiles.mjs';
const contract='engine-selection-declaration/1',variants=['set-selection','logical-movement','select-search-match'];
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const object={type:'object'},text={type:'string',minLength:1},natural={type:'integer',minimum:0,maximum:Number.MAX_SAFE_INTEGER};
const properties={invocation:text,view:object,document:object,sourceRevision:natural,expectedSelection:natural,selections:{type:'array',items:object},primary:natural,unit:text,extend:{type:'boolean'},match:object,direction:text,affinity:text,selectionMode:text,boundary:object,count:{type:'integer',minimum:1,maximum:Number.MAX_SAFE_INTEGER}};
// Lazy reads preserve the existing native-owner import graph initialization.
export function selectionDescriptorInputForms(){
 const forms=[...selectionInputForms(),...boundaryInputForms()].filter(f=>f.family==='F12');
 const known=['selection-edit/1:set-selection','selection-edit/1:logical-movement','selection-edit/1:select-search-match','boundary-command/1:logical-movement'];
 return forms.length===4&&new Set(forms.map(f=>f.contract+':'+f.operation)).size===4&&forms.every(f=>known.includes(f.contract+':'+f.operation)&&f.variant===f.operation&&variants.includes(f.variant))?freeze(forms):null;
}
function schema(variant){
 const forms=selectionDescriptorInputForms()?.filter(f=>f.variant===variant);if(!variants.includes(variant)||!forms?.length)return null;
 if(forms.some(f=>new Set(f.fields).size!==f.fields.length||f.fields.some(k=>!['contract','operation','profile'].includes(k)&&!Object.hasOwn(properties,k))))return null;
 return freeze({$schema:inputSchemaDialect,oneOf:forms.map(f=>({type:'object',properties:Object.fromEntries(f.fields.map(k=>[k,k==='contract'||k==='operation'?{const:f[k]}:k==='profile'?{const:f.profileConstant}:properties[k]])),required:[...f.fields],additionalProperties:false}))});
}
function outcomes(variant){return variants.includes(variant)?freeze({qualifiedLocalTerminalForms:invocationTerminalForms(),effect:'view-selection-only; no-source-or-annotation-mutation',release:'existing-owned-invocation-current-recipient-release; not-guaranteed-disclosure',execution:'existing-issued-invocation-preparation-and-atomic-local-publication',notEstablished:['new-operation-specific-receipt-or-public-result-query','complete-family-events-or-wire-codec','schema-as-native-permission-currentness-or-semantic-validity']}):null;}
export function selectionInputSchemaValid(value,c,variant){const expected=c===contract?schema(variant):null;return expected!==null&&equal(value,expected);}
export function selectionOutcomesValid(value,c,variant){const expected=c===contract?outcomes(variant):null;return expected!==null&&equal(value,expected);}
export function selectionEngineDescriptor(){
 const rows=variants.map(id=>({id,input:{contract,variant:id,coverage:inputSchemaCoverage,schema:schema(id)},outcomes:outcomes(id),requirements:{inputCoverage:inputSchemaCoverage,currentPredicates:'actual-view-source-selection-revisions-and-current-view-write-source-read-rights',opaqueRoles:id==='select-search-match'?['original-live-current-search-match']:id==='logical-movement'?['original-live-retained-boundary-for-unicode-form']:[],sourceWriteGrant:'not-required-for-view-only-selection',notEstablished:['nested-selection-validation-or-unicode-value-codec','visual-geometry-keymap-input-or-presentation-policy','automatic-retargeting-or-complete-family-dispatch']},limits:{values:'existing-owning-request-view-selection-boundary-and-publication-profiles'},cancellation:'owned-prepublication-stop; published-selection-is-not-retracted'}));
 return rows.every(r=>r.input.schema&&r.outcomes)?freeze({family:'F12',contract,version:1,kind:'localCommand',variants:rows,presentation:{}}):null;
}
