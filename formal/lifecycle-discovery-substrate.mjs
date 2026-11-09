// Selected execution after imports and preconstruction, not general host isolation.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import {checkLifecycleDiscovery} from './lifecycle-discovery-checks.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {lifecycleMetadataFixture} from './lifecycle-discovery-product-checks.mjs';
const base=new URL('./',import.meta.url),raw=fs.readFileSync(new URL('native-discovery.mjs',base),'utf8'),marker='export function queryNativeDiscovery(owner,request,c,packet,g){',prepared=[];
for(const[name,injection,call]of [['baseline','',''],['lifecycle-discovery-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-read');",'fs.readFileSync'],['lifecycle-discovery-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-renamed','not-renamed');",'fs.renameSync'],['lifecycle-discovery-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']]){assert.equal(raw.split(marker).length-1,1);const body=raw.replace(marker,marker+injection).replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,base).href+q);prepared.push({name,call,api:await import('data:text/javascript;base64,'+Buffer.from(body).toString('base64'))});}
const pattern=[];let observerCount=0;checkLifecycleDiscovery(prepared[0].api,(api,pack,options)=>{pattern.push(structuredClone(options));return nativeDiscoveryFixture(api,pack,options);},()=>{observerCount++;return lifecycleMetadataFixture();});assert.equal(observerCount,2);
for(const entry of prepared){entry.fixtures=pattern.map(options=>({options,fixture:nativeDiscoveryFixture(entry.api,undefined,options)}));entry.observers=Array.from({length:observerCount},()=>lifecycleMetadataFixture());}
const execute=entry=>{const queue=[...entry.fixtures],observers=[...entry.observers],out=checkLifecycleDiscovery(entry.api,(api,pack,options)=>{const next=queue.shift();assert(next);assert.deepEqual(options,next.options);return next.fixture;},()=>{const next=observers.shift();assert(next);return next;});assert.equal(queue.length,0);assert.equal(observers.length,0);return out;};
const held=[],attempts=[],mutations=[];let baseline;
try{for(const[label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){const descriptor=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;assert(descriptor.configurable||descriptor.writable);held.push({obj,key,descriptor});Object.defineProperty(obj,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});}syncBuiltinESMExports();
 for(const p of prepared){if(p.name==='baseline'){baseline=execute(p);assert.deepEqual(attempts,[]);}else{assert.throws(()=>execute(p),{code:'SUBSTRATE_PROBE_DENIED'});assert.deepEqual(attempts,[p.call]);mutations.push(p.name);attempts.length=0;}}
}finally{for(const{obj,key,descriptor}of held.reverse())Object.defineProperty(obj,key,descriptor);syncBuiltinESMExports();}
for(const{obj,key,descriptor}of held)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),descriptor);
console.log(JSON.stringify({scope:'selected-owned-F17-lifecycle-discovery-substrate',wrappedFunctions:held.length,controls:baseline.controls.length,preconstructedFixtures:pattern.length,preconstructedLifecycleFixtures:observerCount,attemptedBaselineCalls:0,mutations,coherentBaselinePassed:true,importIsolationAllRoutesOrHostContainmentProved:false}));
