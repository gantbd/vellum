// Selected post-import calls only; not an all-host or import-time sandbox.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import {checkNativeSourceContexts,nativeSourceContextFixture} from './native-source-context-checks.mjs';
import {checkSourceContextBoundaries} from './native-source-context-boundaries.mjs';
const base=new URL('./',import.meta.url),raw=fs.readFileSync(new URL('native-source-contexts.mjs',base),'utf8'),kernel=fs.readFileSync(new URL('invocation-publication.mjs',base),'utf8');
function url(s,overrides={}){return 'data:text/javascript;base64,'+Buffer.from(s.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,base).href)+',import.meta.url)')).toString('base64');}
const marker="const context=token('native-source-read-context/1')",prepared=[];
assert.equal(raw.split(marker).length-1,1);
for(const [name,inject,call]of [['baseline','',''],['source-context-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-accessed');",'fs.readFileSync'],['source-context-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');",'fs.renameSync'],['source-context-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']]){
 const api=await import(url(kernel,{'./native-source-contexts.mjs':url(raw.replace(marker,inject+marker))}));
 prepared.push({name,call,api,...(name==='baseline'?{}:{fixture:nativeSourceContextFixture(api)})});
}
const originals=[],unwrapped=[],attempts=[],kills=[];let baseline,boundaries;
try{
 for(const [label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){
  const descriptor=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;if(!descriptor.configurable&&!descriptor.writable){unwrapped.push(label+'.'+key);continue;}
  originals.push({obj,key,descriptor});Object.defineProperty(obj,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied selected substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});
 }
 assert.deepEqual(unwrapped,[]);syncBuiltinESMExports();
 for(const entry of prepared){
  if(entry.name==='baseline'){baseline=checkNativeSourceContexts(entry.api);boundaries=checkSourceContextBoundaries(entry.api);assert.deepEqual(attempts,[]);continue;}
  const x=entry.fixture,before=x.usage(),parent=x.z.usage(),root=x.z.root();assert.throws(()=>x.acquire(),{code:'SUBSTRATE_PROBE_DENIED'});assert.deepEqual(attempts,[entry.call]);assert.deepEqual(x.usage(),before);assert.deepEqual(x.z.usage(),parent);assert.equal(x.z.root(),root);attempts.length=0;kills.push(entry.name);
 }
}finally{for(const {obj,key,descriptor}of originals.reverse())Object.defineProperty(obj,key,descriptor);syncBuiltinESMExports();}
for(const {obj,key,descriptor}of originals)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),descriptor);
console.log(JSON.stringify({scope:'isolated-native-source-context-selected-substrate',controls:baseline.controls.length,boundaryCases:boundaries.cases,wrappedFunctions:originals.length,attemptedBaselineCalls:0,mutations:kills,preinstallationFrameCases:3,coherentNativeBaseline:true,allHostAPIsImportIsolationOrFaultContainmentProved:false}));
