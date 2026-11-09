import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),rules=[
 ['actual-disposal-not-observed','native-version-set-capture.mjs','ds.length!==1||ds[0].live!==true||!equal(facts.stamp','ds.length!==1||false||!equal(facts.stamp'],
 ['actual-owner-fault-not-observed','native-version-set-capture.mjs','const lost=owner.faulted||required.some','const lost=false||required.some'],
 ['observed-loss-not-recorded','native-version-set-capture.mjs','if(lost){held.invalidated=true;return false;}','if(lost){return false;}'],
 ['protected-parent-discharge-bypassed','native-version-set-contexts.mjs','!held?.invalidated||held.holds!==0||!yes(g,discharge)','!held?.invalidated||held.holds!==0'],
 ['live-set-pool-can-retire','native-version-set-contexts.mjs','m.parents.size||m.selectors.size||m.holds.size||!yes(g,discharge)','!yes(g,discharge)'],
 ['protected-pool-discharge-bypassed','native-version-set-contexts.mjs','m.parents.size||m.selectors.size||m.holds.size||!yes(g,discharge)','m.parents.size||m.selectors.size||m.holds.size'],
 ['dependent-aliases-not-retired','native-version-set-contexts.mjs','if(row.parent===alias.parent)m.selectors.delete(selector);','if(false)m.selectors.delete(selector);'],
 ['retirement-refunds-spent-identities','native-version-set-contexts.mjs','return m.parents.delete(alias.parent);','m.issued=0;return m.parents.delete(alias.parent);'],
 ['pool-refunds-other-owner-charges','native-version-set-contexts.mjs','owner.charge-=m.charge;','owner.charge=0;']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-version-set-lifecycle-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,relative,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,'formal',relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);if(name==='protected-pool-discharge-bypassed'){const kernel=path.join(graph.directory,'formal/invocation-publication.mjs');let text=fs.readFileSync(kernel,'utf8');for(const [a,b]of [["if(!Object.values(facts).every(Boolean)||!retire(o,facts))return false;","if(!retire(o,facts))return false;"],["const o=domains.get(domain);if(!o||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;","const o=domains.get(domain);if(!o)return false;"]]){assert.equal(text.split(a).length-1,1,name+': contextual guard');text=text.replace(a,b);}fs.writeFileSync(kernel,text);const contextual=run(graph.directory);assert.equal(contextual.status,0,contextual.stderr);}fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-native-version-set-lifecycle-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,lowerPoolGuardHasOuterGuardsDisabledContextualBaseline:true,F05PhysicalGlobalOrIndependentProof:false}));
