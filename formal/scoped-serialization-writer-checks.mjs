// Actual reference writer, supplied execution evidence; no physical file write.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import * as k from './invocation-publication.mjs';
import {nativeSourceSaveFixture} from './native-source-save-checks.mjs';
import {engineSettingsProfile} from './engine-settings-checks.mjs';
import {readSourceSavePayload,inspectSourceOutput,inspectSourceSave,sourceSaveUsage,sourceSaveCodecUsage} from './source-save.mjs';
try{
 const catalog=JSON.parse(fs.readFileSync(new URL('./invocation-publication.json',import.meta.url))),copy=structuredClone;
 const x=nativeSourceSaveFixture(undefined,undefined,{raw:Array.from(Buffer.from('English\r\nDeutsch\n```mermaid\ngraph LR\n A-->B\n```'))}),c=k.invocationAuthorityContext(x.domain);assert(c);assert(!Object.hasOwn(x.root(),'engineSettings'));
 const p={argumentLimits:Object.fromEntries(['maxDependencies','maxRequestBytes','maxIdentifierBytes','maxInputNodes','maxInputDepth'].map(key=>[key,engineSettingsProfile.argumentLimits[key]])),maxPlanBytes:engineSettingsProfile.maxPlanBytes};
 const h=()=>x.g({...Object.fromEntries([...catalog.admissionGates,...catalog.releaseGates,'companionsReady','settingsWritable','versionsAvailable','outsidePublicationGuard','preparationReserved','ownerControlAllowed','retentionReleased','deliveryDrained','serializationBasisAllowed','serializationHostBasisAllowed'].map(key=>[key,true])),sourceReadable:false,sourceSerializationBasis:x.root(),sourceSerializationContext:c,sourceSerializationProfile:p,requiredDependencies:[],contexts:[],serializationWriteContext:{context:c,document:x.document,writeAllowed:true}});
 assert(k.openInvocationSerializationBasisPool(x.domain,{serializationProfile:p,maxContexts:2,maxIssuedContexts:4,maxContextBytes:4096,maxHeldBytes:16384},h()));
 const basis=k.captureInvocationSerializationBasis(x.domain,x.document,c,h());assert(basis);
 const before=copy(x.d()),oldTask=x.admit();assert.equal(inspectSourceOutput(x.writer,x.root(),x.document,x.g()).comparison,'equal');
 const heldWriter=sourceSaveUsage(x.writer),heldCodec=sourceSaveCodecUsage(x.writer),i=k.issueInvocation(x.domain,c,h());assert(i);
 const request={contract:'scoped-source-serialization/1',operation:'semantic-settings',invocation:i.reference,profile:c.profile,serialization:{profile:'strict-utf8-one-bom/1',bom:true},dependencies:[]};
 const plan=k.prepareInvocationScopedSourceSerialization(x.domain,i.ticket,{route:'dynamic',request:{name:'activate-context',variant:'semantic-settings',version:1,arguments:request}},basis,p,c,h());assert(plan);
 const policy={publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:c.recipient};
 assert.equal(k.submitInvocation(x.domain,i.ticket,{kind:'settings',plan},'dynamic',c,policy,h()).status,'accepted');assert.equal(k.runInvocation(x.domain,i.ticket,c,h(),h()).status,'settled');assert.equal(k.observeInvocation(x.domain,i.ticket,c,policy,h()).outcome.publication,'applied');
 assert.deepEqual(x.d().source,before.source);assert.equal(x.d().revision,before.revision);assert.equal(x.d().serialization.generation,before.serialization.generation+1);assert.deepEqual(x.d().sourceBaseline,before.sourceBaseline);assert.equal(inspectSourceOutput(x.writer,x.root(),x.document,x.g()).comparison,'different');
 assert.deepEqual(sourceSaveUsage(x.writer),heldWriter);assert.deepEqual(sourceSaveCodecUsage(x.writer),heldCodec);
 assert.equal(x.dispatch(oldTask).status,'dispatched');const old=x.submitted.get(oldTask),payload=readSourceSavePayload(x.writer,oldTask,[0,before.sourceBaseline.bytes.length],{...x.g(),retained:true,provider:before.binding.provider,resource:before.binding.resource});assert.equal(payload.status,'serialized-bytes');assert.deepEqual(payload.bytes,before.sourceBaseline.bytes);assert.equal(old.capture.serialization.generation,before.serialization.generation);
 assert.equal(x.written(oldTask).status,'recorded');assert.equal(x.baseline().status,'published');assert.equal(x.d().sourceBaseline.serializationGeneration,before.serialization.generation);assert.equal(x.d().serialization.generation,before.serialization.generation+1);assert.equal(inspectSourceOutput(x.writer,x.root(),x.document,x.g()).comparison,'different');
 assert.equal(inspectSourceSave(x.writer,oldTask,x.g({retained:true})).quiescent,false);assert.equal(x.observe(oldTask,'disposition').status,'recorded');assert.equal(x.observe(oldTask,'quiescence').status,'recorded');
 const newTask=x.admit(x.request('new-scoped-format'));assert.equal(x.dispatch(newTask).status,'dispatched');const current=x.submitted.get(newTask);assert.equal(current.capture.serialization.generation,before.serialization.generation+1);assert.equal(current.capture.payloadByteLength,before.sourceBaseline.bytes.length+3);
 const v={epoch:before.sourceBaseline.resourceVersion.epoch,token:'new-scoped-format-write'};assert.equal(x.written(newTask,v).status,'recorded');assert.equal(x.baseline().status,'published');assert.equal(x.d().sourceBaseline.serializationGeneration,x.d().serialization.generation);assert.equal(inspectSourceOutput(x.writer,x.root(),x.document,x.g()).comparison,'equal');assert.equal(x.d().revision,before.revision);assert.deepEqual(x.d().source,before.source);
 assert(k.retireInvocationSerializationBasis(x.domain,basis,c,h()));assert(k.retireInvocationSerializationBasisPool(x.domain,h()));
 console.log(JSON.stringify({scope:'isolated-scoped-format-actual-writer-composition',sourceReadGrantNotRequired:true,oldCapturePreserved:true,lateOldAcknowledgmentRemainsHistorical:true,newFormatAcknowledgmentMatchesCurrentOutput:true,quiescenceSeparate:true,physicalFilesystemOrPolicyTruthProved:false}));
}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
