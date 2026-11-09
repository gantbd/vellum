import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-contexts.mjs',rules=[
 ['observed-loss-does-not-close-registry','ctx.stopped=true;return false;','return false;'],
 ['retirement-keeps-dependent-selectors','if(row.context===context)m.selectors.delete(key);','if(false)m.selectors.delete(key);'],
 ['context-row-refunds-live-pool-charge','return m.contexts.delete(context);','owner.charge-=m.charge;return m.contexts.delete(context);'],
 ['context-retirement-refunds-spent-issuance','return m.contexts.delete(context);','m.issued=0;return m.contexts.delete(context);'],
 ['active-context-can-retire','if(!ctx?.stopped||ctx.holds!==0||!yes(g,discharge))return false;','if(!ctx||ctx.holds!==0||!yes(g,discharge))return false;']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-annotation-registry-fsm-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
console.log(JSON.stringify({scope:'isolated-native-annotation-registry-trace-sensitivity',baseline:JSON.parse(positive.stdout),mutations:killed,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,unboundedConcurrentPhysicalOrIndependentProof:false}));
