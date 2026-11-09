// Selected post-import calls, not an import sandbox or complete host boundary.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {syncBuiltinESMExports} from 'node:module';
import * as api from './invocation-publication.mjs';
import {captureInputs} from './evidence-harness.mjs';
import {checkSearchGateway} from './search-gateway-checks.mjs';
import {nativeSourceSearchFixture as fixture,nativeSourceSearchCurrent as current} from './native-source-search-checks.mjs';
import {nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
const mode=process.argv.find(a=>a.startsWith('--probe='))?.slice(8);
if(mode){
 const z=mode==='baseline'?null:fixture(),r=z?.request(),domain=z?.x.z.domain,before=z?{parent:api.invocationUsage(domain),source:z.x.usage(),search:z.usage(),root:z.x.z.root()}:null,originals=[],attempts=[];let result;
 try{
  for(const [label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){const d=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;assert(d.configurable||d.writable);originals.push({obj,key,d});Object.defineProperty(obj,key,{configurable:true,enumerable:d.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);throw Error('selected substrate call denied');}});}
  syncBuiltinESMExports();
  if(mode==='baseline'){const b=checkSearchGateway();assert.deepEqual(attempts,[]);result={controls:b.controls.length,boundaries:b.boundaries,forms:b.forms,denials:b.denials,attemptedBaselineCalls:0};}
  else{const envelope={route:'typed',request:{family:4,variant:r.operation,version:1,arguments:r}},limits={maxRequestBytes:4096,maxInputNodes:4096,maxInputDepth:128},out=api.queryInvocationDecodedSearch(domain,z.handle,envelope,limits,api.invocationAuthorityContext(domain),z.x.c,current(r),facts),expected={read:'fs.readFileSync',rename:'fs.renameSync',process:'child_process.spawnSync'}[mode];assert(expected);assert.equal(out.status,'unavailable');assert.deepEqual(attempts,[expected]);assert.equal(api.invocationUsage(domain).faulted,true);assert.equal(api.invocationUsage(domain).declaredHeldBytes,before.parent.declaredHeldBytes);assert.deepEqual(z.x.usage(),before.source);assert.deepEqual(z.usage(),before.search);assert.equal(z.x.z.root(),before.root);assert.deepEqual(api.invocationSearchDecodeUsage(domain),{busy:false,declaredWorkspaceBytes:0});result={mode,attemptedCall:expected,workspaceRefunded:true,noNativeIssuanceOrHold:true,ownerFenced:true};}
 }finally{for(const {obj,key,d}of originals.reverse())Object.defineProperty(obj,key,d);syncBuiltinESMExports();}
 for(const {obj,key,d}of originals)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),d);
 console.log(JSON.stringify({...result,wrappedFunctions:originals.length}));
}else{
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/invocation-publication.mjs',marker='  return queryNativeSourceSearch(o,domain,handle,searchProducerRequest(n),captured.sourceContext,captured.current,captured.facts);',rules=[['read',"process.getBuiltinModule('fs').readFileSync('not-accessed');"],['rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');"],['process',"process.getBuiltinModule('child_process').spawnSync('not-launched');"]],run=(directory,probe)=>cp.spawnSync(process.execPath,[path.join(directory,'formal/search-gateway-substrate-checks.mjs'),'--probe='+probe],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory,'baseline');assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const [name,inject]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(marker).length-1,1);fs.writeFileSync(file,original.replace(marker,inject+marker));const out=run(graph.directory,name);assert.equal(out.status,0,out.stderr);mutations.push(JSON.parse(out.stdout));}
 console.log(JSON.stringify({scope:'isolated-decoded-search-selected-substrate',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphs:true,allHostAPIsImportIsolationPhysicalOrGeneralFaultContainmentProved:false}));
}
