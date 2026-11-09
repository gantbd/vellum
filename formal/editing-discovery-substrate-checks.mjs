// Selected post-import calls only, not a sandbox, packaging or host proof.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import {checkEditingDiscovery} from './editing-discovery-checks.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
const base=new URL('./',import.meta.url),raw=fs.readFileSync(new URL('native-discovery.mjs',base),'utf8'),marker='export function queryNativeDiscovery(owner,request,c,packet,g){',prepared=[];
for(const [name,injection,call]of [['baseline','',''],['discovery-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-accessed');",'fs.readFileSync'],['discovery-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');",'fs.renameSync'],['discovery-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']]){assert.equal(raw.split(marker).length-1,1);const body=raw.replace(marker,marker+injection).replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,base).href+q);prepared.push({name,call,api:await import('data:text/javascript;base64,'+Buffer.from(body).toString('base64'))});}
// The shared generic fixture reads catalog examples while constructing an owner.
// Construct those test inputs before interception, not inside the measured path.
const pattern=[];checkEditingDiscovery(prepared[0].api,(api,pack,options)=>{pattern.push(structuredClone(options));return nativeDiscoveryFixture(api,pack,options);});assert.equal(pattern.length,21);
for(const entry of prepared)entry.fixtures=pattern.map(options=>({options,fixture:nativeDiscoveryFixture(entry.api,undefined,options)}));
const execute=entry=>{const queue=[...entry.fixtures];return checkEditingDiscovery(entry.api,(api,pack,options)=>{const next=queue.shift();assert(next);assert.deepEqual(options,next.options);return next.fixture;});};
const held=[],attempts=[],unwrapped=[];let baseline,kills=[];
try{for(const[label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){const descriptor=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;if(!descriptor.configurable&&!descriptor.writable){unwrapped.push(label+'.'+key);continue;}held.push({obj,key,descriptor});Object.defineProperty(obj,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});}assert.deepEqual(unwrapped,[]);syncBuiltinESMExports();for(const entry of prepared){const {name,call}=entry;if(name==='baseline'){baseline=execute(entry);assert.deepEqual(attempts,[]);continue;}assert.throws(()=>execute(entry),{code:'SUBSTRATE_PROBE_DENIED'});assert.deepEqual(attempts,[call]);kills.push(name);attempts.length=0;}}
finally{for(const{obj,key,descriptor}of held.reverse())Object.defineProperty(obj,key,descriptor);syncBuiltinESMExports();}
for(const{obj,key,descriptor}of held)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),descriptor);
console.log(JSON.stringify({scope:'isolated-editing-discovery-substrate-calls',wrappedFunctions:held.length,controls:baseline.controls.length,attemptedHostCalls:attempts.length,mutations:kills,coherentBaselinePassed:true,importIndependenceOrAllRoutesOrHostTruthProved:false}));
