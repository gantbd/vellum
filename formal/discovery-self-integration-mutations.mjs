import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),file='formal/native-discovery.mjs',rules=[
 [
  "profile-thirteen-disabled",
  "||c.profile===catalog.selfSchemaProfile",
  ""
 ],
 [
  "self-input-disabled",
  "discoverySelfInputSchemaValid(input.schema,contract,variant)",
  "false"
 ],
 [
  "arbitrary-self-input-admitted",
  "discoverySelfInputSchemaValid(input.schema,contract,variant)",
  "true"
 ],
 [
  "arbitrary-self-outcomes-admitted",
  "discoverySelfOutcomesValid(v.outcomes,r.contract,v.id)",
  "true"
 ],
 [
  "self-source-omitted",
  "source=discoverySelfEngineDescriptor()",
  "source=null"
 ],
 [
  "previous-profile-downgraded",
  "previous=builtinRows(m,{...c,profile:catalog.advisorySchemaProfile}),source=discoverySelfEngineDescriptor()",
  "previous=builtinRows(m,{...c,profile:catalog.selectionSchemaProfile}),source=discoverySelfEngineDescriptor()"
 ],
 [
  "inherited-F10-outcome-recognition-lost",
  "![catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)",
  "profile!==catalog.advisorySchemaProfile"
 ],
 [
  "captured-schema-ceiling-widened",
  "selected.schema&&f.schema?",
  "f.schema?"
 ],
 [
  "context-scope-binding-lost",
  "!scopeValid(m,c)||!equal(s.scope,c)",
  "!scopeValid(m,c)"
 ],
 [
  "publication-guard-lost",
  "const m=owners.get(owner);if(g?.outsidePublicationGuard!==true||!request",
  "const m=owners.get(owner);if(!request"
 ],
 [
  "retirement-refunds-issuance",
  "m.contexts.delete(context);return {status:'retired'};",
  "m.contexts.delete(context);m.issued=0;return {status:'retired'};"
 ],
 [
  "advice-current-facts-bypassed",
  "!gates(g,catalog.adviceGates)",
  "false"
 ]
];
const bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);assert(Number.isInteger(bank)&&bank>=0&&bank<3);assert.equal(rules.length,12);assert.equal(new Set(rules.map(r=>r[0])).size,12);
const partition=Array.from({length:3},(_,b)=>rules.map((r,i)=>({r,i})).filter(x=>x.i%3===b)).flat().sort((a,b)=>a.i-b.i);assert.deepEqual(partition.map(x=>x.r[0]),rules.map(r=>r[0]));
for(const [name,from]of rules)if(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1!==1)throw Error('SELF_DISCOVERY_MUTATION_CONSTRUCTION: '+name);
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{
 process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/discovery-self-integration-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const [name,from,to]of rules.filter((_,i)=>i%3===bank)){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));const out=run(graph.directory);assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1800));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1800));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('MUTATION_CONSTRUCTION'),name+': construction failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'owned-existing-F01-self-discovery-sensitivity',bank,bankCount:3,totalMutations:12,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,physicalGlobalHostOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
