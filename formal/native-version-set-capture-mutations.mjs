import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-version-set-capture.mjs',rules=[
 ['source-values-not-from-actual-cut','source:d.source,sourceRevision:d.revision','source:d.source.map(n=>n+1),sourceRevision:d.revision'],
 ['root-measurement-workspace-omitted','captured.bytes+rootWorkspace+','captured.bytes+'],
 ['root-workspace-uses-smaller-local-threshold','rootWorkspace=owner.limits.maxRootBytes','rootWorkspace=p.maxRootBytes'],
 ['actual-parent-headroom-ignored','charge>owner.limits.maxHeldBytes-owner.charge','false'],
 ['root-input-off-by-one','>p.maxRootBytes||','>p.maxRootBytes+1||'],
 ['candidate-capture-off-by-one','copy({...candidate,nativeFacets},p,p.maxCaptureBytes)','copy({...candidate,nativeFacets},p,p.maxCaptureBytes+1)'],
 ['scope-frame-off-by-one','copy({scope:value.scope,invalidated:false},p,p.maxContextBytes)','copy({scope:value.scope,invalidated:false},p,p.maxContextBytes+1)'],
 ['cut-selector-off-by-one','copy(cut,p,p.maxSelectorBytes)','copy(cut,p,p.maxSelectorBytes+1)'],
 ['request-envelope-off-by-one','copy({request,host,facts:g},p,p.maxRequestBytes)','copy({request,host,facts:g},p,p.maxRequestBytes+1)'],
 ['prepared-count-off-by-one','m.cuts.size>=p.maxPrepared','m.cuts.size>p.maxPrepared'],
 ['spent-issuance-off-by-one','m.issued>=p.maxIssued','m.issued>p.maxIssued'],
 ['member-source-grant-ignored','source&&facts.sourceReadable!==true','false'],
 ['member-annotation-grants-ignored',"annotations&&!yes(facts,['annotationReadable','visibilityComplete','projectionOrderAttested'])",'false'],
 ['original-cut-looked-up-by-copy','export function inspectNativeVersionSetCapture(owner,domain,handle,cut,g){const m=pool(owner,domain,handle),held=m?.cuts.get(cut)','export function inspectNativeVersionSetCapture(owner,domain,handle,cut,g){const m=pool(owner,domain,handle),held=m?.cuts.get(structuredClone(cut))'],
 ['capacity-not-enforced','m.cuts.size>=p.maxPrepared','false'],
 ['issuance-not-enforced','m.issued>=p.maxIssued','false'],
 ['issued-budget-not-spent','m.cuts.set(cut,{...metadata,value});m.issued++;return cut;','m.cuts.set(cut,{...metadata,value});return cut;'],
 ['candidate-not-retained','m.cuts.set(cut,{...metadata,value});',''],
 ['observed-loss-not-persistent','if(!held||held.invalidated)return false;','if(!held)return false;'],
 ['other-required-member-loss-ignored','paired.some(({member,facts})=>','paired.slice(0,1).some(({member,facts})=>'],
 ['current-owner-fault-not-observed','const lost=owner.faulted||','const lost=false||'],
 ['current-member-stamp-change-ignored','!equal(facts.stamp,member.stamp)||','false||'],
 ['discard-refunds-spent-issuance',"return !!m&&yes(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])&&m.cuts.delete(cut);","if(m&&yes(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])&&m.cuts.delete(cut)){m.issued--;return true;}return false;"],
 ['protected-discard-gates-ignored',"yes(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])&&m.cuts.delete(cut)",'true&&m.cuts.delete(cut)'],
 ['pool-retirement-keeps-parent-charge','owner.charge-=m.charge;','void m.charge;'],
 ['closed-pool-can-reopen','pools.has(owner)','pools.get(owner)&&!pools.get(owner).closed']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-version-set-capture-assurance-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-native-version-set-preparation-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,nativeInstallationParentQueriesPhysicalOrIndependentProof:false}));
