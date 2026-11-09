// Selected post-import calls only; no packaging or arbitrary-code isolation claim.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import {checkOrdinaryGateway,checkOrdinaryGatewayFault} from './ordinary-gateway-checks.mjs';
const base=new URL('./',import.meta.url),raw=fs.readFileSync(new URL('invocation-publication.mjs',base),'utf8'),marker='  const native=captureEditingIntent(normalized.kind,plan,',prepared=[];
for(const [name,injection,call] of [['baseline','',''],['gateway-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-accessed');",'fs.readFileSync'],['gateway-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-accessed','not-accessed');",'fs.renameSync'],['gateway-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']]){assert.equal(raw.split(marker).length-1,1);const body=raw.replace(marker,injection+marker).replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,base).href+q);prepared.push({name,call,api:await import('data:text/javascript;base64,'+Buffer.from(body).toString('base64'))});}
const held=[],attempts=[],unwrapped=[];let baseline;const kills=[];
try{for(const [label,obj] of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){const descriptor=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;if(!descriptor.configurable&&!descriptor.writable){unwrapped.push(label+'.'+key);continue;}held.push({obj,key,descriptor});Object.defineProperty(obj,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});}assert.deepEqual(unwrapped,[]);syncBuiltinESMExports();for(const entry of prepared){if(entry.name==='baseline'){baseline=checkOrdinaryGateway(entry.api);assert.deepEqual(attempts,[]);}else{checkOrdinaryGatewayFault(entry.api);assert.deepEqual(attempts,[entry.call]);kills.push(entry.name);attempts.length=0;}}}
finally{for(const {obj,key,descriptor} of held.reverse())Object.defineProperty(obj,key,descriptor);syncBuiltinESMExports();}
for(const {obj,key,descriptor} of held)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),descriptor);
console.log(JSON.stringify({scope:'isolated-decoded-ordinary-gateway-substrate',wrappedFunctions:held.length,controls:baseline.controls.length,completeCaptureBoundaries:baseline.completeCaptureBoundaries,attemptedBaselineHostCalls:0,mutations:kills,coherentBaselinePassed:true,importIndependenceOrAllHostAPIsOrProviderTruthProved:false}));
