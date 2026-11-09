// Selected native settings conformance; not all registry/consumer/host truth.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import * as settings from './engine-settings-state.mjs';
import {committedSourceTransforms} from './parser-source.mjs';
const original={...kernel,...settings},copy=structuredClone,id=n=>({authority:'a',id:n,incarnation:1}),key=r=>JSON.stringify([r.authority,r.id,r.incarnation]);
const C=JSON.parse(fs.readFileSync(new URL('./invocation-publication.json',import.meta.url)));
export const engineSettingsContext={authority:'a',engine:'settings-engine',recipient:'reader',purpose:'edit',destination:'local',profile:'execution',continuity:'c1'};
export const engineSettingsPolicy={publication:true,control:true,effect:true,reasons:false,details:false,recovery:false,grant:true,epoch:1,recipient:'reader'};
const good=Object.fromEntries([...C.admissionGates,...C.releaseGates,'companionsReady','settingsWritable','settingsReadable','outsidePublicationGuard','preparationReserved','ownerControlAllowed','retentionReleased','deliveryDrained','versionsAvailable','uniqueDomainAttested','schedulerOwned','reservationHeld'].map(k=>[k,true]));
export const engineSettingsProfile={argumentLimits:{maxChanges:16,maxDependencies:8,maxRequestBytes:65536,maxValueBytes:16384,maxIdentifierBytes:1024,maxInputNodes:16384,maxInputDepth:128},maxStateBytes:131072,maxPlanBytes:65536,maxKeys:16,maxLayers:8,maxTargets:8,maxSchemaNodes:256,maxSchemaDepth:32,maxContexts:4,maxIssuedContexts:16,maxContextBytes:131072,maxHeldBytes:1048576};
export function engineSettingsDefinition(){return {
 schemas:[{key:'scalar',type:{kind:'integer',min:0,max:8},default:1,domain:'source',scopes:['host','workspace','document'],composition:'replace',sensitivity:true},
  {key:'list',type:{kind:'list',element:{kind:'text'}},default:['base'],domain:'serialization',scopes:['host','workspace','document'],composition:'replace',sensitivity:true},
  {key:'map',type:{kind:'map',fields:{left:{kind:'boolean'},right:{kind:'integer',min:0,max:8}}},default:{left:false,right:1},domain:'analysis',scopes:['host','workspace','document'],composition:'merge-map',sensitivity:true}],restrictions:{scalar:[0,1,2,3,4,5,6,7,8]},
 layers:[['h','host'],['w1','workspace'],['w2','workspace'],['d','document']].map(([n,kind])=>({layer:id(n),kind,generation:1,values:{}})),targets:[{target:id('TA'),layers:['h','w1','d'].map(id)},{target:id('TB'),layers:['h','w2'].map(id)}]
};}
export function engineSettingsFixture(api=original,options={}){
 const p=copy({...engineSettingsProfile,...options.profile}),definition=options.definition??engineSettingsDefinition(),compiler=api.compileEngineSettings??settings.compileEngineSettings,configuration=compiler(definition,'a',p);assert(configuration);
 const c=copy(engineSettingsContext),policy=copy(engineSettingsPolicy),state=options.state??{authority:'a',publication:0,documents:[{document:id('A'),live:true,source:[97],revision:1,annotationSet:1,records:[]}],groups:[],frontiers:{},views:[],engineSettings:configuration};
 const domain=api.openInvocationDomain(state,c,{maxSlots:8,maxIssued:32,maxIntentBytes:65536,maxResultBytes:4096,maxRootBytes:262144,maxHeldBytes:8388608,...options.limits},good);assert(domain);const issued=api.issueInvocation(domain,c,good);assert(issued);
 const root=()=>api.invocationAuthorityRoot(domain),host=(extra={})=>({...good,semanticSettingsProfile:p,semanticSettingsContext:c,semanticSettingsBasis:root(),settingsWritableLayers:['h','w1','w2','d'].map(id),requiredDependencies:[],contexts:[],...options.host,...extra});
 const request={contract:'semantic-settings-activation/1',operation:'semantic-settings',invocation:issued.reference,profile:c.profile,changes:[{layer:id('w1'),expectedGeneration:1,key:'scalar',operation:'set',value:2}],dependencies:[],...options.request};
 const envelope=route=>({route,request:route==='typed'?{family:15,variant:'semantic-settings',version:1,arguments:request}:{name:'activate-context',variant:'semantic-settings',version:1,arguments:request}});
 const plan=api.prepareInvocationSemanticSettings(domain,issued.ticket,envelope(options.route??'typed'),p,c,host()),job={kind:'settings',plan};
 return {domain,...issued,request,envelope,plan,job,p,host,root,controlContext:c,controlPolicy:policy,
  submit:(j=job,route=options.route??'typed',g=good)=>api.submitInvocation(domain,issued.ticket,j,route,c,policy,g),
  run:(h=host(),g=good)=>api.runInvocation(domain,issued.ticket,c,g,h),read:(q=policy,g=good)=>api.observeInvocation(domain,issued.ticket,c,q,g),usage:()=>api.invocationUsage(domain)};
}
export function checkEngineSettings(api=original){
 const controls=[],control=(name,f)=>{f();controls.push(name);};let products=0,declarationCases=0,malformedCases=0,budgetCases=0,holdCases=0;
 const compile=api.compileEngineSettings??settings.compileEngineSettings;
 control('typed-and-dynamic-use-real-owned-root-and-one-publication',()=>{
  for(const route of ['typed','dynamic']){const z=engineSettingsFixture(api,{route}),before=z.root();assert(z.plan);assert.equal(z.submit().status,'accepted');assert.equal(z.run().status,'settled');assert.equal(z.read().outcome.publication,'applied');assert.equal(z.root().publication,1);assert.equal(z.root().engineSettings.resolved[key(id('TA'))].scalar.value,2);assert.equal(z.root().engineSettings.layers[1].generation,2);assert.equal(z.root().engineSettings.domains[key(id('TA'))].source,2);assert.equal(z.root().engineSettings.domains[key(id('TB'))].source,1);assert.equal(z.root().documents,before.documents);assert.equal(z.root().groups,before.groups);assert.equal(z.root().views,before.views);products++;}
 });
 control('no-op-does-not-need-unused-version-capacity',()=>{
  const z=engineSettingsFixture(api,{request:{changes:[]}}),root=z.root();assert(z.plan);assert.equal(z.submit().status,'accepted');assert.equal(z.run(z.host({versionsAvailable:false})).status,'settled');assert.equal(z.read().outcome.publication,'unchanged');assert.equal(z.root(),root);
 });
 control('same-preparation-retries-preserve-owned-intent-without-refresh',()=>{
  const z=engineSettingsFixture(api),usage=copy(z.usage()),held=copy(api.invocationSemanticSettingsUsage(z.domain));
  assert.equal(api.prepareInvocationSemanticSettings(z.domain,z.ticket,z.envelope('dynamic'),z.p,z.controlContext,z.host()),z.plan);assert.deepEqual(z.usage(),usage);assert.deepEqual(api.invocationSemanticSettingsUsage(z.domain),held);
  const e=z.envelope('typed');e.request.arguments={...z.request,changes:[{...z.request.changes[0],value:3}]};assert.equal(api.prepareInvocationSemanticSettings(z.domain,z.ticket,e,z.p,z.controlContext,z.host()),null);
 });
 control('independent-layer-work-merges-but-a-stale-same-layer-refuses',()=>{
  for(const layer of ['w1','w2']){
   const z=engineSettingsFixture(api);assert.equal(z.submit().status,'accepted');const i=api.issueInvocation(z.domain,z.controlContext,good);assert(i);
   const e=z.envelope('typed');e.request.arguments={...z.request,invocation:i.reference,changes:[{...z.request.changes[0],layer:id(layer),value:3}]};
   const p=api.prepareInvocationSemanticSettings(z.domain,i.ticket,e,z.p,z.controlContext,z.host());assert(p);assert.equal(api.submitInvocation(z.domain,i.ticket,{kind:'settings',plan:p},'dynamic',z.controlContext,z.controlPolicy,good).status,'accepted');assert.equal(api.runInvocation(z.domain,i.ticket,z.controlContext,good,z.host()).status,'settled');
   const before=z.root();assert.equal(z.run().status,'settled');assert.equal(z.read().outcome.publication,layer==='w1'?'notApplied':'applied');if(layer==='w1')assert.equal(z.root(),before);else{assert.equal(z.root().engineSettings.resolved[key(id('TA'))].scalar.value,2);assert.equal(z.root().engineSettings.resolved[key(id('TB'))].scalar.value,3);assert.equal(z.root().publication,2);}
  }
 });
 control('explicit-equal-default-origin-is-not-an-empty-change',()=>{
  const z=engineSettingsFixture(api,{request:{changes:[{layer:id('w1'),expectedGeneration:1,key:'scalar',operation:'set',value:1}]}});assert(z.plan);assert.equal(z.submit().status,'accepted');assert.equal(z.run().status,'settled');assert.equal(z.read().outcome.publication,'applied');assert.equal(z.root().engineSettings.generation,2);assert.equal(z.root().engineSettings.domains[key(id('TA'))].source,1);
 });
 control('a-changed-contributing-layer-stales-the-whole-captured-resolution',()=>{
  const z=engineSettingsFixture(api);assert.equal(z.submit().status,'accepted');const i=api.issueInvocation(z.domain,z.controlContext,good),e=z.envelope('typed');
  e.request.arguments={...z.request,invocation:i.reference,changes:[{layer:id('h'),expectedGeneration:1,key:'list',operation:'set',value:['peer']}]};const p=api.prepareInvocationSemanticSettings(z.domain,i.ticket,e,z.p,z.controlContext,z.host());assert(p);assert.equal(api.submitInvocation(z.domain,i.ticket,{kind:'settings',plan:p},'typed',z.controlContext,z.controlPolicy,good).status,'accepted');assert.equal(api.runInvocation(z.domain,i.ticket,z.controlContext,good,z.host()).status,'settled');const root=z.root();assert.equal(z.run().status,'settled');assert.equal(z.read().outcome.publication,'notApplied');assert.equal(z.root(),root);
 });
 control('remove-inherits-reset-selects-library-and-maps-merge-only-declared-fields',()=>{
  for(const operation of ['remove','reset']){const d=engineSettingsDefinition();d.layers[0].values.scalar={kind:'set',value:3};d.layers[1].values.scalar={kind:'set',value:2};const z=engineSettingsFixture(api,{definition:d,request:{changes:[{layer:id('w1'),expectedGeneration:1,key:'scalar',operation,value:null}]}});assert(z.plan);z.submit();z.run();assert.equal(z.read().outcome.publication,'applied');assert.equal(z.root().engineSettings.resolved[key(id('TA'))].scalar.value,operation==='remove'?3:1);}
  const z=engineSettingsFixture(api,{request:{changes:[{layer:id('w1'),expectedGeneration:1,key:'map',operation:'set',value:{left:true}}]}});assert(z.plan);z.submit();z.run();assert.deepEqual(z.root().engineSettings.resolved[key(id('TA'))].map.value,{left:true,right:1});
  for(const value of [{right:9},{hidden:1},{left:'true'}])assert.equal(engineSettingsFixture(api,{request:{changes:[{layer:id('w1'),expectedGeneration:1,key:'map',operation:'set',value}]}}).plan,null);
 });
 control('new-context-holds-cannot-overcommit-the-actual-parent-budget',()=>{
  const z=engineSettingsFixture(api),held=api.invocationSemanticSettingsUsage(z.domain),parent=z.usage().declaredHeldBytes-held.declaredHeldBytes;
  const denied=engineSettingsFixture(api,{limits:{maxHeldBytes:parent}});assert.equal(denied.plan,null);assert.equal(api.invocationSemanticSettingsUsage(denied.domain),null);assert.equal(denied.root().publication,0);
 });
 control('same-rank-conflict-cannot-be-hidden-by-a-higher-layer',()=>{
  const d=engineSettingsDefinition();d.layers[1].values.scalar={kind:'set',value:2};d.layers[2].values.scalar={kind:'set',value:3};d.layers[3].values.scalar={kind:'set',value:4};d.targets[0].layers=['h','w1','w2','d'].map(id);assert.equal((api.compileEngineSettings??settings.compileEngineSettings)(d,'a',engineSettingsProfile),null);
 });
 control('unknown-ui-keys-and-illegal-engine-types-refuse',()=>{
  for(const extra of [{key:'theme'},{value:9},{value:'2'},{layer:id('foreign')},{expectedGeneration:2}]){const change={layer:id('w1'),expectedGeneration:1,key:'scalar',operation:'set',value:2,...extra},z=engineSettingsFixture(api,{request:{changes:[change]}});assert.equal(z.plan,null);assert.equal(z.root().publication,0);}
  for(const domain of ['view','input','host']){const d=engineSettingsDefinition();d.schemas[0].domain=domain;assert.equal((api.compileEngineSettings??settings.compileEngineSettings)(d,'a',engineSettingsProfile),null);}
 });
 control('copied-foreign-or-another-ticket-plan-has-no-route',()=>{
  const a=engineSettingsFixture(api),b=engineSettingsFixture(api);assert.deepEqual(a.submit({...a.job,plan:copy(a.plan)}),{status:'unavailable'});assert.deepEqual(b.submit(a.job),{status:'unavailable'});
  const issued=api.issueInvocation(a.domain,a.controlContext,good);assert(issued);assert.deepEqual(api.submitInvocation(a.domain,issued.ticket,a.job,'typed',a.controlContext,a.controlPolicy,good),{status:'unavailable'});
 });
 control('current-authority-context-and-basis-cannot-be-replaced',()=>{
  for(const extra of [{settingsWritable:false},{settingsWritableLayers:[]},{versionsAvailable:false},{semanticSettingsContext:{...engineSettingsContext,recipient:'other'}},{semanticSettingsBasis:{}},{semanticSettingsProfile:{...engineSettingsProfile,maxKeys:0}},{requiredDependencies:[{domain:'extra',identity:'new',generation:1}]}]){const z=engineSettingsFixture(api),before=z.root();assert.equal(z.submit().status,'accepted');assert.equal(z.run(z.host(extra)).status,'settled');assert.equal(z.read().outcome.publication,'notApplied');assert.equal(z.root(),before);}
 });
 control('retained-context-is-an-actual-owned-hold-not-a-supplied-boolean',()=>{
  const z=engineSettingsFixture(api),g={...good},handle=api.acquireInvocationSemanticContext(z.domain,z.controlContext,z.p,g);assert(handle);
  const before=api.readInvocationSemanticContext(z.domain,handle,z.controlContext,g);assert(before);assert(Object.isFrozen(before));assert.equal(z.submit().status,'accepted');assert.equal(z.run().status,'settled');assert.equal(api.readInvocationSemanticContext(z.domain,handle,z.controlContext,g),before);assert.equal(before.resolved[key(id('TA'))].scalar.value,1);assert.equal(z.root().engineSettings.resolved[key(id('TA'))].scalar.value,2);
  assert.equal(api.retireInvocationSemanticSettingsPool(z.domain,g),false);assert.equal(api.releaseInvocationSemanticContext(z.domain,copy(handle),z.controlContext,g),false);assert.equal(api.releaseInvocationSemanticContext(z.domain,handle,z.controlContext,{...g,retentionReleased:false}),false);assert.equal(api.releaseInvocationSemanticContext(z.domain,handle,z.controlContext,g),true);
  assert.equal(api.retireInvocationSemanticSettingsPool(z.domain,g),false);assert.equal(api.retireInvocation(z.domain,z.ticket,g).status,'retired');assert.equal(api.retireInvocationSemanticSettingsPool(z.domain,g),true);assert.equal(api.acquireInvocationSemanticContext(z.domain,z.controlContext,z.p,g),null);
 });
 control('normalized-json-equality-is-not-wire-or-source-byte-equality',()=>{
  const equal=api.equalEngineSettingValues??settings.equalEngineSettingValues;assert.equal(equal({a:1,b:[0]},{b:[-0],a:1},engineSettingsProfile),true);assert.equal(equal(['a','b'],['b','a'],engineSettingsProfile),false);assert.equal(equal('é','e\u0301',engineSettingsProfile),false);
 });
 control('all-six-closed-types-and-mathematical-enums-compile-without-aliases',()=>{
  const types=[[{kind:'boolean'},true],[{kind:'integer',min:-8,max:8},0],[{kind:'text'},'é\u0000'],[{kind:'enum',values:[null,{a:1},['a']]},null],[{kind:'list',element:{kind:'boolean'}},[true,false]],[{kind:'map',fields:{x:{kind:'boolean'}}},{x:true}]];
  for(const [type,value]of types){const d=engineSettingsDefinition();d.schemas[0].type=type;d.schemas[0].default=value;d.restrictions={};const result=compile(d,'a',engineSettingsProfile);assert(result);assert(Object.isFrozen(result));assert.deepEqual(result.resolved[key(id('TA'))].scalar.value,value);assert.notEqual(result.schemas,d.schemas);declarationCases++;
   const bad=copy(d);bad.schemas[0].type.callback='run';assert.equal(compile(bad,'a',engineSettingsProfile),null);malformedCases++;
  }
  const d=engineSettingsDefinition(),out=compile(d,'a',engineSettingsProfile);d.schemas[2].default.left=true;assert.equal(out.schemas[2].default.left,false);
 });
 control('malformed-declarations-refuse-as-data-without-running-accessors',()=>{
  const badTypes=[null,{},[],{kind:'script'},{kind:'boolean',callback:'run'},{kind:'integer',min:4,max:1},{kind:'integer',min:0.5,max:8},{kind:'enum',values:[]},{kind:'enum',values:[0,-0]},{kind:'list'},{kind:'list',element:null},{kind:'map',fields:null}];
  for(const type of badTypes){const d=engineSettingsDefinition();d.schemas[0].type=type;assert.equal(compile(d,'a',engineSettingsProfile),null);malformedCases++;}
  const mutations=[d=>d.schemas.push(copy(d.schemas[0])),d=>d.schemas[0].key='',d=>d.schemas[0].owner='engine',d=>d.schemas[0].scopes=[],d=>d.schemas[0].scopes=['host','host'],d=>d.schemas[0].scopes=['guess'],d=>d.schemas[0].composition='recursive-code',d=>d.schemas[0].default=9,d=>d.restrictions.scalar=[],d=>d.restrictions.unknown=[1],d=>d.restrictions.scalar=['1'],d=>d.layers.push(copy(d.layers[0])),d=>d.layers[0].layer.authority='foreign',d=>d.layers[0].generation=0,d=>d.layers[0].values.scalar=null,d=>d.layers[0].values.scalar={kind:'reset',value:null},d=>d.layers[0].values.unknown={kind:'set',value:1},d=>d.targets.push(copy(d.targets[0])),d=>d.targets[0].layers.push(null),d=>d.targets[0].layers.push(copy(d.targets[0].layers[0])),d=>d.targets[0].layers=[id('missing')],d=>d.targets=null];
  for(const mutate of mutations){const d=engineSettingsDefinition();mutate(d);assert.equal(compile(d,'a',engineSettingsProfile),null);malformedCases++;}
  let calls=0;const d=engineSettingsDefinition();Object.defineProperty(d.schemas[0],'default',{enumerable:true,get(){calls++;throw Error('must not execute');}});assert.equal(compile(d,'a',engineSettingsProfile),null);assert.equal(calls,0);malformedCases++;
  const cycle=engineSettingsDefinition();cycle.schemas[0].type={kind:'list'};cycle.schemas[0].type.element=cycle.schemas[0].type;assert.equal(compile(cycle,'a',engineSettingsProfile),null);malformedCases++;
 });
 control('registered-prototype-keys-are-data-and-view-scope-is-not-a-ui-domain',()=>{
  const d=engineSettingsDefinition();d.schemas[0].key='__proto__';d.restrictions=Object.fromEntries([['__proto__',[1,2]]]);const z=engineSettingsFixture(api,{definition:d,request:{changes:[{layer:id('w1'),expectedGeneration:1,key:'__proto__',operation:'set',value:2}]}});assert(z.plan);z.submit();z.run();assert.equal(z.read().outcome.publication,'applied');const values=z.root().engineSettings.resolved[key(id('TA'))];assert(Object.hasOwn(values,'__proto__'));assert.equal(values.__proto__.value,2);assert.equal(Object.getPrototypeOf(values),Object.prototype);
  const v=engineSettingsDefinition();v.schemas[0].scopes.push('view');v.layers[3].kind='view';const view=engineSettingsFixture(api,{definition:v,request:{changes:[{layer:id('d'),expectedGeneration:1,key:'scalar',operation:'set',value:3}]}});assert(view.plan);view.submit();view.run();assert.equal(view.read().outcome.publication,'applied');assert.equal(view.root().engineSettings.resolved[key(id('TA'))].scalar.value,3);
 });
 control('schema-state-and-plan-budgets-refuse-before-owned-preparation',()=>{
  for(const profile of [{maxStateBytes:0},{maxKeys:0},{maxLayers:0},{maxTargets:0},{maxSchemaNodes:0},{maxSchemaDepth:0},{argumentLimits:{...engineSettingsProfile.argumentLimits,maxInputNodes:0}},{argumentLimits:{...engineSettingsProfile.argumentLimits,maxInputDepth:0}}]){assert.equal(compile(engineSettingsDefinition(),'a',{...engineSettingsProfile,...profile}),null);budgetCases++;}
  for(const profile of [{maxPlanBytes:0},{maxHeldBytes:0},{maxContextBytes:0}]){const z=engineSettingsFixture(api,{profile});assert.equal(z.plan,null);assert.equal(z.root().publication,0);assert.equal(api.invocationSemanticSettingsUsage(z.domain),null);budgetCases++;}
 });
 control('retained-context-capacity-and-lifetime-issuance-do-not-refund-on-release',()=>{
  const z=engineSettingsFixture(api,{profile:{maxContexts:1,maxIssuedContexts:2}}),g={...good};const a=api.acquireInvocationSemanticContext(z.domain,z.controlContext,z.p,g);assert(a);holdCases++;
  assert.equal(api.acquireInvocationSemanticContext(z.domain,z.controlContext,z.p,g),null);assert.equal(api.releaseInvocationSemanticContext(z.domain,a,z.controlContext,g),true);holdCases++;
  const b=api.acquireInvocationSemanticContext(z.domain,z.controlContext,z.p,g);assert(b);assert.equal(api.releaseInvocationSemanticContext(z.domain,b,z.controlContext,g),true);holdCases++;
  assert.equal(api.acquireInvocationSemanticContext(z.domain,z.controlContext,z.p,g),null);assert.equal(api.invocationSemanticSettingsUsage(z.domain).issued,2);assert.equal(api.readInvocationSemanticContext(z.domain,a,z.controlContext,g),null);holdCases++;
 });
 control('required-and-extra-read-dependencies-are-captured-and-current',()=>{
  const required={domain:'read',identity:'required',generation:1},extra={domain:'read',identity:'extra',generation:1};
  for(const mode of ['required-stale','extra-stale','required-expanded','malformed']){const z=engineSettingsFixture(api,{request:{dependencies:[extra]},host:{requiredDependencies:[required],contexts:[required,extra]}});assert(z.plan);z.submit();const h=z.host({requiredDependencies:[copy(required)],contexts:[copy(required),copy(extra)]});if(mode==='required-stale')h.contexts[0].generation++;if(mode==='extra-stale')h.contexts[1].generation++;if(mode==='required-expanded')h.requiredDependencies.push(copy(extra));if(mode==='malformed')h.contexts=[null];const root=z.root();assert.equal(z.run(h).status,'settled');assert.equal(z.read().outcome.publication,'notApplied');assert.equal(z.root(),root);}
 });
 control('captured-native-intent-preserves-negative-zero-and-caller-value-copies',()=>{
  const z=engineSettingsFixture(api,{request:{changes:[{layer:id('w1'),expectedGeneration:1,key:'scalar',operation:'set',value:-0}]}}),e=z.envelope('typed');e.request.arguments={...z.request,changes:[{...z.request.changes[0],value:0}]};assert(z.plan);assert.equal(api.prepareInvocationSemanticSettings(z.domain,z.ticket,e,z.p,z.controlContext,z.host()),null);
  const a=engineSettingsFixture(api,{request:{changes:[{layer:id('w1'),expectedGeneration:1,key:'map',operation:'set',value:{left:true}}]}});assert(a.plan);a.request.changes[0].value.left=false;a.submit();a.run();assert.equal(a.root().engineSettings.resolved[key(id('TA'))].map.value.left,true);
 });
 control('exhausted-changing-counters-refuse-but-unused-domains-and-noops-survive',()=>{
  for(const field of ['publication','generation','layer','domain']){const seed=engineSettingsFixture(api),s=copy(seed.root());if(field==='publication')s.publication=Number.MAX_SAFE_INTEGER;else if(field==='generation')s.engineSettings.generation=Number.MAX_SAFE_INTEGER;else if(field==='layer')s.engineSettings.layers[1].generation=Number.MAX_SAFE_INTEGER;else s.engineSettings.domains[key(id('TA'))].source=Number.MAX_SAFE_INTEGER;const z=engineSettingsFixture(api,{state:s,request:{changes:[{layer:id('w1'),expectedGeneration:field==='layer'?Number.MAX_SAFE_INTEGER:1,key:'scalar',operation:'set',value:2}]}});assert.equal(z.plan,null);}
  const seed=engineSettingsFixture(api),s=copy(seed.root());s.engineSettings.domains[key(id('TA'))].analysis=Number.MAX_SAFE_INTEGER;const z=engineSettingsFixture(api,{state:s});assert(z.plan);z.submit();z.run();assert.equal(z.read().outcome.publication,'applied');
  const n=copy(s);n.publication=Number.MAX_SAFE_INTEGER;n.engineSettings.generation=Number.MAX_SAFE_INTEGER;const noop=engineSettingsFixture(api,{state:n,request:{changes:[]}});assert(noop.plan);noop.submit();const before=noop.root();noop.run(noop.host({versionsAvailable:false}));assert.equal(noop.read().outcome.publication,'unchanged');assert.equal(noop.root(),before);
 });
 control('forged-owned-root-resolution-never-becomes-an-accepted-profile',()=>{
  const seed=engineSettingsFixture(api),state=copy(seed.root());state.engineSettings.resolved[key(id('TA'))].scalar.value=7;const z=engineSettingsFixture(api,{state});assert.equal(z.plan,null);assert.equal(api.acquireInvocationSemanticContext(z.domain,z.controlContext,z.p,good),null);assert.equal(z.root().publication,0);
 });
 control('unrelated-source-publication-and-nonempty-source-journal-survive-settings',()=>{
  const z=engineSettingsFixture(api);z.submit();const i=api.issueInvocation(z.domain,z.controlContext,good),d=z.root().documents[0],plan={contract:'owned-annotation-plan/1',invocation:i.reference,descriptor:i.reference,sources:[{document:d.document,expectedSource:1,expectedAnnotationSet:1,edits:[{lo:0,hi:1,expected:[97],ins:[98]}]}],annotations:[]};
  const h={...z.host(),parserSourceReserved:true,parserSourceLimits:{maxDocuments:8,maxRootBytes:4096,maxBatches:32,maxEdits:64,maxJournalBytes:65536},bounded:true,historyReady:true,sourceWritable:true,targetRestorationValid:true,completeIdentity:true,viewVersionsAvailable:true,maxParticipants:8,viewLimits:{maxRootBytes:4096,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:8192,maxTextBytes:4096,maxEditBytes:8192},viewWorkLimits:{maxViewBatches:32,maxViewEdits:64,maxViewBytes:65536,maxViewPlanBytes:65536}};
  assert.equal(api.submitInvocation(z.domain,i.ticket,{kind:'compound',plan},'typed',z.controlContext,z.controlPolicy,good).status,'accepted');assert.equal(api.runInvocation(z.domain,i.ticket,z.controlContext,good,h).status,'settled');assert.deepEqual(z.root().documents[0].source,[98]);const root=z.root(),journal=committedSourceTransforms(root);assert(journal.length>0);assert.equal(z.run().status,'settled');assert.equal(z.read().outcome.publication,'applied');assert.equal(z.root().documents,root.documents);assert.equal(z.root().groups,root.groups);assert.deepEqual(committedSourceTransforms(z.root()),journal);
 });
 return {scope:'owned-native-engine-settings-activation-only',products,declarationCases,malformedCases,budgetCases,holdCases,controls,actualOwnedConfigurationAndScopedPublicationChecked:true,realParentReservationAndContextHoldChecked:true,closedTypeAndOriginResolutionChecked:true,actualSourceJournalAndAuthoredFrameChecked:true,allRegistryConsumerOrWorkspaceTruthProved:false,publicProjectionOrCompleteValueCodecProved:false,mutableSchemaOrHostPolicyTruthProved:false,physicalConcurrentCrashOrIndependentRefinementProved:false};
}
const mutants=[
 ['write-grant-ignored',"'settingsWritable',",''],
 ['contributing-layer-dependencies-ignored','!same(preparationDependencies(cfg,v.request),v.dependencies)','false'],
 ['current-root-facts-unbound','h.semanticSettingsBasis!==before','false'],
 ['current-context-facts-unbound','!same(h.semanticSettingsContext,v.context)','false'],
 ['changing-version-capacity-ignored','h.versionsAvailable!==true','false'],
 ['effective-domain-not-advanced','d[domain]++;',''],
 ['every-domain-invalidated',"if(cfg.schemas.some(s=>s.domain===domain&&!same(cfg.resolved[k][s.key].value,r[k][s.key].value)))",'if(true)'],
 ['captured-context-refreshed','?v.configuration:null','?owner.root.engineSettings:null'],
 ['parent-budget-overcommitted','charge>owner.limits.maxHeldBytes-owner.charge','false'],
 ['integer-range-ignored','!Number.isSafeInteger(v)||v<t.min||v>t.max','!Number.isSafeInteger(v)'],
 ['required-role-expansion-ignored','!same(v.required,h.requiredDependencies)','false'],
 ['intent-negative-zero-conflated','same(prior.value.request,n.arguments,true)','same(prior.value.request,n.arguments)'],
 ['primitive-type-extra-fields-ignored',"if(['boolean','text'].includes(t.kind)){if(!exact(t,typeForms[t.kind]))return false;}","if(['boolean','text'].includes(t.kind)){}"],
 ['forged-resolution-accepted','return r&&same(r,c.resolved)?c:null;','return c;'],
 ['settings-dispatch-does-nothing','return publishSourceSerialization(state,plan,host,owner,domain,ticket)??publishNativeEngineSettings(state,plan,host,owner,domain,ticket);',"return {status:'unchanged',state};",'editing-intents.mjs'],
 ['settings-loses-source-journal',"if(entry.intent.kind==='settings'&&result.status==='applied')carryRetainedLifecycleSourceJournal(before,result.state);",'','invocation-publication.mjs'],
 ['retired-invocation-keeps-preparation-hold','if(entry.settingsPrepared&&!forgetSourceSerialization(entry.settingsPrepared,o,domain,ticket))forgetNativeEngineSettings(entry.settingsPrepared,o,domain,ticket);','','invocation-publication.mjs']
];
function moduleURL(file,override={},mutation=null){
 let source=fs.readFileSync(new URL(file,import.meta.url),'utf8');if(mutation){assert(source.includes(mutation[1]),mutation[0]);source=source.replaceAll(mutation[1],mutation[2]);}
 source=source.replace(/from '(\.\/[^']+)'/g,(_,p)=>`from ${JSON.stringify(override[p]??new URL(p,import.meta.url).href)}`).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>`new URL(${JSON.stringify(new URL(p,import.meta.url).href)},import.meta.url)`);
 return 'data:text/javascript;base64,'+Buffer.from(source).toString('base64');
}
export async function checkEngineSettingsMutations(){
 const load=async mutation=>{const file=mutation?.[3]??'engine-settings-state.mjs',s=moduleURL('engine-settings-state.mjs',{},file==='engine-settings-state.mjs'?mutation:null),e=moduleURL('editing-intents.mjs',{'./engine-settings-state.mjs':s},file==='editing-intents.mjs'?mutation:null),k=moduleURL('invocation-publication.mjs',{'./engine-settings-state.mjs':s,'./editing-intents.mjs':e},file==='invocation-publication.mjs'?mutation:null);return {...await import(s),...await import(k)};};
 checkEngineSettings(await load(null));const killed=[];for(const mutation of mutants){const api=await load(mutation);assert.throws(()=>checkEngineSettings(api),{code:'ERR_ASSERTION'},mutation[0]);killed.push(mutation[0]);}return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkEngineSettings(),mutationBaselineChecked:true,mutations:await checkEngineSettingsMutations()},null,2));
