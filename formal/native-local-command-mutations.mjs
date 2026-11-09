// Coherent graph mutations; syntax/import/construction failures never count as semantic detection.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const rules=[
 ['native-current-limit-binding-ignored','invocation-publication.mjs','equal(held.nativeLimits,nativeLocalCommandLimitBinding(held.inputFamily,nativeLocalCommandHost(g,held.inputFamily)))','true'],
 ['native-truthy-permission-accepted','invocation-publication.mjs',"gates(h,family==='F13'?['authorized','bounded','notStopped','interpretationCurrent','companionsReady','historyReady','annotationWritable']:['authorized','bounded','notStopped','interpretationCurrent','companionsReady','versionsAvailable','historyReady','invocationFresh'])",'true'],
 ['changed-native-public-intent-reused','invocation-publication.mjs','equal(held.capture,capture.value)','true','nativeLocalCommandPreparation'],
 ['native-parent-debit-omitted','invocation-publication.mjs','entry.ordinaryCharge=charge;o.charge+=charge;','entry.ordinaryCharge=charge;o.charge+=0;','nativeLocalCommandPreparation'],
 // Both guards enforce the same parent capacity: the second also catches a
 // negative remainder after an unchecked base debit, even when extra is zero.
 ['native-parent-headroom-ignored','invocation-publication.mjs','charge>o.limits.maxHeldBytes-o.charge','false','nativeLocalCommandPreparation','extra>o.limits.maxHeldBytes-o.charge','false'],
 ['reversal-plan-retirement-misclassified','invocation-publication.mjs',"held.job.kind==='reversal'?true:",''],
 ['annotation-plan-retirement-misclassified','invocation-publication.mjs',"held.job.kind==='annotation'?true:",''],
 ['native-original-create-allocation-replaced','annotation-editing.mjs','!equal(p.value.allocation,h.annotationAllocation)','false'],
 ['native-history-plan-refreshed-at-service','invocation-publication.mjs','publishReversal(before,entry.plan,host.historyHost)','publishReversal(before,prepareReversal(before,entry.plan.request,host.historyHost),host.historyHost)'],
 ['native-role-hosts-interchanged','invocation-publication.mjs',"return family==='F13'?g?.annotationHost:g?.historyHost;","return family==='F13'?g?.historyHost:g?.annotationHost;"],
 ['annotation-incorrectly-requires-source-write','invocation-publication.mjs',"return gates(g,['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady'])&&h&&","return g.sourceWritable===true&&gates(g,['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady'])&&h&&"],
 ['native-construction-charge-omitted','invocation-publication.mjs','entry.ordinaryCharge+=extra;o.charge+=extra;','entry.ordinaryCharge+=0;o.charge+=0;'],
 ['native-construction-headroom-ignored','invocation-publication.mjs','extra>o.limits.maxHeldBytes-o.charge','false'],
 ['native-required-limit-coordinate-ignored','native-local-command-arguments.mjs','keys.every(k=>Object.hasOwn(v,k)&&natural(v[k]))','true']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-local-command-checks.mjs')],{encoding:'utf8',timeout:60000,maxBuffer:1048576,env:process.env});
const first=run(root);assert.equal(first.status,0,first.stderr);const baseline=JSON.parse(first.stdout),mutations=[];
for(const [name,file,from,to,scope,secondFrom,secondTo] of rules){const c=captureInputs(root),target=path.join(c.directory,'formal',file),raw=fs.readFileSync(target,'utf8'),a=scope?raw.indexOf('function '+scope+'('):0,b=scope?raw.indexOf('// The existing F13/F14',a):raw.length;assert(a>=0&&b>a,name+': scope');let region=raw.slice(a,b);assert.equal(region.split(from).length-1,1,name+': anchor');region=region.replace(from,to);if(secondFrom){assert.equal(region.split(secondFrom).length-1,1,name+': second anchor');region=region.replace(secondFrom,secondTo);}fs.writeFileSync(target,raw.slice(0,a)+region+raw.slice(b));const r=run(c.directory);assert.equal(r.status,1,name+': '+r.stderr);assert(r.stderr.includes('ERR_ASSERTION'),name+': not assertion detection');assert(!r.stderr.includes('SyntaxError'),name+': construction failure');mutations.push(name);}
console.log(JSON.stringify({scope:'standalone-native-F13-F14-binding-mutations',baseline,mutations,
 coherentGraphBaseline:true,constructionFailuresCounted:false,U01Selected:false,
 fullQualificationOrIndependentAssurance:false}));
