import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-version-set-contexts.mjs',rules=[
 ['search-member-currentness-skipped','!validateNativeVersionSetHeld(owner,held,m.profile,g,g)','false'],
 ['search-hold-capacity-ignored','m.holds.size>=m.profile.maxHolds','false'],
 ['search-shared-issuance-ignored','m.issued>=m.profile.maxIssued','false'],
 ['search-parent-hold-not-retained','held.holds++;m.issued++;return lease;','m.issued++;return lease;'],
 ['search-hold-issuance-not-spent','held.holds++;m.issued++;return lease;','held.holds++;return lease;'],
 ['search-hold-not-released','row.held.holds--;return true;','return true;'],
 ['search-protected-discharge-bypassed','row.published!==published||published&&!yes(g,discharge)','row.published!==published'],
 ['parent-retired-with-live-search-result','!held?.invalidated||held.holds!==0||!yes(g,discharge)','!held?.invalidated||!yes(g,discharge)']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-version-set-search-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);
 // Exercise the lower discharge guard independently of the redundant outer
 // result-owner guard. First prove that this contextual baseline still passes.
 if(name==='search-protected-discharge-bypassed'){const outer=path.join(graph.directory,'formal/native-source-search.mjs'),raw=fs.readFileSync(outer,'utf8'),marker='!yes(facts,discharge)||m.results.get(result)!==held||!issuers[held.issuer.kind].retire';assert.equal(raw.split(marker).length-1,1);fs.writeFileSync(outer,raw.replace(marker,'m.results.get(result)!==held||!issuers[held.issuer.kind].retire'));const contextual=run(graph.directory);assert.equal(contextual.status,0,contextual.stderr);}
 fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-native-version-set-search-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,lowerDischargeGuardHasOuterGuardDisabledContextualBaseline:true,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allProfilesAsyncPhysicalGlobalOrIndependentProof:false}));
