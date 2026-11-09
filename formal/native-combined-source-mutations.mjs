import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-contexts.mjs',rules=[
 ['current-root-refreshes-captured-source','const xs=source.bytes,','const xs=owner.root.documents.find(d=>equal(d.document,ctx.scope.document)).source,'],
 ['source-context-compared-by-shape','ctx=r&&m.contexts.get(r.snapshot),source=ctx?.value.source','ctx=r&&[...m.contexts].find(([key])=>equal(key,r.snapshot))?.[1],source=ctx?.value.source'],
 ['source-shape-validation-after-loss',"if(r.operation==='bounded-range'?!(boundary(r.lo)&&boundary(r.hi)&&r.lo<=r.hi):!(natural(r.line)&&r.line<logicalLines(xs).length))return no();",''],
 ['combined-annotation-authority-ignored',"'annotationReadable','projectionCurrent','retained'","'projectionCurrent','retained'"],
 ['unknown-source-grant-invalidates','source&&g?.sourceReadable===false','source&&g?.sourceReadable!==true'],
 ['captured-source-work-ceiling-widened','},r,{grant:true,scope:true,continuity:true,capacity:true},source.profile);','},r,{grant:true,scope:true,continuity:true,capacity:true},{...source.profile,maxContentBytes:source.bytes.length});'],
 ['outer-reply-capacity-ignored','return copy(out,m.profile,m.profile.maxReplyBytes)?Object.freeze(out):no();','return Object.freeze(out);']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-combined-source-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576});
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
console.log(JSON.stringify({scope:'isolated-native-combined-source-sensitivity',baseline:JSON.parse(positive.stdout),mutations:killed,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,F05LifetimePhysicalOrPublicWireProved:false}));
