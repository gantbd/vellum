// Selected metadata execution after imports/fixture creation, not host isolation.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import {selectionDiscoveryFixture} from './selection-discovery-checks.mjs';
import {invocationUsage} from './invocation-publication.mjs';
const base=new URL('./',import.meta.url),raw=fs.readFileSync(new URL('native-discovery.mjs',base),'utf8'),marker='function queryNativeDiscoveryCore(owner,request,c,packet,g){',prepared=[];
for(const[name,injection,call]of [['baseline','',''],['selection-discovery-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-read');",'fs.readFileSync'],['selection-discovery-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-renamed','not-renamed');",'fs.renameSync'],['selection-discovery-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']]){
 if(raw.split(marker).length-1!==1)throw Error('SELECTION_SUBSTRATE_CONSTRUCTION_TARGET');
 const body=raw.replace(marker,marker+injection).replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,base).href+q),api=await import('data:text/javascript;base64,'+Buffer.from(body).toString('base64')),fixtures=[null,'typed','dynamic'].map(route=>{const x=selectionDiscoveryFixture({},api),a=x.open();assert.equal(a.status,'context');return {x,a,route,root:x.z.root(),parent:x.z.usage(),usage:x.usage()};});prepared.push({name,call,api,fixtures});
}
const held=[],attempts=[],mutations=[];let baselineCalls=0,nativeProducts=0,faultCases=0;
try{
 for(const[label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){const descriptor=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;assert(descriptor.configurable||descriptor.writable);held.push({obj,key,descriptor});Object.defineProperty(obj,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});}syncBuiltinESMExports();
 for(const entry of prepared){let injected=0;for(const {x,a,route,root,parent,usage}of entry.fixtures){const query=q=>x.query(a.context,q,x.g(),undefined,route);if(entry.name==='baseline'){
   const p=query({operation:'list',limit:12,continuation:null});assert.equal(p.status,'page');assert.equal(p.entries.length,12);const e=p.entries[11].reference,d=query({operation:'inspect',entry:e});assert.equal(d.status,'descriptor');assert.equal(d.descriptor.variants.length,3);nativeProducts+=2;for(const v of d.descriptor.variants){assert.equal(v.requirements.sourceWriteGrant,'not-required-for-view-only-selection');assert.equal(query({operation:'advisory-enablement',entry:e,variant:v.reference,target:a.target}).enabled,true);nativeProducts++;}assert.deepEqual(attempts,[]);baselineCalls+=5;
  }else{
   if(route===null)assert.throws(()=>query({operation:'list',limit:12,continuation:null}),{code:'SUBSTRATE_PROBE_DENIED'});else{assert.deepEqual(query({operation:'list',limit:12,continuation:null}),{status:'unavailable'});assert.deepEqual(entry.api.nativeDiscoveryDecodeUsage(x.owner),{busy:false,declaredWorkspaceBytes:0,faulted:true});}assert.deepEqual(attempts,[entry.call]);attempts.length=0;assert.equal(x.usage().issued,usage.issued);injected++;faultCases++;
  }
  assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),parent);assert.equal(invocationUsage(x.domain).faulted,false);assert.equal(x.usage().preparations,0);const issued=x.usage().issued;assert.equal(entry.api.stopNativeDiscovery(x.owner,a.context,x.c,x.g()).status,'stopped');assert.equal(entry.api.retireNativeDiscovery(x.owner,a.context,x.g()).status,'retired');assert.equal(x.usage().issued,issued);assert.equal(x.usage().contexts,0);assert.equal(entry.api.retireNativeDiscoveryRegistry(x.owner,x.g()).status,'retired');
 }if(entry.name!=='baseline'){assert.equal(injected,3);mutations.push({name:entry.name,cases:injected});}}
}finally{for(const{obj,key,descriptor}of held.reverse())Object.defineProperty(obj,key,descriptor);syncBuiltinESMExports();}
for(const{obj,key,descriptor}of held)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),descriptor);
console.log(JSON.stringify({scope:'selected-existing-F12-discovery-substrate',wrappedFunctions:held.length,preconstructedBaselineFixtures:3,preconstructedFaultFixtures:9,nativeProducts,baselineCalls,attemptedBaselineCalls:0,faultCases,mutations,originalCleanupAndTemporaryRefundChecked:true,coherentBaselinePassed:true,allHostAPIsImportIsolationPhysicalOrIndependentProof:false}));
