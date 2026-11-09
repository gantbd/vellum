import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),context='formal/native-annotation-contexts.mjs',search='formal/native-source-search.mjs',rules=[
 ['combined-hold-count-ignored',context,'m.holds.size>=m.profile.maxHolds','false'],
 ['combined-hold-count-off-by-one',context,'m.holds.size>=m.profile.maxHolds','m.holds.size>m.profile.maxHolds'],
 ['shared-issuance-off-by-one',context,'m.issued>=m.profile.maxIssued','m.issued>m.profile.maxIssued'],
 ['combined-hold-record-bound-ignored',context,'!copy({context:reference,published:false},m.profile,m.profile.maxSelectorRecordBytes)','false'],
 ['hold-reservation-omitted',context,'(2*p.maxSelectors+p.maxHolds)','(2*p.maxSelectors)'],
 ['actual-document-disposal-ignored',context,'ds[0].live!==true','false'],
 ['actual-owner-fault-ignored',context,'if(owner.faulted||ds.length!==1','if(false||ds.length!==1'],
 ['shared-issuance-ceiling-ignored',context,'m.issued>=m.profile.maxIssued','false'],
 ['hold-row-not-installed',context,'m.holds.set(lease,{context:reference,ctx,published:false});',''],
 ['hold-owner-count-not-incremented',context,'ctx.holds++;m.issued++;return lease;','m.issued++;return lease;'],
 ['held-context-can-retire',context,'ctx.holds!==0||!yes(g,discharge)','false||!yes(g,discharge)'],
 ['discharge-refunds-spent-issuance',context,'m.holds.delete(lease);held.ctx.holds--;return true;','m.holds.delete(lease);held.ctx.holds--;m.issued--;return true;'],
 ['published-hold-phase-not-recorded',context,'held.published=true;return true;','return true;'],
 ['combined-contexts-not-resolved',search,"for(const kind of ['source','combined','versionSet'])","for(const kind of ['source','versionSet'])"],
 ['source-only-contexts-not-resolved',search,"for(const kind of ['source','combined','versionSet'])","for(const kind of ['combined','versionSet'])"],
 ['query-retargets-to-bootstrap-pool',search,'const issuer=locateIssuer(owner,domain,r.context);',"const issuer={kind:'combined',handle:m.sourceHandle};"],
 ['stopped-context-can-release-held-source',context,'return held&&m.contexts.get(held.context)===held.ctx?inspectNativeCombinedSearchContext(owner,domain,handle,held.context,c,g):null;','return held?{snapshot:{ref:held.ctx.value.source.ref,source:held.ctx.value.source.bytes,selector:held.context,profile:held.ctx.scope.profile},sourceProfile:held.ctx.value.source.profile}:null;']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-combined-search-assurance-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,relative,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
{
 const name='all-protected-combined-discharge-gates-bypassed',graph=captureInputs(root);
 for(const [relative,from,to] of [
  [context,'published&&!yes(g,discharge)','false'],
  [search,'if(!yes(facts,discharge)||m.results.get(result)!==held||!issuers','if(false||m.results.get(result)!==held||!issuers'],
  [search,'discardLiteralSearchResult(result,facts)','discardLiteralSearchResult(result,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true})']
 ]){const file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));}
 const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);
}
console.log(JSON.stringify({scope:'isolated-native-combined-search-sensitivity',baseline:JSON.parse(positive.stdout),mutations:killed,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,asyncWorkerPhysicalOrPublicProtocolProved:false}));
