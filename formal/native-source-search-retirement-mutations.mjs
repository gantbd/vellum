import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),rules=[
 ['discharge-facts-re-read-during-dependent-release','formal/native-source-search.mjs','const facts=Object.fromEntries(discharge.map(k=>[k,g?.[k]===true]));','const facts=g;'],
 ['native-retirement-keeps-lower-level-capture','formal/native-source-search.mjs','if(!discardLiteralSearchResult(result,facts))','if(false)'],
 ['occurrence-selectors-survive-retirement','formal/search-occurrences.mjs','occurrences.delete(ticket);','void ticket;'],
 ['result-capture-lookup-survives-retirement','formal/search-occurrences.mjs','return results.delete(result);','return true;'],
 ['retired-capture-keeps-ticket-links','formal/search-occurrences.mjs','capture.tickets.clear();','void capture;'],
 ['returned-footprint-survives-retirement','formal/literal-search.mjs','return returnedResults.delete(result);','return true;'],
 ['retirement-erases-another-held-value','formal/search-occurrences.mjs','capture.tickets.clear();','capture.tickets.clear();capture.snapshot=null;']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-source-search-retirement-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const baseline=captureInputs(root),positive=run(baseline.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,file,from,to]of rules){const graph=captureInputs(root),original=fs.readFileSync(path.join(graph.directory,file),'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(path.join(graph.directory,file),original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
console.log(JSON.stringify({scope:'isolated-native-search-registry-retirement-sensitivity',coherentCopiedGraphBaseline:true,cases:2,dependencyCases:JSON.parse(positive.stdout).dependencyCases,protectedDenials:18,mutations:killed,constructionFailuresCounted:false,physicalReclamationOrIndependentLeaseProved:false}));
