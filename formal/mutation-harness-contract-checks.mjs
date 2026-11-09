// Selected lexical regression guards and control-flow fixtures, not an AST proof.
import assert from 'node:assert/strict';
import fs from 'node:fs';
const reviewed=[
 "annotation-artifact-checks.mjs",
 "annotation-baseline-capacity-checks.mjs",
 "annotation-baseline-checks.mjs",
 "annotation-binding-release-checks.mjs",
 "annotation-claim-checks.mjs",
 "annotation-comparison-checks.mjs",
 "annotation-installation-checks.mjs",
 "annotation-lifecycle-checks.mjs",
 "annotation-load-invocation-checks.mjs",
 "annotation-loaded-baseline-checks.mjs",
 "annotation-membership-editing-checks.mjs",
 "annotation-native-load-checks.mjs",
 "annotation-observation-checks.mjs",
 "annotation-preparation-checks.mjs",
 "annotation-read-checks.mjs",
 "annotation-read-discharge-checks.mjs",
 "annotation-resource-read-checks.mjs",
 "annotation-restoration-checks.mjs",
 "annotation-result-checks.mjs",
 "annotation-retention-checks.mjs",
 "annotation-terminal-hold-checks.mjs",
 "annotation-write-checks.mjs",
 "baseline-reservation-checks.mjs",
 "binding-pair-checks.mjs",
 "configuration-read-checks.mjs",
 "diagnostic-capture-checks.mjs",
 "diagnostic-drain-checks.mjs",
 "discovery-invalidation-checks.mjs",
 "disposed-annotation-producers-checks.mjs",
 "history-context-checks.mjs",
 "history-invalidation-checks.mjs",
 "history-retirement-checks.mjs",
 "invocation-control-checks.mjs",
 "invocation-invalidation-checks.mjs",
 "invocation-observation-checks.mjs",
 "lifecycle-invocation-checks.mjs",
 "load-invocation-checks.mjs",
 "native-annotation-export-checks.mjs",
 "native-discovery-checks.mjs",
 "native-live-lifecycle-checks.mjs",
 "native-retained-lifecycle-checks.mjs",
 "native-source-destinations-checks.mjs",
 "native-source-history-checks.mjs",
 "native-source-read-checks.mjs",
 "native-source-rebind-checks.mjs",
 "native-source-save-checks.mjs",
 "observation-admission-checks.mjs",
 "ordinary-annotation-save-checks.mjs",
 "proposal-validation-checks.mjs",
 "read-dependency-checks.mjs",
 "registration-basis-checks.mjs",
 "registration-event-checks.mjs",
 "registration-invocation-checks.mjs",
 "registration-read-checks.mjs",
 "registration-scoped-checks.mjs",
 "relation-invocation-checks.mjs",
 "relation-lifecycle-checks.mjs",
 "resource-registration-checks.mjs",
 "reversal-result-checks.mjs",
 "saveas-invocation-checks.mjs",
 "source-saveas-authority-checks.mjs",
 "source-scope-checks.mjs",
 "workspace-read-checks.mjs"
];
const read=name=>fs.readFileSync(new URL(name,import.meta.url),'utf8');
function unsafe(source){
 const canonical=[...source.matchAll(/try\{(?:await )?check[A-Za-z0-9_$]*\(await (?:load|graph|loadGraph)\(([^;\n]*?)\)(,\s*(?:true|false))?\);/g)].some(m=>m[1].trim().length>0);
 return canonical||/try\{check[A-Za-z0-9_$]*\(\{\.\.\.original,\.\.\.await load\(/.test(source);
}
async function detection(build,exercise){
 const candidate=await build();let detected=false;
 try{await exercise(candidate);}catch(e){if(e.code!=='ERR_ASSERTION')throw e;detected=true;}
 if(!detected)throw Object.assign(Error('mutation survived'),{code:'ERR_MUTATION_SURVIVED'});
 return true;
}
let behaviorCases=0,sourceShapeCases=0;
for(const error of [Object.assign(Error('setup'),{code:'ERR_ASSERTION'}),SyntaxError('import syntax'),TypeError('import runtime')]){
 await assert.rejects(()=>detection(async()=>{throw error;},()=>assert.fail('must not execute')),e=>e===error);behaviorCases++;
}
assert.equal(await detection(async()=>({}),()=>assert.fail('semantic')),true);behaviorCases++;
await assert.rejects(()=>detection(async()=>({}),()=>{}),{code:'ERR_MUTATION_SURVIVED'});behaviorCases++;
for(const error of [TypeError('semantic runtime'),SyntaxError('semantic syntax')]){
 await assert.rejects(()=>detection(async()=>({}),()=>{throw error;}),e=>e===error);behaviorCases++;
}
const order=[];assert.equal(await detection(async()=>{await Promise.resolve();order.push('built');return {};},()=>{assert.deepEqual(order,['built']);assert.fail('semantic');}),true);behaviorCases++;
for(const source of ['try{checkX(await load(m),true);}','try{await checkX(await loadGraph(a,b),true);}','try{checkX({...original,...await load(raw)});}']){
 assert.equal(unsafe(source),true);sourceShapeCases++;
}
assert.equal(unsafe('try{checkX(await load(),true);const api=await load(m);try{checkX(api,true);}finally{cleanup();}}'),false);sourceShapeCases++;
assert.equal(reviewed.length,63);assert.equal(new Set(reviewed).size,63);
for(const file of reviewed){const source=read(file);assert.equal(unsafe(source),false,file);assert.equal(source.split('const loadedMutationApi=').length-1,1,file);}
const cleanup=read('registration-scoped-checks.mjs');assert(cleanup.includes('try{checkScopedRegistrations(await load(),true);'));assert(cleanup.includes('finally{fs.rmSync(directory,{recursive:true,force:true});}'));
const anchor='!annotationBaselineAnchor(o,document,binding,d.annotationBaseline,domain)',slot="entry.lifecyclePrepared&&(job.kind!=='lifecycle'||job.plan!==entry.lifecyclePrepared)";
assert.equal(read('source-save.mjs').split(anchor).length-1,1);assert(read('annotation-baseline-checks.mjs').includes("['current-baseline-corruption-repaired','"+anchor+"','false']"));
assert.equal(read('invocation-publication.mjs').split(slot).length-1,1);assert(read('lifecycle-invocation-checks.mjs').includes("['prepared-slot-submits-another-job-kind',1,"+JSON.stringify(slot)+",'false']"));
const forget='entry.prepared=null;entry.annotationReceipt=null;entry.reversalReceipt=null;';assert.equal(read('invocation-publication.mjs').split(forget).length-1,1);assert(read('reversal-result-checks.mjs').includes("['forget-keeps-reversal-receipt','invocation-publication.mjs','"+forget+"','entry.prepared=null;entry.annotationReceipt=null;']"));
console.log(JSON.stringify({scope:'selected-mutation-harness-construction-separation-only',reviewedFiles:reviewed.length,behaviorCases,sourceShapeCases,retargetCases:3,cleanupPreserved:true,productionPredicatesChanged:false,arbitrarySyntaxOrAllHarnessesProved:false,independentAssurance:false}));
