import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/source-read-arguments.mjs',rules=[
 ['family-routing-gate-ignored',"if(decoded?.descriptor.id!=='F02')return null;",''],
 ['variant-agreement-ignored','decoded.variant!==args.operation','false'],
 ['unknown-argument-fields-stripped','!exact(args,form.fields)','!args'],
 ['maximum-capture-ceiling-ignored','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['byte-capture-off-by-one','Math.min(maximum,limits.maxRequestBytes)','Math.min(maximum,limits.maxRequestBytes)+1'],
 ['node-capture-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputNodes:limits.maxInputNodes+1,maxRequestBytes:'],
 ['depth-capture-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputDepth:limits.maxInputDepth+1,maxRequestBytes:'],
 ['snapshot-identity-copied','const opaque=args.snapshot,','const opaque=structuredClone(args.snapshot),'],
 ['route-enters-canonical-meaning','{request:r,context:c,limits,opaqueBindings:','{route:e.route,request:r,context:c,limits,opaqueBindings:'],
 ['raw-request-alias-retained','record:capture.value','record:{...capture.value,request:args}'],
 ['authority-frame-extended-fields-accepted','!exact(bound.value,contextFields)','false'],
 ['authority-domain-ignored','r.document.authority!==c.authority','false'],
 ['document-incarnation-zero-admitted','r.document.incarnation<1','false'],
 ['range-order-not-validated','r.lo>r.hi','false'],
 ['range-offset-not-numeric','!natural(r.lo)','false'],
 ['line-offset-not-numeric','!natural(r.line)','false'],
 ['unsupported-profile-rejected-before-owner',"const capture=copySettingsData(","if(r.profile!=='source-read')return null;const capture=copySettingsData("],
 ['extra-document-fields-admitted',"!exact(r.document,['authority','id','incarnation'])",'false']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/source-read-argument-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1000));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-F02-range-line-argument-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,queryGatewayOwnershipAcquisitionWireOrIndependentProof:false}));
