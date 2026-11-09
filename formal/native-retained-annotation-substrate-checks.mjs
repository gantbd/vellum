// Selected post-import staging probes, not import isolation or a host sandbox.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {syncBuiltinESMExports} from 'node:module';
import * as api from './invocation-publication.mjs';
import {captureInputs} from './evidence-harness.mjs';
import {nativeAnnotationQueryFixture as fixture} from './native-annotation-query-checks.mjs';
import {checkRetainedAnnotations,acquireRetainedAnnotation as acquire} from './native-retained-annotation-checks.mjs';
import {checkRetainedAnnotationBoundaries} from './native-retained-annotation-boundaries.mjs';
import {checkCombinedSearchLifecycle} from './native-combined-search-lifecycle-checks.mjs';
const mode=process.argv.find(a=>a.startsWith('--probe='))?.slice(8);
if(mode){
 const z=mode==='baseline'?null:fixture(),before=z?{context:z.usage(),parent:api.invocationUsage(z.x.z.domain),root:z.x.z.current()}:null,originals=[],attempts=[],unwrapped=[];let result;
 try{
  for(const [label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){
   const d=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;if(!d.configurable&&!d.writable){unwrapped.push(label+'.'+key);continue;}
   originals.push({obj,key,d});Object.defineProperty(obj,key,{configurable:true,enumerable:d.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('selected substrate call denied');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});
  }assert.deepEqual(unwrapped,[]);syncBuiltinESMExports();
  if(mode==='baseline'){
   const ownership=checkRetainedAnnotations(),boundaries=checkRetainedAnnotationBoundaries(),lifecycle=checkCombinedSearchLifecycle(true);assert.deepEqual(attempts,[]);result={controls:ownership.controls.length,searchCases:ownership.searchCases,boundaryCases:boundaries.cases,lifecycleCases:lifecycle.cases,attemptedBaselineCalls:0};
  }else{
   const expected={read:'fs.readFileSync',rename:'fs.renameSync',process:'child_process.spawnSync'}[mode];assert(expected);assert.throws(()=>acquire(z),{code:'SUBSTRATE_PROBE_DENIED'});assert.deepEqual(attempts,[expected]);assert.equal(api.invocationUsage(z.x.z.domain).faulted,true);assert.equal(api.invocationUsage(z.x.z.domain).declaredHeldBytes,before.parent.declaredHeldBytes);assert.deepEqual(z.usage(),before.context);assert.equal(z.x.z.current(),before.root);result={mode,attemptedCall:expected,noPartialRegistry:true,originalContextUnchanged:true,spentIssuanceUnchanged:true,ownerFenced:true};
  }
 }finally{for(const {obj,key,d}of originals.reverse())Object.defineProperty(obj,key,d);syncBuiltinESMExports();}
 for(const {obj,key,d}of originals)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),d);
 console.log(JSON.stringify({...result,wrappedFunctions:originals.length}));
}else{
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-contexts.mjs',marker='try{const contexts=new Map(m.contexts),selectors=new Map(m.selectors);',rules=[
  ['read',"process.getBuiltinModule('fs').readFileSync('not-accessed');"],
  ['rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');"],
  ['process',"process.getBuiltinModule('child_process').spawnSync('not-launched');"]
 ],run=(directory,mode)=>cp.spawnSync(process.execPath,[path.join(directory,'formal/native-retained-annotation-substrate-checks.mjs'),'--probe='+mode],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),p=run(positive.directory,'baseline');assert.equal(p.status,0,p.stderr);const mutations=[];
 for(const [mode,inject]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(marker).length-1,1,mode);fs.writeFileSync(file,original.replace(marker,'try{'+inject+marker.slice(4)));const out=run(graph.directory,mode);assert.equal(out.status,0,mode+': '+out.stderr);mutations.push(JSON.parse(out.stdout));}
 console.log(JSON.stringify({scope:'isolated-native-retained-annotation-selected-substrate',baseline:JSON.parse(p.stdout),mutations,coherentCopiedGraphs:true,allHostAPIsImportIsolationPhysicalOrGeneralFaultContainmentProved:false}));
}
