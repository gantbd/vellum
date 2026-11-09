import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),value='formal/invocation-observation-results.mjs',native='formal/invocation-observation.mjs',rules=[
 ['extra-result-fields-admitted',value,'own.length===keys.length&&','own.length>=keys.length&&'],
 ['pending-outcome-admitted',value,"v.phase==='terminal'&&nativeObservationOutcomeValid(v.outcome)",'nativeObservationOutcomeValid(v.outcome)'],
 ['minimal-control-outcome-admitted',value,"operation==='status'&&v.phase==='terminal'","v.phase==='terminal'"],
 ['phase-vocabulary-ignored',value,"!vocabulary.phases.includes(v.phase)",'false'],
 ['observation-identity-ignored',value,'v.observation!==observation','false'],
 ['wait-identity-ignored',value,'v.wait===wait','true'],
 ['outcome-used-for-status',value,"operation==='retained-outcome'&&object(v.observation)","['status','retained-outcome'].includes(operation)&&object(v.observation)"],
 ['outcome-observation-vocabulary-ignored',value,"v.observation!=='available'",'false'],
 ['publication-vocabulary-ignored',value,"!vocabulary.publication.includes(v.publication)",'false'],
 ['control-vocabulary-ignored',value,"!vocabulary.control.includes(v.control)",'false'],
 ['effect-vocabulary-ignored',value,"!vocabulary.effect.includes(v.effect)",'false'],
 ['contradictory-publication-control-admitted',value,"v.publication==='applied'&&v.control==='rejected'",'false'],
 ['reason-vocabulary-ignored',value,'!reasons.includes(v.reasons[i])','false'],
 ['reason-order-ignored',value,'i>0&&reasons.indexOf(v.reasons[i-1])>=reasons.indexOf(v.reasons[i])','false'],
 ['empty-reasons-admitted',value,'v.reasons.length<1','false'],
 ['participant-bound-ignored',value,'dense(v.participants,bounds.participants)','dense(v.participants,bounds.participants+1)'],
 ['participant-type-ignored',value,"v.participants.every(x=>typeof x==='string')",'true'],
 ['count-does-not-match-projection',value,'v.count===v.participants.length','v.count>=0'],
 ['correlation-not-text',value,"typeof v.correlation==='string'",'true'],
 ['recovery-vocabulary-ignored',value,'v.recovery.every(x=>recovery.includes(x))','true'],
 ['context-fault-does-not-fence',native,'e.faulted=true;closeContext(e);','e.faulted=true;'],
 ['fault-refunds-spent-issuance',native,'e.faulted=true;closeContext(e);','e.issued=0;e.controlIssued=0;e.faulted=true;closeContext(e);'],
 ['fault-refunds-held-charge',native,'e.faulted=true;closeContext(e);','e.charge=0;e.faulted=true;closeContext(e);'],
 ['fault-discards-undrained-waiters',native,'e.faulted=true;closeContext(e);','e.waiters.clear();e.faulted=true;closeContext(e);'],
 ['stage-invariant-bypassed',native,'if(!invocationObservationResultValid(frame,operation,token))','if(false)'],
 ['start-wait-invariant-bypassed',native,"),result={status:'waiting',wait};if(!invocationObservationResultValid(result,'bounded-observation-wait',null,wait))","),result={status:'waiting',wait};if(false)"],
 ['continuing-wait-invariant-bypassed',native,"const waiting=()=>{const result={status:'waiting',wait};if(!invocationObservationResultValid(result,'bounded-observation-wait',null,wait))","const waiting=()=>{const result={status:'waiting',wait};if(false)"],
 ['delivery-preinstall-invariant-bypassed',native,"if(!invocationObservationResultValid(result,'bounded-observation-wait',s.token,wait))",'if(false)'],
 ['completion-invariant-bypassed',native,"if(!invocationObservationResultValid(result,'bounded-observation-wait',result.result?.observation,wait))",'if(false)']
];
for(const[name,file,from]of rules)assert.equal(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1,1,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/invocation-observation-result-corpus.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576,timeout:50000}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);assert(!baseline.error&&!baseline.signal);const mutations=[];
 for(const[name,file,from,to]of rules){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1300));assert(!out.error&&!out.signal,name+': process failure');assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1300));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('construction target'),name+': construction/tool failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'isolated-native-F06-result-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,globalPhysicalWireOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
