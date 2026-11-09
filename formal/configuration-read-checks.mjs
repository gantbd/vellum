// Scoped reference evidence, not actual host policy or complete consumer discovery.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import * as reads from './read-dependencies.mjs';
import {queryProposalValidation} from './proposal-validation.mjs';
import {compileEngineSettings} from './engine-settings-state.mjs';
import {engineSettingsDefinition,engineSettingsProfile as profile} from './engine-settings-checks.mjs';
import {invocationPublicationFixture as parent,invocationCompoundJob as compound} from './invocation-publication-checks.mjs';
const copy=structuredClone,id=n=>({authority:'a',id:n,incarnation:1}),key=r=>JSON.stringify([r.authority,r.id,r.incarnation]);
const C=JSON.parse(fs.readFileSync(new URL('./invocation-publication.json',import.meta.url)));
const flags=Object.fromEntries([...C.admissionGates,...C.releaseGates,'companionsReady','settingsWritable','outsidePublicationGuard','preparationReserved','ownerControlAllowed','retentionReleased','deliveryDrained','versionsAvailable','schedulerOwned','reservationHeld'].map(k=>[k,true]));
const policy={publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'};
const limits={maxDependencies:16,maxIdentifierBytes:256};
function setup(){
 const root=copy(parent().root());root.engineSettings=compileEngineSettings(engineSettingsDefinition(),'a',profile);assert(root.engineSettings);
 const z=parent(kernel,root,{maxSlots:8,maxIssued:32,maxIntentBytes:65536,maxResultBytes:4096,maxRootBytes:262144,maxHeldBytes:8388608});
 const dependency=(domain='source',target='TA')=>{const c=z.root().engineSettings,base={expectedSchema:c.schemaGeneration,expectedRestrictions:c.restrictionGeneration,expectedAssociation:c.associationGeneration};return domain==='state'?{kind:'configuration-state',authority:'a',...base,expectedConfiguration:c.generation}:{kind:'configuration-domain',target:id(target),domain,...base,expectedGeneration:c.domains[key(id(target))][domain]};};
 const job=deps=>{const j=compound(z.reference,z.root());j.plan.contract='owned-annotation-plan/4';j.plan.reads=copy(deps);j.plan.scopes=[{profile:'bounded-source-scope/1',document:id('A'),expectedSource:1,replacementRanges:[{lo:0,hi:1}],insertionSites:[]}];return j;};
 const host=deps=>({...z.h,parserSourceLimits:{...z.h.parserSourceLimits,maxRootBytes:262144},sourceScopeLimits:{maxScopes:8,maxReplacementRanges:16,maxInsertionIntervals:16,maxIdentifierBytes:256},configurationReadProfile:copy(profile),readDependencyLimits:copy(limits),readDependencyAuthority:{contract:'native-read-authority/2',context:copy(z.c),grants:deps.map(dependency=>({dependency:copy(dependency),readAllowed:true,presenceAllowed:true,equalityAllowed:true}))}});
 const activate=(layer,setting,value)=>{
  const issued=kernel.issueInvocation(z.domain,z.c,flags);assert(issued);const before=z.root(),request={contract:'semantic-settings-activation/1',operation:'semantic-settings',invocation:issued.reference,profile:z.c.profile,changes:[{layer:id(layer),expectedGeneration:before.engineSettings.layers.find(l=>l.layer.id===layer).generation,key:setting,operation:'set',value}],dependencies:[]};
  const h={...flags,semanticSettingsProfile:profile,semanticSettingsContext:z.c,semanticSettingsBasis:before,settingsWritableLayers:before.engineSettings.layers.map(l=>l.layer),requiredDependencies:[],contexts:[]};
  const plan=kernel.prepareInvocationSemanticSettings(z.domain,issued.ticket,{route:'typed',request:{family:15,variant:'semantic-settings',version:1,arguments:request}},profile,z.c,h);assert(plan);
  assert.equal(kernel.submitInvocation(z.domain,issued.ticket,{kind:'settings',plan},'typed',z.c,policy,flags).status,'accepted');assert.equal(kernel.runInvocation(z.domain,issued.ticket,z.c,flags,h).status,'settled');assert.equal(kernel.observeInvocation(z.domain,issued.ticket,z.c,policy,flags).outcome.publication,'applied');
 };
 return {...z,dependency,job,host,activate};
}
export function checkConfigurationReads(api=reads){
 const controls=[],control=(name,f)=>{f();controls.push(name);};let predicateCases=0,permissionCases=0,interleavingCases=0;
 control('four-current-owner-predicates-and-stale-versions',()=>{for(const domain of ['source','serialization','analysis','state'])for(const route of ['typed','dynamic']){
  const z=setup(),d=z.dependency(domain),h=z.host([d]);assert(api.declaredReadsCurrent(z.root(),[d],h.readDependencyAuthority,z.c,limits,'owned-annotation-plan/4',profile));
  const stale=copy(d);stale[domain==='state'?'expectedConfiguration':'expectedGeneration']++;const sh=z.host([stale]);assert.equal(api.declaredReadsCurrent(z.root(),[stale],sh.readDependencyAuthority,z.c,limits,'owned-annotation-plan/4',profile),false);
  const j=z.job([d]);assert.equal(z.submit(j,route).status,'accepted');z.run(flags,h);assert.equal(z.read().outcome.publication,'applied');assert.deepEqual(z.root().engineSettings,h.configurationReadProfile&&compileEngineSettings(engineSettingsDefinition(),'a',profile));predicateCases+=2;
 }});
 control('exact-complete-predicate-observation-grants',()=>{for(const domain of ['source','serialization','analysis','state'])for(let mask=0;mask<8;mask++){
  const z=setup(),d=z.dependency(domain),h=z.host([d]);['readAllowed','presenceAllowed','equalityAllowed'].forEach((k,i)=>h.readDependencyAuthority.grants[0][k]=!!(mask&(1<<i)));
  assert.equal(api.declaredReadsCurrent(z.root(),[d],h.readDependencyAuthority,z.c,limits,'owned-annotation-plan/4',profile),mask===7);permissionCases++;
 }});
 control('actual-intervening-settings-publication-and-semantic-frames',()=>{
  for(const [layer,setting,value,current] of [['w1','scalar',2,false],['w2','scalar',2,true],['w1','list',['next'],true],['w1','map',{left:true},true],['w1','scalar',1,true]]){
   const z=setup(),d=z.dependency(),whole=z.dependency('state'),j=z.job([d]),h=z.host([d]);assert.equal(z.submit(j).status,'accepted');z.activate(layer,setting,value);
   const before=z.root(),wh=z.host([whole]);assert.equal(api.declaredReadsCurrent(before,[whole],wh.readDependencyAuthority,z.c,limits,'owned-annotation-plan/4',profile),false);
   z.run(flags,h);assert.equal(z.read().outcome.publication,current?'applied':'notApplied');if(!current)assert.equal(z.root(),before);interleavingCases++;
  }
 });
 control('restored-values-do-not-restore-old-domain-versions',()=>{const z=setup(),d=z.dependency(),j=z.job([d]),h=z.host([d]);z.submit(j);z.activate('w1','scalar',2);z.activate('w1','scalar',1);assert.equal(z.root().engineSettings.domains[key(id('TA'))].source,3);const before=z.root();z.run(flags,h);assert.equal(z.read().outcome.publication,'notApplied');assert.equal(z.root(),before);});
 control('interpretation-generations-and-profile-versions-are-not-optional',()=>{const z=setup(),d=z.dependency();for(const k of ['expectedSchema','expectedRestrictions','expectedAssociation']){const bad={...d,[k]:d[k]+1},h=z.host([bad]);assert.equal(api.declaredReadsCurrent(z.root(),[bad],h.readDependencyAuthority,z.c,limits,'owned-annotation-plan/4',profile),false);}for(const old of ['owned-annotation-plan/2','owned-annotation-plan/3','owned-annotation-plan/6'])assert.equal(api.validReadDependencies([d],limits,old),false);assert.equal(api.validReadDependencies([d,d],limits,'owned-annotation-plan/4'),false);for(const bad of [{...d,extra:true},{...d,expectedGeneration:0},{...d,domain:'ui'}])assert.equal(api.validReadDependencies([bad],limits,'owned-annotation-plan/4'),false);});
 control('stale-configuration-noop-is-refused-before-noop-classification',()=>{const z=setup(),d=z.dependency(),j=z.job([d]),h=z.host([d]);j.plan.sources=[];j.plan.annotations=[];j.plan.scopes=[];z.submit(j);z.activate('w1','scalar',2);const before=z.root();z.run(flags,h);assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'notApplied');});
 control('identity-permission-context-and-actual-configuration-are-required',()=>{
  const z=setup(),d=z.dependency();for(const change of [{target:id('missing')},{target:{...id('TA'),incarnation:2}},{target:{...id('TA'),authority:'foreign'}}]){const r={...d,...change},h=z.host([r]);assert.equal(api.declaredReadsCurrent(z.root(),[r],h.readDependencyAuthority,z.c,limits,'owned-annotation-plan/4',profile),false);}
  for(const field of Object.keys(z.c)){const h=z.host([d]);h.readDependencyAuthority.context[field]='other';assert.equal(api.declaredReadsCurrent(z.root(),[d],h.readDependencyAuthority,z.c,limits,'owned-annotation-plan/4',profile),false);}
  const h=z.host([d]);for(const contract of ['native-read-authority/1','unknown'])assert.equal(api.declaredReadsCurrent(z.root(),[d],{...h.readDependencyAuthority,contract},z.c,limits,'owned-annotation-plan/4',profile),false);
  for(const p of [null,{...profile,maxStateBytes:0}])assert.equal(api.declaredReadsCurrent(z.root(),[d],h.readDependencyAuthority,z.c,limits,'owned-annotation-plan/4',p),false);
  const missing=copy(z.root());delete missing.engineSettings;assert.equal(api.declaredReadsCurrent(missing,[d],h.readDependencyAuthority,z.c,limits,'owned-annotation-plan/4',profile),false);
 });
 control('mixed-read-permutations-share-intent-and-replay-does-not-reexecute',()=>{const z=setup(),deps=[z.dependency(),{kind:'source',document:id('B'),expectedSource:1}],j=z.job(deps),h=z.host(deps);assert.equal(z.submit(j).status,'accepted');assert.equal(z.submit({...j,plan:{...j.plan,reads:copy(deps).reverse()}},'dynamic').status,'pending');z.run(flags,h);assert.equal(z.read().outcome.publication,'applied');assert.equal(z.root().groups.length,1);z.activate('w1','scalar',2);const before=z.root();assert.equal(z.submit(j,'dynamic').status,'observe-existing');assert.equal(z.run(flags,h).status,'unavailable');assert.equal(z.root(),before);assert.equal(z.read().outcome.publication,'applied');});
 control('empty-configuration-conjunction-needs-no-configuration-service',()=>{const z=parent(),j=compound(z.reference,z.root());j.plan.contract='owned-annotation-plan/4';j.plan.reads=[];j.plan.scopes=[{profile:'bounded-source-scope/1',document:id('A'),expectedSource:1,replacementRanges:[{lo:0,hi:1}],insertionSites:[]}];assert.equal(z.submit(j).status,'accepted');z.run(flags,{...z.h,sourceScopeLimits:{maxScopes:8,maxReplacementRanges:16,maxInsertionIntervals:16,maxIdentifierBytes:256}});assert.equal(z.read().outcome.publication,'applied');});
 control('advisory-query-uses-the-same-current-owned-configuration',()=>{const z=setup(),d=z.dependency(),j=z.job([d]),h=z.host([d]),q=JSON.parse(fs.readFileSync(new URL('./proposal-validation.json',import.meta.url))),g=Object.fromEntries(q.gates.map(k=>[k,true])),routing={schemaValid:true,enabled:true,grant:true,current:true,ordinaryCapacity:true,releaseAllowed:true};
  const query=()=>{const proposal={contract:q.contract,operation:q.operation,plan:j.plan},before=z.root(),usage=z.usage(),out=queryProposalValidation(z.domain,{route:'typed',request:{family:9,variant:q.operation,version:1},proposal},z.c,routing,g,{...h,proposalValidationBasis:{root:before,context:copy(z.c),request:copy(proposal)}});assert.equal(z.root(),before);assert.deepEqual(z.usage(),usage);return out;};
  assert.equal(query().checksSatisfied,true);z.activate('w1','scalar',2);assert.equal(query().checksSatisfied,false);
 });
 return {scope:'owned-configuration-read-predicates-reference',predicateCases,permissionCases,interleavingCases,controls,actualNativePublicationChecked:true,actualPolicyTruthProved:false,completeConsumerDiscoveryProved:false,implementationRefinement:false,independentCleanroomCertification:false};
}
export async function checkConfigurationReadMutations(){
 const raw=fs.readFileSync(new URL('./read-dependencies.mjs',import.meta.url),'utf8'),mutations=[
  ['schema-version-ignored','cfg.schemaGeneration!==r.expectedSchema','false'],
  ['restriction-version-ignored','cfg.restrictionGeneration!==r.expectedRestrictions','false'],
  ['association-version-ignored','cfg.associationGeneration!==r.expectedAssociation','false'],
  ['whole-configuration-version-ignored','cfg.generation!==r.expectedConfiguration','false'],
  ['domain-version-ignored','cfg.domains[k][r.domain]!==r.expectedGeneration','false']
 ];
 const load=source=>import('data:text/javascript;base64,'+Buffer.from(source.replace(/(['"])(\.\/[^'"\n]+)\1/g,(_,q,p)=>q+new URL(p,import.meta.url).href+q)).toString('base64'));
 checkConfigurationReads(await load(raw));const killed=[];for(const [name,from,to] of mutations){assert(raw.includes(from),name);let failed=false;const loadedMutationApi=await load(raw.replace(from,to));try{checkConfigurationReads(loadedMutationApi);}catch(e){assert.equal(e.code,'ERR_ASSERTION',name+': '+e.message);failed=true;}assert(failed,name);killed.push(name);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkConfigurationReads(),mutations:await checkConfigurationReadMutations(),mutationBaselineChecked:true},null,2));
