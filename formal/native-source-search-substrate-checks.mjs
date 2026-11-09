// Selected post-import probes only, not an import-time or all-host sandbox.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import {checkNativeSourceSearch,nativeSourceSearchFixture as fixture} from './native-source-search-checks.mjs';
import {checkNativeSourceSearchBoundaries} from './native-source-search-boundaries.mjs';
const base=new URL('./',import.meta.url),raw=fs.readFileSync(new URL('native-source-search.mjs',base),'utf8'),kernel=fs.readFileSync(new URL('invocation-publication.mjs',base),'utf8');
function url(s,overrides={}){return 'data:text/javascript;base64,'+Buffer.from(s.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,base).href)+',import.meta.url)')).toString('base64');}
const marker='m.issued++;const result=searchLiteral',prepared=[];assert.equal(raw.split(marker).length-1,1);
for(const [name,inject,call]of [['baseline','',''],['search-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-accessed');",'fs.readFileSync'],['search-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');",'fs.renameSync'],['search-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']]){
 const api=await import(url(kernel,{'./native-source-search.mjs':url(raw.replace(marker,inject+marker))}));prepared.push({name,call,api,...(name==='baseline'?{}:{fixture:fixture(api)})});
}
const originals=[],attempts=[],killed=[],unwrapped=[];let baseline,boundaries;
try{
 for(const [label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){
  const d=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;if(!d.configurable&&!d.writable){unwrapped.push(label+'.'+key);continue;}
  originals.push({obj,key,d});Object.defineProperty(obj,key,{configurable:true,enumerable:d.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied selected substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});
 }assert.deepEqual(unwrapped,[]);syncBuiltinESMExports();
 for(const entry of prepared){
  if(entry.name==='baseline'){baseline=checkNativeSourceSearch(entry.api);boundaries=checkNativeSourceSearchBoundaries(entry.api);assert.deepEqual(attempts,[]);continue;}
  const z=entry.fixture,source=z.x.usage(),search=z.usage(),parent=z.x.z.usage(),root=z.x.z.root();assert.throws(()=>z.query(),{code:'SUBSTRATE_PROBE_DENIED'});assert.deepEqual(attempts,[entry.call]);assert.equal(z.x.z.root(),root);assert.equal(z.x.z.usage().faulted,true);assert.equal(z.x.z.usage().declaredHeldBytes,parent.declaredHeldBytes);assert.deepEqual(z.usage(),search);assert.equal(z.x.usage().holds,0);assert.equal(z.x.usage().issued,source.issued+1);assert.equal(z.x.usage().retainedRoots,source.retainedRoots);assert.equal(z.x.usage().declaredHeldBytes,source.declaredHeldBytes);attempts.length=0;killed.push(entry.name);
 }
}finally{for(const {obj,key,d}of originals.reverse())Object.defineProperty(obj,key,d);syncBuiltinESMExports();}
for(const {obj,key,d}of originals)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),d);
console.log(JSON.stringify({scope:'isolated-native-search-selected-substrate',controls:baseline.controls.length,boundaryCases:boundaries.cases,wrappedFunctions:originals.length,attemptedBaselineCalls:0,mutations:killed,selectedFaultFenceAndUnpublishedHoldCleanupCases:3,allHostAPIsImportIsolationPhysicalOrGeneralFaultContainmentProved:false}));
