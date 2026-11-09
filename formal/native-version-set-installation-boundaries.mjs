import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {newVersionRegistry,acquireVersionSet} from './version-set-contexts.mjs';
import {copySettingsData} from './semantic-settings.mjs';
import {nativeVersionSetCaptureFixture as captureFixture,nativeVersionSetCaptureFacts as facts} from './native-version-set-capture-checks.mjs';
import {nativeVersionSetInstallationFixture as fixture,nativeVersionSetInstallationProfile as profile} from './native-version-set-installation-checks.mjs';
import {nativeVersionSetCurrentFacts as current} from './native-version-set-handoff-checks.mjs';
const size=v=>Buffer.byteLength(JSON.stringify(v)),charge=p=>{
 const bytes=copySettingsData(p,{maxRequestBytes:p.maxRequestBytes,maxInputNodes:p.maxInputNodes,maxInputDepth:p.maxInputDepth}).bytes;
 const stage=p.maxRegistryBytes+p.maxCaptureBytes+p.setLimits.participants*p.maxSelectorRecordBytes;
 return bytes+2*p.maxRequestBytes+3*stage+2*p.maxContextBytes+3*p.maxReplyBytes+p.maxGroups*(p.maxRegistryBytes+p.maxCaptureBytes+p.maxContextBytes+p.maxReplyBytes)+(2*p.maxSelectors+p.maxHolds)*(p.maxSelectorBytes+p.maxSelectorRecordBytes);
};
export function checkNativeVersionSetInstallationBoundaries(){let cases=0;const dimensions=[],visit=(name,f)=>{for(const delta of [-1,0,1]){f(delta);cases++;}dimensions.push(name);};
 const seed=fixture(),candidate=seed.x.read(seed.cut),host=current(seed.x),ready=h=>({...h,registryReady:true,retentionReady:true,replyReady:true,transferReady:true}),acquired=acquireVersionSet(newVersionRegistry('native-set-1',profile.setLimits),candidate,{...ready(host),members:host.members.map(ready)});
 assert.equal(acquired.result.status,'version-set');const context={kind:'native-version-set-context/1'},member={kind:'native-version-set-member/1'},pending=[{kind:'parent',parent:context,internal:acquired.result.context},...acquired.result.participants.map(p=>({kind:'member',parent:context,internal:p.snapshot,document:p.document}))],reply={status:'version-set',context,participants:acquired.result.participants.map(p=>({...p,snapshot:member}))};
 const minima={maxCaptureBytes:size(candidate),maxContextBytes:size({scope:candidate.scope,invalidated:false}),maxRequestBytes:size({host,facts}),maxRegistryBytes:size(acquired.state),maxSelectorBytes:Math.max(size(context),size(member)),maxSelectorRecordBytes:Math.max(...pending.map(size)),maxReplyBytes:size(reply)};
 assert(minima.maxRequestBytes>size(profile));
 for(const [field,min]of Object.entries(minima))visit(field,delta=>{const z=fixture({[field]:min+delta}),before=z.usage(),capture=z.x.usage(),root=z.x.z.root(),out=z.install();assert.equal(out.status,delta>=0?'version-set':'unavailable',field);assert.equal(z.usage().declaredHeldBytes,before.declaredHeldBytes);assert.equal(z.usage().issued,delta>=0?3:0);assert.equal(z.usage().selectors,delta>=0?3:0);assert.equal(z.usage().groups,delta>=0?1:0);assert.equal(z.x.usage().prepared,delta>=0?0:capture.prepared);assert.equal(z.x.usage().issued,capture.issued);assert.equal(z.x.z.root(),root);if(delta<0)assert.deepEqual(z.usage(),before);});
 for(const [field,min]of [['maxGroups',1],['maxSelectors',3],['maxIssued',3]])visit(field,delta=>{const z=fixture({[field]:min+delta}),before=z.usage(),capture=z.x.usage(),out=z.install();assert.equal(out.status,delta>=0?'version-set':'unavailable',field);if(delta<0){assert.deepEqual(z.usage(),before);assert.deepEqual(z.x.usage(),capture);}});
 let fixed={...profile};for(let n=0;n<5;n++)fixed={...fixed,maxHeldBytes:charge(fixed)};assert.equal(fixed.maxHeldBytes,charge(fixed));
 visit('complete-local-reservation',delta=>{const x=captureFixture(),p={...fixed,maxHeldBytes:fixed.maxHeldBytes+delta},before=api.invocationUsage(x.z.domain).declaredHeldBytes;assert.equal(charge(p),fixed.maxHeldBytes);const handle=api.openInvocationVersionSetContexts(x.z.domain,p,facts);assert.equal(handle!==null,delta>=0);assert.equal(api.invocationUsage(x.z.domain).declaredHeldBytes,before+(delta>=0?charge(p)+parentFundedPoolAttachmentBytes(api,x.z.domain,handle):0));});
 const probe=captureFixture(),base=api.invocationUsage(probe.z.domain).declaredHeldBytes,cost=charge(profile)+parentFundedPoolAttachmentBytes(api,seed.x.z.domain,seed.handle),parent={maxSlots:4,maxIssued:16,maxIntentBytes:65536,maxResultBytes:4096,maxRootBytes:262144};
 visit('actual-parent-headroom',delta=>{const x=captureFixture({},{...parent,maxHeldBytes:base+cost+delta}),before=api.invocationUsage(x.z.domain).declaredHeldBytes;assert.equal(before,base);const handle=api.openInvocationVersionSetContexts(x.z.domain,profile,facts);assert.equal(handle!==null,delta>=0);assert.equal(api.invocationUsage(x.z.domain).declaredHeldBytes,before+(delta>=0?cost:0));});
 return {scope:'isolated-native-version-set-installation-exact-boundaries',cases,dimensions,completeCandidateRegistryReplyMetadataAndAliasBoundsChecked:true,physicalGlobalQueryAndAllProfilesProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeVersionSetInstallationBoundaries()));
