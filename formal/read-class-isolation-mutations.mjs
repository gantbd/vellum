import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/read-contexts.mjs',rules=[
 ['unrequested-source-and-revision-retained','const retainedRoot={document:clone(root.document),...(sourceProfile?{sourceRevision:root.sourceRevision,source:clone(root.source)}:{})};','const retainedRoot={document:clone(root.document),sourceRevision:root.sourceRevision,source:clone(root.source)};'],
 ['unrequested-source-units-charged','(sourceProfile?root.source.length:0)+(annotations?annotations.rows.length:0)','root.source.length+(annotations?annotations.rows.length:0)'],
 ['source-byte-cap-applied-to-annotation-only',"request.data.includes('source')&&root.source.length>limits.sourceBytes",'root.source.length>limits.sourceBytes']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/read-class-isolation-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'abstract-read-class-isolation-sensitivity',baseline:JSON.parse(positive.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,nativeVersionSetCaptureOrPhysicalIsolationProved:false}));
