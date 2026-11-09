import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeVersionSetQueryFixture as fixture,checkNativeVersionSetQueries} from './native-version-set-query-checks.mjs';
import {nativeVersionSetCaptureFacts as facts} from './native-version-set-capture-checks.mjs';
import {captureInputs} from './evidence-harness.mjs';
function faultCase(kind){const x=fixture(),domain=x.z.x.z.domain,registry=x.z.usage(),capture=x.z.x.usage(),parent=api.invocationUsage(domain),root=x.z.x.z.root();
 assert.throws(()=>x.query(kind==='source'?x.source():x.annotation()),{message:'VERSION_SET_QUERY_STAGE_FAULT'});assert.equal(api.invocationUsage(domain).faulted,true);assert.equal(api.invocationUsage(domain).declaredHeldBytes,parent.declaredHeldBytes);assert.deepEqual(x.z.usage(),registry);assert.deepEqual(x.z.x.usage(),capture);assert.equal(x.z.x.z.root(),root);assert.equal(x.z.x.prepare(),null);
 const scope=Object.fromEntries(['recipient','purpose','destination','policy','continuity','profile'].map(k=>[k,x.z.x.host[k]]));assert.equal(api.stopInvocationVersionSetContext(domain,x.z.handle,{contract:'read-context/1',operation:'stop-observation',context:x.out.context},scope,facts).status,'stopped');assert.equal(api.retireInvocationVersionSetContext(domain,x.z.handle,x.out.context,facts),true);assert.equal(api.retireInvocationVersionSetContexts(domain,x.z.handle,facts),true);assert.equal(api.retireInvocationVersionSetCapture(domain,x.z.x.handle,facts),true);assert.equal(api.invocationUsage(domain).declaredHeldBytes,x.z.x.before);assert.equal(x.z.x.z.root(),root);
 return {kind,noPartialRegistryOrAliases:true,ownerFenced:true,protectedDischargeChecked:true};
}
const kind=process.argv.find(a=>a.startsWith('--fault='))?.slice(8);
if(kind){assert(['source','annotations'].includes(kind));console.log(JSON.stringify(faultCase(kind)));}
else{
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-version-set-contexts.mjs',rules=[
  ['before-pure-query','const out=queryVersionMember(held.registry,lowered,loweredHost(packet.host));'],
  ['before-native-selector-staging','const pending=[],bindings=new Map(held.bindings),staged=new Map();'],
  ['after-complete-reply-before-publication','Object.freeze(reply);const selectors=new Map(m.selectors);']
 ],baseline=checkNativeVersionSetQueries(),cases=[];
 for(const [name,marker]of rules)for(const kind of ['source','annotations']){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(marker).length-1,1,name);fs.writeFileSync(file,original.replace(marker,"throw Error('VERSION_SET_QUERY_STAGE_FAULT');"+marker));const out=spawnSync(process.execPath,[path.join(graph.directory,'formal/native-version-set-query-fault-checks.mjs'),'--fault='+kind],{encoding:'utf8',env:process.env,maxBuffer:1048576});assert.equal(out.status,0,name+': '+out.stderr);cases.push({name,...JSON.parse(out.stdout)});}
 console.log(JSON.stringify({scope:'isolated-native-version-set-query-precommit-faults',baselineControls:baseline.controls.length,cases,coherentCopiedGraphs:true,postPublicationCrashPhysicalOrGeneralFaultContainmentProved:false}));
}
