import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeAnnotationInstallationFixture as fixture,checkNativeAnnotationInstallation,nativeAnnotationInstallationFacts as facts} from './native-annotation-installation-checks.mjs';
import {captureInputs} from './evidence-harness.mjs';
function faultCase(){const z=fixture(),source=z.x.usage(),registry=z.usage(),parent=api.invocationUsage(z.x.z.domain),root=z.x.z.current();
 assert.throws(()=>z.install(),{message:'ANNOTATION_INSTALL_STAGE_FAULT'});assert.equal(api.invocationUsage(z.x.z.domain).faulted,true);assert.equal(api.invocationUsage(z.x.z.domain).declaredHeldBytes,parent.declaredHeldBytes);assert.deepEqual(z.x.usage(),source);assert.deepEqual(z.usage(),registry);assert.equal(z.x.z.current(),root);assert.equal(z.x.prepare(),null);
 const cleanup={ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true};assert.equal(api.discardInvocationAnnotationCapture(z.x.z.domain,z.x.handle,z.cut,cleanup),true);assert.equal(api.retireInvocationAnnotationCapture(z.x.z.domain,z.x.handle,cleanup),true);assert.equal(api.retireInvocationAnnotationContexts(z.x.z.domain,z.handle,cleanup),true);assert.equal(api.invocationUsage(z.x.z.domain).declaredHeldBytes,z.x.before.declaredHeldBytes);assert.equal(z.x.z.current(),root);
 return {cutUnconsumed:true,noPartialRegistry:true,ownerFenced:true,protectedDischargeChecked:true};
}
if(process.argv.includes('--fault-profile'))console.log(JSON.stringify(faultCase()));
else{
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-contexts.mjs',rules=[
  ['before-registry-candidate','const candidateContexts=new Map(m.contexts),candidateSelectors=new Map(m.selectors);'],
  ['between-private-candidate-rows','candidateSelectors.set(context,record);'],
  ['after-staging-before-consumption','const consumed=consumeNativeAnnotationCapture(owner,domain,capturePool,cut,c,g);']
 ],baseline=checkNativeAnnotationInstallation(),cases=[];
 for(const [name,marker]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(marker).length-1,1,name);fs.writeFileSync(file,original.replace(marker,"throw Error('ANNOTATION_INSTALL_STAGE_FAULT');"+marker));const out=spawnSync(process.execPath,[path.join(graph.directory,'formal/native-annotation-installation-fault-checks.mjs'),'--fault-profile'],{encoding:'utf8',env:process.env,maxBuffer:1048576});assert.equal(out.status,0,name+': '+out.stderr);cases.push({name,...JSON.parse(out.stdout)});}
 console.log(JSON.stringify({scope:'isolated-native-annotation-installation-precommit-faults',baselineControls:baseline.controls.length,cases,coherentCopiedGraphs:true,postConsumptionCrashPhysicalOrGeneralFaultContainmentProved:false}));
}
