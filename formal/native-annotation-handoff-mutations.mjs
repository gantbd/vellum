import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-capture.mjs',rules=[
 ['scope-binding-can-poison-cut','!equal(scope,held.scope)','false'],
 ['loss-is-not-absorbed','held.invalidated=true;return false;','return false;'],
 ['unknown-fact-establishes-loss','required.some(k=>g?.[k]===false)','required.some(k=>g?.[k]!==true)'],
 ['transient-capacity-invalidates-cut','required.some(k=>g?.[k]===false)',"[...required,'resourcesReady'].some(k=>g?.[k]===false)"],
 ['actual-disposal-ignored','ds.length!==1||ds[0].live!==true||required.some','ds.length!==1||false||required.some'],
 ['preflight-outside-guard-ignored',"held.invalidated||g?.outsidePublicationGuard!==true",'held.invalidated'],
 ['cut-identity-compared-by-public-shape','held=m?.cuts.get(cut);if(!held||held.invalidated','held=[...(m?.cuts??[])].find(([key])=>equal(key,cut))?.[1];if(!held||held.invalidated'],
 ['source-grant-not-revalidated',"source&&g?.sourceReadable===false",'false'],
 ['projection-fact-not-revalidated',"'annotationReadable','projectionCurrent','retained'","'annotationReadable','retained'"]
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-annotation-handoff-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
console.log(JSON.stringify({scope:'isolated-native-annotation-current-handoff-sensitivity',baseline:JSON.parse(positive.stdout),mutations:killed,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,atomicInstallationOrQueryProductsProved:false}));
