import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-contexts.mjs',rules=[
 ['all-retained-acquisitions-refused','if(!r||!facts','if(true||!r||!facts'],
 ['actual-disposal-fence-bypassed','ds[0].live!==true','false','currentness'],
 ['actual-owner-fault-fence-bypassed','if(owner.faulted||ds.length!==1','if(false||ds.length!==1','currentness'],
 ['retained-context-capacity-off-by-one','m.contexts.size>=p.maxContexts','m.contexts.size>p.maxContexts'],
 ['retained-selector-capacity-off-by-one','m.selectors.size>=p.maxSelectors','m.selectors.size>p.maxSelectors'],
 ['retained-issuance-capacity-off-by-one','m.issued>=p.maxIssued','m.issued>p.maxIssued'],
 ['new-preparation-reservation-ignored',"!yes(facts,['preparationReserved','resourcesReady'])",'false'],
 ['retained-identity-compared-by-copy','original=m.contexts.get(reference)','original=m.contexts.get(structuredClone(reference))'],
 ['retained-whole-projection-currentness-ignored','if(!currentContext(owner,m,original,c,facts))return no();',''],
 ['capacity-masks-original-context-loss','if(!currentContext(owner,m,original,c,facts))','if(m.contexts.size>=p.maxContexts)return no();if(!currentContext(owner,m,original,c,facts))'],
 ['retained-source-data-expansion-admitted','||r.data.some(k=>!original.value.data.includes(k))',''],
 ['empty-data-observes-bound-loss','r.data.length<1||',''],
 ['duplicate-data-observes-bound-loss','new Set(r.data).size!==r.data.length||',''],
 ['narrowing-keeps-hidden-source',"if(!r.data.includes('source'))delete selected.source;",''],
 ['source-only-narrowing-keeps-annotation-values',"if(!r.data.includes('annotations'))delete selected.annotations;",''],
 ['source-only-release-still-requires-annotation-authority',"ctx.value.data.includes('annotations')?",'true?','currentness'],
 ['source-only-current-source-read-loss-ignored','source&&g?.sourceReadable===false','false','currentness'],
 ['retained-old-content-mutated','const value=copy(selected,p,p.maxCaptureBytes)?.value','const value=copy({...selected,annotations:selected.annotations.map(row=>({...row,content:{body:"changed",version:999}}))},p,p.maxCaptureBytes)?.value'],
 ['retained-issued-budget-not-spent','m.contexts=contexts;m.selectors=selectors;m.issued++;return reply;','m.contexts=contexts;m.selectors=selectors;return reply;'],
 ['retained-parent-copy-not-installed','m.contexts=contexts;m.selectors=selectors;m.issued++;return reply;','m.selectors=selectors;m.issued++;return reply;'],
 ['copied-selectors-reuse-original-map','records:new Map(),continuations:new Map(),holds:0','records:original.records,continuations:original.continuations,holds:0']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-retained-annotation-assurance-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const baseline=captureInputs(root),positive=run(baseline.directory);assert.equal(positive.status,0,positive.stderr);const mutations=[];
for(const [name,from,to,scope]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8'),a=original.indexOf(scope==='currentness'?'function currentContext(':'export function acquireNativeRetainedAnnotationContext('),b=original.indexOf(scope==='currentness'?'export function queryNativeAnnotationContext(':'// Trusted owner inspection only',a),region=original.slice(a,b);assert(a>=0&&b>a);assert.equal(region.split(from).length-1,1,name);fs.writeFileSync(file,original.slice(0,a)+region.replace(from,to)+original.slice(b));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-native-retained-annotation-sensitivity',baseline:JSON.parse(positive.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allHistoryResourcesPublicPhysicalOrIndependentProof:false}));
