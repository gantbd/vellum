import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-version-set-contexts.mjs',rules=[
 ['actual-parent-headroom-ignored','charge>owner.limits.maxHeldBytes-owner.charge','false'],
 ['local-complete-reservation-ignored','charge>p.maxHeldBytes','false'],
 ['installed-pool-charge-not-recorded','owner.charge+=charge;return handle;','return handle;'],
 ['whole-set-currentness-skipped','if(!validateNativeVersionSetCapture(owner,domain,capturePool,cut,packet.host,packet.facts))return no();',''],
 ['group-count-ignored','m.parents.size>=p.maxGroups','false'],
 ['complete-selector-reservation-ignored','count>p.maxSelectors-m.selectors.size','false'],
 ['complete-issuance-reservation-ignored','count>p.maxIssued-m.issued','false'],
 ['complete-registry-byte-bound-ignored','!copy(acquired.state,p,p.maxRegistryBytes)','false'],
 ['whole-reply-envelope-ignored','!copy(reply,p,p.maxReplyBytes)','false'],
 ['parent-row-not-installed','parents.set(context,held);',''],
 ['only-parent-alias-installed','parents.set(context,held);for(const row of pending)selectors.set(row.selector,row.record);','parents.set(context,held);for(const row of pending.slice(0,1))selectors.set(row.selector,row.record);'],
 ['cut-not-consumed','if(!consumeNativeVersionSetCapture(owner,domain,capturePool,cut,packet.host,packet.facts))return no();',''],
 ['native-issuance-not-spent','m.issued+=count;return reply;','return reply;'],
 ['member-stop-does-not-stop-parent',"held.invalidated=true;return {status:'stopped'};","return {status:'stopped'};"],
 ['protected-stop-accepts-foreign-scope','!equal(c,held.scope)','false'],
 ['retirement-keeps-member-aliases','if(row.parent===alias.parent)m.selectors.delete(selector);','if(false)m.selectors.delete(selector);'],
 ['retirement-refunds-issued-identities','return m.parents.delete(alias.parent);','m.issued=0;return m.parents.delete(alias.parent);'],
 ['pool-retirement-keeps-parent-charge','owner.charge-=m.charge;','void m.charge;']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-version-set-installation-assurance-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8'),a=original.indexOf('export function installNativeVersionSetContext('),b=original.indexOf('export function inspectNativeVersionSetContext(',a);assert(a>=0&&b>a);const install=original.slice(a,b),start=install.includes(from)?a:0,end=install.includes(from)?b:original.length,region=original.slice(start,end);assert.equal(region.split(from).length-1,1,name);fs.writeFileSync(file,original.slice(0,start)+region.replace(from,to)+original.slice(end));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-native-version-set-installation-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,memberQueriesF05PhysicalOrIndependentProof:false}));
