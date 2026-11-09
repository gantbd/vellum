import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/invocation-publication.mjs',rules=[
 ['read-authority-domain-ignored','!n||!current(o,n.record.context)','!n',2],
 ['read-workspace-boundary-overaccepted','workspace=7*maximum;if(!natural(workspace)||workspace>o.limits.maxHeldBytes-o.charge)','workspace=7*maximum;if(!natural(workspace)||workspace>o.limits.maxHeldBytes-o.charge+1)',1],
 ['read-workspace-not-refunded','o.charge-=charged;o.sourceReadDecodeCharge=0;o.sourceReadDecoding=false;','o.sourceReadDecodeCharge=0;o.sourceReadDecoding=false;',1],
 ['read-workspace-overrefunded','o.charge-=charged;o.sourceReadDecodeCharge=0;o.sourceReadDecoding=false;','o.charge-=charged+1;o.sourceReadDecodeCharge=0;o.sourceReadDecoding=false;',1],
 ['read-workspace-observation-retained','o.sourceReadDecodeCharge=0;o.sourceReadDecoding=false;','o.sourceReadDecoding=false;',1],
 ['read-busy-observation-retained','o.sourceReadDecodeCharge=0;o.sourceReadDecoding=false;','o.sourceReadDecodeCharge=0;',1],
 ['read-resource-refusal-masks-recognized-loss','if(!packet)return unavailable();\n  const readRequest','if(!packet||packet.value.facts.resourcesReady!==true)return unavailable();\n  const readRequest',1],
 ['source-read-port-omitted','?queryNativeSourceContext:nativeAnnotationContextPoolAvailable','?null:nativeAnnotationContextPoolAvailable',1],
 ['combined-read-port-omitted','?queryNativeCombinedSource:nativeVersionSetContextPoolAvailable','?null:nativeVersionSetContextPoolAvailable',1],
 ['coherent-read-port-omitted','?queryNativeVersionSetMember:null;if(!port)','?null:null;if(!port)',1],
 ['read-original-snapshot-copied','const readRequest=sourceReadProducerRequest(n),readResult=port(o,domain,handle,readRequest,packet.value.current,packet.value.facts);','const readRequest=structuredClone(sourceReadProducerRequest(n)),readResult=port(o,domain,handle,readRequest,packet.value.current,packet.value.facts);',1],
 ['read-source-grant-promoted','readRequest,packet.value.current,packet.value.facts','readRequest,packet.value.current,{...packet.value.facts,sourceReadable:true}',1],
 ['read-recipient-scope-repaired','readRequest,packet.value.current,packet.value.facts','readRequest,{...packet.value.current,recipient:"reader"},packet.value.facts',1]
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/source-read-gateway-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to,count]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,count,name);let edited;if(count===2){const start=original.indexOf('export function queryInvocationDecodedSourceRead('),end=original.indexOf('export function invocationSourceReadDecodeUsage',start);assert(start>=0&&end>start);const region=original.slice(start,end);assert.equal(region.split(from).length-1,1,name);edited=original.slice(0,start)+region.replace(from,to)+original.slice(end);}else edited=original.replace(from,to);fs.writeFileSync(file,edited);const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1000));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-decoded-F02-native-read-gateway-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,physicalHostileProxyAcquisitionOrIndependentProof:false}));
