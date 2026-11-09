import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),file='formal/observation-admission.mjs',rules=[
 ['bootstrap-capability-truthiness','typeof decoderEnabled!==\'boolean\'||',''],
 ['known-refusal-leaks-workspace','if(!context){releaseOccupancy(o,record);','if(!context){record.bytes-=decoder?.workspace??0;releaseOccupancy(o,record);'],
 ['workspace-not-shared-charged','const total=bytes+(decoder?.workspace??0);','const total=bytes;'],
 ['workspace-underreserved','invocationObservationDecoderFrames.length*p.maxRequestBytes','(invocationObservationDecoderFrames.length-1)*p.maxRequestBytes'],
 ['decoder-capability-implicit','decoderLimits!==null&&!o.decoderEnabled','false'],
 ['inadequate-minimum-control-profile-admitted','if(!normalizeInvocationObservationArguments(envelope,c,p,p.maxRequestBytes))return null;','if(false)return null;'],
 ['input-profile-borrowed','return {limits:p,workspace,busy:false,faulted:false};','return {limits:input,workspace,busy:false,faulted:false};'],
 ['pool-stop-disables-protected-decoding',"if(!r||r.phase!=='active'||!d||d.busy","if(!r||o.stopped||r.phase!=='active'||!d||d.busy"],
 ['quarantined-setup-disables-active-control',"if(!r||r.phase!=='active'||!d||d.busy","if(!r||o.faulted||r.phase!=='active'||!d||d.busy"],
 ['busy-reentry-accepted','!d||d.busy||d.faulted','!d||d.faulted'],
 ['retirement-while-decoding-accepted','!r||r.decoder?.busy||r.phase','!r||r.phase'],
 ['busy-exit-not-protected','finally{d.busy=false;}','finally{}'],
 ['fault-not-recorded','}catch{d.faulted=true;return faultInvocationObservationDecoder(r.context);','}catch{return faultInvocationObservationDecoder(r.context);'],
 ['native-observer-not-fenced','}catch{d.faulted=true;return faultInvocationObservationDecoder(r.context);','}catch{d.faulted=true;return unavailable();'],
 ['fault-refunds-held-workspace','}catch{d.faulted=true;return faultInvocationObservationDecoder(r.context);','}catch{for(const s of scopes)o.state.used[s].bytes-=d.workspace;d.faulted=true;return faultInvocationObservationDecoder(r.context);'],
 ['fault-discards-parent-reservation','}catch{d.faulted=true;return faultInvocationObservationDecoder(r.context);','}catch{o.leases.delete(reservation);d.faulted=true;return faultInvocationObservationDecoder(r.context);'],
 ['retirement-leaks-prepaid-workspace','next.used[s].bytes-=r.bytes;','next.used[s].bytes-=r.bytes-(r.decoder?.workspace??0);'],
 ['current-packet-not-copied','const packet=copySettingsData({policy:p,facts:g},d.limits);','const packet={value:{policy:p,facts:g}};'],
 ['forged-context-repaired','const request=invocationObservationProducerRequest(n);','const request={...invocationObservationProducerRequest(n),context:r.context};'],
 ['protected-query-requires-new-headroom','d.busy=true;',"if(admitCapacity(o.profile,o.state,demand(1)).status!=='admitted')return unavailable();d.busy=true;"],
 ['queries-consume-another-admission-step','d.busy=true;',"const extra=admitCapacity(o.profile,o.state,demand(0,0,1));if(extra.status!=='admitted')return unavailable();o.state=extra.state;d.busy=true;"],
 ['retained-decoder-metadata-not-precharged','...(decoderEnabled?{decoder:{limits:{maxRequestBytes:Number.MAX_SAFE_INTEGER,maxInputNodes:Number.MAX_SAFE_INTEGER,maxInputDepth:Number.MAX_SAFE_INTEGER},workspace:Number.MAX_SAFE_INTEGER,busy:false,faulted:false}}:{})','...(decoderEnabled?{decoder:null}:{})'],
 ['readiness-coerced-to-boolean','packet.value.policy,packet.value.facts);','packet.value.policy,Object.fromEntries(Object.entries(packet.value.facts).map(([k,v])=>[k,Boolean(v)])));']
];
const source=fs.readFileSync(path.join(root,file),'utf8');for(const[name,from]of rules)assert.equal(source.split(from).length-1,1,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/observation-decoder-corpus.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576,timeout:50000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);assert(!baseline.error&&!baseline.signal);const mutations=[];
 for(const[name,from,to]of rules){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1300));assert(!out.error&&!out.signal,name+': process failure');assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1300));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('construction target'),name+': construction/tool failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'isolated-prepaid-F06-decoder-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,postDispatchGlobalPhysicalOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
