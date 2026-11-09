import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import {observationResultFixture} from './invocation-observation-result-checks.mjs';
import {invocationUsage} from './invocation-publication.mjs';
import {invocationObservationPolicy as policy,invocationObservationGates as gates} from './invocation-observation-checks.mjs';
const base=new URL('./',import.meta.url),raw=fs.readFileSync(new URL('invocation-observation.mjs',base),'utf8'),marker='export function queryInvocationObservation(context,request,c,p,g){const e=owners.get(context);try{',prepared=[];
for(const[name,injection,call]of [['baseline','',''],['observation-result-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-read');",'fs.readFileSync'],['observation-result-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-renamed','not-renamed');",'fs.renameSync'],['observation-result-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']]){
 assert.equal(raw.split(marker).length-1,1);const code=raw.replace(marker,marker+(injection?"if(g?.testSubstrateProbe===true){"+injection+"}":"")).replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,base).href+q),api=await import('data:text/javascript;base64,'+Buffer.from(code).toString('base64')),fixtures=[];
 for(const operation of ['status','retained-outcome','minimal-control-status'])for(const terminal of [false,true]){const x=observationResultFixture(api),first=x.query(),wait=x.wait(first.observation);assert.equal(wait.status,'waiting');if(terminal)x.finish();fixtures.push({x,operation,terminal,wait});}
 prepared.push({name,call,api,fixtures});
}
const held=[],attempts=[],mutations=[];let baselineCases=0;
try{
 for(const[label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){const descriptor=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;assert(descriptor.configurable||descriptor.writable);held.push({obj,key,descriptor});Object.defineProperty(obj,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});}syncBuiltinESMExports();
 for(const entry of prepared){let cases=0;for(const {x,operation,terminal,wait}of entry.fixtures){const before=entry.api.invocationObservationUsage(x.handle),producer=structuredClone(invocationUsage(x.domain)),root=x.current(),r=x.query(operation,policy,{...gates,testSubstrateProbe:true});assert.deepEqual(invocationUsage(x.domain),producer);assert.equal(x.current(),root);
   if(entry.name==='baseline'){assert.deepEqual(attempts,[]);assert.equal(r.status,operation==='retained-outcome'?(terminal?'outcome':'unavailable'):'execution');assert.equal(entry.api.invocationObservationUsage(x.handle).live,true);baselineCases++;}
   else{assert.deepEqual(attempts,[entry.call]);attempts.length=0;assert.equal(r.status,'unavailable');const after=entry.api.invocationObservationUsage(x.handle);assert.equal(after.live,false);assert.equal(after.issued,before.issued);assert.equal(after.waiters,1);assert.equal(after.waiting,0);assert.equal(after.declaredHeldBytes,before.declaredHeldBytes);assert.equal(entry.api.retireInvocationObservation(x.handle,gates).status,'unavailable');assert.equal(entry.api.retireInvocationWait(x.handle,wait.wait,gates).status,'retired');assert.equal(entry.api.retireInvocationObservation(x.handle,gates).status,'retired');}cases++;
  }if(entry.name!=='baseline')mutations.push({name:entry.name,cases});}
}finally{for(const{obj,key,descriptor}of held.reverse())Object.defineProperty(obj,key,descriptor);syncBuiltinESMExports();}
for(const{obj,key,descriptor}of held)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),descriptor);
console.log(JSON.stringify({scope:'selected-native-F06-result-substrate',wrappedFunctions:held.length,preconstructedFixtures:6,baselineCases,attemptedBaselineCalls:0,mutations,coherentBaselinePassed:true,importIsolationAllRoutesPhysicalOrHostTruthProved:false}));
