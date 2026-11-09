// Selected result-validation calls only, after module/fixture setup.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import {checkDiscoveryResults} from './discovery-result-checks.mjs';
const raw=fs.readFileSync(new URL('./discovery-results.mjs',import.meta.url),'utf8'),marker='export function nativeDiscoveryResultValid(value,request,l){',prepared=[];
for(const [name,injection,call]of [['baseline','',''],['hidden-result-read',"process.getBuiltinModule('fs').readFileSync('not-read');",'fs.readFileSync'],['hidden-result-rename',"process.getBuiltinModule('fs').renameSync('not-renamed','not-renamed');",'fs.renameSync'],['hidden-result-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']]){
 assert.equal(raw.split(marker).length-1,1);const body=raw.replace(marker,marker+injection).replace(/new URL\('(\.\/[^']+)',import.meta.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,import.meta.url).href)+',import.meta.url)');prepared.push({name,call,api:await import('data:text/javascript;base64,'+Buffer.from(body).toString('base64'))});
}
const attempts=[];let calls=0,wrappedFunctions=0;
function guarded(fn,args){
 const held=[];try{
  for(const [label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){const descriptor=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;assert(descriptor.configurable||descriptor.writable);held.push({obj,key,descriptor});Object.defineProperty(obj,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});}
  wrappedFunctions=held.length;syncBuiltinESMExports();calls++;return fn(...args);
 }finally{for(const {obj,key,descriptor}of held.reverse())Object.defineProperty(obj,key,descriptor);syncBuiltinESMExports();for(const {obj,key,descriptor}of held)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),descriptor);}
}
let baseline;const mutations=[];
for(const p of prepared){const api={...p.api,nativeDiscoveryResultValid:(...args)=>guarded(p.api.nativeDiscoveryResultValid,args),discoveryResultMetadataValid:(...args)=>guarded(p.api.discoveryResultMetadataValid,args)};if(p.name==='baseline'){baseline=checkDiscoveryResults(api);assert.deepEqual(attempts,[]);}else{assert.throws(()=>checkDiscoveryResults(api),{code:'SUBSTRATE_PROBE_DENIED'});assert.deepEqual(attempts,[p.call]);attempts.length=0;mutations.push(p.name);}}
console.log(JSON.stringify({scope:'selected-F01-result-validation-substrate',baseline,wrappedFunctions,guardedCalls:calls,attemptedBaselineCalls:0,mutations,coherentBaselinePassed:true,nativeQueryOrImportIsolationOrHostContainmentProved:false}));
