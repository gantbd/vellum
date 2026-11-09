import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),native='formal/native-discovery.mjs',facts='formal/discovery-current-facts.mjs',rules=[
 ['classification-headroom-bypassed',native,'discoveryArgumentClassificationBytes>m.limits.maxHeldBytes-m.charge','false'],
 ['workspace-headroom-bypassed',native,'workspace>m.limits.maxHeldBytes-m.charge','false'],
 ['workspace-unaccounted',native,'m.charge+=workspace','m.charge+=0'],
 ['one-complete-input-frame-omitted',native,'workspace=7*maximum','workspace=6*maximum'],
 ['classification-unaccounted',native,'charged=discoveryArgumentClassificationBytes;m.discoveryDecodeCharge=charged;m.charge+=charged;','charged=0;m.discoveryDecodeCharge=charged;m.charge+=charged;'],
 ['producer-receives-json-role-shadows',native,'queryNativeDiscoveryCore(owner,discoveryProducerRequest(n),','queryNativeDiscoveryCore(owner,n.record.request,'],
 ['producer-context-cloned',native,'queryNativeDiscoveryCore(owner,discoveryProducerRequest(n),','queryNativeDiscoveryCore(owner,{...discoveryProducerRequest(n),context:structuredClone(n.opaque.context)},'],
 ['original-cut-cloned',native,',cut,packet.value.facts)',',structuredClone(cut),packet.value.facts)'],
 ['original-owner-replaced',native,'queryNativeDiscoveryCore(owner,discoveryProducerRequest(n),','queryNativeDiscoveryCore(Object.freeze({kind:"native-discovery-registry/1"}),discoveryProducerRequest(n),'],
 ['fact-byte-bound-overaccepted',facts,'maxRequestBytes:maximum','maxRequestBytes:maximum+1'],
 ['fact-node-bound-overaccepted',facts,'...limits,maxRequestBytes:maximum','...limits,maxInputNodes:limits.maxInputNodes+1,maxRequestBytes:maximum'],
 ['fact-depth-bound-overaccepted',facts,'...limits,maxRequestBytes:maximum','...limits,maxInputDepth:limits.maxInputDepth+1,maxRequestBytes:maximum'],
 ['unrelated-host-bookkeeping-copied',facts,'copySettingsData({facts},{...limits,maxRequestBytes:maximum})','copySettingsData({facts:input},{...limits,maxRequestBytes:maximum})'],
 ['selected-data-getter-executed',facts,'const field=Object.getOwnPropertyDescriptor(input,key)','const field=Object.hasOwn(input,key)?{value:input[key],enumerable:true}:undefined'],
 ['all-host-bookkeeping-enumerated',facts,'const facts={};','const facts={};Object.getOwnPropertyDescriptors(input);'],
 ['recognized-false-loss-omitted',facts,'facts[key]=field.value;','if(field.value!==false)facts[key]=field.value;'],
 ['unknown-fact-repaired-to-true',facts,'if(field.value!==undefined)facts[key]=field.value;','facts[key]=field.value===undefined?true:field.value;'],
 ['unexpected-owner-fault-not-latched',native,'catch{m.discoveryFaulted=true;return no();}','catch{m.discoveryFaulted=false;return no();}'],
 ['temporary-workspace-not-refunded',native,'m.charge-=charged;','m.charge-=0;'],
 ['temporary-workspace-double-refunded',native,'m.charge-=charged;','m.charge-=2*charged;'],
 ['borrow-not-cleared',native,'m.discoveryDecoding=false;','m.discoveryDecoding=true;'],
 ['faulted-owner-can-build-new-cuts',native,'function rootFor(m,g){if(m.discoveryFaulted===true||invocationUsage(m.domain)','function rootFor(m,g){if(false||invocationUsage(m.domain)'],
 ['fault-refunds-spent-issuance',native,'catch{m.discoveryFaulted=true;return no();}','catch{m.issued=0;m.discoveryFaulted=true;return no();}'],
 ['fault-blocks-known-context-discharge',native,'export function stopNativeDiscovery(owner,context,c,g){const m=owners.get(owner),s=m?.contexts.get(context);if(m?.discoveryDecoding===true)return no();','export function stopNativeDiscovery(owner,context,c,g){const m=owners.get(owner),s=m?.contexts.get(context);if(m?.discoveryDecoding===true||m?.discoveryFaulted===true)return no();'],
 ['recursive-decoder-admitted',native,'m.discoveryDecoding===true||m.discoveryFaulted===true','m.discoveryFaulted===true'],
 ['ordinary-query-borrows-owned-decoder',native,'if(m?.discoveryDecoding===true||m?.discoveryFaulted===true)','if(m?.discoveryFaulted===true)'],
 ['descriptor-work-borrows-owned-decoder',native,"export function nativeBuiltinDiscoveryRows(owner,c,g){\n const m=owners.get(owner);if(m?.discoveryDecoding===true)return null;","export function nativeBuiltinDiscoveryRows(owner,c,g){\n const m=owners.get(owner);"],
 ['preparation-borrows-owned-decoder',native,'export function prepareNativeDiscoveryCut(owner,c,registry,visibility,target,advice,g){const m=owners.get(owner);if(m?.discoveryDecoding===true)return null;','export function prepareNativeDiscoveryCut(owner,c,registry,visibility,target,advice,g){const m=owners.get(owner);'],
 ['acquisition-consumes-borrowed-cut',native,'try{const m=owners.get(owner);if(m?.discoveryDecoding===true)return no();','try{const m=owners.get(owner);'],
 ['stop-reenters-owned-decoder',native,'export function stopNativeDiscovery(owner,context,c,g){const m=owners.get(owner),s=m?.contexts.get(context);if(m?.discoveryDecoding===true)return no();','export function stopNativeDiscovery(owner,context,c,g){const m=owners.get(owner),s=m?.contexts.get(context);'],
 ['context-retired-during-borrow',native,'export function retireNativeDiscovery(owner,context,g){const m=owners.get(owner),s=m?.contexts.get(context);if(m?.discoveryDecoding===true)return no();','export function retireNativeDiscovery(owner,context,g){const m=owners.get(owner),s=m?.contexts.get(context);'],
 ['cut-retired-during-borrow',native,'export function retireNativeDiscoveryCut(owner,packet,g){const m=owners.get(owner);if(m?.discoveryDecoding===true)return no();','export function retireNativeDiscoveryCut(owner,packet,g){const m=owners.get(owner);']
],bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);assert.equal(rules.length,32);assert(Number.isInteger(bank)&&bank>=0&&bank<8);
for(const [name,file,from]of rules)if(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1!==1)throw Error('DISCOVERY_MUTATION_CONSTRUCTION_TARGET: '+name);
const partition=Array.from({length:8},(_,b)=>rules.map((r,i)=>({r,i})).filter(x=>x.i%8===b)).flat().sort((a,b)=>a.i-b.i);assert.deepEqual(partition.map(x=>x.r[0]),rules.map(r=>r[0]));assert.equal(new Set(partition.map(x=>x.r[0])).size,32);
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{
 process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/discovery-gateway-corpus.mjs')],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const [name,file,from,to]of rules.filter((_,i)=>i%8===bank)){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));const out=run(graph.directory);assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1600));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1600));assert(!/SyntaxError|DISCOVERY_FAULT_CONSTRUCTION_TARGET|DISCOVERY_MUTATION_CONSTRUCTION_TARGET/.test(out.stderr),name+': construction failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'actual-F01-owner-gateway-sensitivity',bank,bankCount:8,totalMutations:32,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,resultWorkspaceOrGlobalPhysicalOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
