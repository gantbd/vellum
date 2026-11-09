import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),file='formal/registration-query-arguments.mjs',rules=[
 ['extra-envelope-accepted',"!exact(e,['route','request'])","!e"],
 ['unknown-route-accepted',"!['typed','dynamic'].includes(e.route)",'false'],
 ['extra-argument-accepted','!exact(args,form.fields)','!args'],
 ['contract-unbound','args.contract!==form.contract','false'],
 ['operation-unbound','args.operation!==form.operation','false'],
 ['profile-unbound','args.profile!==form.profile','false'],
 ['nested-operation-coerced',"typeof r.operation!=='string'||",''],
 ['extra-nested-fields-accepted','!exact(r,form.requests[r.operation])','false'],
 ['zero-limit-accepted','r.limit<1','r.limit<0'],
 ['fractional-limit-accepted','!Number.isSafeInteger(r.limit)','!Number.isFinite(r.limit)'],
 ['continuation-array-admitted','selector!==null&&!object(selector)','selector!==null&&typeof selector!==\'object\''],
 ['entry-array-admitted',':!object(selector)',":(selector===null||typeof selector!=='object')"],
 ['ordinary-accessors-evaluated',"const d=Object.getOwnPropertyDescriptors(value)","const d=Object.getOwnPropertyDescriptors({...value})"],
 ['hidden-members-ignored',"Reflect.ownKeys(d)","Object.keys(value)"],
 ['foreign-prototypes-admitted',"||![Object.prototype,null].includes(Object.getPrototypeOf(value))",''],
 ['maximum-not-intersected','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['original-context-lost','context=args.context','context=structuredClone(args.context)'],
 ['context-meaning-agreement-ignored','a.context===b.context&&',''],
 ['selector-meaning-agreement-ignored','a.selector===b.selector&&',''],
 ['ordinary-meaning-agreement-ignored','&&equal(a.record,b.record)',''],
 ['producer-context-cloned','context:n.context,request:','context:structuredClone(n.context),request:'],
 ['producer-selector-cloned','[n.selectorRole]:n.selector','[n.selectorRole]:structuredClone(n.selector)'],
 ['producer-request-aliased','request:{...r.request,[n.selectorRole]:n.selector}','request:r.request'],
 ['classification-bound-incorrect','export const registrationQueryArgumentClassificationBytes=Buffer.byteLength','export const registrationQueryArgumentClassificationBytes=1+Buffer.byteLength'],
 ['limit-fields-not-exact','exact(p,limitFields)','p'],
 ['captured-limits-mutable','?Object.freeze(p):null','?p:null'],
 ['policy-meaning-not-captured','context:bound.value,limits,opaqueBindings:','context:{...bound.value,policy:\'stable\'},limits,opaqueBindings:']
];
for(const[name,from]of rules)assert.equal(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1,1,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/registration-query-argument-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,from,to]of rules){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));const out=run(graph.directory);assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1400));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1400));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('construction target'),name+': construction failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'isolated-F07-resource-catalog-argument-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,nativeGatewayChargingResultCodecOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
