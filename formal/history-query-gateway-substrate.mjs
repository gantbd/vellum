// Selected post-import execution, including verification fixture operations.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
const base=new URL('./',import.meta.url),native=fs.readFileSync(new URL('history-contexts.mjs',base),'utf8'),checks=fs.readFileSync(new URL('history-query-gateway-checks.mjs',base),'utf8'),marker='export function queryDecodedHistoryInspection(owner,envelope,inputLimits,c,g){';
assert.equal(native.split(marker).length-1,1);
const lower=raw=>raw.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>`new URL(${JSON.stringify(new URL(p,base).href)},import.meta.url)`),url=raw=>'data:text/javascript;base64,'+Buffer.from(raw).toString('base64'),prepared=[];
for(const [name,injection,call]of [['baseline','',''],['history-decoder-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-read');",'fs.readFileSync'],['history-decoder-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-renamed','not-renamed');",'fs.renameSync'],['history-decoder-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']]){const api=url(lower(native.replace(marker,marker+injection))),body=checks.replace("'./history-contexts.mjs'",JSON.stringify(api));prepared.push({name,call,check:(await import(url(lower(body)))).checkHistoryQueryGateway});}
const held=[],attempts=[],mutations=[];let baseline;
try{for(const[label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){const descriptor=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;assert(descriptor.configurable||descriptor.writable);held.push({obj,key,descriptor});Object.defineProperty(obj,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});}syncBuiltinESMExports();
 for(const p of prepared){if(p.name==='baseline'){baseline=p.check();assert.deepEqual(attempts,[]);}else{assert.throws(()=>p.check(),{code:'SUBSTRATE_PROBE_DENIED'});assert.deepEqual(attempts,[p.call]);mutations.push(p.name);attempts.length=0;}}
}finally{for(const{obj,key,descriptor}of held.reverse())Object.defineProperty(obj,key,descriptor);syncBuiltinESMExports();}
for(const{obj,key,descriptor}of held)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),descriptor);
console.log(JSON.stringify({scope:'selected-native-F04-decoder-substrate',wrappedFunctions:held.length,controls:baseline.controls.length,nativeCases:baseline.nativeCases,denials:baseline.denials,boundaries:baseline.boundaries,attemptedBaselineCalls:0,mutations,coherentBaselinePassed:true,fixtureOperationsIncluded:true,importIsolationAllRoutesOrHostContainmentProved:false}));
