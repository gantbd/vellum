import assert from 'node:assert/strict';import fs from 'node:fs';import {pathToFileURL} from 'node:url';
import {discoverySelfDescriptorCorpus,checkDiscoverySelfDescriptor} from './discovery-self-descriptor-checks.mjs';
const source=fs.readFileSync(new URL('./discovery-self-descriptor.mjs',import.meta.url),'utf8'),rules=[
 ['wrong-family',"freeze({family:'F01',contract","freeze({family:'F02',contract"],
 ['wrong-kind',"version:1,kind:'query'","version:1,kind:'localCommand'"],
 ['existing-operation-omitted',"const variants=['list','inspect','advisory-enablement']","const variants=['list','inspect']"],
 ['empty-profile-admitted',"text={type:'string',minLength:1}","text={type:'string',minLength:0}"],
 ['opaque-roles-made-strings',"opaque={type:'object'}","opaque={type:'string'}"],
 ['zero-integers-admitted',"minimum:1,maximum:Number.MAX_SAFE_INTEGER","minimum:0,maximum:Number.MAX_SAFE_INTEGER"],
 ['unsafe-integers-admitted','maximum:Number.MAX_SAFE_INTEGER','maximum:Number.MAX_VALUE'],
 ['record-exactness-removed','additionalProperties:false','additionalProperties:true'],
 ['required-fields-removed','required:[...required]','required:[]'],
 ['foreign-input-contract','contract:{const:contract}',"contract:{const:'foreign'}"],
 ['foreign-input-operation','operation:{const:id}',"operation:{const:'execute'}"],
 ['arbitrary-continuation-admitted',"continuation:{oneOf:[{type:'null'},opaque]},entry:opaque","continuation:{},entry:opaque"],
 ['coverage-overstated','coverage:{const:r.coverage}',"coverage:{const:'complete'}"],
 ['arbitrary-result-kind-admitted','kind={enum:[...r.kinds]}',"kind={type:'string'}"],
 ['generic-effect-request-kind-lost','kind={enum:[...r.kinds]}',"kind={enum:r.kinds.filter(k=>k!=='effectRequest')}"],
 ['generic-effect-observation-kind-lost','kind={enum:[...r.kinds]}',"kind={enum:r.kinds.filter(k=>k!=='effectObservation')}"],
 ['hidden-variants-rejected','oneOf:[hidden,visible]','oneOf:[visible]'],
 ['partial-visible-facets-admitted',"visible=record(['reference',...r.facets],","visible=record(['reference'],"],
 ['optional-presentation-required','const descriptor=record(r.descriptorFields,',"const descriptor=record([...r.descriptorFields,'presentation'],"],
 ['unknown-presentation-key-admitted','presentation:record([],Object.fromEntries(r.presentation.map(k=>[k,{}])))',"presentation:{type:'object'}"],
 ['integer-advice-admitted',"enabled:{type:'boolean'}","enabled:{type:['boolean','integer']}"],
 ['entry-shape-substitutes-reference-identity',"entries:{type:'array',items:entry}","entries:{type:'array',items:entry,uniqueItems:true}"],
 ['variant-shape-substitutes-reference-identity',"variants:{type:'array',items:{oneOf:[hidden,visible]}}","variants:{type:'array',items:{oneOf:[hidden,visible]},uniqueItems:true}"],
 ['inert-metadata-forced-to-objects',"k==='reference'?opaque:{}","k==='reference'?opaque:{type:'object'}"],
 ['remote-reference-introduced','return freeze({$schema:inputSchemaDialect,...record(f.fields',"return freeze({$schema:inputSchemaDialect,$ref:'https://not-loaded.invalid/schema',...record(f.fields"],
 ['query-declares-publication',"effect:'none'","effect:'publish'"],
 ['context-issuance-inferred',"'context-acquisition-or-public-handle-codec'","'context-issuance-selected'"],
 ['handler-truth-inferred',"'semantic-enablement-or-handler-registration-truth'","'handler-registration-proved'"],
 ['unavailable-outcome-omitted',"'advice','unavailable'],result","'advice'],result"],
 ['input-recognition-ignores-value','const expected=c===contract?input(id):null;return expected!==null&&equal(value,expected);','const expected=c===contract?input(id):null;return expected!==null;'],
 ['result-recognition-ignores-value','const expected=c===contract?outcomes(id):null;return expected!==null&&equal(value,expected);','const expected=c===contract?outcomes(id):null;return expected!==null;'],
 ['input-recognition-ignores-contract','const expected=c===contract?input(id):null;','const expected=input(id);']
];
export async function checkDiscoverySelfDescriptorMutations(bank=0){
 assert(Number.isInteger(bank)&&bank>=0&&bank<8);assert.equal(rules.length,32);assert.equal(new Set(rules.map(r=>r[0])).size,32);
 const partition=Array.from({length:8},(_,b)=>rules.map((r,i)=>({r,i})).filter(x=>x.i%8===b)).flat().sort((a,b)=>a.i-b.i);assert.deepEqual(partition.map(x=>x.r[0]),rules.map(r=>r[0]));
 for(const [name,from]of rules)assert.equal(source.split(from).length-1,1,name);
 const load=async code=>import('data:text/javascript;base64,'+Buffer.from(code.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,import.meta.url).href))).toString('base64'));
 const corpus=discoverySelfDescriptorCorpus(),baseline=checkDiscoverySelfDescriptor(await load(source),corpus),mutations=[];
 for(const [name,from,to]of rules.filter((_,i)=>i%8===bank)){const api=await load(source.replace(from,to));let failed=false;try{checkDiscoverySelfDescriptor(api,corpus);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+e.message);failed=true;}assert(failed,name);mutations.push(name);}
 return {scope:'fixed-existing-F01-self-description-sensitivity',bank,bankCount:8,totalMutations:32,baseline,mutations,nativeCorpusProducedOnceForPureDescriptorVariants:true,constructionOrToolFailuresCounted:false,discoveryRegistrationSelected:false,physicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{const bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);console.log(JSON.stringify(await checkDiscoverySelfDescriptorMutations(bank)));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[audit module]'));process.exitCode=1;}
