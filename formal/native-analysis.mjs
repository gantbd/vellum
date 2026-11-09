// Verification-only live child host. No production process/transport is selected.
import fs from 'node:fs';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {isDeepStrictEqual as equal} from 'node:util';
import {submitAnalysisWork,observeAnalysisWork,acceptAnalysisWork,retireAnalysisWork,analysisResultMatches} from './analysis-work.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./native-analysis.json',import.meta.url),'utf8'));
const expected=JSON.parse(fs.readFileSync(new URL('./tree-sitter-reference.json',import.meta.url),'utf8')).environment;
const sessions=new WeakMap(),copy=structuredClone;
const ingress={authentic:true,evidenceAllowed:true,ingressReady:true,quiescenceAttested:true};
function request(s,body){
  if(s.fault||s.closing||s.pending.size>=catalog.maxPending||s.serial>=catalog.maxMessages)throw Error('native session unavailable');
  const serial=++s.serial,message=JSON.stringify({...body,serial})+'\n';if(Buffer.byteLength(message)>catalog.maxMessageBytes)throw Error('native request exceeds verification transport profile');
  return new Promise((resolve,reject)=>{const timer=setTimeout(()=>s.fail(Error('native verification guard expired')),15000);s.pending.set(serial,{resolve,reject,timer});s.child.stdin.write(message,error=>{if(error)s.fail(error);});});
}
export async function newNativeAnalysisSession(owner,provider,sourceFile=new URL('./native-analysis.py',import.meta.url)){
  const python=process.env.VELLUM_TREE_SITTER_PYTHON;if(!python)throw Error('pinned native verification Python required');
  const child=spawn(python,[fileURLToPath(sourceFile),fileURLToPath(new URL('./',import.meta.url))],{stdio:['pipe','pipe','pipe']}),s={owner,provider:copy(provider),child,records:new Map(),snapshots:new WeakMap(),pending:new Map(),serial:0,issued:0,buffer:Buffer.alloc(0),stderr:'',fault:null,closing:false,exited:false};
  s.fail=error=>{if(s.fault)return;s.fault=error;for(const p of s.pending.values()){clearTimeout(p.timer);p.reject(error);}s.pending.clear();};
  child.stdout.on('data',chunk=>{
    s.buffer=Buffer.concat([s.buffer,chunk]);if(s.buffer.length>catalog.maxMessageBytes*catalog.maxPending){s.fail(Error('native response buffer overflow'));return;}
    for(let i;(i=s.buffer.indexOf(10))>=0;){const line=s.buffer.subarray(0,i);s.buffer=s.buffer.subarray(i+1);let r;try{if(line.length+1>catalog.maxMessageBytes)throw Error('native response too large');r=JSON.parse(line.toString('utf8'));}catch(e){s.fail(e);return;}
      const p=s.pending.get(r.serial);if(!p){s.fail(Error('uncorrelated native response'));return;}s.pending.delete(r.serial);clearTimeout(p.timer);p.resolve(r);
    }
  });
  child.stderr.on('data',chunk=>{s.stderr=(s.stderr+chunk.toString('utf8')).slice(0,8192);});child.on('error',s.fail);child.stdin.on('error',s.fail);
  s.exit=new Promise(resolve=>child.on('close',(code,signal)=>{s.exited=true;if(!s.closing||code!==0||signal){const e=Error(`native host failed (${code}, ${signal}): ${s.stderr}`);if(code===2&&!signal&&s.stderr.startsWith('REFERENCE_ASSERTION:'))e.code='NATIVE_REFERENCE_ASSERTION';s.fail(e);}resolve({code,signal});}));
  try{const r=await request(s,{operation:'metadata'});if(r.kind!=='metadata'||r.profile!==catalog.profile||!equal(r.metadata,expected))throw Error('native host metadata mismatch');}
  catch(e){child.kill();await s.exit;throw e;}
  const ref=Object.freeze({kind:catalog.profile});sessions.set(ref,s);return ref;
}
export async function executeNativeAnalysis(ref,task,state,g){
  const s=sessions.get(ref);if(!s||s.fault||s.closing||!g?.nativeBindingAllowed||!equal(g.provider,s.provider)||s.records.size>=catalog.maxRecords||s.issued>=catalog.maxIssued||s.pending.size>=catalog.maxPending)return {status:'unavailable'};
  const submitted=submitAnalysisWork(s.owner,task,state,g);if(submitted.status!=='submitted')return submitted;
  const {capture,base}=submitted,baseRecord=base===null?null:s.snapshots.get(base),record={task,handle:null,packet:null,retiring:false,dropping:false,facts:null};s.records.set(task,record);s.issued++;
  const event=(kind,extra={})=>({contract:'analysis-evidence/1',task,origin:copy(capture.origin),provider:copy(capture.provider),kind,...extra});
  let r;
  if(base!==null&&(!baseRecord||s.records.get(baseRecord.task)!==baseRecord||baseRecord.retiring||baseRecord.handle===null))r={kind:'failure',reason:'unsupported'};
  else r=await request(s,{operation:'execute',capture,base:baseRecord?.handle??null});
  let observed;
  if(r.kind==='result'){
    if(!Number.isSafeInteger(r.handle)||r.handle<=0||[...s.records.values()].some(p=>p.handle===r.handle)||!equal(r.packet?.context,capture.context)||!equal(r.packet?.source,capture.input.source))throw Error('native result binding mismatch');
    record.handle=r.handle;record.packet=copy(r.packet);record.facts=copy(r.facts);observed=observeAnalysisWork(s.owner,event('result',{packet:r.packet}),ingress);
  }else if(r.kind==='failure')observed=observeAnalysisWork(s.owner,event('failure',{reason:r.reason}),ingress);
  else throw Error('unexpected native result');
  // This synchronous worker turn has returned; no cancellation/result alone supplies this fact.
  const quiet=observeAnalysisWork(s.owner,event('quiescence'),ingress);
  return {status:r.kind==='result'?(['recorded','duplicate'].includes(observed.status)?'recorded':'unavailable'):'failure',evidence:observed.status,quiescence:quiet.status,facts:copy(record.facts)};
}
export function acceptNativeAnalysis(ref,task,state,g){
  const s=sessions.get(ref),record=s?.records.get(task);
  if(!s||s.fault||!record||record.retiring||record.handle===null||!analysisResultMatches(s.owner,task,record.packet,g))return {status:'unavailable'};
  const accepted=acceptAnalysisWork(s.owner,task,state,g);if(accepted.status==='analysis')s.snapshots.set(accepted.snapshot,record);return accepted;
}
export async function retireNativeAnalysis(ref,task,g){
  const s=sessions.get(ref),record=s?.records.get(task);if(!s||s.fault||!g?.nativeBindingAllowed||!record)return {status:'unavailable'};
  if(record.dropping)return {status:'retirement-pending'};
  if(!record.retiring){const retired=retireAnalysisWork(s.owner,task,g);if(retired.status!=='retired')return retired;record.retiring=true;}
  // Logical retirement fences new borrowers before asynchronous native handle removal.
  // The host record continues to count until acknowledgment; failure never implies reclamation.
  if(record.handle!==null){record.dropping=true;try{const r=await request(s,{operation:'drop',handle:record.handle});if(r.kind!=='dropped')return {status:'retirement-pending'};}finally{record.dropping=false;}}
  s.records.delete(task);return {status:'retired'};
}
export async function closeNativeAnalysis(ref){
  const s=sessions.get(ref);if(!s||s.fault||s.records.size||s.pending.size||s.closing)return {status:'unavailable'};
  const pending=request(s,{operation:'close'});s.closing=true;const r=await pending;if(r.kind!=='closed')return {status:'unavailable'};s.child.stdin.end();const exit=await s.exit;return {status:exit.code===0&&!exit.signal?'closed':'unknown'};
}
// Test/supervisor cleanup only. Killing a process is not result, quiescence or retirement evidence.
export async function abortNativeAnalysis(ref){const s=sessions.get(ref);if(s&&!s.exited){s.child.kill();await s.exit;}}
export function nativeAnalysisUsage(ref){const s=sessions.get(ref);return s?{records:s.records.size,pending:s.pending.size,issued:s.issued,retiring:[...s.records.values()].filter(r=>r.retiring).length,faulted:!!s.fault}:null;}
