// Selected post-import probes only; not import isolation or a host sandbox.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {syncBuiltinESMExports} from 'node:module';
import {captureInputs} from './evidence-harness.mjs';
import {checkRetainedSource} from './native-retained-source-checks.mjs';
import {checkRetainedSourceBoundaries} from './native-retained-source-boundaries.mjs';
import {checkRetainedSourceLifecycle} from './native-retained-source-lifecycle-checks.mjs';
import {nativeSourceContextFixture as fixture} from './native-source-context-checks.mjs';
const mode=process.argv.find(a=>a.startsWith('--probe='))?.slice(8);
if(mode){
 const x=mode==='baseline'?null:fixture(),reference=x?.acquire().context,before=x?{context:x.usage(),parent:x.z.usage(),root:x.z.root()}:null,originals=[],attempts=[],unwrapped=[];if(x)assert(reference);let result;
 try{
  for(const [label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){
   const d=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;if(!d.configurable&&!d.writable){unwrapped.push(label+'.'+key);continue;}
   originals.push({obj,key,d});Object.defineProperty(obj,key,{configurable:true,enumerable:d.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('selected substrate call denied');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});
  }assert.deepEqual(unwrapped,[]);syncBuiltinESMExports();
  if(mode==='baseline'){
   const ownership=checkRetainedSource(),boundaries=checkRetainedSourceBoundaries(),lifecycle=checkRetainedSourceLifecycle();assert.deepEqual(attempts,[]);result={controls:ownership.controls.length,searchCases:ownership.searchCases,boundaryCases:boundaries.cases,lifecycleCases:lifecycle.cases,attemptedBaselineCalls:0};
  }else{
   const expected={read:'fs.readFileSync',rename:'fs.renameSync',process:'child_process.spawnSync'}[mode];assert(expected);assert.throws(()=>x.acquire({...x.request,selection:{kind:'retained',reference}}),{code:'SUBSTRATE_PROBE_DENIED'});assert.deepEqual(attempts,[expected]);assert.deepEqual(x.usage(),before.context);assert.deepEqual(x.z.usage(),before.parent);assert.equal(x.z.root(),before.root);result={mode,attemptedCall:expected,noPartialInstallation:true,originalContextUnchanged:true};
  }
 }finally{for(const {obj,key,d}of originals.reverse())Object.defineProperty(obj,key,d);syncBuiltinESMExports();}
 for(const {obj,key,d}of originals)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),d);
 console.log(JSON.stringify({...result,wrappedFunctions:originals.length}));
}else{
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-source-contexts.mjs',marker="const context=token('native-source-read-context/1')",rules=[
  ['read',"process.getBuiltinModule('fs').readFileSync('not-accessed');"],
  ['rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');"],
  ['process',"process.getBuiltinModule('child_process').spawnSync('not-launched');"]
 ],run=(directory,mode)=>cp.spawnSync(process.execPath,[path.join(directory,'formal/native-retained-source-substrate-checks.mjs'),'--probe='+mode],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),p=run(positive.directory,'baseline');assert.equal(p.status,0,p.stderr);const mutations=[];
 for(const [mode,inject]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(marker).length-1,1,mode);fs.writeFileSync(file,original.replace(marker,'if(retained){'+inject+'}'+marker));const out=run(graph.directory,mode);assert.equal(out.status,0,mode+': '+out.stderr);mutations.push(JSON.parse(out.stdout));}
 console.log(JSON.stringify({scope:'isolated-native-retained-source-selected-substrate',baseline:JSON.parse(p.stdout),mutations,coherentCopiedGraphs:true,allHostAPIsImportIsolationPhysicalOrGeneralFaultContainmentProved:false}));
}
