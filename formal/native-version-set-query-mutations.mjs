import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-version-set-contexts.mjs',rules=[
 ['query-registry-byte-bound-ignored','if(!copy(out.state,m.profile,m.profile.maxRegistryBytes))return no();',''],
 ['query-whole-reply-byte-bound-ignored','!copy(reply,m.profile,m.profile.maxReplyBytes)','false'],
 ['query-selector-byte-bound-ignored','!copy(row.selector,m.profile,m.profile.maxSelectorBytes)','false'],
 ['query-selector-metadata-bound-ignored','!copy(row.record,m.profile,m.profile.maxSelectorRecordBytes)','false'],
 ['query-selector-capacity-ignored','pending.length>m.profile.maxSelectors-m.selectors.size','false'],
 ['query-spent-issuance-capacity-ignored','pending.length>m.profile.maxIssued-m.issued','false'],
 ['query-issuance-not-spent','m.issued+=pending.length;return reply;','return reply;'],
 ['native-whole-parent-validation-skipped','!packet||!validateNativeVersionSetHeld(owner,held,m.profile,packet.host,packet.facts)','!packet'],
 ['source-context-exposes-private-selector','reply={...out.result,context:r.snapshot};','reply={...out.result};'],
 ['page-record-exposes-private-selector',"reference:bind(row.reference,'record')","reference:row.reference"],
 ['page-continuation-exposes-private-selector',"bind(out.result.continuation,'continuation')","out.result.continuation"],
 ['record-reference-loses-original-identity','record:{...out.result.record,reference:r.annotation}','record:{...out.result.record,reference:structuredClone(r.annotation)}'],
 ['query-selectors-not-published','held.bindings=bindings;m.selectors=selectors;','held.bindings=bindings;'],
 ['query-registry-not-retained','held.registry=out.state;',''],
 ['query-bindings-not-retained','held.bindings=bindings;',''],
 ['record-accepted-as-context-control',"held=['parent','member'].includes(alias?.kind)?m.parents.get(alias.parent):null,c=copy(scope",'held=alias?m.parents.get(alias.parent):null,c=copy(scope'],
 ['record-accepted-as-context-validation',"held=['parent','member'].includes(alias?.kind)?m.parents.get(alias.parent):null;return !!m",'held=alias?m.parents.get(alias.parent):null;return !!m'],
 ['record-accepted-as-context-retirement',"held=['parent','member'].includes(alias?.kind)?m.parents.get(alias.parent):null;if(!held?.invalidated",'held=alias?m.parents.get(alias.parent):null;if(!held?.invalidated']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-version-set-query-assurance-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-native-version-set-query-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allLimitsF05PhysicalOrIndependentProof:false}));
