// Selected actual native participant checks; no host-policy or physical proof.
import assert from 'node:assert/strict';
import {pathToFileURL,fileURLToPath} from 'node:url';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {projectInputs} from './evidence-harness.mjs';
import {decideSerialization,sourceSerializationGates} from './source-serialization.mjs';
import {engineSettingsProfile as parentProfile,engineSettingsContext as referenceContext} from './engine-settings-checks.mjs';
import {nativeSourceSaveFixture} from './native-source-save-checks.mjs';
import {newSourceCodecRegistry,captureSerializedSource,compareSerializedSources} from './source-codec.mjs';
import {readSourceSavePayload,inspectSourceOutput,inspectSourceSave,sourceSaveUsage,sourceSaveCodecUsage} from './source-save.mjs';
import * as k from './invocation-publication.mjs';
import {engineSettingsFixture} from './engine-settings-checks.mjs';
import {sourceSerializationIntent} from './source-serialization-state.mjs';
import {committedSourceTransforms} from './parser-source.mjs';
const copy=structuredClone;
function fixture(route='typed',options={},api=k){
 const seed=engineSettingsFixture(api),state=copy(seed.root());state.publication=options.publication??state.publication;state.documents[0].revision=options.sourceRevision??state.documents[0].revision;state.documents[0].serialization={generation:options.generation??1,profile:'strict-utf8-one-bom/1',bom:options.initialBom??false};if(options.noRegistry)delete state.engineSettings;if(options.documentId)state.documents[0].document.id=options.documentId;
 const z=engineSettingsFixture(api,{state}),c=z.controlContext,baseArguments=Object.fromEntries(['maxDependencies','maxRequestBytes','maxIdentifierBytes','maxInputNodes','maxInputDepth'].map(key=>[key,z.p.argumentLimits[key]])),p={maxPlanBytes:z.p.maxPlanBytes,...options.profile,argumentLimits:{...baseArguments,...options.profile?.argumentLimits}},document=z.root().documents[0].document;
 const h=(more={})=>({...z.host(),...options.host,sourceSerializationBasis:z.root(),sourceSerializationContext:c,sourceSerializationProfile:p,serializationWriteContext:{context:c,document,writeAllowed:true},...more}),i=api.issueInvocation(z.domain,c,h());assert(i);
 const request={contract:'source-serialization/1',operation:'semantic-settings',invocation:i.reference,profile:c.profile,document,expectedSerialization:state.documents[0].serialization.generation,serialization:{profile:'strict-utf8-one-bom/1',bom:true},dependencies:[],...options.request};
 const envelope=(r=request,q=route)=>({route:q,request:{...(q==='typed'?{family:15}:{name:'activate-context'}),variant:'semantic-settings',version:1,arguments:r}});
 const prepare=(r=request,g=h(),q=route)=>api.prepareInvocationSourceSerialization(z.domain,i.ticket,envelope(r,q),p,c,g),plan=options.noPrepare?null:prepare();if(!options.noPrepare)assert(plan);
 const job={kind:'settings',plan},submit=(q=route,j=job)=>api.submitInvocation(z.domain,i.ticket,j,q,c,z.controlPolicy,h()),run=(more={})=>api.runInvocation(z.domain,i.ticket,c,h(),h(more));
 return {...z,...i,c,p,document,h,host:h,request,envelope,prepare,plan,job,submit,run};
}
export function sourceSerializationFixture(api=k,options={}){return fixture(options.route??'typed',options,api);}
export function checkSourceSerializationReference(){
let publications=0;
for(const route of ['typed','dynamic']){
 const x=fixture(route),before=x.root(),usage=k.invocationUsage(x.domain);assert.equal(x.prepare(x.request,x.h(),route==='typed'?'dynamic':'typed'),x.plan);assert.deepEqual(k.invocationUsage(x.domain),usage);
 assert.equal(x.submit().status,'accepted');assert.equal(x.submit(route==='typed'?'dynamic':'typed').status,'pending');assert.equal(x.run().status,'settled');
 const expected=copy(before);expected.publication++;expected.documents[0].serialization.generation++;expected.documents[0].serialization.bom=true;assert.deepEqual(x.root(),expected);assert(Object.isFrozen(x.root()));
 assert.equal(k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome.publication,'applied');assert.equal(x.submit().status,'observe-existing');assert.equal(k.retireInvocation(x.domain,x.ticket,x.h()).status,'retired');
 const lost=k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h());assert.equal(lost.outcome.observation,'unavailable');assert.equal(lost.outcome.publication,'withheld');assert.equal(lost.outcome.control,'withheld');assert.equal(lost.outcome.correlation,null);publications++;
}
{const x=fixture('typed',{noRegistry:true}),before=x.root();assert(!Object.hasOwn(before,'engineSettings'));assert.equal(x.submit().status,'accepted');assert.equal(x.run().status,'settled');assert.equal(x.root().documents[0].serialization.generation,2);assert(!Object.hasOwn(x.root(),'engineSettings'));assert.equal(k.invocationSemanticSettingsUsage(x.domain),null);}
let serviceCases=0;
for(const field of ['authorized','scope','continuous','live','notStopped','interpretationCurrent','companionsReady','settingsWritable','resourcesReady','versionsAvailable'])for(const value of [false,1,undefined]){const x=fixture(),before=x.root();assert.equal(x.submit().status,'accepted');assert.equal(x.run({[field]:value}).status,'settled');assert.equal(x.root(),before);assert.equal(k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome.publication,'notApplied');serviceCases++;}
for(const field of ['sourceSerializationBasis','sourceSerializationContext','sourceSerializationProfile']){const x=fixture(),before=x.root(),value=field==='sourceSerializationBasis'?copy(before):field==='sourceSerializationContext'?{...x.c,recipient:'another'}:{...x.p,maxPlanBytes:x.p.maxPlanBytes+1};assert.equal(x.submit().status,'accepted');assert.equal(x.run({[field]:value}).status,'settled');assert.equal(x.root(),before);assert.equal(k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome.publication,'notApplied');serviceCases++;}
{const x=fixture(),before=x.root();assert.equal(x.submit().status,'accepted');assert.equal(k.stopInvocation(x.domain,x.ticket,x.h()).status,'stop-requested');assert.equal(x.run().status,'settled');assert.equal(x.root(),before);assert.equal(k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome.control,'stopped');}
{const x=fixture(),y=fixture();assert.deepEqual(x.submit('typed',{kind:'settings',plan:y.plan}),{status:'unavailable'});assert.deepEqual(x.submit('typed',{kind:'settings',plan:copy(x.plan)}),{status:'unavailable'});}
{const x=fixture('typed',{request:{serialization:{profile:'strict-utf8-one-bom/1',bom:false}}}),before=x.root();assert.equal(x.submit().status,'accepted');assert.equal(x.run().status,'settled');assert.equal(x.root(),before);assert.equal(k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome.publication,'unchanged');}
let counterCases=0;
for(const field of ['generation','publication']){const x=fixture('typed',{noPrepare:true,[field]:Number.MAX_SAFE_INTEGER}),before=x.root();assert.equal(x.prepare(),null);assert.equal(x.root(),before);counterCases++;const y=fixture('typed',{[field]:Number.MAX_SAFE_INTEGER,request:{serialization:{profile:'strict-utf8-one-bom/1',bom:false}},host:{versionsAvailable:false}}),root=y.root();assert.equal(y.submit().status,'accepted');assert.equal(y.run({versionsAvailable:false}).status,'settled');assert.equal(y.root(),root);assert.equal(k.observeInvocation(y.domain,y.ticket,y.c,y.controlPolicy,y.h()).outcome.publication,'unchanged');counterCases++;}
{const x=fixture('typed',{sourceRevision:Number.MAX_SAFE_INTEGER});assert.equal(x.submit().status,'accepted');assert.equal(x.run().status,'settled');assert.equal(x.root().documents[0].revision,Number.MAX_SAFE_INTEGER);assert.equal(x.root().documents[0].serialization.generation,2);counterCases++;}
{const x=fixture('typed',{noPrepare:true}),before=x.root(),bad={...copy(x.request),extra:true};assert.equal(x.prepare(bad),null);assert.equal(x.root(),before);assert(x.prepare());}
let profileCases=0;
for(const profile of [{unknown:true},{argumentLimits:{maxInputNodes:0}},{argumentLimits:{maxInputDepth:0}},{argumentLimits:{maxRequestBytes:0}},{argumentLimits:{maxIdentifierBytes:0}},{maxPlanBytes:0},{argumentLimits:{maxDependencies:-1}},{argumentLimits:{unknown:1}}]){const x=fixture('typed',{noPrepare:true,profile}),before=x.root(),usage=k.invocationUsage(x.domain);assert.equal(x.prepare(),null);assert.equal(x.root(),before);assert.deepEqual(k.invocationUsage(x.domain),usage);profileCases++;}
{const id='é'.repeat(15),x=fixture('typed',{noPrepare:true,documentId:id,profile:{argumentLimits:{maxIdentifierBytes:28}}});assert.equal(x.reference.length,28);assert(id.length<28&&Buffer.byteLength(id)>28);assert.equal(x.prepare(),null);profileCases++;}
{const dep={domain:'guard',identity:'é'.repeat(15),generation:1},x=fixture('typed',{noPrepare:true,profile:{argumentLimits:{maxIdentifierBytes:28}},request:{dependencies:[dep]},host:{contexts:[dep]}});assert.equal(x.prepare(),null);profileCases++;}
let malformedCases=0;
for(const field of ['contract','operation','invocation','profile','document','expectedSerialization','serialization','dependencies']){const x=fixture('typed',{noPrepare:true}),r=copy(x.request);delete r[field];const before=x.root(),usage=k.invocationUsage(x.domain);assert.equal(x.prepare(r),null);assert.equal(x.root(),before);assert.deepEqual(k.invocationUsage(x.domain),usage);assert(x.prepare());malformedCases++;}
for(const mutate of [r=>r.contract='unknown',r=>r.operation='supported-registry',r=>r.invocation='foreign',r=>r.profile='foreign',r=>r.expectedSerialization='1',r=>r.expectedSerialization=1.5,r=>r.expectedSerialization=NaN,r=>r.document.incarnation=0,r=>r.serialization.bom=1,r=>r.serialization.bom='false',r=>r.serialization.profile='unselected',r=>r.dependencies=[{domain:'x',identity:'x',generation:0}],r=>r.dependencies.push(r),r=>r.callback=()=>1]){const x=fixture('typed',{noPrepare:true}),r=copy(x.request),before=x.root(),usage=k.invocationUsage(x.domain);mutate(r);assert.equal(x.prepare(r),null);assert.equal(x.root(),before);assert.deepEqual(k.invocationUsage(x.domain),usage);malformedCases++;}
{const x=fixture('typed',{noPrepare:true}),r=copy(x.request);let calls=0;Object.defineProperty(r,'serialization',{enumerable:true,get(){calls++;throw Error('untrusted getter');}});assert.equal(x.prepare(r),null);assert.equal(calls,0);malformedCases++;}
let boundaryCases=0;
{const seed=fixture(),owner=k.sourceSerializationTestOwner(seed.domain),captured=sourceSerializationIntent(seed.plan,owner.limits.maxIntentBytes,owner,seed.domain,seed.ticket),size=v=>Buffer.byteLength(JSON.stringify(v)),cap=size({kind:'settings',plan:captured.value});
 for(const offset of [-1,0,1]){const x=fixture('typed',{noPrepare:true}),o=k.sourceSerializationTestOwner(x.domain),old=o.limits,root=x.root(),usage=k.invocationUsage(x.domain),held=k.invocationSemanticSettingsUsage(x.domain),slot=o.slots.get(x.ticket).settingsPrepared;
  try{o.limits={...old,maxIntentBytes:cap+offset};const plan=x.prepare();assert.equal(Boolean(plan),offset>=0,'parent/'+offset);assert.equal(x.root(),root);assert.deepEqual(k.invocationUsage(x.domain),usage);assert.deepEqual(k.invocationSemanticSettingsUsage(x.domain),held);
   if(plan){const c=sourceSerializationIntent(plan,o.limits.maxIntentBytes,o,x.domain,x.ticket);assert.equal(size({kind:'settings',plan:c.value}),cap);assert.equal(x.submit('typed',{kind:'settings',plan}).status,'accepted');}else assert.equal(o.slots.get(x.ticket).settingsPrepared,slot);boundaryCases++;
  }finally{o.limits=old;}
 }
}
for(const offset of [-1,0,1]){const x=fixture('typed',{noPrepare:true,initialBom:true,request:{serialization:{profile:'strict-utf8-one-bom/1',bom:false}}}),o=k.sourceSerializationTestOwner(x.domain),old=o.limits,root=x.root(),next=copy(root);next.publication++;next.documents[0].serialization={generation:2,profile:'strict-utf8-one-bom/1',bom:false};const cap=Buffer.byteLength(JSON.stringify(next));assert.equal(cap,Buffer.byteLength(JSON.stringify(root))+1);
 try{o.limits={...old,maxRootBytes:cap+offset};const plan=x.prepare();assert.equal(Boolean(plan),offset>=0,'root/'+offset);assert.equal(x.root(),root);if(plan){assert.equal(x.submit('typed',{kind:'settings',plan}).status,'accepted');assert.equal(x.run().status,'settled');assert.deepEqual(x.root(),next);}boundaryCases++;}finally{o.limits=old;}
}
{const seed=fixture('typed',{noPrepare:true}),value=()=>({request:seed.request,context:seed.c,profile:seed.p,required:[],dependencies:[]}),size=v=>Buffer.byteLength(JSON.stringify(v));let low=0,high=65536;assert(size(value())<high);while(low<high){const mid=Math.floor((low+high)/2);seed.p.maxPlanBytes=mid;if(mid<size(value()))low=mid+1;else high=mid;}const threshold=low;
 for(const offset of [-1,0,1]){const x=fixture('typed',{noPrepare:true,profile:{maxPlanBytes:threshold+offset}}),encoded=size({request:x.request,context:x.c,profile:x.p,required:[],dependencies:[]}),before=x.root();assert.equal(threshold+offset>=encoded,offset>=0);const plan=x.prepare();assert.equal(Boolean(plan),offset>=0,'owned-plan/'+offset);assert.equal(x.root(),before);boundaryCases++;}
}
{const x=fixture(),o=k.sourceSerializationTestOwner(x.domain),issued=k.invocationUsage(x.domain).issued;assert(sourceSerializationIntent(x.plan,o.limits.maxIntentBytes,o,x.domain,x.ticket));assert.equal(k.retireInvocation(x.domain,x.ticket,x.h()).status,'retired');assert.equal(sourceSerializationIntent(x.plan,o.limits.maxIntentBytes,o,x.domain,x.ticket),null);assert.equal(k.invocationUsage(x.domain).issued,issued);}
let targetCases=0;
for(const change of [v=>v.writeAllowed=false,v=>v.writeAllowed=1,v=>delete v.writeAllowed,v=>v.document={...v.document,incarnation:2},v=>v.context={...v.context,recipient:'another'}]){const x=fixture(),before=x.root(),grant=copy(x.h().serializationWriteContext);change(grant);assert.equal(x.submit().status,'accepted');assert.equal(x.run({serializationWriteContext:grant}).status,'settled');assert.equal(x.root(),before);assert.equal(k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome.publication,'notApplied');targetCases++;}
{const x=fixture(),j=k.issueInvocation(x.domain,x.c,x.h()),r={...copy(x.request),invocation:j.reference},p=k.prepareInvocationSourceSerialization(x.domain,j.ticket,x.envelope(r),x.p,x.c,x.h());assert(p);assert.equal(k.submitInvocation(x.domain,j.ticket,{kind:'settings',plan:p},'typed',x.c,x.controlPolicy,x.h()).status,'accepted');assert.equal(k.runInvocation(x.domain,j.ticket,x.c,x.h(),x.h()).status,'settled');const before=x.root();assert.equal(before.documents[0].serialization.generation,2);assert.equal(x.submit().status,'accepted');assert.equal(x.run().status,'settled');assert.equal(x.root(),before);assert.equal(k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome.publication,'notApplied');}
{const dependency={domain:'declared',identity:'meaning',generation:1},x=fixture('typed',{request:{dependencies:[dependency]},host:{contexts:[dependency]}}),before=x.root();assert.equal(x.submit().status,'accepted');assert.equal(x.run({contexts:[{...dependency,generation:2}]}).status,'settled');assert.equal(x.root(),before);assert.equal(k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome.publication,'notApplied');}
{const x=fixture(),before=x.root(),dependency={domain:'mandatory',identity:'new-contract',generation:1};assert.equal(x.submit().status,'accepted');assert.equal(x.run({requiredDependencies:[dependency],contexts:[dependency]}).status,'settled');assert.equal(x.root(),before);assert.equal(k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome.publication,'notApplied');}
{const x=fixture();assert.equal(x.submit().status,'accepted');const j=k.issueInvocation(x.domain,x.c,x.h()),d=x.root().documents[0],plan={contract:'owned-annotation-plan/1',invocation:j.reference,descriptor:j.reference,sources:[{document:d.document,expectedSource:1,expectedAnnotationSet:1,edits:[{lo:0,hi:1,expected:[97],ins:[98]}]}],annotations:[]};
 const h=x.h({parserSourceReserved:true,parserSourceLimits:{maxDocuments:8,maxRootBytes:4096,maxBatches:32,maxEdits:64,maxJournalBytes:65536},bounded:true,historyReady:true,sourceWritable:true,targetRestorationValid:true,completeIdentity:true,viewVersionsAvailable:true,maxParticipants:8,viewLimits:{maxRootBytes:4096,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:8192,maxTextBytes:4096,maxEditBytes:8192},viewWorkLimits:{maxViewBatches:32,maxViewEdits:64,maxViewBytes:65536,maxViewPlanBytes:65536}});
 assert.equal(k.submitInvocation(x.domain,j.ticket,{kind:'compound',plan},'typed',x.c,x.controlPolicy,x.h()).status,'accepted');assert.equal(k.runInvocation(x.domain,j.ticket,x.c,x.h(),h).status,'settled');assert.deepEqual(x.root().documents[0].source,[98]);const before=x.root(),journal=committedSourceTransforms(before);assert(journal.length>0);
 assert.equal(x.run().status,'settled');assert.equal(k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome.publication,'applied');assert.deepEqual(x.root().documents[0].source,before.documents[0].source);assert.equal(x.root().documents[0].revision,before.documents[0].revision);assert.deepEqual(x.root().groups,before.groups);assert.deepEqual(committedSourceTransforms(x.root()),journal);
}
return {scope:'owned-native-document-serialization-reference-only',publications,serviceCases,targetCases,profileCases,malformedCases,boundaryCases,counterCases,noConfigurationRegistryRequired:true,planDischargeWithoutIssuanceRefundChecked:true,staleSerializationChecked:true,declaredAndMandatoryDependencyRefusalsChecked:true,cancellationChecked:true,foreignPlansRejected:true,noOpChecked:true,malformedRecoveryChecked:true,actualSameDocumentSourceProgressChecked:true,actualNonemptyJournalCarried:true,policyHostOrIndependentRefinementProved:false};
}

export function checkSourceSerializationDecisions(){
const c=referenceContext,copy=structuredClone,document={authority:'a',id:'D',incarnation:1},profile='strict-utf8-one-bom/1';
const p={argumentLimits:Object.fromEntries(['maxDependencies','maxRequestBytes','maxIdentifierBytes','maxInputNodes','maxInputDepth'].map(k=>[k,parentProfile.argumentLimits[k]])),maxPlanBytes:parentProfile.maxPlanBytes};
const state=source=>({authority:'a',publication:1,documents:[{document,live:true,revision:7,source,serialization:{generation:2,profile,bom:true},annotationSet:3,records:[{body:'annotation'}],sourceBaseline:null}],groups:[{id:'history'}],frontiers:{},views:[]});
const request=bom=>({contract:'source-serialization/1',operation:'semantic-settings',invocation:'metadata',profile:c.profile,document,expectedSerialization:2,serialization:{profile,bom},dependencies:[]});
const host=s=>({...Object.fromEntries(sourceSerializationGates.map(k=>[k,true])),versionsAvailable:true,sourceSerializationBasis:s,sourceSerializationContext:c,sourceSerializationProfile:p,serializationWriteContext:{context:c,document,writeAllowed:true}});
const limits={maxImports:16,maxArtifacts:16,maxIssued:300000,maxHeldBytes:4194304,maxContextBytes:4096,maxInputBytes:8192,maxSourceBytes:8192,maxChunkBytes:8192,maxChunks:128,maxCandidateBytes:65536,maxOutputBytes:8192,maxArtifactBytes:131072,maxCompareBytes:16384,maxCoordinate:4294967295,maxReplyBytes:131072};
const codec=newSourceCodecRegistry(limits),g=Object.fromEntries(['authorized','sourceReadable','scope','continuous','live','notStopped','resourcesReady','retained','replyReady'].map(k=>[k,true]));assert(codec);
const capture=s=>captureSerializedSource(codec,s,{contract:'serialized-source/1',operation:'capture-source',document,expectedSource:s.documents[0].revision,expectedSerialization:s.documents[0].serialization.generation},g);
let frames=0,gateCases=0;
for(const source of [[],Array.from(Buffer.from('English\r\nDeutsch\n```mermaid\ngraph LR\n A-->B\n```')),Array.from(Buffer.from('\uFEFFliteral'))]){
 const before=state(source),saved=copy(before),a=capture(before);assert.equal(a.status,'captured');
 const out=decideSerialization(before,request(false),c,p,host(before));assert.equal(out.status,'applied');assert.deepEqual(before,saved);
 const expected=copy(before);expected.publication++;expected.documents[0].serialization={generation:3,profile,bom:false};assert.deepEqual(out.state,expected);
 const b=capture(out.state);if(source[0]===239){assert.equal(b.status,'unavailable');}else{assert.equal(b.status,'captured');assert.equal(compareSerializedSources(codec,a.artifact,b.artifact,g).comparison,'different');}
 assert.equal(decideSerialization(before,request(true),c,p,host(before)).state,before);frames++;
 const edited=copy(before);edited.documents[0].revision++;edited.documents[0].source.push(97);assert.equal(decideSerialization(edited,request(false),c,p,host(edited)).status,'applied');
 for(const key of sourceSerializationGates)for(const value of [false,1,undefined]){assert.equal(decideSerialization(before,request(false),c,p,{...host(before),[key]:value}),null);gateCases++;}
}
for(const field of ['generation','publication']){const s=state([97]);if(field==='generation')s.documents[0].serialization.generation=Number.MAX_SAFE_INTEGER;else s.publication=Number.MAX_SAFE_INTEGER;const r=request(false);r.expectedSerialization=s.documents[0].serialization.generation;assert.equal(decideSerialization(s,r,c,p,host(s)),null);r.serialization.bom=true;assert.equal(decideSerialization(s,r,c,p,host(s)).state,s);}
for(const mutate of [r=>r.extra=true,r=>r.expectedSerialization++,r=>r.document={...document,incarnation:2},r=>r.serialization.bom=1,r=>r.serialization.profile='optional-unselected']){const s=state([97]),r=copy(request(false));mutate(r);assert.equal(decideSerialization(s,r,c,p,host(s)),null);}
return {scope:'source-serialization-decision-reference-only',frames,gateCases,oldCapturesPreserved:true,ambiguousOutputStillRefusedAtCapture:true,sourceProgressDoesNotRetargetMetadata:true,nativeInvocationOrPolicyMappingProved:false};
}

export function checkSourceSerializationWriter(){
const referenceBounds=parentProfile;
const catalog=JSON.parse(fs.readFileSync(new URL('./invocation-publication.json',import.meta.url))),copy=structuredClone;
const x=nativeSourceSaveFixture(undefined,undefined,{raw:Array.from(Buffer.from('English\r\nDeutsch\n'))}),c=k.invocationAuthorityContext(x.domain);
assert(c);assert(!Object.hasOwn(x.root(),'engineSettings'));
const p={argumentLimits:Object.fromEntries(['maxDependencies','maxRequestBytes','maxIdentifierBytes','maxInputNodes','maxInputDepth'].map(key=>[key,referenceBounds.argumentLimits[key]])),maxPlanBytes:referenceBounds.maxPlanBytes};
const h=()=>x.g({...Object.fromEntries([...catalog.admissionGates,...catalog.releaseGates,'companionsReady','settingsWritable','versionsAvailable','outsidePublicationGuard','preparationReserved','ownerControlAllowed','retentionReleased','deliveryDrained'].map(key=>[key,true])),sourceSerializationBasis:x.root(),sourceSerializationContext:c,sourceSerializationProfile:p,requiredDependencies:[],contexts:[],serializationWriteContext:{context:c,document:x.document,writeAllowed:true}});
const before=copy(x.d()),oldTask=x.admit();assert.equal(inspectSourceOutput(x.writer,x.root(),x.document,x.g()).comparison,'equal');
const heldWriter=sourceSaveUsage(x.writer),heldCodec=sourceSaveCodecUsage(x.writer);
const i=k.issueInvocation(x.domain,c,h());assert(i);const request={contract:'source-serialization/1',operation:'semantic-settings',invocation:i.reference,profile:c.profile,document:x.document,expectedSerialization:before.serialization.generation,serialization:{profile:'strict-utf8-one-bom/1',bom:true},dependencies:[]};
const plan=k.prepareInvocationSourceSerialization(x.domain,i.ticket,{route:'typed',request:{family:15,variant:'semantic-settings',version:1,arguments:request}},p,c,h());assert(plan);
const policy={publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:c.recipient};
assert.equal(k.submitInvocation(x.domain,i.ticket,{kind:'settings',plan},'typed',c,policy,h()).status,'accepted');assert.equal(k.runInvocation(x.domain,i.ticket,c,h(),h()).status,'settled');
assert.deepEqual(x.d().source,before.source);assert.equal(x.d().revision,before.revision);assert.equal(x.d().serialization.generation,before.serialization.generation+1);assert.deepEqual(x.d().sourceBaseline,before.sourceBaseline);assert.equal(inspectSourceOutput(x.writer,x.root(),x.document,x.g()).comparison,'different');
assert.deepEqual(sourceSaveUsage(x.writer),heldWriter);assert.deepEqual(sourceSaveCodecUsage(x.writer),heldCodec);
assert.equal(x.dispatch(oldTask).status,'dispatched');const old=x.submitted.get(oldTask),payload=readSourceSavePayload(x.writer,oldTask,[0,before.sourceBaseline.bytes.length],{...x.g(),retained:true,provider:before.binding.provider,resource:before.binding.resource});assert.equal(payload.status,'serialized-bytes');assert.deepEqual(payload.bytes,before.sourceBaseline.bytes);assert.equal(old.capture.serialization.generation,before.serialization.generation);
assert.equal(x.written(oldTask).status,'recorded');assert.equal(x.baseline().status,'published');assert.equal(x.d().sourceBaseline.serializationGeneration,before.serialization.generation);assert.equal(x.d().serialization.generation,before.serialization.generation+1);assert.equal(inspectSourceOutput(x.writer,x.root(),x.document,x.g()).comparison,'different');
assert.equal(inspectSourceSave(x.writer,oldTask,x.g({retained:true})).quiescent,false);
assert.equal(x.observe(oldTask,'disposition').status,'recorded');assert.equal(x.observe(oldTask,'quiescence').status,'recorded');
const newTask=x.admit(x.request('new-format'));assert.equal(x.dispatch(newTask).status,'dispatched');const current=x.submitted.get(newTask);assert.equal(current.capture.serialization.generation,before.serialization.generation+1);assert.equal(current.capture.payloadByteLength,before.sourceBaseline.bytes.length+3);
const v={epoch:before.sourceBaseline.resourceVersion.epoch,token:'new-format-write'};assert.equal(x.written(newTask,v).status,'recorded');assert.equal(x.baseline().status,'published');assert.equal(x.d().sourceBaseline.serializationGeneration,x.d().serialization.generation);assert.equal(inspectSourceOutput(x.writer,x.root(),x.document,x.g()).comparison,'equal');assert.equal(x.d().revision,before.revision);assert.deepEqual(x.d().source,before.source);
return {scope:'source-serialization-writer-reference-only',oldCapturePreserved:true,lateOldAcknowledgmentRemainsHistorical:true,currentOutputChangesWithoutSourceRevision:true,newFormatAcknowledgmentMatchesCurrentOutput:true,quiescenceSeparate:true,physicalFilesystemOrPolicyTruthProved:false};
}

export function checkSourceSerializationFault(){
const x=sourceSerializationFixture(),before=x.root(),owner=k.sourceSerializationTestOwner(x.domain),issued=k.invocationUsage(x.domain).issued;
assert.equal(x.submit().status,'accepted');const host={...x.h()};Object.defineProperty(host,'sourceSerializationProfile',{get(){throw Error('injected trusted fact fault');}});
assert.equal(k.runInvocation(x.domain,x.ticket,x.c,x.h(),host).status,'faulted');assert.equal(x.root(),before);assert.equal(k.invocationUsage(x.domain).faulted,true);
let outcome=k.observeInvocation(x.domain,x.ticket,x.c,x.controlPolicy,x.h()).outcome;assert.equal(outcome.publication,'unknown');assert.equal(outcome.control,'faulted');
assert.deepEqual(k.stopInvocation(x.domain,x.ticket,x.h()),{status:'unavailable'});assert.equal(k.issueInvocation(x.domain,x.c,x.h()),null);assert.equal(k.invocationUsage(x.domain).issued,issued);
assert.equal(k.retireInvocation(x.domain,x.ticket,x.h()).status,'retired');assert.equal(sourceSerializationIntent(x.plan,owner.limits.maxIntentBytes,owner,x.domain,x.ticket),null);assert.equal(k.invocationUsage(x.domain).issued,issued);
return {scope:'source-serialization-fault-reference-only',actualCommonPublisherFaulted:true,unknownNotNotApplied:true,rootPreservedInThisInjectedTrace:true,newAdmissionFenced:true,protectedRetirementDischarged:true,issuanceNotRefunded:true,physicalContainmentOrAllFaultsProved:false};
}


const mutations=[["stale-metadata-condition","source-serialization.mjs","s.generation!==r.expectedSerialization","false"],["target-write-grant","source-serialization.mjs","grant.writeAllowed!==true","false"],["target-context-binding","source-serialization.mjs","||!equal(grant.context,c)",""],["declared-dependency-currentness","source-serialization-state.mjs","extra&&equal(extra,p.value.dependencies)","true"],["whole-parent-preflight","source-serialization-state.mjs","||size({kind:'settings',plan:data.value})>owner.limits.maxIntentBytes",""],["candidate-root-preflight","source-serialization-state.mjs","||size(d.state)>owner.limits.maxRootBytes",""],["retirement-discharge","invocation-publication.mjs","!forgetSourceSerialization(entry.settingsPrepared,o,domain,ticket)","!false"],["identifier-byte-bound","source-serialization.mjs","Buffer.byteLength(v)<=p.argumentLimits.maxIdentifierBytes","true"],["strict-current-grants","source-serialization.mjs","gates.every(k=>g?.[k]===true)","gates.every(k=>g?.[k])"],["unnecessary-configuration-registry","source-serialization-state.mjs","const d=decideSerialization(before,r,c,p,h)","if(!before.engineSettings)return null;const d=decideSerialization(before,r,c,p,h)"],["closed-product-profile","source-serialization.mjs","export function sourceSerializationProfileValid(p){return","export function sourceSerializationProfileValid(p){return true||"]];
export const sourceSerializationMutationNames=Object.freeze(mutations.map(m=>m[0]));
export async function checkSourceSerialization(){
 const root=fileURLToPath(new URL('..',import.meta.url)),inputs=projectInputs(root),directory=fs.mkdtempSync(path.join(os.tmpdir(),'vellum-source-serialization-checks-'));
 const accessor='export function sourceSerializationTestOwner(domain){return domains.get(domain);}';
 let index=0;
 const graph=mutation=>{
  const target=path.join(directory,String(index++));fs.mkdirSync(target);fs.mkdirSync(path.join(target,'formal'));
  for(const file of inputs)fs.copyFileSync(path.join(root,file),path.join(target,file));
  const kernel=path.join(target,'formal/invocation-publication.mjs'),original=fs.readFileSync(kernel,'utf8');assert(!original.includes(accessor));fs.writeFileSync(kernel,original+'\n'+accessor+'\n');
  if(mutation){const file=path.join(target,'formal',mutation[1]),source=fs.readFileSync(file,'utf8');assert.equal(source.split(mutation[2]).length,2,mutation[0]);fs.writeFileSync(file,source.replace(mutation[2],mutation[3]));}
  return import(pathToFileURL(path.join(target,'formal/source-serialization-checks.mjs')).href);
 };
 try{
  const baseline=await graph(null),native=baseline.checkSourceSerializationReference(),pure=baseline.checkSourceSerializationDecisions(),writer=baseline.checkSourceSerializationWriter(),fault=baseline.checkSourceSerializationFault();
  const killed=[];
  for(const mutation of mutations){const api=await graph(mutation);let failed=false;try{api.checkSourceSerializationReference();}catch(e){assert.equal(e.code,'ERR_ASSERTION',mutation[0]+': '+String(e.message).slice(0,120));failed=true;}assert(failed,mutation[0]);killed.push(mutation[0]);}
  return {scope:'owned-native-source-serialization-only',native,pure,writer,fault,mutationBaselineChecked:true,mutations:killed,testAccessorOnlyInGeneratedGraphs:true,actualOwnedPlanAndCommonSettingsJobChecked:true,physicalAccountingOrHostPolicyTruthProved:false,publicWireOrCompleteSettingsFamilyProved:false,concurrentCrashOrIndependentRefinementProved:false};
 }finally{fs.rmSync(directory,{recursive:true,force:true});}
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(await checkSourceSerialization()));}catch(e){console.error(String(e.stack??e));process.exitCode=1;}
