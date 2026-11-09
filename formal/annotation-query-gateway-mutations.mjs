import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/invocation-publication.mjs',rules=[
 ['annotation-domain-binding-ignored','!annotationQuery||!current(o,annotationQuery.record.context)','!annotationQuery'],
 ['annotation-workspace-overaccepted','annotationWorkspace>o.limits.maxHeldBytes-o.charge','annotationWorkspace>o.limits.maxHeldBytes-o.charge+1'],
 ['annotation-workspace-not-refunded','o.charge-=annotationCharge;o.annotationQueryDecodeCharge=0;','o.annotationQueryDecodeCharge=0;'],
 ['annotation-workspace-overrefunded','o.charge-=annotationCharge;o.annotationQueryDecodeCharge=0;','o.charge-=annotationCharge+1;o.annotationQueryDecodeCharge=0;'],
 ['annotation-workspace-observation-retained','o.annotationQueryDecodeCharge=0;o.annotationQueryDecoding=false;','o.annotationQueryDecoding=false;'],
 ['annotation-busy-observation-retained','o.annotationQueryDecodeCharge=0;o.annotationQueryDecoding=false;','o.annotationQueryDecodeCharge=0;'],
 ['annotation-native-loss-masked-by-transient-refusal','if(!annotationPacket)return unavailable();','if(!annotationPacket||annotationPacket.value.facts.resourcesReady!==true)return unavailable();'],
 ['annotation-native-port-omitted','?queryNativeAnnotationContext:nativeVersionSetContextPoolAvailable','?null:nativeVersionSetContextPoolAvailable'],
 ['annotation-coherent-port-omitted','?queryNativeVersionSetMember:null;if(!annotationPort)','?null:null;if(!annotationPort)'],
 ['annotation-all-original-selectors-cloned','const annotationRequest=annotationQueryProducerRequest(annotationQuery),annotationResult=','const annotationRequest=structuredClone(annotationQueryProducerRequest(annotationQuery)),annotationResult='],
 ['annotation-secondary-selector-cloned','const annotationRequest=annotationQueryProducerRequest(annotationQuery),annotationResult=','const annotationRequest=({...annotationQueryProducerRequest(annotationQuery),[annotationQuery.opaque.role]:structuredClone(annotationQuery.opaque.selector)}),annotationResult='],
 ['annotation-projection-grant-promoted','annotationPacket.value.current,annotationPacket.value.facts);','annotationPacket.value.current,{...annotationPacket.value.facts,projectionCurrent:true});'],
 ['annotation-recipient-scope-repaired','annotationRequest,annotationPacket.value.current','annotationRequest,{...annotationPacket.value.current,recipient:"reader"}'],
 ['annotation-coherent-peer-grant-promoted','annotationRequest,annotationPacket.value.current','annotationRequest,{...annotationPacket.value.current,...(annotationPacket.value.current.members?{members:annotationPacket.value.current.members.map(m=>({...m,sourceReadable:true}))}:{})}']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/annotation-query-gateway-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1200));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1200));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-decoded-F03-native-query-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allPublicProductsPhysicalHostOrIndependentProof:false}));
