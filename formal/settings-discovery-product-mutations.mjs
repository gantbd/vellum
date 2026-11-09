import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),file='formal/settings-query-descriptor.mjs',rules=[
 ['record-fields-not-closed','additionalProperties:false','additionalProperties:true'],
 ['record-fields-optional','required:[...fields]','required:[]'],
 ['integer-floor-unbounded','minimum:Number.MIN_SAFE_INTEGER','minimum:Number.MIN_SAFE_INTEGER-1'],
 ['integer-ceiling-unbounded','maximum:Number.MAX_SAFE_INTEGER','maximum:Number.MAX_SAFE_INTEGER+1'],
 ['enum-allows-empty',"values:{type:'array',minItems:1","values:{type:'array',minItems:0"],
 ['enum-duplicates-allowed',"values:{type:'array',minItems:1,uniqueItems:true","values:{type:'array',minItems:1,uniqueItems:false"],
 ['list-child-unconstrained','element:ref','element:{}'],
 ['map-child-unconstrained','additionalProperties:ref','additionalProperties:{}'],
 ['map-empty-field-name-allowed',"fields:{type:'object',propertyNames:text","fields:{type:'object',propertyNames:{type:'string'}"],
 ['scope-duplicates-allowed',"scopes:{type:'array',minItems:1,uniqueItems:true","scopes:{type:'array',minItems:1,uniqueItems:false"],
 ['scope-allows-empty',"scopes:{type:'array',minItems:1","scopes:{type:'array',minItems:0"],
 ['merge-map-type-unconstrained',"kind:{const:'map'}","kind:{type:'string'}"],
 ['capability-type-not-checked',"type:{$ref:'#/$defs/settingType'}","type:{}"],
 ['unavailable-adds-private-reason',"record(d.forms.unavailable,{status:{const:'unavailable'}})","record([...d.forms.unavailable,'reason'],{status:{const:'unavailable'},reason:{type:'string'}})"],
 ['context-shadow-not-object','context:opaque,target:opaque,key:text,value:{}','context:{},target:opaque,key:text,value:{}'],
 ['events-overclaimed',"events:'no-asynchronous-events-in-this-synchronous-profile'","events:'public-events-complete'"],
 ['coverage-overclaimed',"coverage:'native-value-structure-only'","coverage:'runtime-authority-proved'"],
 ['local-reference-replaced-by-remote',"const ref={$ref:'#/$defs/settingType'}","const ref={$ref:'https://not-loaded.invalid/type'}"],
 ['type-kind-partition-widened',"kind:{const:kind}","kind:{type:'string'}"],
 ['input-recognizer-bypassed','expected!==null&&equal(value,expected);}\nexport function settingsQueryOutcomesValid','expected!==null;}\nexport function settingsQueryOutcomesValid'],
 ['outcome-recognizer-bypassed','return expected!==null&&equal(value,expected);}\nexport function settingsQueryEngineDescriptor','return expected!==null;}\nexport function settingsQueryEngineDescriptor']
],bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);assert(Number.isInteger(bank)&&bank>=0&&bank<2);
for(const[name,from]of rules)assert.equal(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1,1,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/settings-discovery-product-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,from,to]of rules.filter((_,i)=>i%2===bank)){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));const out=run(graph.directory);assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1400));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1400));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('construction target'),name+': construction failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'guard-bound-F07-discovery-product-sensitivity',bank,bankCount:2,totalMutations:rules.length,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allProductsHostPhysicalOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
