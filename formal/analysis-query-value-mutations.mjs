import assert from 'node:assert/strict';import fs from 'node:fs';import {pathToFileURL} from 'node:url';
import {analysisQueryValueCorpus,checkAnalysisQueryValues} from './analysis-query-value-checks.mjs';
const source=fs.readFileSync(new URL('./analysis-query-values.mjs',import.meta.url),'utf8'),rules=[
 ['unknown-operation-admitted',"if(!['supported-analysis','supplied-evidence-inspection'].includes(operation))return false;",""],
 ['unavailable-extra-fields-admitted',"if(v.status==='unavailable')return Boolean(exact(v,['status']));","if(v.status==='unavailable')return true;"],
 ['result-status-not-bound-to-operation','v.status!==status||',''],
 ['closed-object-fields-widened','Object.keys(v).length===ks.length&&',''],
 ['identity-empty-label-admitted','v[k].length>0','v[k].length>=0'],
 ['context-not-validated','!context(v.context)||',''],
 ['language-profile-not-bound',"&&v.languageProfile==='pinned-markdown-javascript-reference/1'",""],
 ['zero-generation-rejected','n>=0','n>0'],
 ['unsafe-integers-admitted','Number.isSafeInteger(n)','Number.isInteger(n)'],
 ['fractional-generations-admitted','Number.isSafeInteger(n)','Number.isFinite(n)'],
 ['parse-state-widened',"!['parsed','syntax-errors'].includes(v.parseState)","false"],
 ['coverage-widened',"!['complete-within-profile','partial'].includes(v.coverage)","false"],
 ['capture-sequence-extra-properties-admitted','||Reflect.ownKeys(descriptors).length!==length+1',''],
 ['capture-tuple-length-corrupted','row.length!==4','row.length!==3'],
 ['capture-layer-widened',"!['host-block','host-inline','child'].includes(layer)","false"],
 ['capture-tag-not-scalar-string',"typeof name!=='string'||Buffer.from(name,'utf8').toString('utf8')!==name","false"],
 ['capture-tag-lossy-utf8-admitted',"||Buffer.from(name,'utf8').toString('utf8')!==name",""],
 ['zero-width-capture-admitted','lo>=hi','lo>hi'],
 ['capture-order-overlap-ignored','||lo<prior',''],
 ['adjacent-capture-incorrectly-rejected','lo<prior','lo<=prior'],
 ['last-capture-validation-omitted','i<rows.length','i<rows.length-1'],
 ['summary-requires-captures','if(!capture)return true;','if(!capture)return false;'],
 ['nonenumerable-required-field-admitted',"||descriptors[k].enumerable!==true",""],
 ['getter-observed-before-data-guard','function fields(value){',"function fields(value){if(value&&Object.getOwnPropertyDescriptor(value,'status')?.get)value.status;"]
];
export async function checkAnalysisQueryValueMutations(){
 assert.equal(rules.length,24);assert.equal(new Set(rules.map(r=>r[0])).size,24);
 for(const [name,from]of rules)assert.equal(source.split(from).length-1,1,name);
 const load=s=>import('data:text/javascript;base64,'+Buffer.from(s).toString('base64')),corpus=analysisQueryValueCorpus(),baseline=checkAnalysisQueryValues(await load(source),corpus),mutations=[];
 for(const [name,from,to]of rules){const api=await load(source.replace(from,to));let failed=false;try{checkAnalysisQueryValues(api,corpus);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+e.message);failed=true;}assert(failed,name);mutations.push(name);}
 assert.deepEqual(checkAnalysisQueryValues(await load(source),corpus),baseline);
 return {scope:'closed-existing-F08-analysis-query-value-sensitivity',baseline,mutations,nativeCorpusProducedOnceForPureCheckerVariants:true,coherentBaselineAfterMutationsPassed:true,constructionOrToolFailuresCounted:false,nativeReleaseIntegrationOrPublicCodecSelected:false,physicalGlobalRawIngressOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkAnalysisQueryValueMutations()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[audit module]'));process.exitCode=1;}
