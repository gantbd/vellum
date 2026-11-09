// Selected execution after module and fixture setup, not a host sandbox.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import cp from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import * as kernel from './invocation-publication.mjs';
import {checkDiscoveryGateway,discoveryGatewayFixture} from './discovery-gateway-checks.mjs';
const raw=fs.readFileSync(new URL('./native-discovery.mjs',import.meta.url),'utf8'),marker='  const packet=captureNativeDiscoveryQueryCurrentFacts(g,p,maximum);',prepared=[];
for(const [name,injection,call]of [['baseline','',''],['decoder-hidden-read',"process.getBuiltinModule('fs').readFileSync('not-read');",'fs.readFileSync'],['decoder-hidden-rename',"process.getBuiltinModule('fs').renameSync('not-renamed','not-renamed');",'fs.renameSync'],['decoder-hidden-process',"process.getBuiltinModule('child_process').spawnSync('not-launched');",'child_process.spawnSync']]){
 if(raw.split(marker).length-1!==1)throw Error('DISCOVERY_SUBSTRATE_CONSTRUCTION_TARGET');
 const body=raw.replace(marker,injection+marker).replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,import.meta.url).href)).replace(/new URL\('(\.\/[^']+)',import.meta.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,import.meta.url).href)+',import.meta.url)');
 prepared.push({name,call,api:await import('data:text/javascript;base64,'+Buffer.from(body).toString('base64'))});
}
const pattern=[];
checkDiscoveryGateway(prepared[0].api,(op,route,profile,options,api)=>{pattern.push({op,route,profile,options:structuredClone(options)});return discoveryGatewayFixture(op,route,profile,options,api);});
prepared[0].fixtures=pattern.map(r=>({...r,fixture:discoveryGatewayFixture(r.op,r.route,r.profile,r.options,prepared[0].api)}));
for(const p of prepared.slice(1))p.fixtures=['list','inspect','advisory-enablement'].flatMap(op=>['typed','dynamic'].map(route=>discoveryGatewayFixture(op,route,10,{},p.api)));
const held=[],attempts=[],mutations=[];let baseline,checkedCases=0;
try{
 for(const [label,obj]of [['fs',fs],['fs.promises',fsp],['child_process',cp]])for(const key of Object.keys(obj)){
  const descriptor=Object.getOwnPropertyDescriptor(obj,key);if(typeof obj[key]!=='function')continue;assert(descriptor.configurable||descriptor.writable);held.push({obj,key,descriptor});Object.defineProperty(obj,key,{configurable:true,enumerable:descriptor.enumerable,writable:true,value:()=>{attempts.push(label+'.'+key);const e=Error('denied substrate call');e.code='SUBSTRATE_PROBE_DENIED';throw e;}});
 }
 syncBuiltinESMExports();
 const queue=[...prepared[0].fixtures];baseline=checkDiscoveryGateway(prepared[0].api,(op,route,profile,options,api)=>{const next=queue.shift();assert(next);assert.equal(op,next.op);assert.equal(route,next.route);assert.equal(profile,next.profile);assert.deepEqual(options,next.options);assert.equal(api,prepared[0].api);return next.fixture;});assert.equal(queue.length,0);assert.deepEqual(attempts,[]);
 for(const p of prepared.slice(1)){
  let cases=0;for(const x of p.fixtures){
   assert.equal(x.query().status,'unavailable');assert.deepEqual(attempts,[p.call]);attempts.length=0;const b=x.last();assert.equal(x.root(),b.root);assert.deepEqual(x.x.z.usage(),b.parent);assert.equal(kernel.invocationUsage(x.x.domain).faulted,false);
   assert.deepEqual(x.decodeUsage(),{busy:false,declaredWorkspaceBytes:0,faulted:true});assert.deepEqual(x.usage(),{...b.usage,preparations:b.usage.preparations-1});const issued=x.usage().issued;assert.equal(x.retire().status,'retired');assert.equal(x.usage().issued,issued);cases++;checkedCases++;
  }mutations.push({name:p.name,cases});
 }
}finally{for(const {obj,key,descriptor}of held.reverse())Object.defineProperty(obj,key,descriptor);syncBuiltinESMExports();}
for(const {obj,key,descriptor}of held)assert.deepEqual(Object.getOwnPropertyDescriptor(obj,key),descriptor);
console.log(JSON.stringify({scope:'selected-actual-F01-owner-gateway-substrate',baseline,wrappedFunctions:held.length,preconstructedBaselineFixtures:pattern.length,preconstructedFaultFixtures:checkedCases,attemptedBaselineCalls:0,mutations,coherentBaselinePassed:true,originalContextCleanupAndOnlyTemporaryRefundChecked:true,allHostAPIsImportIsolationPhysicalOrGeneralFaultContainmentProved:false}));
