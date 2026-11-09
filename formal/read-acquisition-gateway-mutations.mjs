import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),kernel='formal/invocation-publication.mjs',rules=[
 ['acquisition-authority-domain-ignored',kernel,'!normalized||!current(o,normalized.record.context)','!normalized'],
 ['acquisition-workspace-overaccepted',kernel,'acquisitionWorkspace>o.limits.maxHeldBytes-o.charge','acquisitionWorkspace>o.limits.maxHeldBytes-o.charge+1'],
 ['acquisition-workspace-not-refunded',kernel,'o.charge-=acquisitionCharge;o.readAcquisitionDecodeCharge=0;','o.readAcquisitionDecodeCharge=0;'],
 ['acquisition-workspace-overrefunded',kernel,'o.charge-=acquisitionCharge;o.readAcquisitionDecodeCharge=0;','o.charge-=acquisitionCharge+1;o.readAcquisitionDecodeCharge=0;'],
 ['acquisition-workspace-observation-retained',kernel,'o.readAcquisitionDecodeCharge=0;o.readAcquisitionDecoding=false;','o.readAcquisitionDecoding=false;'],
 ['acquisition-busy-observation-retained',kernel,'o.readAcquisitionDecodeCharge=0;o.readAcquisitionDecoding=false;','o.readAcquisitionDecodeCharge=0;'],
 ['acquisition-loss-masked-by-transient-refusal',kernel,'if(!currentPacket||!prepared)return unavailable();','if(!currentPacket||!prepared||currentPacket.value.facts.resourcesReady!==true)return unavailable();'],
 ['annotation-cut-binding-omitted',kernel,"prepared.kind!=='prepared'||!nativeAnnotationCutMatchesRequest(o,domain,prepared.pool,prepared.cut,request)","prepared.kind!=='prepared'"],
 ['coherent-cut-binding-omitted',kernel,"prepared.kind!=='prepared'||!nativeVersionSetCutMatchesRequest(o,domain,prepared.pool,prepared.cut,request)","prepared.kind!=='prepared'"],
 ['source-preparation-ignored',kernel,"request.contract!=='read-context/1'||prepared.kind!=='none'","request.contract!=='read-context/1'"],
 ['annotation-retained-preparation-ignored',kernel,"if(annotation){if(request.contract!=='read-context/1')return unavailable();if(retained){if(prepared.kind!=='none')return unavailable();","if(annotation){if(request.contract!=='read-context/1')return unavailable();if(retained){"],
 ['coherent-retained-preparation-ignored',kernel,"if(request.contract!=='read-version-set/1')return unavailable();if(retained){if(prepared.kind!=='none')return unavailable();","if(request.contract!=='read-version-set/1')return unavailable();if(retained){"],
 ['retained-original-request-cloned',kernel,"request=readAcquisitionProducerRequest(normalized),retained=","request=structuredClone(readAcquisitionProducerRequest(normalized)),retained="],
 ['annotation-cut-document-ignored','formal/native-annotation-capture.mjs','&&equal(r.document,held.value.document)&&r.profile','&&r.profile'],
 ['annotation-cut-profile-ignored','formal/native-annotation-capture.mjs','&&r.profile===held.scope.profile&&equal(r.data','&&equal(r.data'],
 ['annotation-cut-class-ignored','formal/native-annotation-capture.mjs','&&equal(r.data,held.value.data);',';'],
 ['coherent-cut-profile-ignored','formal/native-version-set-capture.mjs','&&r.profile===held.scope.profile&&exact(r.selection','&&exact(r.selection'],
 ['coherent-cut-member-count-ignored','formal/native-version-set-capture.mjs','&&r.participants.length===held.value.members.length&&r.participants.every','&&r.participants.every'],
 ['coherent-cut-member-document-ignored','formal/native-version-set-capture.mjs',"&&equal(p.document,held.value.members[i].document)&&equal(p.data","&&equal(p.data"],
 ['coherent-cut-member-class-ignored','formal/native-version-set-capture.mjs','&&equal(p.data,held.value.members[i].candidate.data));',');']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/read-acquisition-gateway-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const baselineGraph=captureInputs(root),baseline=run(baselineGraph.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,relative,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name+': exact construction target');fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1200));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1200));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-decoded-F02-native-acquisition-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allPublicProductsPhysicalHostOrIndependentProof:false}));
