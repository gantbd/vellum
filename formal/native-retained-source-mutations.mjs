import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-source-contexts.mjs',rules=[
 ['retained-reference-cloned-before-lookup','reference=retained?selection.reference:null','reference=retained?structuredClone(selection.reference):null'],
 ['retained-lookup-skips-actual-disposal','||!rootLive(owner,s.document)){close(ctx,true);return null;}','||false){close(ctx,true);return null;}'],
 ['retained-lookup-skips-actual-owner-fault','if(owner.faulted||loss.some(k=>g?.[k]===false)||!rootLive(owner,s.document)){close(ctx,true);return null;}','if(false||loss.some(k=>g?.[k]===false)||!rootLive(owner,s.document)){close(ctx,true);return null;}'],
 ['retained-release-unknown-facts-treated-as-true','return yes(g,release)?{m,ctx}:null;','return {m,ctx};'],
 ['retained-bytes-refreshed-from-current','source:retained?original.ctx.snapshot.source:d.source','source:retained?rootLive(owner,s.document).source:d.source'],
 ['retained-revision-refreshed-from-current','const ref=retained?original.ctx.snapshot.ref:','const ref=retained?{...original.ctx.snapshot.ref,revision:rootLive(owner,s.document).revision}:'],
 ['capacity-masks-retained-loss','const original=retained?searchSource','if(m.contexts.size>=p.maxContexts)return no();const original=retained?searchSource'],
 ['retained-context-capacity-off-by-one','m.contexts.size>=p.maxContexts','m.contexts.size>p.maxContexts'],
 ['retained-issuance-capacity-off-by-one','m.issued>=p.maxIssued','m.issued>p.maxIssued'],
 ['retained-request-copy-off-by-one',':request,p,p.maxRequestBytes)?.value;',':request,p,p.maxRequestBytes+1)?.value;'],
 ['retirement-refunds-spent-retained-issuance','return m.contexts.delete(reference);','m.issued--;return m.contexts.delete(reference);'],
 ['retained-acquisition-always-refused','if(retained&&!original||owner.faulted','if(retained||owner.faulted'],
 ['data-scope-expansion-admitted',"||!equal(r.data,['source']))return no();",')return no();']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-retained-source-assurance-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const mutations=[];
for(const [name,from,to]of rules){
 const graph=captureInputs(root),file=path.join(graph.directory,relative);let original=fs.readFileSync(file,'utf8');
 assert.equal(original.split(from).length-1,1,name);
 if(name==='retained-lookup-skips-actual-owner-fault'){
  // Isolate retained lookup from the newer admission and later query guards.
  // The unchanged lookup must still pass this contextual baseline before mutation.
  const kernel=path.join(graph.directory,'formal/invocation-publication.mjs'),raw=fs.readFileSync(kernel,'utf8'),guard='if(!o||o.faulted||o.admissionMutation||o.admissionFence&&allowFenced!==true)';
  assert.equal(raw.split(guard).length-1,1,name+': contextual admission fault guard');
  fs.writeFileSync(kernel,raw.replace(guard,'if(!o||o.admissionMutation||o.admissionFence&&allowFenced!==true)'));
  const queryGuard='if(owner.faulted||loss.some(k=>g?.[k]===false)||!rootLive(owner,s.document)){close(ctx,true);return no();}';
  assert.equal(original.split(queryGuard).length-1,1,name+': contextual later query fault guard');
  original=original.replace(queryGuard,'if(false||loss.some(k=>g?.[k]===false)||!rootLive(owner,s.document)){close(ctx,true);return no();}');
  fs.writeFileSync(file,original);const contextual=run(graph.directory);assert.equal(contextual.status,0,contextual.stderr);
 }
 fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);
 assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);
}
console.log(JSON.stringify({scope:'isolated-native-retained-source-sensitivity',baseline:JSON.parse(positive.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,retainedLookupHasAdmissionAndLaterQueryFaultGuardsDisabledContextualBaseline:true,arbitraryHistoryCombinedVersionSetPhysicalOrIndependentProof:false}));
