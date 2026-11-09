import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),file='formal/lifecycle-results.mjs',source=fs.readFileSync(path.join(root,file),'utf8');
const rules=[
 ['extra-fields-accepted','keys.length===fields.length&&',''],
 ['foreign-prototypes-accepted','||![Object.prototype,null].includes(Object.getPrototypeOf(v))',''],
 ['accessors-accepted',"&&Object.hasOwn(descriptors[k],'value')",''],
 ['unknown-document-operation-accepted','d.operations.includes(v.operation)','true'],
 ['document-status-open','d.statuses.includes(v.status)','true','first'],
 ['load-status-correlation-lost','v.status===d.activation[v.activation]','true'],
 ['annotation-status-correlation-lost','v.status===phase[v.disposition]','true'],
 ['annotation-phase-distinction-lost','const phase=d.phases[v.phase]',"const phase={...d.phases.restore,...d.phases['adopt-baseline']}"],
 ['relation-kind-open','d.kinds.includes(v.kind)','true'],
 ['relation-status-open','d.statuses.includes(v.status)','true','last'],
 ['result-wrapper-open',"exact(v,['status','result'])",'Boolean(v)'],
 ['unknown-receiver-kind-accepted',"if(!['lifecycle','relation'].includes(kind))return false;",''],
 ['invalid-unicode-label-accepted',"&&Buffer.from(v,'utf8').toString('utf8')===v",''],
 ['invalid-identity-incarnation-accepted','Number.isSafeInteger(v.incarnation)&&v.incarnation>=0',"typeof v.incarnation==='number'"]
];
const constructors=[
 ['lifecycle-invocation-state.mjs','nativeLifecycleReceiptValid','discard(prepared)',"readNativeRetainedLifecycle(prepared.token,domain,before,host)"],
 ['load-invocation-state.mjs','nativeLifecycleReceiptValid','discard(prepared)',"readNativeSourceRead(prepared.token,domain,before,host)"],
 ['annotation-load-invocation-state.mjs','nativeLifecycleReceiptValid','discardNativeAnnotationLoad(token)',"readNativeAnnotationLoad(token,domain,before,owner.context,host)"],
 ['saveas-invocation-state.mjs','nativeLifecycleReceiptValid','discardSourceDestinationTransition(token)',"readSourceDestinationTransition(token,domain,before,host)"],
 ['relation-invocation-state.mjs','nativeRelationReceiptValid','discardNativeRelationLifecycle(token)',"readNativeRelationLifecycle(token,owner,domain,before,host)"]
];
const baselineProgram=`import {checkLifecycleResults} from './formal/lifecycle-result-checks.mjs';try{console.log(JSON.stringify({ok:true,result:checkLifecycleResults()}));}catch(error){if(error.code!=='ERR_ASSERTION')throw error;console.log(JSON.stringify({semanticAssertion:true}));process.exitCode=1;}`;
const faultProgram=`import assert from 'node:assert/strict';import {activationEnvelopeFixture,activationEnvelopeIds} from './formal/lifecycle-activation-envelope-checks.mjs';let cases=0,nativeDiscards=0,producerCommitCalls=0;try{for(const id of activationEnvelopeIds.concat(['create-scratch','attach-view','detach-view','membership','dispose']))for(const route of ['typed','dynamic']){globalThis.__receiptFaultEnabled=false;globalThis.__receiptNativeDiscarded=false;let x,p;if(activationEnvelopeIds.includes(id)){x=activationEnvelopeFixture(id,route);p=x.prepare();}else{const {lifecycleEnvelopeFixture}=await import('./formal/lifecycle-envelope-checks.mjs');const kernel=await import('./formal/invocation-publication.mjs');x=lifecycleEnvelopeFixture(id,route);const plan=x.prepare();p={job:{kind:'lifecycle',plan},route};x.submit=p=>kernel.submitInvocation(x.domain,x.ticket,p.job,p.route,x.controlContext,x.controlPolicy,x.facts());x.run=()=>kernel.runInvocation(x.domain,x.ticket,x.controlContext,x.facts(),x.host);}assert(p&&p.job.plan,id);assert.equal(x.submit(p).status,'accepted');const before=x.root(),usage=x.usage();globalThis.__receiptProducerCommits=0;globalThis.__receiptFaultEnabled=true;assert.equal(x.run().status,'faulted',id);assert.equal(globalThis.__receiptNativeDiscarded,true,id);assert.equal(globalThis.__receiptProducerCommits,0,id);producerCommitCalls+=globalThis.__receiptProducerCommits;assert.equal(x.root(),before);assert.equal(x.usage().faulted,true);assert.equal(x.usage().issued,usage.issued);assert.equal(x.usage().starts,usage.starts+1);assert.equal(x.prepare(),null);assert.equal(x.retire({...x.facts(),retentionReleased:false}).status,'unavailable');assert.equal(x.retire().status,'retired');assert.equal(x.usage().slots,usage.slots-1);cases++;nativeDiscards++;}console.log(JSON.stringify({cases,nativeDiscards,knownTicketRetired:true,rootUnchanged:true,producerCommitCalls}));}catch(error){if(error.code!=='ERR_ASSERTION')throw error;console.log(JSON.stringify({semanticAssertion:true}));process.exitCode=1;}`;
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{
 process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=(directory,program)=>{const out=spawnSync(process.execPath,['--input-type=module','-e',program],{cwd:directory,encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:60000});assert(!out.error&&!out.signal,'process/tool failure');return out;};
 const baselineOut=run(captureInputs(root).directory,baselineProgram);assert.equal(baselineOut.status,0,baselineOut.stderr);const baseline=JSON.parse(baselineOut.stdout);assert.equal(baseline.ok,true);
 const mutations=[];for(const [name,before,after,mode]of rules){
  assert.equal(source.split(before).length-1,mode?2:1,name+': construction target');let changed;if(mode==='last'){const i=source.lastIndexOf(before);changed=source.slice(0,i)+after+source.slice(i+before.length);}else changed=source.replace(before,after);
  const copy=captureInputs(root);fs.writeFileSync(path.join(copy.directory,file),changed);const out=run(copy.directory,baselineProgram);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1000));assert.equal(JSON.parse(out.stdout).semanticAssertion,true,name);mutations.push(name);
 }
 const inject=(directory,broken=null)=>{
  for(const [name,validator,discard,read]of constructors){const target=path.join(directory,'formal',name),original=fs.readFileSync(target,'utf8'),guard='if(!'+validator+'(receipt)){'+discard+";throw Error('native lifecycle receipt protocol fault');}";
   assert.equal(original.split(guard).length-1,1,name);
   const check=broken?.name===name&&broken.kind==='guard'?'false':'!'+validator+'(receipt)',cleanup=broken?.name===name&&broken.kind==='discard'?'':discard+';';
   const replacement="if(globalThis.__receiptFaultEnabled)receipt.extra=true;if("+check+"){"+cleanup+"globalThis.__receiptNativeDiscarded=("+read+"===null);throw Error('native lifecycle receipt protocol fault');}";
   let changed=original.replace(guard,replacement);const commit={'lifecycle-invocation-state.mjs':'consumeNativeRetainedLifecycle(s.token)','load-invocation-state.mjs':'commitNativeSourceRead(s.token)','annotation-load-invocation-state.mjs':'commitNativeAnnotationLoad(s.token,domain,before,owner.context,host)','saveas-invocation-state.mjs':'commitSourceDestinationTransition(s.token)','relation-invocation-state.mjs':'commitNativeRelationLifecycle(s.token)'}[name];assert.equal(changed.split(commit).length-1,1,name);changed=changed.replace(commit,'(globalThis.__receiptProducerCommits=(globalThis.__receiptProducerCommits??0)+1,'+commit+')');fs.writeFileSync(target,changed);
  }
 };
 const baselineCopy=captureInputs(root);inject(baselineCopy.directory);const baselineFault=run(baselineCopy.directory,faultProgram);assert.equal(baselineFault.status,0,baselineFault.stderr);const faults=JSON.parse(baselineFault.stdout);assert.equal(faults.cases,28);assert.equal(faults.nativeDiscards,28);assert.equal(faults.knownTicketRetired,true);assert.equal(faults.producerCommitCalls,0);
 for(const [name]of constructors)for(const kind of ['guard','discard']){const copy=captureInputs(root);inject(copy.directory,{name,kind});const out=run(copy.directory,faultProgram),label=name+':'+kind;assert.equal(out.status,1,label+': '+out.stderr.slice(0,1000));assert.equal(JSON.parse(out.stdout).semanticAssertion,true,label);mutations.push(label);}
 passed=true;console.log(JSON.stringify({scope:'selected-native-lifecycle-result-sensitivity',baseline:baseline.result,mutations,faults,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,newPublicQueryOrWireOrPhysicalOrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const key of keys)if(saved[key]===undefined)delete process.env[key];else process.env[key]=saved[key];}
