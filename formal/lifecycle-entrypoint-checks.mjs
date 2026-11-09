// Initialization regression evidence, separate from behavioral mutation detection.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const here=path.dirname(fileURLToPath(import.meta.url)),root=path.dirname(here);
const seen=new Set();
function visit(name){
 if(seen.has(name))return;seen.add(name);
 for(const match of fs.readFileSync(path.join(here,name),'utf8').matchAll(/(?:from\s*|import\s*)['"]\.\/([^'"]+\.mjs)['"]/g))visit(match[1]);
}
for(const name of ['engine-facade.mjs','invocation-publication.mjs','semantic-settings.mjs','read-search-composition.mjs','native-discovery.mjs','discovery-arguments.mjs','discovery-results.mjs','source-load-request.mjs','ordinary-save-request.mjs','native-operation-observation.mjs','observation-admission.mjs','observation-stop-facade.mjs'])visit(name);
assert.equal(seen.size,142);
const probe=(directory,names)=>spawnSync(process.execPath,['--input-type=module','-e',
 'for(const name of '+JSON.stringify(names)+')await import(new URL("./formal/"+name,import.meta.url));console.log("entrypoints-initialized");'],
 {cwd:directory,encoding:'utf8',timeout:30000,maxBuffer:1024*1024});
function success(result){
 assert.equal(result.error,undefined);assert.equal(result.signal,null);
 assert.equal(result.status,0,result.stderr);assert.equal(result.stderr,'');
 assert.equal(result.stdout.trim(),'entrypoints-initialized');
}
for(const name of [...seen].toSorted())success(probe(root,[name]));
const names=[...seen].toSorted();success(probe(root,names));success(probe(root,names.toReversed()));
const source=fs.readFileSync(path.join(here,'lifecycle-arguments.mjs'),'utf8');
const before='function declarations(){const forms=documentLifecycleInputForms(),wrapper=lifecycleInvocationInputForm();';
assert.equal(source.split(before).length-1,1);
const scope=createCaptureScope(),priorTransient=process.env.VELLUM_TRANSIENT_PROOF_CAPTURES,priorScope=process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE;let passed=false;
try{
 process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 // Verifier-generated negative fixture only. Preserve the project source.
 const copy=captureInputs(root).directory;
 // This owned capture is retained as failure evidence if any assertion fails.
 const mutated=source.replace(before,'const forms=documentLifecycleInputForms(),wrapper=lifecycleInvocationInputForm();\nfunction declarations(){');
 fs.writeFileSync(path.join(copy,'formal/lifecycle-arguments.mjs'),mutated);
 const result=probe(copy,['lifecycle-products.mjs']);
 assert.equal(result.error,undefined);assert.equal(result.signal,null);assert.equal(result.status,1);
 assert.match(result.stderr,/ReferenceError: Cannot access 'catalog' before initialization/);
 assert(!result.stdout.includes('entrypoints-initialized'));
 // Do not count the intentional module-construction error as a semantic mutant.
 passed=true;
}finally{
 if(priorTransient===undefined)delete process.env.VELLUM_TRANSIENT_PROOF_CAPTURES;else process.env.VELLUM_TRANSIENT_PROOF_CAPTURES=priorTransient;
 if(priorScope===undefined)delete process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE;else process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=priorScope;
 assert.equal(releaseCaptureScope(scope,passed),passed);
}
console.log(JSON.stringify({scope:'selected-native-lifecycle-entrypoint-initialization',
 isolatedEntries:seen.size,orderedImports:2,knownInitializationNegativeControls:1,
 constructionFailuresCountedAsSemanticMutants:false,publicBehaviorChanged:false,
 arbitraryDynamicImportsOrProductionInitializationProved:false}));
