// Negative evidence only: selected native owner acquisition still crosses the fence.
// Run against a retained frozen input directory containing the admission-fence primitive.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {pathToFileURL} from 'node:url';

export async function inspectNativeShutdownInventoryGaps(directory){
 const root=path.resolve(directory),base=pathToFileURL(path.join(root,'formal')+path.sep);
 const capture=JSON.parse(fs.readFileSync(path.join(root,'.vellum-input-snapshot.json')));
 const digest=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
 for(const [relative,hash] of Object.entries(capture.hashes))assert.equal(digest(path.join(root,relative)),hash,relative);
 const imp=name=>import(new URL(name,base));
 const {nativeSourceReadApi:api}=await imp('native-source-read-checks.mjs');
 const {nativeSourceSaveFixture}=await imp('native-source-save-checks.mjs');
 const {annotationObservationContext:context}=await imp('annotation-observation-checks.mjs');
 const {resourceReadLimits,resourceReadCodecLimits}=await imp('resource-read-checks.mjs');
 const history=await imp('history-contexts.mjs');
 const {historyContextScope,historyContextGates}=await imp('history-context-checks.mjs');
 assert.equal(typeof api.sealInvocationOrdinaryAdmission,'function','requires the actual fence candidate');
 const z=nativeSourceSaveFixture(),g={...z.g(),ownerControlAllowed:true,controlContextCurrent:true,controlScopeValid:true,protectedControlReady:true};
 assert(api.sealInvocationOrdinaryAdmission(z.domain,z.root(),context,g));
 assert.equal(api.invocationOrdinaryAdmissionOpen(z.domain),false);
 const before=api.invocationUsage(z.domain);
 const readers=Array.from({length:2},()=>api.newInvocationSourceReadRegistry(z.domain,resourceReadLimits,resourceReadCodecLimits,z.profile,z.g()));
 assert(readers.every(Boolean));assert.notEqual(readers[0],readers[1]);assert.deepEqual(api.invocationUsage(z.domain),before);
 // Existing fixture limits, not a selected production default or aggregate budget.
 const limits={maxContexts:8,maxIssued:32,maxGroups:32,maxScopes:32,maxParticipants:8,maxDetails:8,maxCaptureBytes:32768,maxBasisBytes:32768,maxReplyBytes:4096,maxHeldBytes:1048576};
 const histories=Array.from({length:2},()=>history.openHistoryInspection(z.domain,historyContextScope,limits,historyContextGates));
 assert(histories.every(Boolean));assert.notEqual(histories[0],histories[1]);assert.deepEqual(api.invocationUsage(z.domain),before);
 const {invocationObservationFixture}=await imp('invocation-observation-checks.mjs');
 const q=invocationObservationFixture();assert.equal(q.admit().status,'accepted');
 assert(api.sealInvocationOrdinaryAdmission(q.domain,q.current(),context,g));
 const observationBefore=api.invocationUsage(q.domain),contexts=[q.open(),q.open()];
 assert(contexts.every(Boolean));assert.notEqual(contexts[0],contexts[1]);assert.deepEqual(api.invocationUsage(q.domain),observationBefore);
 // All newly acquired handles remain owned; no invented whole-owner cleanup.
 assert.equal(api.invocationUsage(z.domain).faulted,false);assert.equal(api.invocationUsage(q.domain).faulted,false);
 return {scope:'negative-actual-fence-future-owner-probe',sourceInputFingerprint:crypto.createHash('sha256').update(JSON.stringify(capture.hashes)).digest('hex'),sourceReadRegistriesAfterFence:readers.length,historyOwnersAfterFence:histories.length,directObservationContextsAfterFence:contexts.length,domainChargesUnchanged:true,completeShutdownProved:false,implementationOrIndependentCertification:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
 try{assert(process.argv[2],'supply a retained frozen input directory');console.log(JSON.stringify(await inspectNativeShutdownInventoryGaps(process.argv[2])));}
 catch(e){console.error(String(e.message).slice(0,1200));process.exitCode=1;}
}
