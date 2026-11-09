import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const rules=[
 ['private-annotation-dependency-ignored','annotation-compounds.mjs','d.revision!==input.expectedSource||d.annotationSet!==input.expectedAnnotationSet','d.revision!==input.expectedSource'],
 ['changed-public-intent-reused','invocation-publication.mjs','sameExactBatchArguments({record:held.capture.arguments},{record})&&equal(held.capture,capture.value)','true'],
 ['current-batch-service-rights-ignored','invocation-publication.mjs',"entry.ordinaryPrepared?.inputFamily==='F11-exact-batch'&&!exactBatchPermissions(host)",'false'],
 ['canonical-owner-headroom-ignored','invocation-publication.mjs','capture.bytes>o.limits.maxHeldBytes-o.charge','false','exactBatchPreparation'],
 ['compound-data-retirement-misclassified','invocation-publication.mjs',"held.job.kind==='compound'?true:",''],
 ['public-replay-not-canonicalized','invocation-publication.mjs','const record={...n.record,request:{...r,sources:shadow.sources.map(({expectedAnnotationSet,...s})=>s),reads:shadow.reads,scopes:shadow.scopes}};','const record=n.record;'],
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/exact-batch-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const initial=run(root);assert.equal(initial.status,0,initial.stderr);const baseline=JSON.parse(initial.stdout),mutations=[];
for(const [name,file,from,to,scope] of rules){const graph=captureInputs(root),target=path.join(graph.directory,'formal',file),raw=fs.readFileSync(target,'utf8'),a=scope?raw.indexOf('function '+scope+'('):0,b=scope?raw.indexOf('// Preparation remains private:',a):raw.length;assert(a>=0&&b>a);const region=raw.slice(a,b);assert.equal(region.split(from).length-1,1,name);fs.writeFileSync(target,raw.slice(0,a)+region.replace(from,to)+raw.slice(b));const r=run(graph.directory);assert.equal(r.status,1,name+': '+r.stderr);assert(r.stderr.includes('ERR_ASSERTION'),name);assert(!r.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'source-only-exact-batch-native-binding',baseline,mutations,coherentGraphBaseline:true,constructionFailuresCounted:false,mixedAuthoringPhysicalOrIndependentProof:false}));
