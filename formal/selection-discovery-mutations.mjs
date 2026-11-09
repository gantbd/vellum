import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),file='formal/native-discovery.mjs',rules=[
 ['profile-eleven-disabled','||c.profile===catalog.selectionSchemaProfile',''],
 ['selection-input-disabled','selectionInputSchemaValid(input.schema,contract,variant)','false'],
 ['arbitrary-selection-input-admitted','selectionInputSchemaValid(input.schema,contract,variant)','true'],
 ['arbitrary-selection-outcomes-admitted','selectionOutcomesValid(v.outcomes,r.contract,v.id)','true'],
 ['fixed-selection-source-omitted','source=selectionEngineDescriptor()','source=null'],
 ['previous-profile-downgraded','previous=builtinRows(m,{...c,profile:catalog.lifecycleSchemaProfile}),source=selectionEngineDescriptor()','previous=builtinRows(m,{...c,profile:catalog.settingsSchemaProfile}),source=selectionEngineDescriptor()'],
 ['old-read-recognition-lost-in-eleven','![catalog.readResultSchemaProfile,catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','![catalog.readResultSchemaProfile,catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile].includes(profile)'],
 ['old-history-recognition-lost-in-eleven','![catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','![catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile].includes(profile)'],
 ['old-observation-recognition-lost-in-eleven','![catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','![catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile].includes(profile)'],
 ['old-settings-recognition-lost-in-eleven','![catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','![catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile].includes(profile)'],
 ['old-lifecycle-recognition-lost-in-eleven','![catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','profile!==catalog.lifecycleSchemaProfile'],
 ['entry-capacity-overaccepted','rows.length<=l.maxEntries','rows.length<=l.maxEntries+1'],
 ['row-bytes-overaccepted','size(r)<=l.maxRowBytes','size(r)<=l.maxRowBytes+1'],
 ['metadata-bytes-overaccepted','size(v)<=l.maxMetadataBytes','size(v)<=l.maxMetadataBytes+1'],
 ['metadata-nodes-overaccepted','--budget.left<0','--budget.left< -1'],
 ['metadata-depth-overaccepted','depth>l.maxMetadataDepth','depth>l.maxMetadataDepth+1'],
 ['packet-bytes-overaccepted','size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes','size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes+1'],
 ['capture-origin-marker-uncharged','p.builtin===true&&size({rows,target,builtin:true})>m.limits.maxCaptureBytes','false'],
 ['reply-bytes-overaccepted','if(queryReplyBytes>m.limits.maxReplyBytes)return no();','if(queryReplyBytes>m.limits.maxReplyBytes+1)return no();'],
 ['captured-schema-ceiling-expanded','selected.schema&&f.schema?','f.schema?'],
 ['generic-borrows-fixed-origin','if((p.builtin===true)!==(s.builtin===true))return no();','if(false)return no();'],
 ['known-loss-does-not-stop','stop(m,s);return null;','return null;'],
 ['retirement-skips-drain',"!s?.stopped||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])",'!s?.stopped'],
 ['retirement-refunds-issuance',"m.contexts.delete(context);return {status:'retired'};","m.contexts.delete(context);m.issued=0;return {status:'retired'};"]
];
const bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);assert(Number.isInteger(bank)&&bank>=0&&bank<4);assert.equal(rules.length,24);assert.equal(new Set(rules.map(r=>r[0])).size,24);
const partition=Array.from({length:4},(_,b)=>rules.map((r,i)=>({r,i})).filter(x=>x.i%4===b)).flat().sort((a,b)=>a.i-b.i);assert.deepEqual(partition.map(x=>x.r[0]),rules.map(r=>r[0]));
for(const[name,from]of rules)if(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1!==1)throw Error('SELECTION_DISCOVERY_MUTATION_CONSTRUCTION: '+name);
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{
 process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/selection-discovery-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,from,to]of rules.filter((_,i)=>i%4===bank)){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));const out=run(graph.directory);assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1800));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1800));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('MUTATION_CONSTRUCTION'),name+': construction failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'owned-existing-F12-discovery-sensitivity',bank,bankCount:4,totalMutations:24,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,physicalGlobalHostOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
