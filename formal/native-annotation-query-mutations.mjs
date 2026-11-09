import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-contexts.mjs',rules=[
 ['whole-page-selector-reservation-omitted','pending.length>m.profile.maxSelectors-m.selectors.size','false'],
 ['whole-page-issuance-reservation-omitted','pending.length>m.profile.maxIssued-m.issued','false'],
 ['whole-page-reply-reservation-omitted','if(!copy(reply,m.profile,m.profile.maxReplyBytes))return no();',''],
 ['repeated-page-reissues-records','const existing=ctx.records.get(row.reference);','const existing=null;'],
 ['repeated-page-reissues-continuation','cursorKey===null?null:ctx.continuations.get(cursorKey)','null'],
 ['record-can-cross-context',"selected?.kind!=='record'||selected.context!==r.snapshot","selected?.kind!=='record'"],
 ['continuation-can-cross-context',"selected?.kind!=='continuation'||selected.context!==r.snapshot||selected.limit!==r.limit","selected?.kind!=='continuation'||selected.limit!==r.limit"],
 ['foreign-scope-can-poison-context','if(!scope||!equal(scope,ctx.scope))return false;','if(!scope)return false;'],
 ['observed-loss-not-absorbed','ctx.stopped=true;return false;','return false;'],
 ['unknown-facts-invalidate-context','required.some(k=>g?.[k]===false)','required.some(k=>g?.[k]!==true)'],
 ['truthy-current-facts-accepted','ks.every(k=>g?.[k]===true)','ks.every(k=>g?.[k])'],
 ['raw-record-reference-released','reference:records.get(row.reference)','reference:row.reference'],
 ['raw-continuation-offset-released','continuation:continuation??null','continuation:out.next'],
 ['retirement-keeps-query-aliases','if(row.context===context)m.selectors.delete(key);','if(false)m.selectors.delete(key);']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-annotation-query-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
console.log(JSON.stringify({scope:'isolated-native-annotation-query-sensitivity',baseline:JSON.parse(positive.stdout),mutations:killed,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,sourceQueriesAsyncPhysicalOrPublicWireProved:false}));
