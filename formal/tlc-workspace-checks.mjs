// Selected pinned-tool concurrency regression, not engine or host isolation.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import {spawn} from 'node:child_process';
import {sliceCases} from './slices.mjs';
import {evaluateToolResult} from './evidence-harness.mjs';
const jar=process.env.VELLUM_TLA_JAR;if(!jar)throw Error('TLC_WORKSPACE_TOOL_FAILURE: configure pinned jar');
assert.equal(crypto.createHash('sha256').update(fs.readFileSync(jar)).digest('hex'),'936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88');
const source=fs.readFileSync(new URL('./check.mjs',import.meta.url),'utf8'),marker="'-Djava.rmi.server.hostname=127.0.0.1',`-Djava.io.tmpdir=${work}`,'-cp',jar,'tlc2.TLC'";
const guarded=s=>assert.equal(s.split(marker).length-1,2,'both TLC launch sites need their owned work directory');guarded(source);let sourceGuardControls=0;
for(const changed of [source.replace(marker,marker.replace('`-Djava.io.tmpdir=${work}`,','')),source.replaceAll(marker,marker.replace('`-Djava.io.tmpdir=${work}`,','')),source.replace(marker,marker.replace('${work}','/shared/tmp'))]){assert.throws(()=>guarded(changed),{code:'ERR_ASSERTION'});sourceGuardControls++;}
const c=sliceCases.find(c=>c.id==='NoOpPublication-CapacityNoopRejected');assert(c&&c.expected==='witness');
const directories=Array.from({length:2},()=>fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'vellum-tlc-workspace-'))));assert.notEqual(...directories);
function run(directory){
 for(const file of ['NoOpPublication.tla','EditorContracts.tla'])fs.copyFileSync(new URL('./'+file,import.meta.url),path.join(directory,file));
 const constants=Object.entries(c.constants).map(([k,v])=>`  ${k} = ${typeof v==='boolean'?String(v).toUpperCase():v}`).join('\n');fs.writeFileSync(path.join(directory,'probe.cfg'),`SPECIFICATION Spec\nCHECK_DEADLOCK FALSE\nCONSTANTS\n${constants}\nINVARIANTS\n${c.invariants.join('\n')}\nPROPERTIES\n${c.properties.join('\n')}\n`);
 return new Promise((resolve,reject)=>{const child=spawn('java',['-Xmx128m','-XX:+UseParallelGC','-Djava.rmi.server.hostname=127.0.0.1',`-Djava.io.tmpdir=${directory}`,'-cp',jar,'tlc2.TLC','-workers','1','-fp','0','-metadir',path.join(directory,'states'),'-config','probe.cfg','NoOpPublication.tla'],{cwd:directory,stdio:['ignore','pipe','pipe']}),parts={stdout:[],stderr:[]};let timer=setTimeout(()=>{child.kill();reject(Error('TLC_WORKSPACE_TOOL_FAILURE: timeout'));},30000);child.stdout.on('data',v=>parts.stdout.push(v));child.stderr.on('data',v=>parts.stderr.push(v));child.on('error',e=>{clearTimeout(timer);reject(e);});child.on('close',(status,signal)=>{clearTimeout(timer);resolve({directory,status,signal,stdout:Buffer.concat(parts.stdout).toString('utf8'),stderr:Buffer.concat(parts.stderr).toString('utf8')});});});
}
let passed=false;
try{const results=await Promise.all(directories.map(run));for(const result of results){const accepted=evaluateToolResult(result,c.expected,c.violation);if(result.signal||result.status===null)throw Error('TLC_WORKSPACE_TOOL_FAILURE');assert.equal(accepted.passed,true,accepted.output);const paths=[...accepted.output.matchAll(/^Parsing file (.+(?:Naturals|Sequences)\.tla)$/gm)].map(m=>m[1]);assert(paths.length>=2);assert(paths.every(p=>path.dirname(p)===result.directory));assert(paths.some(p=>path.basename(p)==='Sequences.tla'));assert(!accepted.output.includes('Cannot find source file'));}
 passed=true;console.log(JSON.stringify({scope:'selected-pinned-TLC-owned-temporary-workspace',concurrentProcesses:results.length,sourceGuardControls,bothLaunchSitesGuarded:true,standardModulesResolvedInsideDistinctOwnedDirectories:true,expectedWitnessAccepted:true,failedPriorReportReclassified:false,arbitraryToolHostIsolationOrEngineProof:false}));
}finally{if(passed)for(const directory of directories)fs.rmSync(directory,{recursive:true});}
