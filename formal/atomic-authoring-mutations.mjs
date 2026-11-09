// Coherent graph sensitivity. Syntax/import/construction failures are not semantic detections.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const rules=[
 ['component-admission-authority-bypassed','atomic-authoring-intent.mjs','request,h,h.annotationLimits)','request,{...h,annotationWritable:true},h.annotationLimits)'],
 ['component-current-authority-bypassed','atomic-authoring-intent.mjs',"!annotationEditPreparationCurrent(x.basis==='candidate'?candidate:before,x.native,h)",'false'],
 ['original-create-allocation-replaced','annotation-editing.mjs','!equal(p.value.allocation,h.annotationAllocation)','false'],
 ['candidate-basis-treated-as-before','atomic-authoring-intent.mjs',"a.target?.basis==='candidate'?'candidate':'before'","'before'"],
 ['private-source-companion-dependency-ignored','annotation-compounds.mjs','d.revision!==input.expectedSource||d.annotationSet!==input.expectedAnnotationSet','d.revision!==input.expectedSource'],
 ['component-scope-binding-ignored','atomic-authoring-intent.mjs','e.component===index&&e.invocation===invocation&&equal(e.document,request.document)&&e.operation===request.operation&&',''],
 ['duplicate-authored-fields-admitted','atomic-authoring-intent.mjs','!distinctFields(lowered)||',''],
 ['actual-parent-bundle-debit-omitted','invocation-publication.mjs','entry.ordinaryCharge=baseCharge;o.charge+=baseCharge;','entry.ordinaryCharge=baseCharge;o.charge+=0;'],
 ['changed-public-intent-reused','invocation-publication.mjs','equal(held.capture,capture.value)','true',1,'atomicAuthoringPreparation'],
 ['current-source-authority-ignored','invocation-publication.mjs','!atomicAuthoringPermissions(host,entry.plan.sources)','false'],
 ['final-component-root-cap-ignored','invocation-publication.mjs','!atomicAuthoringCandidateFits(held.mixedBundle,o,domain,ticket,result.state)','false'],
 ['original-candidate-source-context-ignored','annotation-editing.mjs','r.target.sourceRevision===d.revision','true',2],
 ['explicit-mixed-profile-ignored','atomic-authoring-arguments.mjs','r.profile!==form.profileConstant','false']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/atomic-authoring-checks.mjs')],{encoding:'utf8',timeout:60000,maxBuffer:1048576,env:process.env});
const first=run(root);assert.equal(first.status,0,first.stderr);const baseline=JSON.parse(first.stdout),mutations=[];
for(const [name,file,from,to,count=1,scope] of rules){
 const c=captureInputs(root),target=path.join(c.directory,'formal',file),raw=fs.readFileSync(target,'utf8'),a=scope?raw.indexOf('function '+scope+'('):0,b=scope?raw.indexOf('// No private plan,',a):raw.length;assert(a>=0&&b>a,name+': scope');const region=raw.slice(a,b);assert.equal(region.split(from).length-1,count,name+': construction anchor');
 fs.writeFileSync(target,raw.slice(0,a)+region.replaceAll(from,to)+raw.slice(b));const r=run(c.directory);assert.equal(r.status,1,name+': '+r.stderr);assert(r.stderr.includes('ERR_ASSERTION'),name+': not semantic assertion');assert(!r.stderr.includes('SyntaxError'),name+': syntax failure');mutations.push(name);
}
console.log(JSON.stringify({scope:'owned-public-atomic-authoring-binding-mutations',baseline,mutations,
 coherentGraphBaseline:true,constructionFailuresCounted:false,U01Selected:false,
 fullQualificationOrIndependentAssurance:false}));
