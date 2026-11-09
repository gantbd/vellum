// Coherent copied-graph sensitivity; module/tool failures are not semantic kills.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const rules=[
  ['broadcast-instead-of-single-replacement','selection-products.mjs','prepareSelectionProduct(state,lowered,g,limits,[edit])','prepareSelectionProduct(state,lowered,g,limits,null)'],
  ['primary-zero-instead-of-original','selection-products.mjs','const caret=edit.lo+edit.ins.length,primary=target.v.primary;','const caret=edit.lo+edit.ins.length,primary=0;'],
  ['missing-exact-expected-content','selection-products.mjs','if(!equal(edit.expected,target.d.source.slice(edit.lo,edit.hi)))return null;',''],
  ['missing-original-composition-intent','selection-products.mjs','p.compositionRequest=copy(request);',''],
  ['drop-secondary-mapping','selection-products.mjs','const mapped=target.v.selections.map(s=>mapSelection(target.d.source,[edit],s));','const mapped=target.v.selections.map(s=>copy(s));'],
  ['wrong-primary-caret','selection-products.mjs','const caret=edit.lo+edit.ins.length,primary=target.v.primary;','const caret=edit.lo,primary=target.v.primary;'],
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/composition-binding-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const baseline=run(root);assert.equal(baseline.status,0,baseline.stderr);const control=JSON.parse(baseline.stdout),mutations=[];
for(const [name,file,from,to] of rules){const graph=captureInputs(root),target=path.join(graph.directory,'formal',file),source=fs.readFileSync(target,'utf8');assert.equal(source.split(from).length-1,1,name);fs.writeFileSync(target,source.replace(from,to));const r=run(graph.directory);assert.equal(r.status,1,name+': '+r.stderr);assert(r.stderr.includes('ERR_ASSERTION'),name+': not an assertion failure');assert(!r.stderr.includes('SyntaxError'),name+': construction failure');mutations.push(name);}
console.log(JSON.stringify({scope:'primary-only-accepted-composition-reference',baseline:control,mutations,coherentGraphBaseline:true,constructionFailuresCounted:false,fullSuiteOrNativeIMEProved:false}));
