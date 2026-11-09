import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),rules=[
 ['invocation-publication.mjs','authority-domain-not-checked','const n=normalizeSearchArguments(envelope,authorityContext,p,maximum);if(!n||!current(o,n.record.context))','const n=normalizeSearchArguments(envelope,authorityContext,p,maximum);if(!n)'],
 ['invocation-publication.mjs','workspace-boundary-overaccepted','workspace=8*maximum;if(!natural(workspace)||workspace>o.limits.maxHeldBytes-o.charge)','workspace=8*maximum;if(!natural(workspace)||workspace>o.limits.maxHeldBytes-o.charge+1)'],
 ['invocation-publication.mjs','workspace-not-refunded','o.charge-=charged;o.searchDecodeCharge=0;o.searchDecoding=false;','o.searchDecodeCharge=0;o.searchDecoding=false;'],
 ['invocation-publication.mjs','workspace-overrefunded','o.charge-=charged;o.searchDecodeCharge=0;o.searchDecoding=false;','o.charge-=charged+1;o.searchDecodeCharge=0;o.searchDecoding=false;'],
 ['invocation-publication.mjs','workspace-observation-retained','o.searchDecodeCharge=0;o.searchDecoding=false;','o.searchDecoding=false;'],
 ['invocation-publication.mjs','busy-observation-retained','o.searchDecodeCharge=0;o.searchDecoding=false;','o.searchDecodeCharge=0;'],
 ['invocation-publication.mjs','source-loss-masked-by-query-capacity','if(!captured)return unavailable();','if(!captured||captured.current.capacity!==true)return unavailable();'],
 ['invocation-publication.mjs','source-loss-masked-by-host-resources','if(!captured)return unavailable();','if(!captured||captured.facts.resourcesReady!==true)return unavailable();'],
 ['invocation-publication.mjs','current-request-repaired-from-command','captured.sourceContext,captured.current,captured.facts','captured.sourceContext,{...captured.current,request:searchProducerRequest(n)},captured.facts'],
 ['search-arguments.mjs','current-opaque-replaced-with-null','context:opaque}},facts:packet.value.facts','context:null}},facts:packet.value.facts'],
 ['search-arguments.mjs','current-capacity-promoted','current:{...packet.value.current,request:','current:{...packet.value.current,capacity:true,request:'],
 ['search-arguments.mjs','source-grant-promoted','facts:packet.value.facts','facts:{...packet.value.facts,sourceReadable:true}'],
 ['search-arguments.mjs','source-scope-repaired','sourceContext:packet.value.sourceContext','sourceContext:{...packet.value.sourceContext,recipient:"reader"}']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/search-gateway-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [relative,name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,'formal',relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-synchronous-F05-decoded-search-gateway-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,physicalHostileProxyAsyncPublicEventsOrIndependentProof:false}));
