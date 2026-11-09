// Semantic sensitivity in coherent copied graphs; construction failures do not count.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const rules=[
 ['captured-component-ceiling-widened','atomic-annotation-results.mjs','p.value.ceilings[index]','{identity:true,contentVersion:true,targetGeneration:true}'],
 ['current-component-grant-ignored','atomic-annotation-results.mjs','const grant=grantAt(g,receipts.length,index);',"const grant={annotationResultGrant:{invocation:p.value.values[index].request.invocation,annotation:p.value.values[index].request.annotation,identity:true,contentVersion:true,targetGeneration:true}};"],
 ['parent-headroom-ignored','atomic-annotation-results.mjs','charge>available','false'],
 ['changed-original-plan-accepted','atomic-annotation-results.mjs','Boolean(p&&equal(p.value.plan,plan))','Boolean(p)'],
 ['historical-vector-reconstructed-from-current-root','invocation-publication.mjs','entry.atomicAnnotationReceipts,index,g','stageAtomicAnnotationResults(held.atomicReceiptHold,o,domain,ticket,{status:"applied",state:o.root}),index,g'],
 ['forgotten-receipt-vector-retained','invocation-publication.mjs','entry.registrationReceipt=null;entry.atomicAnnotationReceipts=null;','entry.registrationReceipt=null;'],
 ['parent-reservation-not-debited','invocation-publication.mjs','o.charge+=retained.charge;','o.charge+=0;'],
 ['forget-prematurely-refunds-live-preparation','invocation-publication.mjs','entry.registrationReceipt=null;entry.atomicAnnotationReceipts=null;','domains.get(domain).charge-=entry.ordinaryCharge;entry.ordinaryCharge=0;entry.registrationReceipt=null;entry.atomicAnnotationReceipts=null;'],
 ['forget-accepts-truthy-nonboolean-discharge','invocation-publication.mjs',"if(entry?.ordinaryPrepared?.atomicReceiptHold&&!gates(g,['retentionReleased','deliveryDrained']))return unavailable();",'']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/atomic-annotation-result-checks.mjs')],{encoding:'utf8',timeout:60000,maxBuffer:1048576,env:process.env});
const first=run(root);assert.equal(first.status,0,first.stderr);const baseline=JSON.parse(first.stdout),mutations=[];
for(const [name,file,from,to] of rules){
 const c=captureInputs(root),target=path.join(c.directory,'formal',file),raw=fs.readFileSync(target,'utf8');assert.equal(raw.split(from).length-1,1,name);
 fs.writeFileSync(target,raw.replace(from,to));const result=run(c.directory);
 assert.equal(result.status,1,name+': '+result.stderr);assert(result.stderr.includes('ERR_ASSERTION'),name+': not assertion detection');assert(!result.stderr.includes('SyntaxError'),name+': construction failure');mutations.push(name);
}
console.log(JSON.stringify({scope:'owned-native-atomic-annotation-receipt-mutations',baseline,mutations,
 coherentGraphBaseline:true,constructionFailuresCounted:false,publicMixedIngressOrU01Selected:false,
 completeQualificationOrIndependentAssurance:false}));
