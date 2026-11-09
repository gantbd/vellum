// Selected semantic sensitivity, not independent review or native mutation proof.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import {checkSelectionDescriptor} from './selection-descriptor-checks.mjs';
const source=fs.readFileSync(new URL('./selection-engine-descriptor.mjs',import.meta.url),'utf8');
const mutations=[
 ['wrong-family',"family:'F12',contract,version:1","family:'F11',contract,version:1"],
 ['wrong-kind',"kind:'localCommand',variants:rows","kind:'query',variants:rows"],
 ['wrong-version','contract,version:1','contract,version:2'],
 ['native-family-filter-widened',"f.family==='F12'",'true'],
 ['native-form-missing',"const forms=[...selectionInputForms(),...boundaryInputForms()]","const forms=[...selectionInputForms()]"],
 ['extra-fields-admitted','additionalProperties:false','additionalProperties:true'],
 ['required-fields-erased','required:[...f.fields]','required:[]'],
 ['contract-value-unconstrained',"k==='contract'||k==='operation'?{const:f[k]}","k==='contract'||k==='operation'?text"],
 ['profile-value-unconstrained',"k==='profile'?{const:f.profileConstant}","k==='profile'?text"],
 ['safe-natural-negative-admitted','minimum:0,maximum:Number.MAX_SAFE_INTEGER','minimum:-1,maximum:Number.MAX_SAFE_INTEGER'],
 ['safe-natural-ceiling-removed','minimum:0,maximum:Number.MAX_SAFE_INTEGER','minimum:0'],
 ['count-zero-admitted',"count:{type:'integer',minimum:1","count:{type:'integer',minimum:0"],
 ['empty-text-admitted',"text={type:'string',minLength:1}","text={type:'string'}"],
 ['extension-type-widened',"extend:{type:'boolean'}","extend:{}"],
 ['input-recognizer-bypassed','expected!==null&&equal(value,expected);','expected!==null;'],
 ['outcome-recognizer-bypassed','export function selectionOutcomesValid(value,c,variant){const expected=c===contract?outcomes(variant):null;return expected!==null&&equal(value,expected);}','export function selectionOutcomesValid(value,c,variant){return true;}'],
 ['source-effect-claimed',"effect:'view-selection-only; no-source-or-annotation-mutation'","effect:'source-mutation'"],
 ['source-write-required',"sourceWriteGrant:'not-required-for-view-only-selection'","sourceWriteGrant:'required'"],
 ['coverage-overstated','coverage:inputSchemaCoverage,schema:schema(id)',"coverage:'complete-native-semantics',schema:schema(id)"]
];
assert.equal(mutations.length,19);assert.equal(new Set(mutations.map(m=>m[0])).size,19);
const baseline=checkSelectionDescriptor(),killed=[];
for(const [name,before,after]of mutations){
 if(!source.includes(before))throw Error('SELECTION_DESCRIPTOR_MUTATION_CONSTRUCTION: '+name);
 const code=source.replace(before,after).replaceAll(/from '(\.\/[^']+)'/g,(_,relative)=>"from '"+new URL(relative,import.meta.url).href+"'");
 const api=await import('data:text/javascript;base64,'+Buffer.from(code+'\n// '+name).toString('base64'));
 let detected=false;try{checkSelectionDescriptor(api);}catch(error){if(error.code!=='ERR_ASSERTION')throw error;detected=true;}assert(detected,'surviving selection descriptor mutation '+name);killed.push(name);
}
console.log(JSON.stringify({scope:'fixed-existing-F12-selection-descriptor-sensitivity',baseline,mutations:killed,constructionOrToolFailureIsNotSemanticDetection:true,discoveryRegistrationSelected:false,hostPhysicalOrIndependentProof:false}));
