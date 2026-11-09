import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/read-acquisition-arguments.mjs',rules=[
 ['family-gate-ignored',"decoded?.descriptor.id!=='F02'||",''],
 ['variant-agreement-ignored',"decoded.variant!=='snapshot-handoff'",'false'],
 ['extra-request-fields-admitted','!exact(args,form.fields)','!args'],
 ['selection-object-coerced',"typeof selection.kind!=='string'",'false'],
 ['extra-selection-fields-admitted',"!exact(selection,['kind',...form.selections[selection.kind]])",'false'],
 ['retained-reference-copied','opaque=retained?selection.reference:null','opaque=retained?structuredClone(selection.reference):null'],
 ['maximum-capture-ceiling-ignored','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['byte-bound-off-by-one','Math.min(maximum,limits.maxRequestBytes)','Math.min(maximum,limits.maxRequestBytes)+1'],
 ['node-bound-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputNodes:limits.maxInputNodes+1,maxRequestBytes:'],
 ['depth-bound-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputDepth:limits.maxInputDepth+1,maxRequestBytes:'],
 ['class-set-order-not-canonical',"?[...v].sort():null","?[...v]:null"],
 ['participant-order-rewritten','request={...r,participants};','request={...r,participants:participants.toSorted((a,b)=>a.document.id.localeCompare(b.document.id))};'],
 ['duplicate-class-accepted','new Set(v).size===v.length','true'],
 ['empty-class-set-accepted','v.length>=1','true'],
 ['unknown-class-accepted','v.every(k=>single.data.includes(k))','true'],
 ['duplicate-participant-accepted','keys.has(key)','false'],
 ['one-slot-can-be-two-incarnations','[p.document.authority,p.document.id]','[p.document.authority,p.document.id,p.document.incarnation]'],
 ['zero-document-incarnation-admitted','v.incarnation>=1','true'],
 ['foreign-document-authority-admitted','v.authority===authority','true'],
 ['foreign-set-authority-admitted','r.authority!==c.authority','false'],
 ['authority-frame-extra-fields-admitted','!exact(bound.value,contextFields)','false'],
 ['caller-data-retained','record:capture.value','record:{...capture.value,request:args}'],
 ['route-enters-canonical-meaning','{request,context:c,limits,opaqueBindings:','{route:e.route,request,context:c,limits,opaqueBindings:']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/read-acquisition-argument-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1000));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-F02-read-acquisition-argument-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,preparedCutsGatewayOwnershipWireOrIndependentProof:false}));
