// Selected handler-declaration provenance, not all-family registration/host truth.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as discovery from './native-discovery.mjs';
import * as annotations from './annotation-editing.mjs';
import * as history from './history-reversal.mjs';
import {publishAnnotationPlan} from './annotation-compounds.mjs';
import {nativeDiscoveryFixture} from './native-discovery-checks.mjs';
import {annotationEditingFixture} from './annotation-editing-checks.mjs';
import {annotationMembershipEditingFixture} from './annotation-membership-editing-checks.mjs';
import {nativeSourceDestinationSourceEdit as sourceEdit} from './native-source-destinations-checks.mjs';
const original={...discovery,...annotations,...history},copy=structuredClone,no={status:'unavailable'};
const expected={
 create:['contract','operation','invocation','document','profile','dependencies','body','target'],
 'update-body':['contract','operation','invocation','document','profile','dependencies','annotation','expectedContent','body'],
 reattach:['contract','operation','invocation','document','profile','dependencies','annotation','expectedTarget','target'],
 delete:['contract','operation','invocation','document','profile','dependencies','annotation','expectedContent','expectedTarget'],
 undo:['contract','operation','invocation','authority','target','profile','dependencies'],
 redo:['contract','operation','invocation','authority','target','profile','dependencies']
};
function fixture(api=original,options={}){
 const x=nativeDiscoveryFixture(api,undefined,options),rows=()=>api.nativeBuiltinDiscoveryRows(x.owner,x.scope,x.g());
 const prepare=(o={})=>api.prepareNativeBuiltinDiscoveryCut(x.owner,o.scope??x.scope,o.visibility??x.visibility(rows()??[]),o.target??null,o.advice??null,o.g??x.g());
 const open=(o={})=>{const packet=prepare(o);assert(packet);return api.acquireNativeDiscovery(x.owner,x.scope,packet,o.acquireG??x.g());};
 const query=(context,request,o={})=>{const packet=o.packet??prepare(o);assert(packet);return api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile:x.scope.profile,context,...request},o.queryScope??x.scope,packet,o.queryG??x.g());};
 return {...x,rows,prepare,open,query};
}
function reversalFixture(api){
 const document={authority:'a',id:'D',incarnation:1},initial={authority:'a',publication:0,documents:[{document,source:[97],revision:1,annotationSet:1,records:[]}],groups:[],frontiers:{},views:[]};
 const h={owner:{},recipient:'caller',profile:'p',...Object.fromEntries(['authorized','bounded','notStopped','interpretationCurrent','companionsReady','versionsAvailable','historyReady','invocationFresh','sourceWritable','targetRestorationValid','completeIdentity'].map(k=>[k,true])),maxParticipants:8,
  limits:{maxDependencies:4,maxParticipants:8,maxChanges:8,maxPlanBytes:8192,maxViewBatches:32,maxViewEdits:64,maxViewBytes:32768,maxViewPlanBytes:16384},viewVersionsAvailable:true,
  viewLimits:{maxRootBytes:128,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:2048,maxTextBytes:32,maxEditBytes:256},
  requiredDependencies:[],contexts:[],groups:[{reference:'g',internal:'G',authority:'a'}],scopes:[]};
 const changed=publishAnnotationPlan(initial,{contract:'owned-annotation-plan/1',invocation:'G',descriptor:'G',annotations:[],sources:[{document,expectedSource:1,expectedAnnotationSet:1,edits:[{lo:0,hi:1,expected:[97],ins:[98]}]}]},h);
 assert.equal(changed.status,'applied');
 const request=operation=>({contract:'history-reversal/1',operation,invocation:'reverse-'+operation,authority:'a',target:{kind:'group',group:'g'},profile:'p',dependencies:[]});
 const p=api.prepareReversal(changed.state,request('undo'),h);assert(p);const undone=api.publishReversal(changed.state,p,h);assert.equal(undone.status,'applied');
 return {h,request,state:operation=>operation==='undo'?changed.state:undone.state};
}
function frozen(v){if(v&&typeof v==='object'){assert(Object.isFrozen(v));Object.values(v).forEach(frozen);}}
export function checkBuiltinDiscovery(api=original){
 const controls=[],control=(name,f)=>{f();controls.push(name);};let handlerProducts=0,missingFields=0,descriptors=0,facetCases=0;
 control('handler-owned-declarations-are-exact-frozen-and-independent',()=>{
  for(const source of [api.annotationEngineDescriptor(),api.historyEngineDescriptor()]){
   frozen(source);assert.equal(source.kind,'localCommand');assert.deepEqual(source.presentation,{});
   for(const variant of source.variants){assert.deepEqual(Object.keys(variant.input.fields).sort(),expected[variant.id].toSorted());assert.equal(variant.input.contract,source.contract);assert.equal(variant.input.variant,variant.id);descriptors++;}
  }
  const a=api.annotationEngineDescriptor(),b=api.annotationEngineDescriptor();assert.notEqual(a,b);assert.notEqual(a.variants[0].input.fields,b.variants[0].input.fields);
 });
 control('declared-annotation-fields-match-real-handler-acceptance',()=>{
  for(const operation of ['create','update-body','reattach','delete']){
   const z=['create','delete'].includes(operation)?annotationMembershipEditingFixture(undefined,operation):annotationEditingFixture(undefined,operation==='update-body'?'body':'target-0');
   assert.deepEqual(Object.keys(z.request).sort(),expected[operation].toSorted());assert(api.prepareAnnotationEdit(z.root(),z.request,z.h,z.al));handlerProducts++;
   for(const field of expected[operation]){const bad=copy(z.request);delete bad[field];assert.equal(api.prepareAnnotationEdit(z.root(),bad,z.h,z.al),null,operation+'/'+field);missingFields++;}
   assert.equal(api.prepareAnnotationEdit(z.root(),{...z.request,keymap:'ui'},z.h,z.al),null);
  }
 });
 control('declared-reversal-fields-match-real-handler-acceptance',()=>{
  const z=reversalFixture(api);for(const operation of ['undo','redo']){
   const request=z.request(operation),state=z.state(operation);assert.deepEqual(Object.keys(request).sort(),expected[operation].toSorted());assert(api.prepareReversal(state,request,z.h));handlerProducts++;
   for(const field of expected[operation]){const bad=copy(request);delete bad[field];assert.equal(api.prepareReversal(state,bad,z.h),null,operation+'/'+field);missingFields++;}
   assert.equal(api.prepareReversal(state,{...request,keymap:'ui'},z.h),null);
  }
 });
 control('builtin-origin-is-derived-not-a-caller-attestation',()=>{
  const x=fixture(api),rows=x.rows();assert.equal(rows.length,2);assert.deepEqual(rows.map(r=>r.command.authority),[x.scope.authority,x.scope.authority]);
  rows.forEach((r,i)=>assert.equal(r.command.id,JSON.stringify([x.scope.engine,'builtin',['F13','F14'][i]])));
  const p=x.prepare({g:x.g({acceptedRegistryAttested:false,schemaDescriptionsAttested:false})});assert(p);assert.equal(api.acquireNativeDiscovery(x.owner,x.scope,p,x.g()).status,'context');
  const changed=x.rows();changed[0].variants[0].input.fields.injected='code';assert(!Object.hasOwn(x.rows()[0].variants[0].input.fields,'injected'));
 });
 control('all-selected-descriptors-use-current-scoped-release-and-frame-native-state',()=>{
  const x=fixture(api),root=x.z.root(),usage=copy(x.z.usage()),r=x.open();assert.equal(r.status,'context');
  const p=x.query(r.context,{operation:'list',limit:2,continuation:null});assert.equal(p.status,'page');assert.equal(p.entries.length,2);
  for(let i=0;i<2;i++){const d=x.query(r.context,{operation:'inspect',entry:p.entries[i].reference});assert.equal(d.status,'descriptor');assert.deepEqual(d.descriptor.variants.map(v=>v.input),x.rows()[i].variants.map(v=>v.input));}
  assert.equal(x.z.root(),root);assert.deepEqual(x.z.usage(),usage);
  assert.deepEqual(x.query(r.context,{operation:'list',limit:2,continuation:null},{queryG:x.g({authorized:false})}),no);
 });
 control('generic-lookalike-cuts-cannot-substitute-for-builtin-origin-in-either-direction',()=>{
  for(const builtin of [false,true]){
   const x=fixture(api),rows=x.rows(),generic=()=>api.prepareNativeDiscoveryCut(x.owner,x.scope,rows,x.visibility(rows),null,null,x.g());
   const first=builtin?x.prepare():generic(),r=api.acquireNativeDiscovery(x.owner,x.scope,first,x.g());assert.equal(r.status,'context');
   const before=copy(x.usage()),wrong=builtin?generic():x.prepare();
   assert.deepEqual(x.query(r.context,{operation:'list',limit:2,continuation:null},{packet:wrong}),no);assert.equal(x.usage().issued,before.issued);assert.equal(x.usage().selectors,before.selectors);
   const right=builtin?x.prepare():generic();assert.equal(x.query(r.context,{operation:'list',limit:2,continuation:null},{packet:right}).status,'page');
  }
 });
 control('owner-scope-root-and-preparation-resources-remain-required',()=>{
  const x=fixture(api),y=fixture(api),p=x.prepare();assert.equal(api.nativeBuiltinDiscoveryRows(copy(x.owner),x.scope,x.g()),null);
  assert.equal(api.nativeBuiltinDiscoveryRows(x.owner,x.scope,x.g({ownerControlAllowed:false})),null);
  assert.equal(api.nativeBuiltinDiscoveryRows(x.owner,{...x.scope,engine:'foreign'},x.g()),null);
  assert.deepEqual(api.acquireNativeDiscovery(y.owner,y.scope,p,y.g()),no);assert.deepEqual(api.acquireNativeDiscovery(x.owner,x.scope,copy(p),x.g()),no);
  for(const field of ['visibilityAttested','visibilityComplete','projectionOrderAttested','outsidePublicationGuard','resourcesReady','preparationReserved','authorized','scope','continuous','live','notStopped','discoveryAllowed','discoveryInterpretationCurrent'])assert.equal(x.prepare({g:x.g({[field]:false})}),null,field);
 });
 control('selected-source-honors-registry-and-packet-metadata-bounds',()=>{
  for(const limits of [{maxEntries:1},{maxVariants:3},{maxMetadataBytes:0},{maxMetadataDepth:0},{maxMetadataNodes:0},{maxRowBytes:0},{maxPacketBytes:0}]){
   const x=fixture(api,{limits});assert.equal(x.prepare(),null,JSON.stringify(limits));
   if(!Object.hasOwn(limits,'maxPacketBytes'))assert.equal(x.rows(),null,JSON.stringify(limits));
  }
 });
 control('private-origin-is-included-in-the-capture-byte-bound',()=>{
  const seed=fixture(api),rows=seed.rows().map(r=>({key:JSON.stringify([r.command.authority,r.command.id,r.command.incarnation]),schema:true,presentation:true,advice:true,variants:r.variants.map(v=>v.id),value:{version:r.version,kind:r.kind,variants:copy(r.variants),presentation:{}}}));
  const charge=Buffer.byteLength(JSON.stringify({rows,target:null,builtin:true}));
  for(const delta of [-1,0]){const x=fixture(api,{limits:{maxCaptureBytes:charge+delta}}),p=x.prepare();assert(p);const before=copy(x.usage()),r=api.acquireNativeDiscovery(x.owner,x.scope,p,x.g());assert.equal(r.status,delta<0?'unavailable':'context');if(delta<0){assert.equal(x.usage().issued,before.issued);assert.equal(x.usage().contexts,before.contexts);}}
 });
 control('stale-root-cuts-refuse-but-unrelated-edits-preserve-unchanged-descriptors',()=>{
  const x=fixture(api),p=x.prepare(),r=x.open();sourceEdit(x.z.z.z,x.api,1);
  assert.deepEqual(api.acquireNativeDiscovery(x.owner,x.scope,p,x.g()),no);
  assert.equal(x.query(r.context,{operation:'list',limit:2,continuation:null}).status,'page');
  assert.equal(api.retireNativeDiscoveryCut(x.owner,p,x.g()).status,'retired');
 });
 control('origin-bound-context-retirement-preserves-spent-issuance',()=>{
  const x=fixture(api),r=x.open();x.query(r.context,{operation:'list',limit:2,continuation:null});const issued=x.usage().issued;
  assert.equal(api.stopNativeDiscovery(x.owner,r.context,x.scope,x.g()).status,'stopped');assert.equal(x.usage().issued,issued);
  const p=x.prepare();assert(p);
  assert.deepEqual(api.queryNativeDiscovery(x.owner,{contract:'contract-discovery/1',profile:x.scope.profile,context:r.context,operation:'list',limit:2,continuation:null},x.scope,p,x.g()),no);
  // Query refusal occurs before consuming the caller's known prepared packet.
  assert.equal(x.usage().preparations,1);
  assert.equal(api.retireNativeDiscoveryCut(x.owner,p,x.g()).status,'retired');
  assert.equal(api.retireNativeDiscovery(x.owner,r.context,x.g()).status,'retired');assert.equal(x.usage().issued,issued);
  assert.equal(api.retireNativeDiscoveryRegistry(x.owner,x.g()).status,'retired');
  assert.equal(api.openNativeDiscoveryRegistry(x.domain,x.l,x.g()),null);
 });
 control('schema-and-variant-disclosure-remain-independent-of-origin',()=>{
  for(const schema of [false,true])for(const visible of [false,true]){
   const x=fixture(api,{schema,variantVisible:()=>visible}),r=x.open(),p=x.query(r.context,{operation:'list',limit:2,continuation:null});assert.equal(p.status,'page');
   for(const entry of p.entries){const d=x.query(r.context,{operation:'inspect',entry:entry.reference});assert.equal(d.status,'descriptor');assert.equal(d.descriptor.variants.length,visible?(entry===p.entries[0]?4:2):0);for(const v of d.descriptor.variants)assert.equal(Object.hasOwn(v,'input'),schema);facetCases++;}
  }
 });
 control('source-origin-does-not-create-advisory-target-or-execution-authority',()=>{
  const x=fixture(api),r=x.open();assert.equal(r.target,null);const p=x.query(r.context,{operation:'list',limit:2,continuation:null}),d=x.query(r.context,{operation:'inspect',entry:p.entries[0].reference});
  assert.deepEqual(x.query(r.context,{operation:'advisory-enablement',entry:p.entries[0].reference,variant:d.descriptor.variants[0].reference,target:null}),no);
 });
 return {scope:'selected-builtin-handler-declaration-discovery-only',handlerProducts,missingFields,descriptors,facetCases,controls,selectedHandlerDeclarationOriginChecked:true,exactFieldsComparedToActualPreparationChecked:true,privateOriginBindingAndNoLookalikeDowngradeChecked:true,allFamilyRegistryOrHandlerTruthProved:false,currentPolicyOrEnablementTruthProved:false,publicWireOrRustCorrespondenceProved:false,physicalConcurrentCrashOrIndependentRefinementProved:false};
}
const mutations=[
 ['ignore-origin-on-refresh','native-discovery.mjs','if((p.builtin===true)!==(s.builtin===true))return no();',''],
 ['forget-prepared-origin','native-discovery.mjs','if(packet)m.packets.get(packet).builtin=true;',''],
 ['forget-captured-origin','native-discovery.mjs','...(p.builtin===true?{builtin:true}:{}),entries:','entries:'],
 ['require-unproven-caller-origin','native-discovery.mjs','{...g,acceptedRegistryAttested:true,schemaDescriptionsAttested:true}','g'],
 ['foreign-owner-command-identity','native-discovery.mjs','rows.push({command:{authority:m.authority,id:JSON.stringify','rows.push({command:{authority:"foreign",id:JSON.stringify'],
 ['unbounded-owner-source-inspection','native-discovery.mjs','\n return registryValid(rows,m.limits,c.profile)?rows:null;','\n return rows;'],
 ['unaccounted-private-origin','native-discovery.mjs','(p.builtin===true&&size({rows,target,builtin:true})>m.limits.maxCaptureBytes)','false'],
 ['omit-annotation-specific-fields','annotation-editing.mjs','fields:copy({...native.common,...native.variants[id]})','fields:copy(native.common)'],
 ['mutable-source-descriptor','history-reversal.mjs','return freeze({family:catalog.family','return ({family:catalog.family']
];
function moduleURL(file,mutation,overrides={}){
 let raw=fs.readFileSync(new URL(file,import.meta.url),'utf8');if(mutation?.[1]===file){assert.equal(raw.split(mutation[2]).length-1,1,`unique mutation target: ${mutation[0]}`);raw=raw.replace(mutation[2],mutation[3]);}
 raw=raw.replace(/from '(\.\/[^']+)'/g,(_,p)=>`from ${JSON.stringify(overrides[p]??new URL(p,import.meta.url).href)}`).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>`new URL(${JSON.stringify(new URL(p,import.meta.url).href)},import.meta.url)`);
 return 'data:text/javascript;base64,'+Buffer.from(raw).toString('base64');
}
export async function checkBuiltinDiscoveryMutations(){
 const load=async mutation=>{const a=moduleURL('annotation-editing.mjs',mutation),h=moduleURL('history-reversal.mjs',mutation),d=moduleURL('native-discovery.mjs',mutation,{'./annotation-editing.mjs':a,'./history-reversal.mjs':h});return {...await import(a),...await import(h),...await import(d)};};
 checkBuiltinDiscovery(await load(null));const killed=[];
 for(const m of mutations){const api=await load(m);assert.throws(()=>checkBuiltinDiscovery(api),{code:'ERR_ASSERTION'},m[0]);killed.push(m[0]);}
 return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkBuiltinDiscovery(),mutationBaselineChecked:true,mutations:await checkBuiltinDiscoveryMutations()},null,2));
