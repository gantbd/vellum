import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-annotation-contexts.mjs',rules=[
 ['parent-charge-omitted','owner.charge+=charge;','void charge;'],
 ['actual-parent-headroom-ignored','charge>owner.limits.maxHeldBytes-owner.charge','false'],
 ['context-limit-ignored','m.contexts.size>=p.maxContexts','false'],
 ['selector-limit-ignored','m.selectors.size>=p.maxSelectors','false'],
 ['issuance-limit-ignored','m.issued>=p.maxIssued','false'],
 ['reply-envelope-bound-ignored','!copy(reply,p,p.maxReplyBytes)','false'],
 ['context-selector-bound-ignored','!copy(context,p,p.maxSelectorBytes)','false'],
 ['selector-record-bound-ignored','!copy(record,p,p.maxSelectorRecordBytes)','false'],
 ['combined-source-profile-probe-ignored',"if(probe.status!=='content')return no();",''],
 ['context-row-not-installed','candidateContexts.set(context,{scope,value,annotations,records:new Map(),continuations:new Map(),holds:0,stopped:false});',''],
 ['selector-row-not-installed','candidateSelectors.set(context,record);',''],
 ['context-issuance-not-spent','m.issued++;return reply;','return reply;'],
 ['active-context-can-retire','!ctx?.stopped','!ctx'],
 ['context-retirement-keeps-selector','if(row.context===context)m.selectors.delete(key);','if(false)m.selectors.delete(key);'],
 ['pool-charge-not-refunded','owner.charge-=m.charge;','void m.charge;'],
 ['closed-context-pool-can-reopen','pools.has(owner)','pools.get(owner)&&!pools.get(owner).closed'],
 ['stop-can-cross-scope','if(!ctx||!scope||!equal(scope,ctx.scope))return no();','if(!ctx||!scope)return no();'],
 ['cut-not-consumed','const consumed=consumeNativeAnnotationCapture(owner,domain,capturePool,cut,c,g);','const consumed={};']
];
const run=directory=>{const checks=pathToFileURL(path.join(directory,'formal/native-annotation-installation-checks.mjs')).href,bounds=pathToFileURL(path.join(directory,'formal/native-annotation-installation-boundaries.mjs')).href,code='import {checkNativeAnnotationInstallation as check} from '+JSON.stringify(checks)+';import {checkAnnotationInstallationBoundaries as bounds} from '+JSON.stringify(bounds)+';console.log(JSON.stringify({baseline:check(),boundaries:bounds()}));';return spawnSync(process.execPath,['--input-type=module','-e',code],{encoding:'utf8',env:process.env,maxBuffer:1048576});};
const graph=captureInputs(root),positive=run(graph.directory);assert.equal(positive.status,0,positive.stderr);const killed=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8'),a=original.indexOf('export function installNativeAnnotationContext('),b=original.indexOf('export function acquireNativeRetainedAnnotationContext(',a);assert(a>=0&&b>a);const region=original.slice(a,b);let mutated;if(region.includes(from)){assert.equal(region.split(from).length-1,1,name);mutated=original.slice(0,a)+region.replace(from,to)+original.slice(b);}else{assert.equal(original.split(from).length-1,1,name);mutated=original.replace(from,to);}fs.writeFileSync(file,mutated);const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);killed.push(name);}
console.log(JSON.stringify({scope:'isolated-native-annotation-installation-sensitivity',...JSON.parse(positive.stdout),mutations:killed,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,queryProductsOrPhysicalRefinementProved:false}));
