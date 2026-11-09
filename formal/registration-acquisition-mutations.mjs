import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),shape='formal/registration-acquisition-results.mjs',native='formal/registration-reads.mjs',rules=[
 ['extra-result-fields',shape,'!exact(value,forms[s.value])','false',1],
 ['original-result-alias-unbound',shape,'value.context===expectedContext','true',1],
 ['coverage-unbound',shape,"value.coverage==='captured-permitted'",'true',1],
 ['result-status-coerced',shape,"typeof s.value!=='string'||",'',1],
 ['native-frame-check-bypassed',native,"if(reply.status!=='context'||!registrationAcquisitionResultValid(reply,context))throw Error('REGISTRATION_ACQUISITION_RESULT_INVARIANT');",'',1],
 ['native-late-frame-check-bypassed',native,'!registrationAcquisitionResultValid(reply,attempt.context)','false',1],
 ['installation-disposition-unbound',native,"(reply.status==='context')!==m.contexts.has(attempt.context)",'false',1],
 ['candidate-scope-unbound',native,'equal(state.scope,attempt.scope)','true',1],
 ['candidate-rows-unbound',native,'equal(state.rows,attempt.rows)','true',1],
 ['candidate-state-extra-fields',native,"exact(state,['scope','rows','entries','continuations','stopped','invalidated'])",'state',1],
 ['candidate-selectors-not-empty',native,'state.entries.size===0','true',1],
 ['candidate-stop-state-ignored',native,'state.stopped===false','true',1],
 ['native-late-state-check-omitted',native,"reply.status==='context'&&!acquisitionStateBound(m.contexts.get(attempt.context),attempt)",'false',1],
 ['registry-not-fenced',native,'catch{m.acquisitionFaulted=true;','catch{',1],
 ['registry-fence-ignored',native,'m.acquisitionFaulted!==true&&invocationUsage(m.domain)?.faulted===false','invocationUsage(m.domain)?.faulted===false',1],
 ['unreturned-context-not-invalidated',native,'closeContext(m,attempt.context,m.contexts.get(attempt.context),true);','',1],
 ['cleanup-receipt-not-retained',native,"m.packets.set(packet,freeze({phase:'failed-acquisition',context:attempt.context,scope:attempt.scope}))",'void packet',1],
 ['fault-refunds-issued-context',native,'catch{m.acquisitionFaulted=true;','catch{m.issued=0;m.acquisitionFaulted=true;',1],
 ['cut-cleanup-skips-delivery-drain',native,"!gates(g,['deliveryDrained'])",'false',1],
 ['cut-cleanup-refunds-issuance',native,'m.contexts.delete(p.context);','m.contexts.delete(p.context);m.issued--;',1],
 ['cut-cleanup-keeps-inaccessible-context',native,'m.contexts.delete(p.context);','void p.context;',1],
 ['context-retirement-bypasses-cleanup-cut',native,"||[...m.packets.values()].some(p=>p.phase==='failed-acquisition'&&p.context===context)",'',1],
 ['context-installation-not-committed',native,'m.contexts.set(context,state);m.issued++;return reply;','m.issued++;return reply;',1],
 ['context-issuance-not-spent',native,'m.contexts.set(context,state);m.issued++;return reply;','m.contexts.set(context,state);return reply;',1]
],bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);assert(Number.isInteger(bank)&&bank>=0&&bank<2);
for(const[name,file,from,,count]of rules)assert.equal(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1,count,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/registration-acquisition-corpus.mjs')],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,file,from,to]of rules.filter((_,i)=>i%2===bank)){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replaceAll(from,to));const out=run(graph.directory);assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1400));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1400));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('construction target'),name+': construction failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'native-resource-catalog-acquisition-result-and-fault-sensitivity',bank,bankCount:2,totalMutations:rules.length,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,completePublicPhysicalOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
