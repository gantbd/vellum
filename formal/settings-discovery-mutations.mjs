import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),file='formal/native-discovery.mjs',rules=[
 ['profile-nine-not-recognized','||c.profile===catalog.settingsSchemaProfile',''],
 ['settings-input-schema-disabled','settingsQueryInputSchemaValid(input.schema,contract,variant)','false'],
 ['arbitrary-settings-input-admitted','settingsQueryInputSchemaValid(input.schema,contract,variant)','true'],
 ['arbitrary-settings-outcomes-admitted','settingsQueryOutcomesValid(v.outcomes,r.contract,v.id)','true'],
 ['older-read-recognition-lost','![catalog.readResultSchemaProfile,catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','![catalog.readResultSchemaProfile,catalog.historySchemaProfile,catalog.observationSchemaProfile].includes(profile)'],
 ['older-history-recognition-lost','![catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','![catalog.historySchemaProfile,catalog.observationSchemaProfile].includes(profile)'],
 ['older-observation-recognition-lost','![catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','profile!==catalog.observationSchemaProfile'],
 ['fixed-settings-source-omitted','source=settingsQueryEngineDescriptor()','source=null'],
 ['previous-profile-downgraded','previous=builtinRows(m,{...c,profile:catalog.observationSchemaProfile}),source=settingsQueryEngineDescriptor()','previous=builtinRows(m,{...c,profile:catalog.historySchemaProfile}),source=settingsQueryEngineDescriptor()'],
 ['unknown-twelve-admitted','||c.profile===catalog.settingsSchemaProfile',"||c.profile===catalog.settingsSchemaProfile||c.profile==='owned-contract-discovery/14'"],
 ['foreign-authority-admitted','&&c.authority===m.authority',''],
 ['foreign-engine-admitted','&&c.engine===m.engine',''],
 ['entry-capacity-overaccepted','rows.length<=l.maxEntries','rows.length<=l.maxEntries+1'],
 ['variant-capacity-overaccepted','r.variants.length<=l.maxVariants','r.variants.length<=l.maxVariants+1'],
 ['row-byte-bound-overaccepted','size(r)<=l.maxRowBytes','size(r)<=l.maxRowBytes+1'],
 ['metadata-byte-bound-overaccepted','size(v)<=l.maxMetadataBytes','size(v)<=l.maxMetadataBytes+1'],
 ['metadata-node-bound-overaccepted','--budget.left<0','--budget.left< -1'],
 ['metadata-depth-overaccepted','depth>l.maxMetadataDepth','depth>l.maxMetadataDepth+1'],
 ['packet-byte-bound-overaccepted','size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes','size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes+1'],
 ['capture-marker-charge-omitted','p.builtin===true&&size({rows,target,builtin:true})>m.limits.maxCaptureBytes','false'],
 ['reply-byte-bound-overaccepted','if(queryReplyBytes>m.limits.maxReplyBytes)return no();','if(queryReplyBytes>m.limits.maxReplyBytes+1)return no();'],
 ['captured-schema-ceiling-expanded','selected.schema&&f.schema?','f.schema?'],
 ['generic-origin-borrows-builtin','if((p.builtin===true)!==(s.builtin===true))return no();','if(false)return no();'],
 ['advice-facts-bypassed','!gates(g,catalog.adviceGates)','false'],
 ['advice-target-copy-admitted','request.target!==s.target.reference','false'],
 ['known-loss-does-not-stop','stop(m,s);return null;','return null;'],
 ['metadata-withdrawal-ignored','if(current===null||!equal(current,s.rows)){stop(m,s);return no();}','if(false){stop(m,s);return no();}'],
 ['query-publication-guard-bypassed','const m=owners.get(owner);if(g?.outsidePublicationGuard!==true||!request','const m=owners.get(owner);if(!request'],
 ['retirement-skips-drain',"if(!s?.stopped||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))","if(!s?.stopped||g?.ownerControlAllowed!==true)"],
 ['retirement-refunds-issuance',"m.contexts.delete(context);return {status:'retired'};","m.contexts.delete(context);m.issued=0;return {status:'retired'};"]
],bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);assert(Number.isInteger(bank)&&bank>=0&&bank<3);
for(const[name,from]of rules)assert.equal(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1,1,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/settings-discovery-metadata-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,from,to]of rules.filter((_,i)=>i%3===bank)){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));const out=run(graph.directory);assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1400));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1400));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('construction target'),name+': construction failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'owned-F07-discovery-integration-sensitivity',bank,bankCount:3,totalMutations:rules.length,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allProductsHostPhysicalOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
