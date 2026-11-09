import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),rules=[
 ['single-envelope-guard-omitted','read-contexts.mjs',"!out||typeof out.status!=='string'||!Object.hasOwn(singleResultFields,out.status)||!nativeReadProductExact(out,singleResultFields[out.status])",'false'],
 ['single-context-null-admitted','read-contexts.mjs','out.context!==null&&',''],
 ['single-context-array-admitted','read-contexts.mjs','&&!Array.isArray(out.context)',''],
 ['single-native-identity-ignored','read-contexts.mjs','(expectedContext===null||out.context===expectedContext)&&',''],
 ['single-retained-parent-reused','read-contexts.mjs',"(request.selection?.kind!=='retained'||out.context!==request.selection.reference)&&",''],
 ['class-count-ignored','read-contexts.mjs','v.length<1||v.length>2||v.length!==expected.length','false'],
 ['class-density-ignored','read-contexts.mjs','!Object.hasOwn(v,i)||',''],
 ['requested-classes-ignored','read-contexts.mjs','||!expected.includes(v[i])',''],
 ['duplicate-classes-admitted','read-contexts.mjs','||i>0&&v[i]===v[0]',''],
 ['set-envelope-guard-omitted','version-set-contexts.mjs',"!out||typeof out.status!=='string'||!Object.hasOwn(nativeSetResultFields,out.status)||!nativeReadProductExact(out,nativeSetResultFields[out.status])",'false'],
 ['set-context-null-admitted','version-set-contexts.mjs','out.context===null||',''],
 ['set-context-array-admitted','version-set-contexts.mjs','Array.isArray(out.context)||',''],
 ['set-native-identity-ignored','version-set-contexts.mjs','expectedContext!==null&&out.context!==expectedContext||',''],
 ['set-retained-parent-reused','version-set-contexts.mjs',"request.selection?.kind==='retained'&&out.context===request.selection.reference||",''],
 ['set-member-count-ignored','version-set-contexts.mjs','||out.participants.length!==request.participants.length',''],
 ['set-empty-admitted','version-set-contexts.mjs','out.participants.length<1||',''],
 ['set-member-fields-open','version-set-contexts.mjs',"!nativeReadProductExact(p,['document','snapshot','data'])||",''],
 ['set-member-document-ignored','version-set-contexts.mjs','!equal(p.document,r.document)||',''],
 ['set-member-classes-ignored','version-set-contexts.mjs','!readProjectionClassesMatch(p.data,r.data)||',''],
 ['set-member-null-alias-admitted','version-set-contexts.mjs','p.snapshot===null||',''],
 ['set-member-array-alias-admitted','version-set-contexts.mjs','Array.isArray(p.snapshot)||',''],
 ['set-parent-as-member-admitted','version-set-contexts.mjs','||p.snapshot===out.context',''],
 ['set-member-aliases-repeat','version-set-contexts.mjs','if(out.participants[j].snapshot===p.snapshot)return false;','if(false)return false;']
];
const run=d=>spawnSync(process.execPath,[path.join(d,'formal/read-acquisition-result-checks.mjs')],{encoding:'utf8',maxBuffer:1048576,env:process.env}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const[name,relative,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,'formal',relative),raw=fs.readFileSync(file,'utf8');assert.equal(raw.split(from).length-1,1,name);fs.writeFileSync(file,raw.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,900));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,900));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-read-acquisition-result-shape-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,fullPublicProductsPhysicalOrIndependentProof:false}));
