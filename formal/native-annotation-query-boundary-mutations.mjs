import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-contexts.mjs',rules=[
 ['exact-selector-capacity-refused','pending.length>m.profile.maxSelectors-m.selectors.size','pending.length>=m.profile.maxSelectors-m.selectors.size'],
 ['exact-issuance-capacity-refused','pending.length>m.profile.maxIssued-m.issued','pending.length>=m.profile.maxIssued-m.issued'],
 ['exact-page-row-limit-refused','r.limit>m.profile.annotationProfile.maxPageRows','r.limit>=m.profile.annotationProfile.maxPageRows'],
 ['page-reply-off-by-one','copy(reply,m.profile,m.profile.maxReplyBytes)','copy(reply,m.profile,m.profile.maxReplyBytes-1)'],
 ['selector-metadata-off-by-one','copy(p.record,m.profile,m.profile.maxSelectorRecordBytes)','copy(p.record,m.profile,m.profile.maxSelectorRecordBytes-1)'],
 ['annotation-packet-off-by-one','},m.profile,m.profile.maxRequestBytes)?.value;return value?','},m.profile,m.profile.maxRequestBytes-1)?.value;return value?'],
 ['source-packet-off-by-one','value=copy({...request,snapshot:null},m.profile,m.profile.maxRequestBytes)?.value','value=copy({...request,snapshot:null},m.profile,m.profile.maxRequestBytes-1)?.value'],
 ['captured-source-content-off-by-one','},r,{grant:true,scope:true,continuity:true,capacity:true},source.profile);','},r,{grant:true,scope:true,continuity:true,capacity:true},{...source.profile,maxContentBytes:source.profile.maxContentBytes-1});'],
 ['captured-source-reply-off-by-one','},r,{grant:true,scope:true,continuity:true,capacity:true},source.profile);','},r,{grant:true,scope:true,continuity:true,capacity:true},{...source.profile,maxReplyBytes:source.profile.maxReplyBytes-1});'],
 ['outer-source-reply-off-by-one','copy(out,m.profile,m.profile.maxReplyBytes)','copy(out,m.profile,m.profile.maxReplyBytes-1)']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-annotation-query-boundaries.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
console.log(JSON.stringify({scope:'isolated-native-annotation-query-boundary-sensitivity',baseline:JSON.parse(positive.stdout),mutations:killed,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allProfilesPhysicalOrIndependentProof:false}));
