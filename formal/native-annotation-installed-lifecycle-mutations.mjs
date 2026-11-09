import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-contexts.mjs',rules=[
 ['actual-disposal-does-not-fence-context','ds.length!==1||ds[0].live!==true||required.some','ds.length!==1||false||required.some'],
 ['actual-owner-fault-does-not-fence-context','if(owner.faulted||ds.length!==1','if(false||ds.length!==1'],
 ['protected-context-discharge-bypassed','if(!ctx?.stopped||ctx.holds!==0||!yes(g,discharge))return false;','if(!ctx?.stopped||ctx.holds!==0)return false;'],
 ['live-context-pool-can-retire','m.contexts.size||m.selectors.size||!yes(g,discharge)','false||!yes(g,discharge)'],
 ['retirement-keeps-dependent-aliases','if(row.context===context)m.selectors.delete(key);','if(false)m.selectors.delete(key);'],
 ['pool-refunds-other-owner-obligations','owner.charge-=m.charge;','owner.charge=0;']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-annotation-installed-lifecycle-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
console.log(JSON.stringify({scope:'isolated-installed-annotation-lifecycle-sensitivity',baseline:JSON.parse(positive.stdout),mutations:killed,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,physicalConcurrentCrashOrIndependentProof:false}));
