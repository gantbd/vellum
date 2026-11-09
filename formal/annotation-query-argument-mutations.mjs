import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/annotation-query-arguments.mjs',rules=[
 ['family-routing-gate-ignored',"if(decoded?.descriptor.id!=='F03')return null;",''],
 ['variant-agreement-ignored','decoded.variant!==args.operation','false'],
 ['extra-argument-fields-accepted','!exact(args,form.fields)','!args'],
 ['maximum-capture-ceiling-ignored','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['byte-bound-off-by-one','Math.min(maximum,limits.maxRequestBytes)','Math.min(maximum,limits.maxRequestBytes)+1'],
 ['node-bound-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputNodes:limits.maxInputNodes+1,maxRequestBytes:'],
 ['depth-bound-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputDepth:limits.maxInputDepth+1,maxRequestBytes:'],
 ['original-snapshot-cloned','snapshot:args.snapshot,role,selector','snapshot:structuredClone(args.snapshot),role,selector'],
 ['original-secondary-selector-cloned','snapshot:args.snapshot,role,selector','snapshot:args.snapshot,role,selector:structuredClone(selector)'],
 ['route-enters-canonical-meaning','{request:r,context:c,limits,opaqueBindings:','{route:e.route,request:r,context:c,limits,opaqueBindings:'],
 ['raw-request-alias-retained','record:capture.value','record:{...capture.value,request:args}'],
 ['authority-frame-extra-fields-admitted','!exact(bound.value,contextFields)','false'],
 ['foreign-document-authority-admitted','r.document.authority!==c.authority','false'],
 ['zero-document-incarnation-admitted','r.document.incarnation<1','false'],
 ['fractional-document-incarnation-admitted','!natural(r.document.incarnation)','false'],
 ['extra-document-fields-admitted',"!exact(r.document,['authority','id','incarnation'])",'false'],
 ['zero-page-limit-admitted','r.limit<1','false'],
 ['fractional-page-limit-admitted','!natural(r.limit)','false'],
 ['continuation-shape-not-checked',"selector!==null&&!object(selector):!object(selector)","false:!object(selector)"],
 ['record-selector-shape-not-checked',"selector!==null&&!object(selector):!object(selector)","selector!==null&&!object(selector):false"],
 ['unsupported-profile-preempted',"const capture=copySettingsData(","if(r.profile!=='source-read')return null;const capture=copySettingsData("],
 ['snapshot-identity-ignored-in-agreement','a.opaque.snapshot===b.opaque.snapshot&&',''],
 ['secondary-identity-ignored-in-agreement','a.opaque.selector===b.opaque.selector&&','']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/annotation-query-argument-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1200));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1200));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-F03-annotation-query-argument-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,gatewayOwnershipFullPublicOrIndependentProof:false}));
