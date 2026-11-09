import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),rules=[
 ['replacement-admission-bypasses-native-owner','formal/invocation-publication.mjs','if(!nativeSourceSearchAllowsPreparation(o,domain,n.result,g.searchCurrent,g))return null;',''],
 ['selection-admission-bypasses-native-owner','formal/invocation-publication.mjs',"if(n.record.request.operation==='select-search-match'&&!nativeSourceSearchAllowsSelectionPreparation(o,domain,n.opaque,g.searchCurrent,g))return null;",''],
 ['native-result-issuer-not-recorded','formal/native-source-search.mjs','resultOwners.set(result,handle);','void result;'],
 ['native-occurrence-owner-not-recorded','formal/native-source-search.mjs','for(const match of result.matches)occurrenceResults.set(match.reference,result);',''],
 ['prepared-consumer-skips-native-currentness','formal/invocation-publication.mjs','!nativeSearchConsumerCurrent(o,domain,entry.ordinaryPrepared,host)','false'],
 ['live-managed-replacement-always-refused','formal/invocation-publication.mjs',"if(held.inputFamily==='F11')return nativeSourceSearchAllowsPreparation(o,domain,held.result,g.searchCurrent,g);","if(held.inputFamily==='F11')return false;"],
 ['live-managed-selection-always-refused','formal/invocation-publication.mjs',"if(held.inputFamily==='F12'&&held.capture.arguments.request.operation==='select-search-match')return nativeSourceSearchAllowsSelectionPreparation(o,domain,held.opaque,g.searchCurrent,g);","if(held.inputFamily==='F12'&&held.capture.arguments.request.operation==='select-search-match')return false;"]
];
const issuer=process.argv[2]??'source';assert(['source','combined','retained-source','retained-combined','version-set','retained-version-set'].includes(issuer));
const runner=issuer==='source'?'formal/native-source-search-consumer-witness.mjs':issuer==='combined'?'formal/native-combined-search-consumer-checks.mjs':issuer==='retained-source'?'formal/native-retained-source-consumer-checks.mjs':issuer==='version-set'?'formal/native-version-set-search-consumer-checks.mjs':issuer==='retained-version-set'?'formal/native-retained-version-set-consumer-checks.mjs':'formal/native-retained-annotation-consumer-checks.mjs';
const run=directory=>spawnSync(process.execPath,[path.join(directory,runner)],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const baseline=captureInputs(root),positive=run(baseline.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,file,from,to]of rules){const graph=captureInputs(root),original=fs.readFileSync(path.join(graph.directory,file),'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(path.join(graph.directory,file),original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
console.log(JSON.stringify({scope:'isolated-native-search-dependent-consumer-sensitivity',baseline:JSON.parse(positive.stdout),coherentCopiedGraphBaseline:true,mutations:killed,independentPreparedOwnershipTransferProved:false,constructionFailuresCounted:false}));
