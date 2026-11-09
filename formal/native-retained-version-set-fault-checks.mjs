import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeRetainedVersionSetFixture as fixture,checkNativeRetainedVersionSets} from './native-retained-version-set-checks.mjs';
import {nativeVersionSetCaptureFacts as facts} from './native-version-set-capture-checks.mjs';
import {captureInputs} from './evidence-harness.mjs';
function faultCase(){const x=fixture(),domain=x.z.x.z.domain,registry=x.z.usage(),capture=x.z.x.usage(),parent=api.invocationUsage(domain),root=x.z.x.z.root();
 assert.throws(()=>x.acquire(),{message:'RETAINED_SET_STAGE_FAULT'});assert.equal(api.invocationUsage(domain).faulted,true);assert.equal(api.invocationUsage(domain).declaredHeldBytes,parent.declaredHeldBytes);assert.deepEqual(x.z.usage(),registry);assert.deepEqual(x.z.x.usage(),capture);assert.equal(x.z.x.z.root(),root);assert.equal(x.z.x.prepare(),null);
 assert.equal(x.stop(x.out.context).status,'stopped');assert.equal(api.retireInvocationVersionSetContext(domain,x.z.handle,x.out.context,facts),true);assert.equal(api.retireInvocationVersionSetContexts(domain,x.z.handle,facts),true);assert.equal(api.retireInvocationVersionSetCapture(domain,x.z.x.handle,facts),true);assert.equal(api.invocationUsage(domain).declaredHeldBytes,x.z.x.before);assert.equal(x.z.x.z.root(),root);
 return {originalRegistryUnchanged:true,noPartialAliasesOrIssuance:true,ownerFenced:true,protectedDischargeChecked:true};
}
if(process.argv.includes('--fault-profile'))console.log(JSON.stringify(faultCase()));
else{
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-version-set-contexts.mjs',rules=[
  ['before-retained-registry-candidate','const retainedParents=new Map(m.parents),retainedSelectors=new Map(m.selectors),held='],
  ['between-retained-candidate-rows','retainedParents.set(context,held);'],
  ['after-retained-staging-before-publication','m.parents=retainedParents;m.selectors=retainedSelectors;']
 ],baseline=checkNativeRetainedVersionSets(),cases=[];
 for(const [name,marker]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(marker).length-1,1,name);fs.writeFileSync(file,original.replace(marker,"throw Error('RETAINED_SET_STAGE_FAULT');"+marker));const out=spawnSync(process.execPath,[path.join(graph.directory,'formal/native-retained-version-set-fault-checks.mjs'),'--fault-profile'],{encoding:'utf8',env:process.env,maxBuffer:1048576});assert.equal(out.status,0,name+': '+out.stderr);cases.push({name,...JSON.parse(out.stdout)});}
 console.log(JSON.stringify({scope:'isolated-native-retained-version-set-precommit-faults',baselineControls:baseline.controls.length,cases,coherentCopiedGraphs:true,postPublicationCrashPhysicalOrGeneralFaultContainmentProved:false}));
}
