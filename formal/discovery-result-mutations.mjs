import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/discovery-results.mjs',rules=[
 ['extra-result-fields-admitted','!exact(v,forms[v.status])','false'],
 ['extra-request-fields-admitted','!exact(r,[...catalog.common,...catalog.variants[r.operation]])','false'],
 ['malformed-request-profile-admitted','!utf8(r.profile)','false'],
 ['original-context-binding-ignored','v.context!==r.context','false'],
 ['page-coverage-widened','v.coverage!==declarations.coverage','false'],
 ['page-request-limit-ignored','Math.min(r.limit,l.maxPageRows)','l.maxPageRows'],
 ['page-owner-limit-overaccepted','Math.min(r.limit,l.maxPageRows)','Math.min(r.limit,l.maxPageRows+1)'],
 ['page-entry-fields-widened','!exact(e,entryFields)','false'],
 ['page-reference-uniqueness-lost','refs.has(e.reference)','false'],
 ['version-domain-ignored','natural(x.version)&&',''],
 ['zero-version-admitted','x.version>0','true'],
 ['unknown-command-kind-admitted','declarations.kinds.includes(x.kind)','true'],
 ['advice-entry-correlation-lost','v.entry===r.entry','true'],
 ['advice-variant-correlation-lost','v.variant===r.variant','true'],
 ['advice-target-correlation-lost','v.target===r.target','true'],
 ['truthy-advice-value-admitted',"typeof v.enabled==='boolean'",'true'],
 ['descriptor-operation-correlation-lost',"r.operation!=='inspect'",'false'],
 ['descriptor-entry-correlation-lost','d.reference!==r.entry','false'],
 ['descriptor-extra-fields-admitted',"!exact(d,[...descriptorFields,...(Object.hasOwn(d,'presentation')?['presentation']:[])])",'false'],
 ['variant-count-overaccepted','dense(d.variants,l.maxVariants)','dense(d.variants,l.maxVariants+1)'],
 ['variant-facet-partition-widened',"!exact(f,['reference',...(visible?facets:[])])",'false'],
 ['variant-reference-uniqueness-lost','refs.has(f.reference)','false'],
 ['presentation-vocabulary-widened','Object.keys(p).some(k=>!declarations.presentation.includes(k))','false'],
 ['metadata-validation-bypassed','export function discoveryResultMetadataValid(value,l){','export function discoveryResultMetadataValid(value,l){return true;'],
 ['metadata-byte-overaccepted','bytes>l.maxMetadataBytes','bytes>l.maxMetadataBytes+1'],
 ['metadata-node-overaccepted','++nodes>l.maxMetadataNodes','++nodes>l.maxMetadataNodes+1'],
 ['metadata-depth-overaccepted','depth>l.maxMetadataDepth','depth>l.maxMetadataDepth+1'],
 ['malformed-metadata-string-admitted','if(!utf8(v))return false;','if(false)return false;'],
 ['malformed-metadata-key-admitted','if(!utf8(k))return false;','if(false)return false;'],
 ['nonfinite-metadata-number-admitted',"typeof v==='number'&&Number.isFinite(v)","typeof v==='number'"],
 ['nonenumerable-data-admitted',"&&ds[k].enumerable===true",''],
 ['opaque-array-role-admitted',"&&!Array.isArray(v)",'']
],bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);assert(Number.isInteger(bank)&&bank>=0&&bank<4);
for(const[name,from]of rules)assert.equal(fs.readFileSync(path.join(root,relative),'utf8').split(from).length-1,1,name+': construction target');
const partition=Array.from({length:4},(_,b)=>rules.map((r,i)=>({r,i})).filter(x=>x.i%4===b)).flat().sort((a,b)=>a.i-b.i);assert.deepEqual(partition.map(x=>x.r[0]),rules.map(r=>r[0]));assert.equal(new Set(partition.map(x=>x.r[0])).size,rules.length);
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{
 process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/discovery-result-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,from,to]of rules.filter((_,i)=>i%4===bank)){const graph=captureInputs(root),file=path.join(graph.directory,relative),raw=fs.readFileSync(file,'utf8');fs.writeFileSync(file,raw.replace(from,to));const out=run(graph.directory);assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1200));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1200));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('construction target'),name+': construction failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'closed-F01-result-product-sensitivity',bank,bankCount:4,totalMutations:rules.length,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,nativePublicationFaultOrPhysicalOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
