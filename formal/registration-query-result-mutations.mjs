import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),shape='formal/registration-query-results.mjs',native='formal/registration-reads.mjs',rules=[
 ['extra-result-fields',shape,'!exact(value,declarations.forms[status.value])','false',1],
 ['context-binding-lost',shape,'value.context!==context','false',1],
 ['coverage-widened',shape,"value.coverage!=='captured-permitted'",'false',1],
 ['entry-fields-not-closed',shape,"!exact(v,[...declarations.entryFields,...(described?['descriptor']:[])])",'false',1],
 ['inspect-entry-binding-lost',shape,'value.entry.reference===request?.entry','true',1],
 ['write-procedure-not-validated',shape,'text(d.procedure)','true',1],
 ['write-capabilities-not-boolean',shape,"declarations.writeFields.filter(k=>k!=='procedure').every(k=>typeof d[k]==='boolean')",'true',1],
 ['read-descriptor-extra-fields',shape,'!exact(d,declarations.readFields[profile.value])','false',1],
 ['read-capability-not-boolean',shape,"typeof d.atomicRead!=='boolean'",'false',1],
 ['line-break-vocabulary-ignored',shape,'declarations.insertionLineBreaks.includes(d.insertionLineBreak)','true',1],
 ['sparse-page-admitted',shape,'dense(value.entries)','Array.isArray(value.entries)',1],
 ['duplicate-page-reference-admitted',shape,'new Set(value.entries.map(v=>v.reference)).size===value.entries.length','true',1],
 ['page-limit-ignored',shape,'value.entries.length<=request.limit','true',1],
 ['page-row-count-unbound',shape,'value.entries.length!==end-start','false',1],
 ['page-table-unbound',shape,'row.table===rows[index]?.value.table','true',1],
 ['page-original-reference-unbound',shape,'row.reference===reference','true',1],
 ['entry-selector-kind-unbound',shape,"s?.kind==='entry'",'true',1],
 ['selector-context-unbound',shape,'s.context===context','true',2],
 ['entry-selector-index-unbound',shape,'s.index===index','true',1],
 ['cursor-original-reference-unbound',shape,'value.continuation!==continuation','false',1],
 ['cursor-limit-unbound',shape,'s.limit===limit','true',1],
 ['cursor-start-unbound',shape,'s.start===end','true',1],
 ['native-inspect-meaning-unbound',native,'equal(value,ctx.rows[s.index]?.value)','true',1],
 ['native-preinstall-page-check-omitted',native,"if(!queryResultBound(m,ctx,request,reply,entries,continuations,selectors))throw Error('REGISTRATION_QUERY_RESULT_INVARIANT');",'',1],
 ['native-inspect-preflight-check-omitted',native,"if(!queryResultBound(m,ctx,request,reply))throw Error('REGISTRATION_QUERY_RESULT_INVARIANT');return size(reply)",'return size(reply)',1],
 ['native-late-release-check-omitted',native,"if(!queryResultBound(m,ctx,request,reply))throw Error('REGISTRATION_QUERY_RESULT_INVARIANT');return reply;",'return reply;',1],
 ['fault-does-not-invalidate',native,'catch{closeContext(m,request.context,ctx,true);return no();}','catch{return no();}',1],
 ['fault-drops-retained-context',native,'catch{closeContext(m,request.context,ctx,true);return no();}','catch{closeContext(m,request.context,ctx,true);m.contexts.delete(request.context);return no();}',1],
 ['fault-refunds-spent-issuance',native,'catch{closeContext(m,request.context,ctx,true);return no();}','catch{closeContext(m,request.context,ctx,true);m.issued=0;return no();}',1],
 ['fault-consumes-uncertain-cut',native,'catch{closeContext(m,request.context,ctx,true);return no();}','catch{closeContext(m,request.context,ctx,true);m.packets.delete(packet);return no();}',1],
 ['ordinary-inspect-pressure-invalidates',native,'return size(reply)<=m.limits.maxReplyBytes?reply:no();',"if(size(reply)>m.limits.maxReplyBytes){closeContext(m,request.context,ctx,true);return no();}return reply;",1],
 ['retirement-skips-drain',native,"!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])||[...m.packets.values()]","!gates(g,['ownerControlAllowed','retentionReleased'])||[...m.packets.values()]",1],
 ['retirement-refunds-issuance',native,"m.contexts.delete(context);return {status:'retired'};","m.contexts.delete(context);m.issued=0;return {status:'retired'};",1]
],bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);assert(Number.isInteger(bank)&&bank>=0&&bank<3);
for(const[name,file,from,,count]of rules)assert.equal(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1,count,name+': construction target');
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/registration-query-result-corpus.mjs')],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:60000}),baseline=run(captureInputs(root).directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
 for(const[name,file,from,to]of rules.filter((_,i)=>i%3===bank)){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replaceAll(from,to));const out=run(graph.directory);assert(!out.error&&!out.signal,name+': process/tool failure');assert.equal(out.status,1,name+': '+out.stderr.slice(0,1400));assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1400));assert(!out.stderr.includes('SyntaxError')&&!out.stderr.includes('construction target'),name+': construction failure');mutations.push(name);}
 passed=true;console.log(JSON.stringify({scope:'native-F07-registration-query-result-and-fault-sensitivity',bank,bankCount:3,totalMutations:rules.length,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,completePublicPhysicalOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
