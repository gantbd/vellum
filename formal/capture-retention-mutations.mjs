import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),rules=[
 ['standalone-success-cleanup-omitted','evidence-harness.mjs','if(transient&&!scope)trackTransientCapture(directory);','if(false)trackTransientCapture(directory);'],
 ['unexpected-failure-artifacts-reclaimed','evidence-harness.mjs','if(code!==0)return;','if(false)return;'],
 ['explicit-retention-ignored','evidence-harness.mjs',"process.env.VELLUM_TRANSIENT_PROOF_CAPTURES==='1'&&retain!==true","process.env.VELLUM_TRANSIENT_PROOF_CAPTURES==='1'"],
 ['full-worker-snapshot-made-disposable','evidence-harness.mjs','const captured=captureInputs(root,{retain:true});','const captured=captureInputs(root);'],
 ['child-captures-escape-owning-scope','evidence-harness.mjs',"path.join(scope||os.tmpdir(),'vellum-proof-inputs-')","path.join(os.tmpdir(),'vellum-proof-inputs-')"],
 ['failed-checker-scope-reclaimed','evidence-harness.mjs','if(!record||passed!==true)return false;','if(!record)return false;'],
 ['directory-identity-guard-omitted','evidence-harness.mjs',"if(!stat.isDirectory()||stat.isSymbolicLink()||stat.dev!==record.device||stat.ino!==record.inode||fs.realpathSync(record.directory)!==record.directory)",'if(false)'],
 ['cleanup-failure-does-not-demote-pass','check.mjs',"catch(error){passed=false;output+='\\nCAPTURE_CLEANUP_FAILURE: '","catch(error){output+='\\nCAPTURE_CLEANUP_FAILURE: '"],
 ['expected-counterexample-scope-retained','check.mjs','releaseCaptureScope(captures,passed);','releaseCaptureScope(captures,passed&&p.status===0);']
];
const scope=createCaptureScope(),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;const run=d=>spawnSync(process.execPath,[path.join(d,'formal/capture-retention-checks.mjs')],{encoding:'utf8',maxBuffer:2097152,env:process.env}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,relative,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,'formal',relative),raw=fs.readFileSync(file,'utf8');assert.equal(raw.split(from).length-1,1,name);fs.writeFileSync(file,raw.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1400));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1400));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'isolated-verifier-capture-retention-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,enginePhysicalGlobalQuotaOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
