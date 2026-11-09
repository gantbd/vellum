import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import {registrationResultValueCorpus} from './registration-result-value-checks.mjs';
const mutants=[
 ['extra-fields-accepted','Object.keys(value).length===keys.length&&',''],
 ['hidden-fields-accepted',"descriptors[k].enumerable!==true","false"],
 ['empty-label-accepted',"v.length>0&&",''],
 ['invalid-utf8-accepted',"&&Buffer.from(v,'utf8').toString('utf8')===v",''],
 ['negative-integer-accepted','&&v>=0',''],
 ['unsafe-integer-accepted','Number.isSafeInteger(v)','Number.isInteger(v)'],
 ['fractional-integer-accepted','Number.isSafeInteger(v)',"typeof v==='number'"],
 ['unknown-result-status-accepted',"!['applied','unchanged'].includes(result.status)",'false'],
 ['unknown-envelope-status-accepted',"envelope.status!=='result'",'false'],
 ['extra-array-properties-accepted','Reflect.ownKeys(descriptors).length!==length+1','false'],
 // Both the key census and per-index check reject holes; omit both for this fault.
 ['sparse-array-accepted',"if(!natural(length)||Reflect.ownKeys(descriptors).length!==length+1)return null;\n const result=[];for(let i=0;i<length;i++){const d=descriptors[String(i)];if(!d||!Object.hasOwn(d,'value')||d.enumerable!==true)return null;result.push(d.value);}","if(!natural(length))return null;\n const result=[];for(let i=0;i<length;i++){const d=descriptors[String(i)];if(!d)continue;if(!Object.hasOwn(d,'value')||d.enumerable!==true)return null;result.push(d.value);}"],
 ['unknown-table-accepted',"!['read','write'].includes(row.table)",'false'],
 ['duplicate-host-slot-accepted','if(seen.has(slot))return false;',''],
 ['out-of-domain-basis-accepted','row.basis>=list.length','false'],
 ['duplicate-basis-accepted','||seen.has(row.basis)',''],
 ['unknown-basis-state-accepted',"!['absent','present'].includes(row.state)",'false'],
 ['non-dense-created-index-accepted','row.index!==i','false'],
 ['duplicate-reference-accepted','||seen.has(row.reference)',''],
 ['null-reference-accepted','!row.reference||',''],
 ['primitive-reference-accepted',"typeof row.reference!=='object'",'false']
];
export async function checkRegistrationResultValueMutations(corpus=null){
 const {cases}=corpus??await registrationResultValueCorpus(),source=fs.readFileSync(new URL('./registration-result-values.mjs',import.meta.url),'utf8');
 const run=validate=>cases.every(c=>validate(c.value,c.kind)===c.accepted);
 const load=async code=>import('data:text/javascript;base64,'+Buffer.from(code).toString('base64'));
 assert(run((await load(source)).registrationResultValueValid));const killed=[];
 for(const [name,from,to]of mutants){assert.equal(source.split(from).length-1,1,name);const api=await load(source.replace(from,to));assert.equal(run(api.registrationResultValueValid),false,name);killed.push(name);}
 assert.equal(new Set(killed).size,mutants.length);return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{const corpus=await registrationResultValueCorpus();console.log(JSON.stringify({scope:'existing-native-F16-released-value-sensitivity',baseline:corpus.summary,mutations:await checkRegistrationResultValueMutations(corpus),mutationBaselineChecked:true}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'[audit module]'));process.exitCode=1;}
