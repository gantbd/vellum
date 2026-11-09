import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),native='formal/native-discovery.mjs',descriptor='formal/lifecycle-engine-descriptor.mjs',rules=[
 ['profile-ten-disabled',native,'||c.profile===catalog.lifecycleSchemaProfile',''],
 ['lifecycle-input-disabled',native,'lifecycleInputSchemaValid(input.schema,contract,variant)','false'],
 ['arbitrary-lifecycle-input-admitted',native,'lifecycleInputSchemaValid(input.schema,contract,variant)','true'],
 ['arbitrary-lifecycle-outcomes-admitted',native,'lifecycleOutcomesValid(v.outcomes,r.contract,v.id)','true'],
 ['fixed-lifecycle-source-omitted',native,'source=lifecycleEngineDescriptor()','source=null'],
 ['previous-profile-downgraded',native,'previous=builtinRows(m,{...c,profile:catalog.settingsSchemaProfile}),source=lifecycleEngineDescriptor()','previous=builtinRows(m,{...c,profile:catalog.observationSchemaProfile}),source=lifecycleEngineDescriptor()'],
 ['old-read-recognition-lost-in-ten',native,'![catalog.readResultSchemaProfile,catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','![catalog.readResultSchemaProfile,catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile].includes(profile)'],
 ['old-history-recognition-lost-in-ten',native,'![catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','![catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile].includes(profile)'],
 ['old-observation-recognition-lost-in-ten',native,'![catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','![catalog.observationSchemaProfile,catalog.settingsSchemaProfile].includes(profile)'],
 ['old-settings-recognition-lost-in-ten',native,'![catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','profile!==catalog.settingsSchemaProfile'],
 ['entry-capacity-overaccepted',native,'rows.length<=l.maxEntries','rows.length<=l.maxEntries+1'],
 ['variant-capacity-overaccepted',native,'r.variants.length<=l.maxVariants','r.variants.length<=l.maxVariants+1'],
 ['row-bytes-overaccepted',native,'size(r)<=l.maxRowBytes','size(r)<=l.maxRowBytes+1'],
 ['metadata-bytes-overaccepted',native,'size(v)<=l.maxMetadataBytes','size(v)<=l.maxMetadataBytes+1'],
 ['metadata-nodes-overaccepted',native,'--budget.left<0','--budget.left< -1'],
 ['metadata-depth-overaccepted',native,'depth>l.maxMetadataDepth','depth>l.maxMetadataDepth+1'],
 ['packet-bytes-overaccepted',native,'size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes','size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes+1'],
 ['capture-origin-marker-uncharged',native,'p.builtin===true&&size({rows,target,builtin:true})>m.limits.maxCaptureBytes','false'],
 ['reply-bytes-overaccepted',native,'if(queryReplyBytes>m.limits.maxReplyBytes)return no();','if(queryReplyBytes>m.limits.maxReplyBytes+1)return no();'],
 ['generic-borrows-fixed-origin',native,'if((p.builtin===true)!==(s.builtin===true))return no();','if(false)return no();'],
 ['captured-schema-ceiling-expanded',native,'selected.schema&&f.schema?','f.schema?'],
 ['retirement-refunds-issuance',native,"m.contexts.delete(context);return {status:'retired'};","m.contexts.delete(context);m.issued=0;return {status:'retired'};"],
 ['container-expected-tuple-replaced-by-scalar',descriptor,"k==='expected'?expected:properties[k]??null","k==='expected'?natural:properties[k]??null"],
 ['input-extra-fields-allowed',descriptor,'required:[...fields],additionalProperties:false','required:[...fields],additionalProperties:true']
];
const bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);assert(Number.isInteger(bank)&&bank>=0&&bank<6);
const partition=Array.from({length:6},(_,b)=>rules.map((r,i)=>({r,i})).filter(x=>x.i%6===b)).flat().sort((a,b)=>a.i-b.i);assert.deepEqual(partition.map(x=>x.r[0]),rules.map(r=>r[0]));assert.equal(new Set(partition.map(x=>x.r[0])).size,rules.length);
for(const[name,file,from]of rules)assert.equal(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1,1,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{
 process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/lifecycle-discovery-metadata-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,file,from,to]of rules.filter((_,i)=>i%6===bank)){
  const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));const out=run(graph.directory);
  assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1400));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1400));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('construction target'),name+': construction failure');mutations.push(name);
 }
 passed=true;console.log(JSON.stringify({scope:'owned-F17-discovery-integration-sensitivity',bank,bankCount:6,totalMutations:rules.length,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allProductsHostPhysicalOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
