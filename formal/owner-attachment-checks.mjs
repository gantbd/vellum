// Selected actual native factories and parent edges; not complete shutdown.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {nativeSourceReadApi as api} from './native-source-read-checks.mjs';
import {nativeSourceSaveFixture} from './native-source-save-checks.mjs';
import {resourceReadLimits as readLimits,resourceReadCodecLimits as codecLimits} from './resource-read-checks.mjs';
import {annotationObservationContext as context,annotationInvocationLimits as parentLimits} from './annotation-observation-checks.mjs';
import {openHistoryInspection} from './history-contexts.mjs';
import {historyContextScope,historyContextGates} from './history-context-checks.mjs';
import {invocationObservationFixture,invocationObservationGates,invocationObservationContext} from './invocation-observation-checks.mjs';
import {retireInvocationObservation,closeInvocationObservation,invocationObservationUsage} from './invocation-observation.mjs';
const copy=structuredClone,historyLimits={maxContexts:8,maxIssued:32,maxGroups:32,maxScopes:32,maxParticipants:8,maxDetails:8,maxCaptureBytes:32768,maxBasisBytes:32768,maxReplyBytes:4096,maxHeldBytes:1048576};
const control=(z,more={})=>({...z.g(),ownerControlAllowed:true,controlContextCurrent:true,controlScopeValid:true,protectedControlReady:true,...more});
const inventory=z=>kernel.invocationOwnerAttachmentInventory(z.domain,context,control(z));
const reader=(z,g=z.g())=>api.newInvocationSourceReadRegistry(z.domain,readLimits,codecLimits,z.profile,g);
const history=(z,g=historyContextGates)=>openHistoryInspection(z.domain,historyContextScope,historyLimits,g);
const seal=z=>kernel.sealInvocationOrdinaryAdmission(z.domain,z.root(),context,control(z));
const testHandle=()=>Object.freeze({kind:'test-owner/1'});
export function checkOwnerAttachments(){
 const controls=[],check=(name,f)=>{try{f();}catch(e){e.message=name+': '+e.message;throw e;}controls.push(name);};let boundaries=0;
 check('committed-edge-releases-its-admission-section-before-other-factories-run',()=>{const domain=kernel.openInvocationDomain({authority:context.authority,publication:0,documents:[]},context,parentLimits,invocationObservationGates);assert(domain);const owner=testHandle(),lease=kernel.beginInvocationOwnerAttachment(domain,'history',0,owner);assert(lease);assert.equal(kernel.commitInvocationOwnerAttachment(domain,lease,owner),true);assert(kernel.sealInvocationOrdinaryAdmission(domain,kernel.invocationAuthorityRoot(domain),context,{ownerControlAllowed:true,controlContextCurrent:true,controlScopeValid:true,protectedControlReady:true}));});
 check('actual-source-read-and-history-factories-attach-distinct-owners-to-the-real-domain',()=>{
  const z=nativeSourceSaveFixture(),root=z.root(),before=inventory(z),usage=api.invocationUsage(z.domain),r=reader(z),h=history(z);assert(r&&h);
  const after=inventory(z);assert.equal(after.entries.length,before.entries.length+2);assert(after.entries.some(e=>e.owner===r&&e.kind==='source-read'&&e.additionalChildReservationBytes===readLimits.maxHeldBytes+2*readLimits.maxIssued*readLimits.maxCaptureBytes));assert(after.entries.some(e=>e.owner===h&&e.kind==='history'&&e.additionalChildReservationBytes===historyLimits.maxHeldBytes));assert(after.entries.every(e=>e.declaredFrameBytes>0));assert.equal(api.invocationUsage(z.domain).declaredHeldBytes-usage.declaredHeldBytes,after.declaredHeldBytes-before.declaredHeldBytes);assert.equal(z.root(),root);
 });
 check('all-three-reproduced-future-acquisition-gaps-close-after-the-actual-fence',()=>{
  const z=nativeSourceSaveFixture();assert(seal(z));const before=inventory(z),usage=api.invocationUsage(z.domain);assert.equal(reader(z),null);assert.equal(history(z),null);assert.deepEqual(inventory(z),before);assert.deepEqual(api.invocationUsage(z.domain),usage);
  const q=invocationObservationFixture();assert.equal(q.admit().status,'accepted');assert(kernel.sealInvocationOrdinaryAdmission(q.domain,q.current(),invocationObservationContext,control(z)));const b=kernel.invocationUsage(q.domain);assert.equal(q.open(),null);assert.deepEqual(kernel.invocationUsage(q.domain),b);
 });
 check('failed-and-throwing-factory-validation-refund-the-parent-and-unwind-the-section',()=>{
  const z=nativeSourceSaveFixture(),before=api.invocationUsage(z.domain),inv=inventory(z);assert.equal(reader(z,z.g({ownerControlAllowed:false})),null);assert.equal(history(z,{...historyContextGates,granted:false}),null);assert.deepEqual(api.invocationUsage(z.domain),before);assert.deepEqual(inventory(z),inv);
  const g=z.g();Object.defineProperty(g,'ownerControlAllowed',{get(){throw Error('fixed-owner-factory-fault');}});assert.throws(()=>reader(z,g),/fixed-owner-factory-fault/);assert.deepEqual(api.invocationUsage(z.domain),before);assert.deepEqual(inventory(z),inv);assert(seal(z));
 });
 check('factory-validation-cannot-reenter-fence-installation',()=>{
  for(const kind of ['source-read','history']){const z=nativeSourceSaveFixture(),g=kind==='source-read'?z.g():{...historyContextGates};let n=0;Object.defineProperty(g,kind==='source-read'?'ownerControlAllowed':'granted',{get(){n++;assert.equal(seal(z),null);return true;}});assert(kind==='source-read'?reader(z,g):history(z,g));assert(n>0);assert(seal(z));}
 });
 check('parent-headroom-includes-the-child-ceiling-and-positive-edge-frame',()=>{
  const seed=nativeSourceSaveFixture(),cost=inventory(seed).entries.find(e=>e.kind==='source-read'),charge=cost.additionalChildReservationBytes+cost.declaredFrameBytes;
  for(const delta of [-1,0,1]){const z=nativeSourceSaveFixture(),before=api.invocationUsage(z.domain),units=parentLimits.maxHeldBytes-before.declaredHeldBytes-charge-delta;assert(units>=0);const lease=api.reserveInvocationRegistrationReadCharge(z.domain,units,control(z));assert(lease);const full=api.invocationUsage(z.domain),old=inventory(z);assert.equal(Boolean(reader(z)),delta>=0);assert.equal(api.invocationUsage(z.domain).declaredHeldBytes,full.declaredHeldBytes+(delta>=0?charge:0));assert.equal(inventory(z).entries.length,old.entries.length+(delta>=0?1:0));assert(api.releaseInvocationRegistrationReadCharge(z.domain,lease,{...control(z),retentionReleased:true,deliveryDrained:true}));boundaries++;}
 });
 check('existing-parent-capacity-bounds-multiple-native-registries-without-a-new-default',()=>{
  const z=nativeSourceSaveFixture(),root=z.root();let count=0;while(reader(z)){count++;assert(count<32,'fixed fixture bound, not a production owner limit');}assert(count>0);const before=api.invocationUsage(z.domain),inv=inventory(z);assert.equal(reader(z),null);assert.deepEqual(api.invocationUsage(z.domain),before);assert.deepEqual(inventory(z),inv);assert.equal(z.root(),root);assert(before.declaredHeldBytes<=parentLimits.maxHeldBytes);
 });
 check('native-observation-retirement-releases-only-its-original-edge-once-after-fencing',()=>{
  const q=invocationObservationFixture();assert.equal(q.admit().status,'accepted');const before=kernel.invocationUsage(q.domain),ctx=q.open();assert(ctx);const charged=kernel.invocationUsage(q.domain),inv=kernel.invocationOwnerAttachmentInventory(q.domain,context,{ownerControlAllowed:true}),edge=inv.entries.find(e=>e.owner===ctx);assert(edge);assert.equal(edge.kind,'observation');assert.equal(charged.declaredHeldBytes-before.declaredHeldBytes,edge.additionalChildReservationBytes+edge.declaredFrameBytes);
  const z=nativeSourceSaveFixture();assert(kernel.sealInvocationOrdinaryAdmission(q.domain,q.current(),context,control(z)));const fenced=kernel.invocationUsage(q.domain);assert.equal(retireInvocationObservation(ctx,{...invocationObservationGates,retentionReleased:false}).status,'unavailable');assert.deepEqual(kernel.invocationUsage(q.domain),fenced);assert.equal(closeInvocationObservation(ctx,context,invocationObservationGates).status,'stopped');assert.equal(retireInvocationObservation(ctx,invocationObservationGates).status,'retired');assert.equal(invocationObservationUsage(ctx),null);assert.equal(kernel.invocationUsage(q.domain).declaredHeldBytes,fenced.declaredHeldBytes-edge.additionalChildReservationBytes-edge.declaredFrameBytes);const done=kernel.invocationUsage(q.domain);assert.equal(retireInvocationObservation(ctx,invocationObservationGates).status,'unavailable');assert.deepEqual(kernel.invocationUsage(q.domain),done);assert.equal(kernel.invocationOrdinaryAdmissionOpen(q.domain),false);
 });
 check('original-domain-context-and-strict-supervision-bind-inventory-release',()=>{
  const z=nativeSourceSaveFixture();for(const key of Object.keys(context))assert.equal(kernel.invocationOwnerAttachmentInventory(z.domain,{...context,[key]:'foreign'},control(z)),null);for(const v of [false,1,undefined])assert.equal(kernel.invocationOwnerAttachmentInventory(z.domain,context,control(z,{ownerControlAllowed:v})),null);
  assert.equal(kernel.invocationOwnerAttachmentInventory(copy(z.domain),context,control(z)),null);const inv=inventory(z);assert(Object.isFrozen(inv)&&Object.isFrozen(inv.entries)&&inv.entries.every(Object.isFrozen));assert.equal(inv.completeFactoryCoverageProved,false);assert.equal(inv.wholeOwnerDischargeProved,false);
 });
 check('copied-or-foreign-leases-cannot-abort-or-release-a-real-pending-edge',()=>{
  const z=nativeSourceSaveFixture(),other=nativeSourceSaveFixture(),before=api.invocationUsage(z.domain),lease=kernel.beginInvocationOwnerAttachment(z.domain,'history',17,testHandle());assert(lease);assert.equal(kernel.abortInvocationOwnerAttachment(z.domain,copy(lease)),false);assert.equal(kernel.abortInvocationOwnerAttachment(other.domain,lease),false);assert.equal(seal(z),null);assert.equal(kernel.releaseInvocationOwnerAttachment(z.domain,lease,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true}),false);assert(kernel.abortInvocationOwnerAttachment(z.domain,lease));assert.deepEqual(api.invocationUsage(z.domain),before);assert.equal(kernel.abortInvocationOwnerAttachment(z.domain,lease),false);assert(seal(z));
 });
 check('graph-edges-do-not-conflate-an-actual-shared-node-with-one-domain-owner',()=>{
  const z=nativeSourceSaveFixture(),peer=nativeSourceSaveFixture(),node=Object.freeze({kind:'test-shared-owner/1'}),a=kernel.beginInvocationOwnerAttachment(z.domain,'source-writer',1,node);assert(a);assert(kernel.commitInvocationOwnerAttachment(z.domain,a,node));const b=kernel.beginInvocationOwnerAttachment(peer.domain,'source-writer',1,node);assert(b);assert(kernel.commitInvocationOwnerAttachment(peer.domain,b,node));assert(seal(z));const peerBefore=inventory(peer);assert(kernel.releaseInvocationOwnerAttachment(z.domain,a,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true}));assert.deepEqual(inventory(peer),peerBefore);assert(inventory(peer).entries.some(e=>e.owner===node));assert(kernel.invocationOrdinaryAdmissionOpen(peer.domain));
 });
 check('duplicate-edge-cannot-replace-original-lease-or-funding',()=>{
  const z=nativeSourceSaveFixture(),existing=inventory(z).entries[0].owner,before=api.invocationUsage(z.domain),lease=kernel.beginInvocationOwnerAttachment(z.domain,'source-read',1,existing);assert(lease);assert.equal(kernel.commitInvocationOwnerAttachment(z.domain,lease,existing),false);assert(kernel.abortInvocationOwnerAttachment(z.domain,lease));assert.deepEqual(api.invocationUsage(z.domain),before);
 });
 check('edge-release-requires-original-lease-and-every-strict-discharge-fact',()=>{
  const z=nativeSourceSaveFixture(),owner=Object.freeze({kind:'test-retired-owner/1'}),lease=kernel.beginInvocationOwnerAttachment(z.domain,'history',19,owner);assert(lease);assert(kernel.commitInvocationOwnerAttachment(z.domain,lease,owner));const before=api.invocationUsage(z.domain),inv=inventory(z),g={ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true};
  for(const key of Object.keys(g))for(const value of [false,1,undefined]){assert.equal(kernel.releaseInvocationOwnerAttachment(z.domain,lease,{...g,[key]:value}),false);assert.deepEqual(api.invocationUsage(z.domain),before);assert.deepEqual(inventory(z),inv);}
  assert.equal(kernel.releaseInvocationOwnerAttachment(z.domain,copy(lease),g),false);assert.deepEqual(api.invocationUsage(z.domain),before);assert(kernel.releaseInvocationOwnerAttachment(z.domain,lease,g));const after=api.invocationUsage(z.domain);assert.equal(kernel.releaseInvocationOwnerAttachment(z.domain,lease,g),false);assert.deepEqual(api.invocationUsage(z.domain),after);
 });
 check('unknown-kind-invalid-budget-and-fake-domain-refuse-without-parent-mutation',()=>{
  const z=nativeSourceSaveFixture(),before=api.invocationUsage(z.domain);for(const [kind,units] of [['surprise',0],['history',-1],['history',0.5],['history',Number.MAX_SAFE_INTEGER],['history',undefined]])assert.equal(kernel.beginInvocationOwnerAttachment(z.domain,kind,units,testHandle()),null);assert.equal(kernel.beginInvocationOwnerAttachment(copy(z.domain),'history',0,testHandle()),null);assert.deepEqual(api.invocationUsage(z.domain),before);assert(seal(z));
 });
 check('metadata-frame-covers-pending-map-key-edge-and-captured-report-independently',()=>{
  const z=nativeSourceSaveFixture();for(const e of inventory(z).entries){const max=Number.MAX_SAFE_INTEGER,lease={kind:'native-owner-attachment/1'},section={kind:'native-admission-section/1'},bytes=v=>Buffer.byteLength(JSON.stringify(v));
   const pending={section,lease,kind:e.kind,units:max,ownerTemplate:e.owner,frame:max,charge:max},retained={owner:e.owner,lease,kind:e.kind,units:max,frame:max,charge:max},report={entries:[{owner:e.owner,kind:e.kind,additionalChildReservationBytes:max,declaredFrameBytes:max}],pending:false,declaredHeldBytes:max,completeFactoryCoverageProved:false,wholeOwnerDischargeProved:false};assert(e.declaredFrameBytes>=bytes(pending)+bytes(retained)+bytes(report));
  }
  const before=api.invocationUsage(z.domain),template=testHandle(),lease=kernel.beginInvocationOwnerAttachment(z.domain,'history',0,template);assert(lease);assert.equal(kernel.commitInvocationOwnerAttachment(z.domain,lease,Object.freeze({kind:'larger-undeclared-owner-handle/1'})),false);assert(kernel.abortInvocationOwnerAttachment(z.domain,lease));assert.deepEqual(api.invocationUsage(z.domain),before);
  for(const shape of [undefined,{},Object.freeze({kind:'x',extra:1}),{kind:'mutable'},Object.freeze({kind:''})])assert.equal(kernel.beginInvocationOwnerAttachment(z.domain,'history',0,shape),null);assert.deepEqual(api.invocationUsage(z.domain),before);
 });
 return {scope:'selected-actual-native-owner-attachment-reference',controls,boundaries,actualParentChildCeilingsAndEdgeFramesChecked:true,threeNativeFutureAcquisitionGapsChecked:true,observationNativeRetirementRefundChecked:true,sharedEdgeRelationOnlyNotActualWriterIntegration:true,allNativeFactoriesAdmissionsDependenciesOrShutdownProved:false,physicalResourceTruthOrIndependentCertification:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkOwnerAttachments()));}catch(e){console.error(e.code+': '+String(e.message).slice(0,1200));process.exitCode=1;}
