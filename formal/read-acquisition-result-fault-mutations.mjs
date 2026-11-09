import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),single=l=>l.startsWith(' if(!singleReadAcquisitionResultValid(reply,'),multiple=l=>l.startsWith(' if(!versionSetAcquisitionResultValid(reply,'),current=l=>l.includes("selection:{kind:'capture-current'}"),rules=[
 ['source-acquisition-guard-omitted','native-source-contexts.mjs',single,null],
 ['annotation-current-acquisition-guard-omitted','native-annotation-contexts.mjs',l=>single(l)&&current(l),null],
 ['annotation-retained-acquisition-guard-omitted','native-annotation-contexts.mjs',l=>single(l)&&!current(l),null],
 ['coherent-current-acquisition-guard-omitted','native-version-set-contexts.mjs',l=>multiple(l)&&current(l),null],
 ['coherent-retained-acquisition-guard-omitted','native-version-set-contexts.mjs',l=>multiple(l)&&!current(l),null],
 ['coherent-current-root-meaning-ignored','native-version-set-contexts.mjs',l=>multiple(l)&&current(l),'meaning'],
 ['coherent-retained-root-meaning-ignored','native-version-set-contexts.mjs',l=>multiple(l)&&!current(l),'meaning'],
 ['northbound-acquisition-result-guard-omitted','invocation-publication.mjs',l=>l.startsWith('function checkedReadAcquisitionReply('),'facade']
];
const run=d=>spawnSync(process.execPath,[path.join(d,'formal/read-acquisition-result-fault-checks.mjs')],{encoding:'utf8',maxBuffer:2097152,env:process.env}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const[name,relative,select,kind]of rules){const graph=captureInputs(root),file=path.join(graph.directory,'formal',relative),raw=fs.readFileSync(file,'utf8'),matches=raw.split('\n').filter(select);assert.equal(matches.length,1,name);const from=matches[0];let to=' if(false){owner.faulted=true;return no();}';
 if(kind==='meaning'){const expected=current(from)?"selection:{kind:'capture-current'}":"selection:request.selection";to=" if(!versionSetAcquisitionResultValid(reply,{contract:'read-version-set/1',participants:acquired.result.participants,"+expected+"},context)||reply.participants.some((row,i)=>row.snapshot!==pending[i+1].selector)){owner.faulted=true;return no();}";}
 else if(kind==='facade')to=from.replace('!readAcquisitionResultValid(result,request)','false');
 fs.writeFileSync(file,raw.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1100));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1100));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-read-acquisition-result-fault-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,postReleaseCrashPhysicalOrIndependentProof:false}));
