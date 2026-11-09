import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/history-contexts.mjs',rules=[
 ['classification-headroom-unchecked','historyQueryArgumentClassificationBytes>o.limits.maxHeldBytes-o.charge','false'],
 ['classification-not-parent-charged','o.charge+=charged;',''],
 ['workspace-underreserved','workspace=7*maximum','workspace=6*maximum'],
 ['workspace-headroom-unchecked','workspace>o.limits.maxHeldBytes-o.charge','false'],
 ['workspace-not-parent-charged','o.charge+=workspace;',''],
 ['workspace-not-recorded','charged+=workspace;o.historyDecodeCharge=charged;o.charge+=workspace;','charged+=workspace;o.charge+=workspace;'],
 ['workspace-not-in-refund-tally','charged+=workspace;','charged+=0;'],
 ['temporary-charge-not-refunded','o.charge-=charged;',''],
 ['only-classification-refunded','o.charge-=charged;','o.charge-=historyQueryArgumentClassificationBytes;'],
 ['temporary-charge-overrefunded','o.charge-=charged;','o.charge-=charged+1;'],
 ['busy-fence-ignored','||o.historyDecoding||','||false||'],
 ['busy-fence-not-installed','o.historyDecoding=true;','o.historyDecoding=false;'],
 ['busy-fence-not-cleared','o.historyDecoding=false;}','o.historyDecoding=true;}'],
 ['temporary-charge-record-not-cleared','o.historyDecodeCharge=0;',''],
 ['faulted-owner-readmits-decoding','o.faulted||o.historyDecoding','o.historyDecoding'],
 ['decoder-fault-does-not-fence','catch{o.faulted=true;return unavailable();}finally{','catch{return unavailable();}finally{'],
 ['raw-facts-bypass-capture','packet.value.facts','g'],
 ['facts-byte-envelope-ignored','maxRequestBytes:maximum','maxRequestBytes:Number.MAX_SAFE_INTEGER'],
 ['facts-node-envelope-ignored','copySettingsData({facts:g},{...p,maxRequestBytes:maximum})','copySettingsData({facts:g},{...p,maxInputNodes:Number.MAX_SAFE_INTEGER,maxRequestBytes:maximum})'],
 ['facts-depth-envelope-ignored','copySettingsData({facts:g},{...p,maxRequestBytes:maximum})','copySettingsData({facts:g},{...p,maxInputDepth:Number.MAX_SAFE_INTEGER,maxRequestBytes:maximum})'],
 ['capture-limit-substituted-for-decoder-intent','const maximum=p.maxRequestBytes','const maximum=o.limits.maxCaptureBytes'],
 ['failed-facts-copy-becomes-invariant-fault','if(!packet)return unavailable();','if(false)return unavailable();'],
 ['raw-shadow-substitutes-for-original-context','historyQueryProducerRequest(n)','n.record.request'],
 ['original-context-cloned-for-dispatch','historyQueryProducerRequest(n)','{...historyQueryProducerRequest(n),context:structuredClone(n.opaque)}'],
 ['current-facts-truthiness-coerced','copySettingsData({facts:g}','copySettingsData({facts:Object.fromEntries(Object.entries(g).map(([k,v])=>[k,Boolean(v)]))}'],
 ['lookalike-owner-substituted-for-original','return queryHistoryInspection(owner,historyQueryProducerRequest(n)','return queryHistoryInspection(Object.freeze({kind:"history-inspection-owner/1"}),historyQueryProducerRequest(n)'],
 ['malformed-limits-become-invariant-fault','if(!p)return unavailable();','if(false)return unavailable();']
];
for(const[name,from]of rules)assert.equal(fs.readFileSync(path.join(root,relative),'utf8').split(from).length-1,1,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;const files=['history-query-gateway-checks.mjs','history-query-gateway-faults.mjs'],run=(directory,file)=>spawnSync(process.execPath,[path.join(directory,'formal',file)],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:20000}),positive=captureInputs(root),baseline=[];
 for(const file of files){const out=run(positive.directory,file);assert.equal(out.status,0,out.stderr);baseline.push(JSON.parse(out.stdout));}const mutations=[];
 for(const[name,from,to]of rules){const graph=captureInputs(root),target=path.join(graph.directory,relative),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));let detected=false;
  for(const file of files){const out=run(graph.directory,file);assert([0,1].includes(out.status)&&!out.error&&!out.signal,name+': process/tool failure');assert(!out.stderr.includes('SyntaxError'),name);if(out.status===1){assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1200));detected=true;}else assert.equal(JSON.parse(out.stdout).scope,file.includes('faults')?'fixed-F04-decoder-owner-fault-and-reentry':'actual-owner-decoded-F04-history-query-gateway');}
  assert(detected,name+': survived complete gateway/fault corpus');mutations.push(name);
 }
 passed=true;console.log(JSON.stringify({scope:'isolated-actual-owner-F04-decoder-sensitivity',baseline:baseline[0],faults:baseline[1],mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,globalPhysicalAllPublicOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
