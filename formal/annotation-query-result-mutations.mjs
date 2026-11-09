import assert from 'node:assert/strict';
import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/annotation-queries.mjs',rules=[
 ['status-vocabulary-open',"typeof out.status!=='string'||!Object.hasOwn(nativeResultFields,out.status)||!nativeExact(out,nativeResultFields[out.status])",'false'],
 ['extra-result-fields-admitted','!nativeExact(out,nativeResultFields[out.status])','false'],
 ['snapshot-binding-ignored','out.snapshot!==request.snapshot','false'],
 ['page-operation-ignored',"request.operation!=='enumerate'||",''],
 ['page-limit-ignored','out.records.length>request.limit||',''],
 ['continuation-shape-ignored',"!(out.continuation===null||opaque(out.continuation))",'false'],
 ['page-density-ignored','!Object.hasOwn(out.records,i)||',''],
 ['record-shape-ignored','!nativeRecord(out.records[i])||',''],
 ['continuation-record-role-collision-ignored','||out.records[i].reference===out.continuation',''],
 ['duplicate-record-alias-admitted','if(out.records[j].reference===out.records[i].reference)return false;','if(false)return false;'],
 ['incomplete-nonterminal-page-admitted','out.continuation===null||out.records.length===request.limit','true'],
 ['row-extra-fields-admitted','nativeExact(v,fields)&&',''],
 ['row-reference-shape-ignored','opaque(v.reference)&&',''],
 ['body-domain-ignored',"typeof v.content.body==='string'&&",''],
 ['body-version-domain-ignored','natural(v.content.version)','true'],
 ['location-shape-ignored','!nativeExact(v,nativeValueFields.location)||',''],
 ['target-generation-domain-ignored','!natural(v.targetGeneration)','false'],
 ['placement-state-vocabulary-open','nativeValueFields.attachedStates.includes(p?.state)&&',''],
 ['placement-source-version-ignored','natural(p.sourceRevision)&&',''],
 ['target-shape-ignored','compoundTargetShape(p.target)','true'],
 ['record-original-reference-ignored','&&out.record.reference===request.annotation',''],
 ['location-original-reference-ignored','out.reference===request.annotation&&','']
];
const run=d=>spawnSync(process.execPath,[path.join(d,'formal/annotation-query-result-checks.mjs')],{encoding:'utf8',maxBuffer:1048576,env:process.env}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const[name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),raw=fs.readFileSync(file,'utf8');assert.equal(raw.split(from).length-1,1,name);fs.writeFileSync(file,raw.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,900));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,900));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-annotation-query-result-shape-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,completePublicProductsPhysicalOrIndependentProof:false}));
