// Owned first-admission reference; no production ticket registry or native codec.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {publishAnnotationPlan,reverseAnnotationGroup} from './annotation-compounds.mjs';
import {mergeDependencies} from './annotation-operations.mjs';
import {prepareReversalViews,validateReversalViews,completeReversalViews} from './history-views.mjs';
import {prepareReversalResultAdmission,captureReversalResult,releaseReversalResult,validReversalResultReceipt} from './reversal-results.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./history-reversal.json',import.meta.url),'utf8'));
const copy=structuredClone;
const owned=new WeakMap();
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const label=v=>typeof v==='string'&&v.length>0;
const dependency=d=>exact(d,catalog.dependencyFields)&&label(d.domain)&&label(d.identity)&&Number.isSafeInteger(d.generation)&&d.generation>0;
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const gates=['authorized','bounded','notStopped','interpretationCurrent','companionsReady','versionsAvailable','historyReady','invocationFresh'];
const contextKeys=ds=>[...new Map(ds.map(d=>[JSON.stringify([d.domain,d.identity]),d])).values()]
  .sort((a,b)=>JSON.stringify([a.domain,a.identity]).localeCompare(JSON.stringify([b.domain,b.identity])));

function shape(r,h){
  if(!exact(r,catalog.common)||r.contract!==catalog.contract||!catalog.variants.includes(r.operation)||
      !label(r.invocation)||!label(r.authority)||!label(r.profile)||!Array.isArray(r.dependencies)||
      r.dependencies.length>h.limits.maxDependencies||!r.dependencies.every(dependency))return false;
  const t=r.target;
  return t&&Object.hasOwn(catalog.targets,t.kind)&&exact(t,catalog.targets[t.kind])&&
    (t.kind==='group'?label(t.group):label(t.scope)&&label(t.expectedGroup));
}
function captureParticipants(state,group){
  const documents=[];
  for(const change of group.changes){
    const d=state.documents.find(d=>equal(d.document,change.document));
    if(!d||d.live===false)return null;
    if(!documents.some(x=>equal(x.document,d.document)))documents.push({document:copy(d.document),sourceRevision:d.revision,annotationSet:d.annotationSet});
  }
  return {documents,frontiers:group.scopes.map(scope=>({scope,frontier:copy(state.frontiers[scope])}))};
}
export function prepareReversal(state,request,host){
  if(!shape(request,host)||request.authority!==state.authority||request.profile!==host.profile||!gates.every(k=>host[k]))return null;
  const t=request.target,groupRef=t.kind==='group'?t.group:t.expectedGroup;
  const binding=host.groups.find(g=>g.reference===groupRef&&g.authority===state.authority);
  if(!binding)return null;
  const group=state.groups.find(g=>g.id===binding.internal);
  if(!group?.valid||group.scopes.length>host.limits.maxParticipants||group.changes.length>host.limits.maxChanges)return null;
  if(t.kind==='scope-frontier'){
    const s=host.scopes.find(s=>s.reference===t.scope&&s.authority===state.authority);
    if(!s||!group.scopes.includes(s.internal)||state.frontiers[s.internal]?.[request.operation]?.at(-1)!==group.id)return null;
  }
  if(!mergeDependencies(host.requiredDependencies,request.dependencies,host.contexts,host.limits.maxDependencies))return null;
  const participants=captureParticipants(state,group);if(!participants)return null;
  const viewPart=prepareReversalViews(state,group,request.operation,host.viewLimits,host.limits);
  if(!viewPart||!validateReversalViews(state,viewPart,host.viewLimits,host.limits,host.viewVersionsAvailable))return null;
  const plan={request:copy(request),group:copy(group),participants,viewPart,recipient:host.recipient,profile:host.profile,
    requiredDependencies:copy(contextKeys(host.requiredDependencies)),dependencies:copy(contextKeys([...host.requiredDependencies,...request.dependencies]))};
  if(Buffer.byteLength(JSON.stringify(plan),'utf8')>host.limits.maxPlanBytes)return null;
  freeze(plan);owned.set(plan,host.owner);return plan;
}
export function publishReversal(state,plan,host){
  const refuse=()=>({status:'not-published',state});
  if(!plan||!owned.has(plan)||owned.get(plan)!==host.owner||plan.recipient!==host.recipient||plan.profile!==host.profile||
      state.authority!==plan.request.authority||!gates.every(k=>host[k]))return refuse();
  if(!equal(plan.requiredDependencies,contextKeys(host.requiredDependencies))||
      !mergeDependencies(plan.dependencies,[],host.contexts,host.limits.maxDependencies))return refuse();
  const group=state.groups.find(g=>g.id===plan.group.id);
  if(!equal(group,plan.group)||!equal(captureParticipants(state,group),plan.participants)||
      group.scopes.length>host.limits.maxParticipants||group.changes.length>host.limits.maxChanges||
      Buffer.byteLength(JSON.stringify(plan),'utf8')>host.limits.maxPlanBytes)return refuse();
  if(!validateReversalViews(state,plan.viewPart,host.viewLimits,host.limits,host.viewVersionsAvailable))return refuse();
  // Both pure helpers produce private candidates; only the complete root is returned.
  const reversed=reverseAnnotationGroup(state,group.id,plan.request.operation,host);
  return completeReversalViews(state,plan.viewPart,reversed,host.viewLimits,host.limits,host.viewVersionsAvailable);
}

export function admitReversalResult(plan,c,g,maximum){return owned.has(plan)?prepareReversalResultAdmission(plan,c,g,maximum):null;}
export function captureReversalResultReceipt(plan,result){if(!owned.has(plan))return null;const receipt=captureReversalResult(plan,result);if(!validReversalResultReceipt(receipt))throw Error('invalid owned reversal receipt');return receipt;}
export {releaseReversalResult};

// Uses the private request declarations consumed by shape()/prepareReversal().
// Current group/frontier, view, policy and retention facts remain independent.
export function historyEngineDescriptor(){
 return freeze({family:catalog.family,contract:catalog.contract,version:1,kind:'localCommand',
  variants:catalog.variants.map(id=>({id,
   input:{contract:catalog.contract,variant:id,fields:copy(catalog.fieldTypes)},
   outcomes:copy(catalog.results),requirements:{admission:copy(gates),currentPredicates:'group-frontier-participant-view-and-owner-specific'},
   limits:{names:['maxDependencies','maxParticipants','maxChanges','maxPlanBytes'],values:'current-execution-and-view-profiles'},
   cancellation:'owned-prepublication-stop; accepted-publication-is-not-retracted'})),presentation:{}});
}

export function checkHistoryReversal(){
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  assert.deepEqual(catalog.variants.toSorted(),families.find(f=>f.id==='F14').variants.toSorted());
  const doc=id=>({authority:'a',id,incarnation:1}),scope=id=>`source:${JSON.stringify(doc(id))}`;
  const record={id:'note',incarnation:1,body:'old',bodyVersion:1,target:{sourceRevision:1,target:{kind:'range',start:0,end:1}},targetGeneration:1,
    placement:{state:'attached',sourceRevision:1,target:{kind:'range',start:0,end:1}}};
  const initial={authority:'a',publication:0,documents:['A','B','C'].map(id=>({document:doc(id),source:[97],revision:1,annotationSet:1,records:id==='A'?[copy(record)]:[]})),groups:[],frontiers:{},views:[]};
  const host={owner:{},recipient:'caller',profile:'p',...Object.fromEntries(gates.map(k=>[k,true])),targetRestorationValid:true,completeIdentity:true,maxParticipants:8,
    limits:{maxDependencies:4,maxParticipants:8,maxChanges:8,maxPlanBytes:8192,maxViewBatches:32,maxViewEdits:64,maxViewBytes:32768,maxViewPlanBytes:16384},
    viewVersionsAvailable:true,viewLimits:{maxRootBytes:128,maxInputViews:8,maxViews:8,maxInputSelections:8,maxSelections:8,maxSelectionBytes:2048,maxTextBytes:32,maxEditBytes:256},
    requiredDependencies:[{domain:'interpretation',identity:'history',generation:1}],contexts:[{domain:'interpretation',identity:'history',generation:1},{domain:'read',identity:'C',generation:1}],
    groups:[{reference:'g',internal:'G',authority:'a'},{reference:'h',internal:'H',authority:'a'}],scopes:[{reference:'a',internal:scope('A'),authority:'a'},{reference:'b',internal:scope('B'),authority:'a'}]};
  const edit=(state,id,ids,annotations=[])=>publishAnnotationPlan(state,{contract:'owned-annotation-plan/1',invocation:id,descriptor:`candidate-${id}`,annotations,
    sources:ids.map(name=>{const d=state.documents.find(d=>d.document.id===name);return {document:doc(name),expectedSource:d.revision,expectedAnnotationSet:d.annotationSet,
      edits:[{lo:0,hi:1,expected:copy(d.source),ins:[d.source[0]+1]}]};})},host).state;
  const base=edit(initial,'G',['A','B']);
  const common={contract:catalog.contract,invocation:'reverse-1',authority:'a',profile:'p',dependencies:[]};
  const request=(operation='undo',target={kind:'group',group:'g'})=>({...common,operation,target});
  const controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const refuse=(name,state=base,r=request(),h=host)=>{
    const before=copy({state,r}),plan=prepareReversal(state,r,h),out=publishReversal(state,plan,h);
    assert.deepEqual(out,{status:'not-published',state});assert.deepEqual({state,r},before);controls.push(name);
  };
  const plan=prepareReversal(base,request(),host),undone=publishReversal(base,plan,host);
  assert.equal(undone.status,'applied');
  const redone=publishReversal(undone.state,prepareReversal(undone.state,request('redo'),host),host);
  assert.equal(redone.status,'applied');
  control('two-source-undo-redo-one-publication-each',undone.state.publication===base.publication+1&&redone.state.publication===base.publication+2&&equal(redone.state.documents.map(d=>d.source),base.documents.map(d=>d.source)));
  control('fresh-reversal-versions',undone.state.documents[0].revision===3&&redone.state.documents[0].revision===4);
  control('no-new-authored-group',equal(redone.state.groups,base.groups));
  let gateCases=0;
  for(const [state,operation] of [[base,'undo'],[undone.state,'redo']])for(let mask=0;mask<256;mask++){
    const h={...host,...Object.fromEntries(gates.map((k,i)=>[k,!!(mask&(1<<i))]))},r=request(operation);
    const before=copy({state,r}),p=prepareReversal(state,r,host),out=publishReversal(state,p,h);
    assert.equal(out.status,mask===255?'applied':'not-published');if(mask!==255)assert.deepEqual(out.state,state);
    assert.deepEqual({state,r},before);gateCases++;
  }
  const scopeRequest=request('undo',{kind:'scope-frontier',scope:'a',expectedGroup:'g'});
  control('scope-frontier-resolves-exact-group',publishReversal(base,prepareReversal(base,scopeRequest,host),host).status==='applied');
  const later=edit(base,'H',['B']);
  refuse('later-hidden-participant-blocks-complete-group',later);
  control('old-preparation-does-not-retarget',publishReversal(later,plan,host).status==='not-published');
  control('scope-preparation-does-not-retarget',publishReversal(later,prepareReversal(base,scopeRequest,host),host).status==='not-published');
  const unrelated=edit(base,'C-edit',['C']);
  control('unrelated-document-does-not-stale-group',publishReversal(unrelated,plan,host).status==='applied');
  const body={kind:'body',document:doc('A'),id:'note',incarnation:1,expectedBody:1,body:'new'};
  const laterBody=edit(base,'H',[],[body]);
  control('private-mapping-dependency-stales-preparation',publishReversal(laterBody,plan,host).status==='not-published');
  const freshBodyUndo=publishReversal(laterBody,prepareReversal(laterBody,request(),host),host);
  control('reprepare-preserves-later-body',freshBodyUndo.status==='applied'&&freshBodyUndo.state.documents[0].records[0].body==='new'&&freshBodyUndo.state.documents[0].records[0].bodyVersion===2);
  const compound=edit(initial,'G',['A','B'],[body]);
  control('source-and-authored-body-compound',publishReversal(compound,prepareReversal(compound,request(),host),host).state.documents[0].records[0].body==='old');
  for(const field of catalog.common){const bad=request();delete bad[field];refuse(`missing-${field}`,base,bad);}
  refuse('unknown-contract',base,{...request(),contract:'history-reversal/2'});
  refuse('unknown-variant',base,{...request(),operation:'jump-branch'});
  refuse('unknown-field',base,{...request(),callback:'execute'});
  refuse('wrong-authority',base,{...request(),authority:'other'});
  refuse('unknown-profile',base,{...request(),profile:'other'});
  refuse('raw-group-id-not-reference',base,request('undo',{kind:'group',group:'G'}));
  refuse('unknown-target-field',base,request('undo',{kind:'group',group:'g',scope:'a'}));
  refuse('unknown-target-kind',base,request('undo',{kind:'latest'}));
  refuse('wrong-scope-incarnation',base,request('undo',{kind:'scope-frontier',scope:'retired-a',expectedGroup:'g'}));
  refuse('wrong-expected-frontier',base,request('undo',{kind:'scope-frontier',scope:'a',expectedGroup:'h'}));
  refuse('wrong-direction-frontier',base,request('redo'));
  refuse('caller-dependency-conflict',base,{...request(),dependencies:[{domain:'interpretation',identity:'history',generation:2}]});
  refuse('caller-dependency-missing',base,{...request(),dependencies:[{domain:'read',identity:'missing',generation:1}]});
  refuse('invalid-dependency',base,{...request(),dependencies:[{domain:'read',identity:'C',generation:0}]});
  refuse('mandatory-dependency-missing',base,request(),{...host,contexts:[]});
  refuse('participant-bound',base,request(),{...host,limits:{...host.limits,maxParticipants:1}});
  refuse('change-bound',base,request(),{...host,limits:{...host.limits,maxChanges:1}});
  refuse('owned-plan-byte-bound',base,request(),{...host,limits:{...host.limits,maxPlanBytes:1}});
  refuse('dependency-union-bound',base,{...request(),dependencies:[host.contexts[1]]},{...host,limits:{...host.limits,maxDependencies:1}});
  const rejectPrepared=(name,p=plan,s=base,h=host)=>control(name,equal(publishReversal(s,p,h),{status:'not-published',state:s}));
  rejectPrepared('copied-plan-not-owned',copy(plan));
  rejectPrepared('different-engine-incarnation',plan,base,{...host,owner:{}});
  rejectPrepared('different-recipient',plan,base,{...host,recipient:'other'});
  rejectPrepared('different-profile-after-preparation',plan,base,{...host,profile:'other'});
  rejectPrepared('mandatory-scope-expansion',plan,base,{...host,requiredDependencies:[...host.requiredDependencies,host.contexts[1]]});
  rejectPrepared('mandatory-context-stale',plan,base,{...host,contexts:[{...host.contexts[0],generation:2}]});
  const extraRequest={...request(),dependencies:[host.contexts[1]]},extraPlan=prepareReversal(base,extraRequest,host);
  const twoRequired={...host,requiredDependencies:copy(host.contexts)},twoPlan=prepareReversal(base,request(),twoRequired);
  control('mandatory-dependency-order-is-not-new-scope',publishReversal(base,twoPlan,{...twoRequired,requiredDependencies:[...twoRequired.requiredDependencies].reverse()}).status==='applied');
  rejectPrepared('read-only-dependency-stale',extraPlan,base,{...host,contexts:[host.contexts[0],{...host.contexts[1],generation:2}]});
  const altered=copy(base);altered.groups[0].changes[0].before=[99];altered.groups[0].changes[0].inverseSequence[0].ins=[99];
  rejectPrepared('retained-journal-changed',plan,altered);
  // Isolate journal identity from the view part's independent expected-source guard.
  const alteredExpectation=copy(base);alteredExpectation.groups[0].changes[0].forward[0].expected=[99];
  assert.equal(publishReversal(alteredExpectation,plan,host).status,'not-published','retained expected-byte metadata changed');
  const replaced=copy(base);replaced.documents[1].document.incarnation=2;rejectPrepared('participant-incarnation-changed',plan,replaced);
  const pruned=copy(base);pruned.groups=[];rejectPrepared('group-retention-lost',plan,pruned);
  const staleBudget={...host,limits:{...host.limits,maxParticipants:1}};rejectPrepared('final-bound-shrunk',plan,base,staleBudget);
  const original=request();const frozen=prepareReversal(base,original,host);original.target.group='h';
  control('caller-alias-cannot-retarget',publishReversal(base,frozen,host).status==='applied'&&Object.isFrozen(frozen.request.target));
  control('diagnostics-not-authority',equal(publishReversal(base,plan,host),publishReversal(base,plan,{...host,diagnosticFailed:true})));
  rejectPrepared('already-reversed-plan-does-not-reexecute',plan,undone.state);
  return {status:'history-reversal-admission-only',gateCases,controls,membershipReversalProved:false,generalTargetRestorationProved:false,fullReplayCompositionProved:false,implementationRefinement:false};
}
export async function checkReversalMutations(){
  const source=fs.readFileSync(new URL('./history-reversal.mjs',import.meta.url),'utf8');
  const mutants=[
    ['copied-plan-is-owned',"!owned.has(plan)||owned.get(plan)!==host.owner","false"],
    ['omit-private-annotation-set',"sourceRevision:d.revision,annotationSet:d.annotationSet","sourceRevision:d.revision"],
    ['ignore-required-scope-expansion',"!equal(plan.requiredDependencies,contextKeys(host.requiredDependencies))","false"],
    ['ignore-retained-journal-change',"!equal(group,plan.group)","false"],
    ['ignore-read-only-dependency',"!mergeDependencies(plan.dependencies,[],host.contexts,host.limits.maxDependencies)","false"],
    ['ignore-invocation-freshness',"!gates.every(k=>host[k]))return refuse();","!gates.filter(k=>k!=='invocationFresh').every(k=>host[k]))return refuse();"]
  ];
  const killed=[];
  for(const [name,from,to] of mutants){
    assert(source.includes(from),name);
    let code=source.replace(from,to);
    for(const file of ['annotation-compounds.mjs','annotation-operations.mjs','history-views.mjs','reversal-results.mjs'])
      code=code.replace(`'./${file}'`,JSON.stringify(new URL(`./${file}`,import.meta.url).href));
    for(const file of ['history-reversal.json','operation-families.json'])
      code=code.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(`./${file}`,import.meta.url).href)})`);
    const module=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    assert.throws(()=>module.checkHistoryReversal(),{code:'ERR_ASSERTION'},name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkHistoryReversal(),mutations:await checkReversalMutations()},null,2));
