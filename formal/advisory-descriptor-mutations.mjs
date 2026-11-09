import assert from 'node:assert/strict';import fs from 'node:fs';import {pathToFileURL} from 'node:url';
import {checkAdvisoryDescriptor} from './advisory-descriptor-checks.mjs';
const source=fs.readFileSync(new URL('./advisory-engine-descriptor.mjs',import.meta.url),'utf8');
const mutations=[
 ['wrong-family',"return d&&schema&&meaning?freeze({family:d.family","return d&&schema&&meaning?freeze({family:'F11'"],
 ['wrong-kind',"kind:'query'","kind:'localCommand'"],
 ['wrong-variant',"variants:[{id:d.operation","variants:[{id:'exact-batch'"],
 ['wrong-input-coverage',"coverage:inputSchemaCoverage,schema","coverage:'complete-value-codec',schema"],
 ['input-extra-fields-accepted','additionalProperties:false','additionalProperties:true'],
 ['input-plan-not-required','required:[...fields]','required:fields.filter(k=>k!==\'plan\')'],
 ['wrong-plan-kind',"plan:{type:'object'}","plan:{type:'array'}"],
 ['foreign-input-contract',"contract:{const:d.contract}","contract:{const:'foreign'}"],
 ['foreign-input-operation',"operation:{const:d.operation}","operation:{const:'execute'}"],
 ['foreign-result-status',"status:{const:'advice'}","status:{const:'success'}"],
 ['foreign-result-profile',"profile:{const:d.profile}","profile:{const:'foreign'}"],
 ['coerced-result-boolean',"checksSatisfied:{type:'boolean'}","checksSatisfied:{type:['boolean','integer']}"],
 ['advice-declares-publication',"effect:'none'","effect:'publish'"],
 ['negative-advice-omitted',"forms:['advice','unavailable']","forms:['unavailable']"],
 ['advice-promises-execution',"'execution-permission-or-reservation'","'permission-granted'"],
 ['input-recognition-ignores-value','&&equal(value,schema)',''],
 ['result-recognition-ignores-value','&&equal(value,expected)','']
];
export async function checkAdvisoryDescriptorMutations(){
 const load=async code=>import('data:text/javascript;base64,'+Buffer.from(code.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,import.meta.url).href))).toString('base64'));
 const baseline=checkAdvisoryDescriptor(await load(source)),killed=[];
 for(const [name,from,to]of mutations){assert.equal(source.split(from).length-1,1,name);const api=await load(source.replace(from,to));let failed=false;try{checkAdvisoryDescriptor(api);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+e.message);failed=true;}assert(failed,name);killed.push(name);}assert.equal(new Set(killed).size,mutations.length);
 return {scope:'fixed-existing-F10-advisory-descriptor-sensitivity',baseline,mutations:killed,constructionAndToolFailuresCounted:false,discoveryRegistrationSelected:false,physicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkAdvisoryDescriptorMutations()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[audit module]'));process.exitCode=1;}
