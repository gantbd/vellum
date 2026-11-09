// Validation only. Imports and native production precede interception.
import assert from 'node:assert/strict';
import fs from 'node:fs';import fsp from 'node:fs/promises';import cp from 'node:child_process';import {syncBuiltinESMExports} from 'node:module';
import {registrationResultValueCorpus} from './registration-result-value-checks.mjs';
const raw=fs.readFileSync(new URL('./registration-result-values.mjs',import.meta.url),'utf8'),marker='export function registrationResultValueValid(value,kind){',prepared=[];
assert.equal(raw.split(marker).length-1,1);
for(const [name,injection,call]of [['baseline','',''],['hidden-read',"process.getBuiltinModule('fs').readFileSync('not-read');",'fs.readFileSync'],['hidden-rename',"process.getBuiltinModule('fs').renameSync('not-renamed','not-renamed');",'fs.renameSync'],['hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']])prepared.push({name,call,api:await import('data:text/javascript;base64,'+Buffer.from(raw.replace(marker,marker+injection)).toString('base64'))});
const corpus=await registrationResultValueCorpus(),held=[],attempts=[],mutations=[],check=api=>{for(const c of corpus.cases)assert.equal(api.registrationResultValueValid(c.value,c.kind),c.accepted);};
try{
 for(const [label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){const descriptor=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;assert(descriptor.configurable||descriptor.writable);held.push({obj,key,descriptor});Object.defineProperty(obj,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});}syncBuiltinESMExports();
 for(const p of prepared){if(p.name==='baseline'){check(p.api);assert.deepEqual(attempts,[]);}else{assert.throws(()=>check(p.api),{code:'SUBSTRATE_PROBE_DENIED'});assert.deepEqual(attempts,[p.call]);mutations.push(p.name);attempts.length=0;}}
}finally{for(const {obj,key,descriptor}of held.reverse())Object.defineProperty(obj,key,descriptor);syncBuiltinESMExports();}
for(const {obj,key,descriptor}of held)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),descriptor);
console.log(JSON.stringify({scope:'selected-F16-released-value-validation-substrate',wrappedFunctions:held.length,preconstructedNativeReplies:corpus.summary.nativeReplies,preconstructedValueCases:corpus.cases.length,attemptedBaselineCalls:0,mutations,producerExecutionImportIsolationOrPhysicalOrIndependentProof:false}));
