import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/discovery-arguments.mjs',rules=[
 ['variant-agreement-ignored','decoded.variant!==args.operation','false'],
 ['extra-arguments-admitted','!exact(args,form.fields)','!args'],
 ['maximum-ceiling-ignored','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['byte-bound-off-by-one','Math.min(maximum,limits.maxRequestBytes)','Math.min(maximum,limits.maxRequestBytes)+1'],
 ['node-bound-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputNodes:limits.maxInputNodes+1,maxRequestBytes:'],
 ['depth-bound-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputDepth:limits.maxInputDepth+1,maxRequestBytes:'],
 ['original-roles-cloned','roles.map(k=>[k,args[k]])','roles.map(k=>[k,structuredClone(args[k])])'],
 ['route-enters-canonical-meaning','{request:r,context:bound.value,limits,opaqueBindings:','{route:e.route,request:r,context:bound.value,limits,opaqueBindings:'],
 ['raw-request-alias-retained','record:capture.value','record:{...capture.value,request:args}'],
 ['authority-frame-extra-fields-admitted','!exact(bound.value,contextFields)','false'],
 ['profile-value-domain-ignored','!text(r.profile)','false'],
 ['zero-list-limit-admitted','r.limit===0','false'],
 ['fractional-list-limit-admitted','!natural(r.limit)','false'],
 ['unsupported-profile-preempted','const capture=copySettingsData(',"if(r.profile!=='owned-contract-discovery/1')return null;const capture=copySettingsData("],
 ['context-identity-agreement-ignored','Object.keys(a.opaque).every(k=>a.opaque[k]===b.opaque[k])',"Object.keys(a.opaque).filter(k=>k!=='context').every(k=>a.opaque[k]===b.opaque[k])"],
 ['entry-identity-agreement-ignored','Object.keys(a.opaque).every(k=>a.opaque[k]===b.opaque[k])',"Object.keys(a.opaque).filter(k=>k!=='entry').every(k=>a.opaque[k]===b.opaque[k])"],
 ['variant-identity-agreement-ignored','Object.keys(a.opaque).every(k=>a.opaque[k]===b.opaque[k])',"Object.keys(a.opaque).filter(k=>k!=='variant').every(k=>a.opaque[k]===b.opaque[k])"],
 ['target-identity-agreement-ignored','Object.keys(a.opaque).every(k=>a.opaque[k]===b.opaque[k])',"Object.keys(a.opaque).filter(k=>k!=='target').every(k=>a.opaque[k]===b.opaque[k])"],
 ['cursor-identity-agreement-ignored','Object.keys(a.opaque).every(k=>a.opaque[k]===b.opaque[k])',"Object.keys(a.opaque).filter(k=>k!=='continuation').every(k=>a.opaque[k]===b.opaque[k])"],
 ['ordinary-argument-agreement-ignored','&&equal(a.record,b.record)',''],
 ['producer-roles-cloned','...n.opaque','...structuredClone(n.opaque)'],
 ['producer-roles-replaced-by-shadows','{...n.record.request,...n.opaque}','{...n.record.request}'],
 ['classification-bound-incorrect','export const discoveryArgumentClassificationBytes=Buffer.byteLength','export const discoveryArgumentClassificationBytes=1+Buffer.byteLength'],
 ['captured-limit-fields-not-exact','exact(p,limitFields)','p'],
 ['captured-limits-mutable','?Object.freeze(p):null','?p:null'],
 ['opaque-array-role-admitted',"&&!Array.isArray(v)",''],
 ['nonenumerable-fields-admitted',"||d[k].enumerable!==true",''],
 ['policy-meaning-not-captured','context:bound.value,limits,opaqueBindings:','context:{...bound.value,policy:"stable"},limits,opaqueBindings:']
];
for(const[name,from]of rules)assert.equal(fs.readFileSync(path.join(root,relative),'utf8').split(from).length-1,1,name+': construction target');
const selectedBank=process.argv.find(a=>a.startsWith('--bank=')),bank=selectedBank===undefined?null:Number(selectedBank.slice(7));assert(bank===null||Number.isInteger(bank)&&bank>=0&&bank<2);
const partition=Array.from({length:2},(_,b)=>rules.map((r,i)=>({r,i})).filter(x=>x.i%2===b)).flat().sort((a,b)=>a.i-b.i);assert.deepEqual(partition.map(x=>x.r[0]),rules.map(r=>r[0]));assert.equal(new Set(partition.map(x=>x.r[0])).size,rules.length);
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{
 process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/discovery-argument-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,from,to]of rules.filter((_,i)=>bank===null||i%2===bank)){const graph=captureInputs(root),file=path.join(graph.directory,relative),raw=fs.readFileSync(file,'utf8');fs.writeFileSync(file,raw.replace(from,to));const out=run(graph.directory);assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1200));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1200));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('construction target'),name+': construction failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'isolated-F01-discovery-argument-sensitivity',bank,bankCount:2,totalMutations:rules.length,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,gatewayOwnershipFullPublicOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
