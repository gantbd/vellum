import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/history-query-arguments.mjs',rules=[
 ['variant-agreement-ignored','decoded.variant!==args.operation','false'],
 ['extra-arguments-admitted','!exact(args,form.fields)','!args'],
 ['maximum-ceiling-ignored','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['byte-bound-off-by-one','Math.min(maximum,limits.maxRequestBytes)','Math.min(maximum,limits.maxRequestBytes)+1'],
 ['node-bound-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputNodes:limits.maxInputNodes+1,maxRequestBytes:'],
 ['depth-bound-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputDepth:limits.maxInputDepth+1,maxRequestBytes:'],
 ['original-history-handle-cloned','const opaque=args.context,','const opaque=structuredClone(args.context),'],
 ['route-enters-canonical-meaning','{request:r,context:c,limits,opaqueBindings:','{route:e.route,request:r,context:c,limits,opaqueBindings:'],
 ['raw-request-alias-retained','record:capture.value','record:{...capture.value,request:args}'],
 ['authority-frame-extra-fields-admitted','!exact(bound.value,contextFields)','false'],
 ['scope-value-domain-ignored','!text(r.scope)','false'],
 ['profile-value-domain-ignored','!text(r.profile)','false'],
 ['direction-vocabulary-ignored','!form.directions.includes(r.direction)','false'],
 ['group-value-domain-ignored',"r.operation!=='frontier'&&r.group!==null&&!text(r.group)",'false'],
 ['null-frontier-selection-refused',"r.group!==null&&!text(r.group)","!text(r.group)"],
 ['unsupported-profile-preempted','const capture=copySettingsData(',"if(r.profile!=='history-projection')return null;const capture=copySettingsData("],
 ['original-handle-agreement-ignored','a.opaque===b.opaque&&',''],
 ['ordinary-argument-agreement-ignored','&&equal(a.record,b.record)',''],
 ['producer-handle-cloned','context:n.opaque','context:structuredClone(n.opaque)'],
 ['producer-handle-replaced-by-shadow','context:n.opaque','context:n.record.request.context'],
 ['classification-bound-incorrect','export const historyQueryArgumentClassificationBytes=Buffer.byteLength','export const historyQueryArgumentClassificationBytes=1+Buffer.byteLength'],
 ['captured-limit-fields-not-exact','exact(p,limitFields)','p'],
 ['captured-limits-mutable','?Object.freeze(p):null','?p:null'],
 ['policy-meaning-not-captured','context:c,limits,opaqueBindings:','context:{...c,policy:"stable"},limits,opaqueBindings:']
];
for(const[name,from]of rules){const source=fs.readFileSync(path.join(root,relative),'utf8');assert.equal(source.split(from).length-1,1,name+': construction target');}
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/history-query-argument-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),raw=fs.readFileSync(file,'utf8');fs.writeFileSync(file,raw.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1200));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1200));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'isolated-F04-history-query-argument-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,gatewayOwnershipFullPublicOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
