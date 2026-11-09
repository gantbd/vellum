import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-version-set-contexts.mjs',rules=[
 ['retained-registry-growth-bound-ignored','!copy(acquired.state,p,p.maxRegistryBytes)','false'],
 ['retained-alias-metadata-growth-bound-ignored','!copy(row.record,p,p.maxSelectorRecordBytes)','false'],
 ['member-selector-accepted-as-retained-parent',"alias?.kind==='parent'?m.parents.get(reference):null",'alias?m.parents.get(alias.parent):null'],
 ['original-whole-parent-currentness-skipped','if(!validateNativeVersionSetHeld(owner,original,p,packet.host,packet.facts))return no();',''],
 ['retained-group-capacity-ignored','m.parents.size>=p.maxGroups','false'],
 ['retained-complete-selector-capacity-ignored','count>p.maxSelectors-m.selectors.size','false'],
 ['retained-spent-issuance-capacity-ignored','count>p.maxIssued-m.issued','false'],
 ['complete-retained-packet-bound-ignored','host,facts:g},p,p.maxRequestBytes)?.value','host,facts:g},p,Number.MAX_SAFE_INTEGER)?.value'],
 ['retained-source-refreshed-from-current-root','source:member.candidate.root.source','source:owner.root.documents.find(d=>equal(d.document,member.document)).source'],
 ['annotation-only-copy-retains-source',"...(source?{source:member.candidate.root.source,sourceRevision:member.candidate.root.sourceRevision}:{})","...({source:member.candidate.root.source,sourceRevision:member.candidate.root.sourceRevision})"],
 ['annotation-only-copy-retains-source-profile','sourceProfile:source?member.candidate.sourceProfile:null','sourceProfile:member.candidate.sourceProfile'],
 ['source-only-copy-retains-annotations','annotations:annotations?member.candidate.annotations:null','annotations:member.candidate.annotations'],
 ['source-only-copy-retains-private-annotation-facets',"original.value.nativeFacets.filter(row=>selected.some(({member,data})=>equal(member.document,row.document)&&data.includes('annotations')))",'original.value.nativeFacets'],
 ['explicit-member-subset-ignored','selected.map(({member,data})=>','original.value.members.map(member=>({member,data:member.candidate.data})).map(({member,data})=>'],
 ['duplicate-logical-owner-accepted','keys.has(key)||matches.length!==1','matches.length!==1'],
 ['closed-class-domain-ignored','participant.data.some(k=>!matches[0].candidate.data.includes(k))','false'],
 ['retained-parent-not-installed','retainedParents.set(context,held);',''],
 ['retained-member-aliases-not-installed','for(const row of pending)retainedSelectors.set(row.selector,row.record);',''],
 ['retained-issuance-not-spent','m.issued+=count;return reply;','return reply;']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-retained-version-set-assurance-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8'),a=original.indexOf('export function acquireNativeRetainedVersionSetContext('),b=original.indexOf('function normalizedMemberQuery(',a);assert(a>=0&&b>a);const region=original.slice(a,b);assert.equal(region.split(from).length-1,1,name);fs.writeFileSync(file,original.slice(0,a)+region.replace(from,to)+original.slice(b));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-native-retained-version-set-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allHistoryProfilesAsyncPhysicalOrIndependentProof:false}));
