import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/source-products.mjs',rules=[
 ['result-guard-not-used','if(!sourceReadResultValid(out,request,expectedRef))','if(false)'],
 ['result-status-object-coerced',"typeof out.status!=='string'||",''],
 ['result-extra-fields-admitted','!exact(out,resultFields[out.status])','false'],
 ['original-result-context-ignored','out.context!==request.snapshot||',''],
 ['result-unit-ignored',"out.unit!=='utf8-byte'||",''],
 ['result-reference-shape-ignored','!exact(out.ref,refFields)||',''],
 ['result-reference-binding-ignored','||expectedRef!==null&&!equal(out.ref,expectedRef)',''],
 ['result-request-document-binding-ignored','||out.ref.authority!==request.document.authority||out.ref.document!==request.document.id||out.ref.incarnation!==request.document.incarnation',''],
 ['result-reference-numbers-ignored','||!natural(out.ref.incarnation)||!natural(out.ref.revision)',''],
 ['result-reference-text-ignored',"!['authority','document'].every(k=>typeof out.ref[k]==='string'&&out.ref[k].length>0)||",''],
 ['source-byte-integer-ignored','!Number.isInteger(v[i])||',''],
 ['source-byte-ceiling-ignored','||v[i]>255',''],
 ['inherited-bytes-admitted','!Object.hasOwn(v,i)||',''],
 ['range-start-binding-ignored','out.range[0]===request.lo&&',''],
 ['range-end-binding-ignored','out.range[1]===request.hi&&',''],
 ['range-number-domain-ignored','out.range.every(natural)&&',''],
 ['range-byte-count-ignored','&&out.source.length===out.range[1]-out.range[0]',''],
 ['line-index-binding-ignored','out.line===request.line&&',''],
 ['line-span-extra-fields-admitted','exact(v,spanFields)&&',''],
 ['line-span-number-domain-ignored','natural(v.lo)&&natural(v.hi)&&',''],
 ['line-span-byte-count-ignored','&&v.source.length===v.hi-v.lo',''],
 ['line-span-join-ignored','out.body.hi===out.delimiter.lo&&',''],
 ['delimiter-length-ignored','out.delimiter.source.length<=2&&',''],
 ['delimiter-vocabulary-ignored',"(out.delimiter.source.length===0||out.delimiter.source.length===1&&[10,13].includes(out.delimiter.source[0])||out.delimiter.source[0]===13&&out.delimiter.source[1]===10)",'true']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/source-read-result-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1200));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1200));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-source-read-result-shape-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,completeResultCodecPhysicalOrIndependentProof:false}));
