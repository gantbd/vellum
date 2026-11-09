import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),file='formal/invocation-publication.mjs';
const original=fs.readFileSync(path.join(root,file),'utf8'),start=original.indexOf('function captureLifecycleActivationPorts('),entryStart=original.indexOf('export function prepareInvocationLifecycleActivationEnvelope(',start),end=original.indexOf('\n}\n',entryStart)+3;
assert(start>=0&&entryStart>start&&end>entryStart);const region=original.slice(start,end);
const rules=[
 ['shared-preparation-guard-not-set','entry.ordinaryPreparing=true;','entry.ordinaryPreparing=false;'],
 ['shared-preparation-guard-not-cleared','finally{endLifecycleEnvelopeWorkspace(o,workspace);entry.ordinaryPreparing=false;}','finally{endLifecycleEnvelopeWorkspace(o,workspace);}'],
 ['native-fault-not-fenced','catch{o.faulted=true;return null;}','catch{return null;}'],
 ['outside-publication-gate-ignored',"!gates(g,['outsidePublicationGuard','preparationReserved'])",'false'],
 ['native-port-record-open','||!equal(keys.toSorted(),expected.toSorted())',''],
 ['source-load-authority-cloned','profile,bound.authority,n.record.context,g);break;','profile,structuredClone(bound.authority),n.record.context,g);break;','first'],
 ['rebind-inspection-cloned','profile,bound.inspection,n.record.context,g);break;','profile,structuredClone(bound.inspection),n.record.context,g);break;'],
 ['source-load-host-facts-cloned','profile,bound.authority,n.record.context,g);break;','profile,bound.authority,n.record.context,structuredClone(g));break;','first'],
 ['container-kind-substituted',"kind='relation';plan=prepareRelationIntent","kind='lifecycle';plan=prepareRelationIntent"],
 ['native-plan-withheld','return Object.freeze({job:Object.freeze({kind,plan}),route:n.route});','return null;'],
 ['native-plan-substituted','return Object.freeze({job:Object.freeze({kind,plan}),route:n.route});','return Object.freeze({job:Object.freeze({kind,plan:{}}),route:n.route});'],
 ['route-substituted','return Object.freeze({job:Object.freeze({kind,plan}),route:n.route});',"return Object.freeze({job:Object.freeze({kind,plan}),route:'typed'});"]
];
const baselineProgram=`import {checkActivationEnvelope} from './formal/lifecycle-activation-envelope-checks.mjs';try{console.log(JSON.stringify({ok:true,result:checkActivationEnvelope()}));}catch(error){if(error.code!=='ERR_ASSERTION')throw error;console.log(JSON.stringify({semanticAssertion:true}));process.exitCode=1;}`;
const faultProgram=`import assert from 'node:assert/strict';import {activationEnvelopeFixture,activationEnvelopeIds} from './formal/lifecycle-activation-envelope-checks.mjs';let cases=0;for(const id of activationEnvelopeIds)for(const route of ['typed','dynamic']){const x=activationEnvelopeFixture(id,route),before=x.root(),usage=x.usage();globalThis.__activationEnvelopeInstalled=false;globalThis.__activationEnvelopeDischarged=false;assert.equal(x.prepare(),null);assert.equal(globalThis.__activationEnvelopeInstalled,process.env.VELLUM_ACTIVATION_FAULT==='release');assert.equal(x.root(),before);assert.deepEqual(x.usage(),{...usage,faulted:true});assert.equal(x.prepare(),null);assert.equal(x.nativePrepare(),null);for(const field of ['ownerControlAllowed','retentionReleased','deliveryDrained'])assert.equal(x.retire({...x.facts(),[field]:false}).status,'unavailable');assert.equal(x.retire().status,'retired');assert.equal(globalThis.__activationEnvelopeDischarged,true);assert.equal(x.usage().slots,usage.slots-1);assert.equal(x.usage().issued,usage.issued);assert.equal(x.usage().starts,usage.starts);assert.equal(x.usage().declaredHeldBytes,usage.declaredHeldBytes);assert.equal(x.retire().status,'unavailable');assert.equal(x.prepare(),null);cases++;}console.log(JSON.stringify({cases,knownTicketDischarge:true,installedBeforeFault:process.env.VELLUM_ACTIVATION_FAULT==='release'?cases:0}));`;
const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
try{
 process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;
 const run=(directory,program,extra={})=>{const out=spawnSync(process.execPath,['--input-type=module','-e',program],{cwd:directory,encoding:'utf8',env:{...process.env,...extra},maxBuffer:2097152,timeout:60000});assert(!out.error&&!out.signal,'process/tool failure');return out;};
 const baselineOut=run(captureInputs(root).directory,baselineProgram);assert.equal(baselineOut.status,0,baselineOut.stderr);const baseline=JSON.parse(baselineOut.stdout);assert.equal(baseline.ok,true);
 const mutations=[];for(const [name,before,after,mode]of rules){const count=region.split(before).length-1;assert.equal(count,mode==='first'?2:1,name+': construction target');const copy=captureInputs(root),changed=original.slice(0,start)+region.replace(before,after)+original.slice(end);fs.writeFileSync(path.join(copy.directory,file),changed);const out=run(copy.directory,baselineProgram);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1200));assert.equal(JSON.parse(out.stdout).semanticAssertion,true,name);mutations.push(name);}
 const faults=[];for(const checkpoint of ['capture','release']){
  const marker='// Lifecycle activation envelope '+checkpoint+' checkpoint.';assert.equal(region.split(marker).length-1,1);
  const copy=captureInputs(root),witness=checkpoint==='release'?"Boolean(plan&&(kind==='relation'?entry.relationPrepared:entry.lifecyclePrepared)===plan)":"false",changed=original.slice(0,start)+region.replace(marker,marker+"\n  globalThis.__activationEnvelopeInstalled="+witness+";\n  throw Error('fixed activation envelope "+checkpoint+" fault');")+original.slice(end);
  const retirement='if(entry.relationPrepared)forgetRelationInvocation(entry.relationPrepared);';assert.equal(changed.split(retirement).length-1,1);const checked=changed.replace(retirement,retirement+'globalThis.__activationEnvelopeDischarged=(!entry.lifecyclePrepared||captureLifecycleInvocation(entry.lifecyclePrepared,o,domain,ticket,o.limits.maxIntentBytes)===null)&&(!entry.relationPrepared||captureRelationInvocation(entry.relationPrepared,o,domain,ticket,o.limits.maxIntentBytes)===null);');
  fs.writeFileSync(path.join(copy.directory,file),checked);const out=run(copy.directory,faultProgram,{VELLUM_ACTIVATION_FAULT:checkpoint});assert.equal(out.status,0,checkpoint+': '+out.stderr);const result=JSON.parse(out.stdout);assert.equal(result.cases,18);assert.equal(result.knownTicketDischarge,true);assert.equal(result.installedBeforeFault,checkpoint==='release'?18:0);faults.push({checkpoint,...result});
 }
 passed=true;console.log(JSON.stringify({scope:'selected-F17-activation-envelope-sensitivity',baseline:baseline.result,mutations,faults,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,temporaryParentOrPhysicalAccountingProved:false,fullF17OrIndependentProof:false}));
}finally{releaseCaptureScope(scope,passed);for(const key of keys)if(saved[key]===undefined)delete process.env[key];else process.env[key]=saved[key];}
