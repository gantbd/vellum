// Scoped history inspection over the actual normalized compound registry.
// Current grants, complete retained history and companion truth remain premises.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {publishAnnotationPlan,reverseAnnotationGroup} from './annotation-compounds.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./history-queries.json',import.meta.url),'utf8'));
const copy=structuredClone;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const unavailable=()=>({status:'unavailable'});
const gates=['granted','retained','live','continuous','transferReady','historyCurrent'];
export const historyQueryDirections=Object.freeze(['undo','redo']);
const inputForms=freeze(Object.entries(catalog.variants).map(([operation,extra])=>({family:catalog.family,contract:catalog.contract,variant:operation,operation,fields:[...catalog.common,...extra],directions:historyQueryDirections})));
export function historyQueryInputForms(){return Object.freeze([...inputForms]);}

// bindings are owner-resolved projection references, not caller-supplied grants.
export function captureHistory(state,binding,limits){
  if(state.groups.length>limits.maxGroups||Object.keys(state.frontiers).length>limits.maxScopes||
      state.groups.some(g=>g.scopes.length>limits.maxParticipants)||
      binding.scopes.length>limits.maxScopes||binding.groups.length>limits.maxGroups)return null;
  for(const rows of [binding.scopes,binding.groups])
    if(new Set(rows.map(r=>r.internal)).size!==rows.length||new Set(rows.map(r=>r.reference)).size!==rows.length)return null;
  const n={binding:copy(binding),limits:copy(limits),groups:copy(state.groups.map(g=>({id:g.id,valid:g.valid,scopes:g.scopes}))),frontiers:copy(state.frontiers)};
  if(Buffer.byteLength(JSON.stringify(n),'utf8')>limits.maxCaptureBytes)return null;
  return freeze(n);
}

export function readHistory(capture,request,current){
  if(!capture||!request||!Object.hasOwn(catalog.variants,request.operation)||
      !exact(request,[...catalog.common,...catalog.variants[request.operation]])||request.contract!==catalog.contract)return unavailable();
  const {binding,groups,frontiers,limits}=capture;
  if(!historyQueryDirections.includes(request.direction)||request.context!==binding.context||request.profile!==binding.profile||
      !equal(current.binding,binding)||!gates.every(k=>current[k]))return unavailable();
  const scope=binding.scopes.find(s=>s.reference===request.scope);
  if(!scope||!scope.complete[request.direction])return unavailable();
  const top=frontiers[scope.internal]?.[request.direction]?.at(-1)??null;
  const publicGroup=id=>binding.groups.find(g=>g.internal===id);
  const choose=request.operation==='frontier'||request.group===null?top:
    publicGroup(top)?.reference===request.group?top:
      binding.groups.find(g=>g.reference===request.group)?.internal;
  if(choose===null){
    if(request.operation==='frontier')return {status:'frontier',context:binding.context,group:null};
    if(request.operation==='availability')return {status:'availability',context:binding.context,value:'empty'};
    return unavailable();
  }
  const group=groups.find(g=>g.id===choose),publicEntry=publicGroup(choose);
  if(!group?.valid||!publicEntry||!group.scopes.includes(scope.internal)||!publicEntry.evidenceReady)return unavailable();
  if(request.operation==='frontier')return {status:'frontier',context:binding.context,group:publicEntry.reference};
  const blocked=group.scopes.filter(s=>frontiers[s]?.[request.direction]?.at(-1)!==group.id);
  if(request.operation==='availability')return {status:'availability',context:binding.context,value:blocked.length?'blocked':'eligible'};
  if(!binding.details)return unavailable();
  // This is the complete permitted projection, not a complete participant graph.
  const details=blocked.flatMap(s=>{
    const visibleScope=binding.scopes.find(x=>x.internal===s);
    const visibleGroup=publicGroup(frontiers[s]?.[request.direction]?.at(-1));
    return visibleScope&&visibleGroup?[{scope:visibleScope.reference,group:visibleGroup.reference}]:[];
  }).sort((a,b)=>a.scope<b.scope?-1:a.scope>b.scope?1:0);
  if(details.length>limits.maxDetails)return unavailable();
  return {status:'blocking-detail',context:binding.context,blockers:details};
}

export function checkHistoryQueries(){
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  assert.deepEqual(Object.keys(catalog.variants).sort(),families.find(f=>f.id==='F04').variants.toSorted());
  const doc=id=>({authority:'a',id,incarnation:1}),scope=id=>`source:${JSON.stringify(doc(id))}`;
  const initial={authority:'a',publication:0,documents:['A','B'].map(id=>({document:doc(id),source:[97],revision:1,annotationSet:1,records:[]})),groups:[],frontiers:{}};
  const checks={authorized:true,bounded:true,notStopped:true,interpretationCurrent:true,companionsReady:true,versionsAvailable:true,historyReady:true,completeIdentity:true,maxParticipants:8,targetRestorationValid:true};
  const edit=(state,id,ids)=>publishAnnotationPlan(state,{contract:'owned-annotation-plan/1',invocation:id,descriptor:`candidate-${id}`,annotations:[],sources:ids.map(name=>{
    const d=state.documents.find(d=>d.document.id===name);
    return {document:doc(name),expectedSource:d.revision,expectedAnnotationSet:d.annotationSet,edits:[{lo:0,hi:1,expected:copy(d.source),ins:[d.source[0]+1]}]};
  })},checks).state;
  const base=edit(initial,'G',['A','B']),later=edit(base,'H',['B']);
  const binding={context:'owned-history-context',profile:'p',recipient:'r',purpose:'inspect',destination:'view',policy:'epoch1',continuity:'c1',details:true,
    scopes:[{internal:scope('A'),reference:'scope-a',complete:{undo:true,redo:true}},{internal:scope('B'),reference:'scope-b',complete:{undo:true,redo:true}}],
    groups:[{internal:'G',reference:'group-g',evidenceReady:true},{internal:'H',reference:'group-h',evidenceReady:true}]};
  const limits={maxGroups:8,maxScopes:8,maxParticipants:8,maxDetails:2,maxCaptureBytes:8192};
  const cap=captureHistory(later,binding,limits),current={binding:copy(binding),...Object.fromEntries(gates.map(k=>[k,true]))};
  const request=operation=>({contract:catalog.contract,operation,context:binding.context,profile:'p',scope:'scope-a',direction:'undo',...(operation==='frontier'?{}:{group:'group-g'})});
  const controls=[];const control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const deny=(name,r=request('availability'),c=current,capture=cap)=>control(name,equal(readHistory(capture,r,c),unavailable()));
  let releaseCases=0,pairedCases=0;
  for(const operation of Object.keys(catalog.variants))for(let mask=0;mask<64;mask++){
    const c={...copy(current),...Object.fromEntries(gates.map((k,i)=>[k,!!(mask&(1<<i))]))};
    const before=copy({cap,c,r:request(operation)}),out=readHistory(cap,request(operation),c);
    assert.equal(out.status,mask===63?({frontier:'frontier',availability:'availability','permitted-blocking-detail':'blocking-detail'})[operation]:'unavailable');
    assert.deepEqual({cap,c,r:request(operation)},before);releaseCases++;
  }
  control('whole-group-blocked',readHistory(cap,request('availability'),current).value==='blocked');
  control('visible-blocker-only',equal(readHistory(cap,request('permitted-blocking-detail'),current).blockers,[{scope:'scope-b',group:'group-h'}]));
  const undone=reverseAnnotationGroup(later,'H','undo',checks);
  assert.equal(undone.status,'applied');
  const ready=captureHistory(undone.state,binding,limits);
  control('actual-prerequisite-undo-makes-group-eligible',readHistory(ready,request('availability'),current).value==='eligible');
  control('actual-complete-reversal-applies',reverseAnnotationGroup(undone.state,'G','undo',checks).status==='applied');
  control('inspection-not-a-reservation',reverseAnnotationGroup(later,'G','undo',checks).status==='not-published');
  control('inspection-is-not-write-authority',reverseAnnotationGroup(undone.state,'G','undo',{...checks,authorized:false}).status==='not-published');
  for(let secret=0;secret<8;secret++){
    const world=copy(later);world.frontiers[scope('B')].undo=['G',`private-${secret}`];world.groups[1].id=`private-${secret}`;
    const hidden={...copy(binding),scopes:[copy(binding.scopes[0])],groups:[copy(binding.groups[0])]};
    const l=captureHistory(later,hidden,limits),r=captureHistory(world,hidden,limits),c={...current,binding:hidden};
    for(const operation of Object.keys(catalog.variants)){
      assert.deepEqual(readHistory(l,request(operation),c),readHistory(r,request(operation),c));pairedCases++;
    }
    control(`hidden-blocker-${secret}-not-in-detail`,equal(readHistory(r,request('permitted-blocking-detail'),c).blockers,[]));
  }
  for(const k of ['context','profile','recipient','purpose','destination','policy','continuity'])
    deny(`current-${k}`,undefined,{...current,binding:{...copy(binding),[k]:'other'}});
  deny('regrant-cannot-revive-old-context',undefined,{...current,continuous:false});
  deny('stale-history-not-current-advice',undefined,{...current,historyCurrent:false});
  deny('unknown-contract',{...request('availability'),contract:'history-query/2'});
  deny('unknown-operation',{...request('availability'),operation:'execute'});
  deny('unknown-field',{...request('availability'),offset:0});
  deny('missing-field',Object.fromEntries(Object.entries(request('availability')).filter(([k])=>k!=='group')));
  deny('unknown-scope',{...request('availability'),scope:'private-scope'});
  deny('unknown-group',{...request('availability'),group:'private-group'});
  deny('wrong-direction',{...request('availability'),direction:'earlier'});
  deny('raw-group-is-not-projection-reference',{...request('availability'),group:'G'});
  const noDetails={...copy(binding),details:false};
  deny('details-require-separate-projection',request('permitted-blocking-detail'),{...current,binding:noDetails},captureHistory(later,noDetails,limits));
  const missing=copy(binding);missing.groups[0].evidenceReady=false;
  deny('missing-evidence-not-eligible',undefined,{...current,binding:missing},captureHistory(later,missing,limits));
  const incomplete=copy(binding);incomplete.scopes[0].complete.undo=false;
  deny('unknown-history-floor-not-empty',{...request('availability'),group:null},{...current,binding:incomplete},captureHistory(initial,incomplete,limits));
  const empty=captureHistory(initial,binding,limits);
  control('established-empty-frontier',equal(readHistory(empty,request('frontier'),current),{status:'frontier',context:binding.context,group:null}));
  control('established-empty-availability',readHistory(empty,{...request('availability'),group:null},current).value==='empty');
  control('redo-is-separate',readHistory(cap,{...request('availability'),direction:'redo',group:null},current).value==='empty');
  const hiddenHead={...copy(binding),groups:[]};
  deny('withheld-frontier-not-empty',request('frontier'),{...current,binding:hiddenHead},captureHistory(later,hiddenHead,limits));
  deny('detail-bound-not-partial-success',request('permitted-blocking-detail'),current,captureHistory(later,binding,{...limits,maxDetails:0}));
  control('capture-bound',captureHistory(later,binding,{...limits,maxCaptureBytes:1})===null);
  control('group-bound',captureHistory(later,binding,{...limits,maxGroups:1})===null);
  control('scope-bound',captureHistory(later,binding,{...limits,maxScopes:1})===null);
  control('participant-bound',captureHistory(later,binding,{...limits,maxParticipants:1})===null);
  control('duplicate-reference-refused',captureHistory(later,{...binding,groups:[binding.groups[0],binding.groups[0]]},limits)===null);
  deny('failed-capture-not-empty',undefined,current,null);
  const saved=copy(cap);binding.groups[0].reference='mutated';
  control('input-does-not-alias',equal(cap,saved));
  const result=readHistory(cap,request('permitted-blocking-detail'),current);result.blockers[0].scope='mutated';
  control('result-does-not-alias',equal(cap,saved));
  control('diagnostics-not-authority',equal(readHistory(cap,request('availability'),current),readHistory(cap,request('availability'),{...current,diagnosticFailed:true})));
  control('captured-registry-frozen',Object.isFrozen(cap.frontiers));
  return {status:'history-query-products-only',variants:3,releaseCases,pairedCases,controls,
    historyContextIssuanceProved:false,fullReversalEligibilityProved:false,implementationRefinement:false};
}
export async function checkHistoryQueryMutations(){
  const source=fs.readFileSync(new URL('./history-queries.mjs',import.meta.url),'utf8');
  const mutants=[
    ['ignore-other-participant-frontiers',"group.scopes.filter(s=>frontiers[s]?.[request.direction]?.at(-1)!==group.id)","group.scopes.filter(s=>s===scope.internal&&frontiers[s]?.[request.direction]?.at(-1)!==group.id)"],
    ['ignore-current-history',"!gates.every(k=>current[k])","!gates.filter(k=>k!=='historyCurrent').every(k=>current[k])"],
    ['ignore-current-projection',"!equal(current.binding,binding)","false"],
    ['expose-private-blocker',"return visibleScope&&visibleGroup?[{scope:visibleScope.reference,group:visibleGroup.reference}]:[];","return [{scope:visibleScope?.reference??s,group:visibleGroup?.reference??frontiers[s]?.[request.direction]?.at(-1)}];"],
    ['unknown-floor-as-empty',"if(!scope||!scope.complete[request.direction])return unavailable();","if(!scope)return unavailable();"]
  ];
  const killed=[];
  for(const [name,from,to] of mutants){
    assert(source.includes(from),name);
    const code=source.replace(from,to).replace("'./annotation-compounds.mjs'",JSON.stringify(new URL('./annotation-compounds.mjs',import.meta.url).href))
      .replace("new URL('./history-queries.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./history-queries.json',import.meta.url).href)})`)
      .replace("new URL('./operation-families.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./operation-families.json',import.meta.url).href)})`);
    const module=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    assert.throws(()=>module.checkHistoryQueries(),{code:'ERR_ASSERTION'},name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkHistoryQueries(),mutations:await checkHistoryQueryMutations()},null,2));
