// Post-import selected-call probe, not import isolation or a host sandbox.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {syncBuiltinESMExports} from 'node:module';
import * as analysis from './analysis-work.mjs';
import * as kernel from './invocation-publication.mjs';
import {analysisQueryDiscoveryFixture} from './analysis-query-discovery-checks.mjs';
import {analysisWorkFixture} from './analysis-work-checks.mjs';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const mode=process.argv.find(a=>a.startsWith('--probe='))?.slice(8),copy=structuredClone;
if(mode){
 const z=analysisQueryDiscoveryFixture(),seed=analysisWorkFixture(),before={root:z.x.root(),owner:copy(analysis.analysisUsage(z.x.owner)),parent:copy(kernel.invocationUsage(z.x.z.domain))},saved=[],attempts=[],unwrapped=[];let result;
 try{
  for(const [label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){
   const d=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;if(!d.configurable&&!d.writable){unwrapped.push(label+'.'+key);continue;}
   saved.push({obj,key,d});Object.defineProperty(obj,key,{configurable:true,enumerable:d.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const error=Error('selected substrate call denied');error.code='SUBSTRATE_PROBE_DENIED';throw error;}});
  }assert.deepEqual(unwrapped,[]);syncBuiltinESMExports();
  if(mode==='baseline'){
   let queries=0;for(const operation of ['supported-analysis','supplied-evidence-inspection'])for(const route of ['typed','dynamic']){assert.equal(z.run(z.request(operation),route).status,operation==='supported-analysis'?'captures':'analysis-summary');queries++;}
   const context=z.open();assert.equal(context.status,'context');const page=z.query(context.context,{operation:'list',limit:1,continuation:null});assert.equal(page.entries.length,1);const descriptor=z.query(context.context,{operation:'inspect',entry:page.entries[0].reference});assert.equal(descriptor.status,'descriptor');assert.equal(descriptor.descriptor.variants.length,2);
   assert.deepEqual(attempts,[]);assert.deepEqual(analysis.analysisUsage(z.x.owner),before.owner);result={queries,discoveryVariants:2,attemptedBaselineCalls:0};
  }else{
   const expected={read:'fs.readFileSync',rename:'fs.renameSync',process:'child_process.spawnSync'}[mode];assert(expected);assert.deepEqual(z.run(),{status:'unavailable'});assert.deepEqual(attempts,[expected]);assert.deepEqual(analysis.analysisUsage(z.x.owner),{...before.owner,queryFaulted:true});result={mode,attemptedCall:expected,temporaryRefundChecked:true,retainedTaskPreserved:true,analysisOwnerFenced:true};
  }
  assert.equal(z.x.root(),before.root);assert.deepEqual(kernel.invocationUsage(z.x.z.domain),before.parent);
 }finally{for(const {obj,key,d}of saved.reverse())Object.defineProperty(obj,key,d);syncBuiltinESMExports();}
 for(const {obj,key,d}of saved)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),d);
 if(mode!=='baseline'){
  assert.equal(analysis.controlAnalysisWork(z.x.owner,z.a.task,'cancel',{controlAllowed:true}).status,'stopped');assert.equal(analysis.observeAnalysisWork(z.x.owner,z.x.event(z.a.task,'quiescence'),seed.ingress).status,'quiescent');assert.equal(analysis.retireAnalysisWork(z.x.owner,z.a.task,seed.retirement).status,'retired');result.protectedLateDischargeChecked=true;
 }
 console.log(JSON.stringify({...result,wrappedFunctions:saved.length}));
}else{
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),scope=createCaptureScope(),previous=process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE,transient=process.env.VELLUM_TRANSIENT_PROOF_CAPTURES;let passed=false;
 try{
  process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';
  const run=(directory,mode)=>cp.spawnSync(process.execPath,[path.join(directory,'formal/analysis-query-substrate-checks.mjs'),'--probe='+mode],{env:process.env,encoding:'utf8',timeout:60000,maxBuffer:2097152});
  const positive=captureInputs(root),out=run(positive.directory,'baseline');assert.equal(out.status,0,out.stderr);const baseline=JSON.parse(out.stdout),mutations=[];
  for(const [mode,inject]of [['read',"process.getBuiltinModule('fs').readFileSync('not-accessed');"],['rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');"],['process',"process.getBuiltinModule('child_process').spawnSync('not-launched');"]]){
   const graph=captureInputs(root);assert.deepEqual(graph.hashes,positive.hashes);const file=path.join(graph.directory,'formal/analysis-work.mjs'),raw=fs.readFileSync(file,'utf8'),marker='const normalized=normalizeAnalysisQueryArguments(envelope,c,limits,o.limits.maxQueryBytes);';assert.equal(raw.split(marker).length-1,1,'probe construction');fs.writeFileSync(file,raw.replace(marker,inject+marker));
   const result=run(graph.directory,mode);assert(!result.error&&!result.signal,'probe tool failure');assert.equal(result.status,0,result.stderr);mutations.push(JSON.parse(result.stdout));
  }
  passed=true;console.log(JSON.stringify({scope:'selected-actual-F08-query-and-discovery-substrate',baseline,mutations,coherentCopiedGraphs:true,importIsolationAllHostAPIsPhysicalOrIndependentProof:false}));
 }finally{if(previous===undefined)delete process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE;else process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=previous;if(transient===undefined)delete process.env.VELLUM_TRANSIENT_PROOF_CAPTURES;else process.env.VELLUM_TRANSIENT_PROOF_CAPTURES=transient;releaseCaptureScope(scope,passed);}
}
