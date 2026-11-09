import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeVersionSetInstallationFixture as fixture,checkNativeVersionSetInstallation} from './native-version-set-installation-checks.mjs';
import {captureInputs} from './evidence-harness.mjs';
function faultCase(){const z=fixture(),source=z.x.usage(),registry=z.usage(),parent=api.invocationUsage(z.x.z.domain),root=z.x.z.root();
 assert.throws(()=>z.install(),{message:'VERSION_SET_INSTALL_STAGE_FAULT'});assert.equal(api.invocationUsage(z.x.z.domain).faulted,true);assert.equal(api.invocationUsage(z.x.z.domain).declaredHeldBytes,parent.declaredHeldBytes);assert.deepEqual(z.x.usage(),source);assert.deepEqual(z.usage(),registry);assert.equal(z.x.z.root(),root);assert.equal(z.x.prepare(),null);
 const cleanup={ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true};assert.equal(api.discardInvocationVersionSetCapture(z.x.z.domain,z.x.handle,z.cut,cleanup),true);assert.equal(api.retireInvocationVersionSetCapture(z.x.z.domain,z.x.handle,cleanup),true);assert.equal(api.retireInvocationVersionSetContexts(z.x.z.domain,z.handle,cleanup),true);assert.equal(api.invocationUsage(z.x.z.domain).declaredHeldBytes,z.x.before);assert.equal(z.x.z.root(),root);
 return {cutUnconsumed:true,noPartialRegistry:true,ownerFenced:true,protectedDischargeChecked:true};
}
if(process.argv.includes('--fault-profile'))console.log(JSON.stringify(faultCase()));
else{
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-version-set-contexts.mjs',rules=[
  ['before-registry-candidate','const parents=new Map(m.parents),selectors=new Map(m.selectors),held='],
  ['between-private-candidate-rows','parents.set(context,held);'],
  ['after-staging-before-consumption','if(!consumeNativeVersionSetCapture(owner,domain,capturePool,cut,packet.host,packet.facts))return no();']
 ],baseline=checkNativeVersionSetInstallation(),cases=[];
 for(const [name,marker]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(marker).length-1,1,name);fs.writeFileSync(file,original.replace(marker,"throw Error('VERSION_SET_INSTALL_STAGE_FAULT');"+marker));const out=spawnSync(process.execPath,[path.join(graph.directory,'formal/native-version-set-installation-fault-checks.mjs'),'--fault-profile'],{encoding:'utf8',env:process.env,maxBuffer:1048576});assert.equal(out.status,0,name+': '+out.stderr);cases.push({name,...JSON.parse(out.stdout)});}
 console.log(JSON.stringify({scope:'isolated-native-version-set-installation-precommit-faults',baselineControls:baseline.controls.length,cases,coherentCopiedGraphs:true,postConsumptionCrashPhysicalOrGeneralFaultContainmentProved:false}));
}
