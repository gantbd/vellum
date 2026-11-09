// Selected post-import execution, not complete host containment or package isolation.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {syncBuiltinESMExports} from 'node:module';
import * as api from './invocation-publication.mjs';
import {captureInputs} from './evidence-harness.mjs';
import {checkAnnotationQueryGateway,annotationQueryGatewayFixture as fixture} from './annotation-query-gateway-checks.mjs';
const mode=process.argv.find(a=>a.startsWith('--probe='))?.slice(8);
if(mode){
 const fixtures=[];if(mode!=='baseline')for(const kind of ['annotations','combined','version-set'])for(const retained of [false,true]){const z=fixture(kind,retained);fixtures.push({z,parent:api.invocationUsage(z.domain),native:z.usage(),root:z.root()});}
 const originals=[],attempts=[];let result;
 try{
  for(const [label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){const d=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;assert(d.configurable||d.writable);originals.push({obj,key,d});Object.defineProperty(obj,key,{configurable:true,enumerable:d.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);throw Error('selected substrate call denied');}});}
  syncBuiltinESMExports();
  if(mode==='baseline'){const b=checkAnnotationQueryGateway();assert.deepEqual(attempts,[]);result={controls:b.controls.length,nativeCases:b.nativeCases,boundaries:b.boundaries,denials:b.denials,attemptedBaselineCalls:0};}
  else{const expected={read:'fs.readFileSync',rename:'fs.renameSync',process:'child_process.spawnSync'}[mode];assert(expected);for(const {z,parent,native,root}of fixtures){const before=attempts.length;assert.equal(z.query().status,'unavailable');assert.deepEqual(attempts.slice(before),[expected]);assert.deepEqual(api.invocationUsage(z.domain),{...parent,faulted:true});assert.deepEqual(z.usage(),native);assert.equal(z.root(),root);assert.deepEqual(z.decodeUsage(),{busy:false,declaredWorkspaceBytes:0});}result={mode,attemptedCall:expected,cases:fixtures.length,workspaceRefunded:true,nativeOwnerUnchanged:true,ownerFenced:true};}
 }finally{for(const {obj,key,d}of originals.reverse())Object.defineProperty(obj,key,d);syncBuiltinESMExports();}
 for(const {obj,key,d}of originals)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),d);
 console.log(JSON.stringify({...result,wrappedFunctions:originals.length}));
}else{
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/invocation-publication.mjs',marker='  const annotationRequest=annotationQueryProducerRequest(annotationQuery),annotationResult=annotationPort(o,domain,handle,annotationRequest,annotationPacket.value.current,annotationPacket.value.facts);',rules=[['read',"process.getBuiltinModule('fs').readFileSync('not-accessed');"],['rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');"],['process',"process.getBuiltinModule('child_process').spawnSync('not-launched');"]],run=(directory,probe)=>cp.spawnSync(process.execPath,[path.join(directory,'formal/annotation-query-gateway-substrate-checks.mjs'),'--probe='+probe],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory,'baseline');assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const [name,inject]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(marker).length-1,1);fs.writeFileSync(file,original.replace(marker,inject+marker));const out=run(graph.directory,name);assert.equal(out.status,0,out.stderr);mutations.push(JSON.parse(out.stdout));}
 console.log(JSON.stringify({scope:'isolated-decoded-F03-query-selected-substrate',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphs:true,allHostAPIsImportIsolationPhysicalOrGeneralFaultContainmentProved:false}));
}
