import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-capture.mjs',rules=[
 ['parent-charge-omitted','owner.charge+=charge;','void charge;'],
 ['parent-headroom-ignored','charge>owner.limits.maxHeldBytes-owner.charge','false'],
 ['truthy-preparation-facts-accepted','ks.every(k=>g?.[k]===true)','ks.every(k=>g?.[k])'],
 ['combined-source-grant-promoted',"r.data.includes('source')&&g?.sourceReadable!==true",'false'],
 ['annotation-only-captures-source',"...(r.data.includes('source')?{source:","...(true?{source:"],
 ['hidden-record-is-captured','if(!grant.visible)continue;',''],
 ['body-facet-widened','...(grant.body?{content:','...(true?{content:'],
 ['target-facet-widened','...(grant.target?{location:','...(true?{location:'],
 ['captured-body-is-not-authoritative','body:row.body,version:row.bodyVersion',"body:'forged',version:row.bodyVersion"],
 ['combined-source-is-not-authoritative','bytes:d.source','bytes:[0]'],
 ['duplicate-grant-defenses-bypassed','if(keys.has(key))return null;',''],
 ['prepared-count-not-enforced','m.cuts.size>=p.maxPrepared','false'],
 ['spent-issuance-not-enforced','m.issued>=p.maxIssued','false'],
 ['closed-pool-can-reopen','pools.has(owner)','pools.get(owner)&&!pools.get(owner).closed'],
 ['pool-charge-not-refunded','owner.charge-=m.charge;','void m.charge;'],
 ['owned-return-value-is-shared','structuredClone(held.value)','held.value']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-annotation-capture-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);let changed=original.replace(from,to);if(name==='duplicate-grant-defenses-bypassed'){assert.equal(changed.split('rows.length!==d.records.length').length-1,1);changed=changed.replace('rows.length!==d.records.length','false');}fs.writeFileSync(file,changed);const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
console.log(JSON.stringify({scope:'isolated-native-annotation-preparation-sensitivity',baseline:JSON.parse(positive.stdout),mutations:killed,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,installedContextOrQueryProductsProved:false}));
