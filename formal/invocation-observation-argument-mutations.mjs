import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/invocation-observation-arguments.mjs',rules=[
 ['variant-agreement-ignored','decoded.variant!==args.operation','false'],
 ['classification-bound-incorrect','export const invocationObservationArgumentClassificationBytes=Buffer.byteLength','export const invocationObservationArgumentClassificationBytes=1+Buffer.byteLength'],
 ['extra-arguments-admitted','!exact(args,form.fields)','!args'],
 ['maximum-ceiling-ignored','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['byte-bound-off-by-one','Math.min(maximum,limits.maxRequestBytes)','Math.min(maximum,limits.maxRequestBytes)+1'],
 ['node-bound-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputNodes:limits.maxInputNodes+1,maxRequestBytes:'],
 ['depth-bound-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputDepth:limits.maxInputDepth+1,maxRequestBytes:'],
 ['original-context-cloned','const context=args.context,','const context=structuredClone(args.context),'],
 ['original-ordinary-reference-cloned','after=waiting?args.after:null','after=waiting?structuredClone(args.after):null'],
 ['route-enters-canonical-meaning','{request:r,context:bound.value,limits,opaqueBindings:','{route:e.route,request:r,context:bound.value,limits,opaqueBindings:'],
 ['raw-request-alias-retained','record:captured.value','record:{...captured.value,request:args}'],
 ['authority-frame-extra-fields-admitted','!exact(bound.value,contextFields)','false'],
 ['invocation-value-domain-ignored','!text(r.invocation)','false'],
 ['profile-value-domain-ignored','!text(r.profile)','false'],
 ['zero-wait-budget-admitted','args.budget<1','args.budget<0'],
 ['context-agreement-ignored','a.context===b.context&&',''],
 ['ordinary-reference-agreement-ignored','a.after===b.after&&',''],
 ['producer-context-cloned','context:n.context','context:structuredClone(n.context)'],
 ['producer-reference-cloned','{after:n.after}','{after:structuredClone(n.after)}'],
 ['captured-limit-fields-not-exact','exact(p,limitFields)','p'],
 ['captured-limits-mutable','?Object.freeze(p):null','?p:null']
];
const source=fs.readFileSync(path.join(root,relative),'utf8');for(const[name,from]of rules)assert.equal(source.split(from).length-1,1,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{
 process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/invocation-observation-argument-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576,timeout:50000}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);assert(!baseline.error&&!baseline.signal);const mutations=[];
 for(const[name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),raw=fs.readFileSync(file,'utf8');fs.writeFileSync(file,raw.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1300));assert(!out.error&&!out.signal,name+': process failure');assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1300));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'isolated-F06-observation-argument-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,gatewayGlobalClockPhysicalWireOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
