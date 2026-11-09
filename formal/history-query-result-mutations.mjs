import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),shape='formal/history-query-results.mjs',native='formal/history-contexts.mjs',rules=[
 ['extra-result-fields-admitted',shape,'own.length===keys.length','true'],
 ['result-accessors-admitted',shape,"Object.hasOwn(d[k],'value')",'true'],
 ['result-prototype-unrestricted',shape,'![Object.prototype,null].includes(Object.getPrototypeOf(v))','false'],
 ['status-value-coerced',shape,"typeof status!=='string'||",''],
 ['context-identity-unchecked',shape,'out.context!==request.context','false'],
 ['frontier-operation-unchecked',shape,"request.operation==='frontier'",'true'],
 ['frontier-group-domain-unchecked',shape,'out.group===null||label(out.group)','true'],
 ['availability-operation-unchecked',shape,"request.operation==='availability'",'true'],
 ['availability-vocabulary-unchecked',shape,'values.includes(out.value)','true'],
 ['detail-operation-unchecked',shape,"request.operation!=='permitted-blocking-detail'",'false'],
 ['detail-capacity-ignored',shape,'dense(out.blockers,maxDetails)','dense(out.blockers,Number.MAX_SAFE_INTEGER)'],
 ['invalid-capacity-admitted',shape,'!natural(maxDetails)||',''],
 ['detail-array-extra-fields-admitted',shape,'own.length!==a.length+1','false'],
 ['detail-array-accessors-admitted',shape,"!Object.hasOwn(d[i],'value')",'false'],
 ['detail-array-prototype-unrestricted',shape,'Object.getPrototypeOf(a)!==Array.prototype','false'],
 ['empty-alias-admitted',shape,'s.length>0&&',''],
 ['malformed-unicode-alias-admitted',shape,"&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Buffer.from(s))===s",''],
 ['detail-order-and-uniqueness-ignored',shape,'i>0&&out.blockers[i-1].scope>=row.scope','false'],
 ['frontier-projection-alias-unchecked',shape,'binding.groups.some(g=>g.reference===out.group)','true'],
 ['detail-projection-scope-unchecked',shape,'binding.scopes.some(s=>s.reference===r.scope)','true'],
 ['detail-projection-group-unchecked',shape,'binding.groups.some(g=>g.reference===r.group)','true'],
 ['detail-disclosure-ceiling-unchecked',shape,'binding.details===true','true'],
 ['lower-original-binding-check-omitted',native,'!historyQueryResultValid(out,privateRequest,o.limits.maxDetails)','false'],
 ['copied-context-check-omitted',native,'!historyQueryResultValid(copied,request,o.limits.maxDetails)','false'],
 ['copied-value-frame-check-omitted',native,'!equal(copied,result)','false'],
 ['invariant-fault-does-not-fence',native,'catch{o.faulted=true;return unavailable();}\n}','catch{return unavailable();}\n}'],
 ['faulted-owner-allows-new-context',native,'o.stopped||o.faulted||!equal(o.context,c)','o.stopped||!equal(o.context,c)'],
 ['faulted-owner-still-resolves-aliases',native,'o.stopped||o.faulted||!e?.live','o.stopped||!e?.live'],
 ['fault-drops-retained-obligations',native,'catch{o.faulted=true;return unavailable();}\n}','catch{o.entries.clear();o.faulted=true;return unavailable();}\n}'],
 ['fault-refunds-lifetime-issuance',native,'catch{o.faulted=true;return unavailable();}\n}','catch{o.issued=0;o.faulted=true;return unavailable();}\n}'],
 ['fault-prevents-protected-retirement',native,'export function retireHistoryInspection(owner,context,g){const o=owners.get(owner);','export function retireHistoryInspection(owner,context,g){const o=owners.get(owner);if(o?.faulted)return unavailable();']
];
for(const[name,file,from]of rules)assert.equal(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1,1,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;const run=(directory,file)=>spawnSync(process.execPath,[path.join(directory,'formal',file)],{encoding:'utf8',env:process.env,maxBuffer:2097152}),positive=captureInputs(root),baseline=run(positive.directory,'history-query-result-checks.mjs'),faults=run(positive.directory,'history-query-result-faults.mjs');assert.equal(baseline.status,0,baseline.stderr);assert.equal(faults.status,0,faults.stderr);const mutations=[];
 for(const[name,file,from,to]of rules){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));let detected=false;
  for(const checker of ['history-query-result-checks.mjs','history-query-result-faults.mjs']){const out=run(graph.directory,checker);assert([0,1].includes(out.status),name+': unexpected process disposition');assert(!out.stderr.includes('SyntaxError'),name);if(out.status===1){assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1200));detected=true;}else{const r=JSON.parse(out.stdout);assert.equal(r.scope,checker.includes('faults')?'fixed-native-F04-history-result-invariant-probes':'closed-native-F04-history-query-result-values');}}
  assert(detected,name+': survived complete result/fault corpus');mutations.push(name);
 }
 passed=true;console.log(JSON.stringify({scope:'isolated-native-F04-history-result-sensitivity',baseline:JSON.parse(baseline.stdout),faults:JSON.parse(faults.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allResultSemanticsGatewayPhysicalOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
