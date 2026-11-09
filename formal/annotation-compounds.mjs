// Normalized owned-plan reference; not native decoding or a complete history engine.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {validateCanonicalBatch,emitByOrigin,applySequentialWithInverses} from './text-oracles.mjs';
import {mapAttachment} from './mapping-profiles.mjs';
import {changeAnnotation} from './annotation-records.mjs';
import {prepareSourceJournal,commitSourceJournal} from './parser-source.mjs';
import {validCompoundTarget} from './annotation-targets.mjs';
import {isReadDependencyPlan,readDependencyPlanFields,declaredReadsCurrent} from './read-dependencies.mjs';
import {isSourceScopePlan,declaredWriteScopesCurrent} from './source-scopes.mjs';
import {membershipPlanReady,membershipReady,validMembershipRoot,createMembership,deleteMembership,reverseMembership} from './annotation-membership.mjs';
export {validCompoundTarget};
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-compounds.json',import.meta.url),'utf8'));
const copy=structuredClone;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const identity=d=>JSON.stringify({authority:d.authority,id:d.id,incarnation:d.incarnation});
const sourceScope=d=>`source:${identity(d)}`;
const annotationScope=(d,r)=>`annotation:${identity(d)}:${r.id}:${r.incarnation}`;
const gateNames=['authorized','bounded','notStopped','interpretationCurrent','companionsReady','versionsAvailable','historyReady'];
const locationEqual=(a,b)=>equal(a,b);
const natural=n=>Number.isSafeInteger(n)&&n>=0;
const publicationVersionsValid=(state,next)=>natural(state.publication)&&state.publication<Number.MAX_SAFE_INTEGER&&next.documents.every(d=>natural(d.revision)&&natural(d.annotationSet)&&d.records.every(r=>natural(r.bodyVersion)&&natural(r.targetGeneration)));

export function mapCompoundPlacement(source,edits,placement,revision){
  if(placement.state==='detached')return copy(placement);
  const t=placement.target;
  if(t.kind==='document')return {state:placement.state,sourceRevision:revision,target:copy(t)};
  const mapped=mapAttachment(source,edits,{profile:t.kind==='range'?'content':t.profile,
    affinity:t.kind==='range'?'left':t.affinity==='before'?'left':'right',lo:t.kind==='range'?t.start:t.offset,
    hi:t.kind==='range'?t.end:t.offset,evidence:placement.state==='affected'?'affected':'mapped'});
  if(mapped.evidence==='detached')return {state:'detached'};
  const target=t.kind==='range'?{kind:'range',start:mapped.location.lo,end:mapped.location.hi}:{...t,offset:mapped.location.lo};
  return {state:mapped.evidence==='affected'?'affected':'attached',sourceRevision:revision,target};
}
function targetFor(intent,before,after,descriptor){
  if(!intent||!Object.hasOwn(catalog.targetBases,intent.basis)||!exact(intent,catalog.targetBases[intent.basis]))return null;
  if(intent.basis==='candidate'){
    if(intent.descriptor!==descriptor||!validCompoundTarget(after.source,intent.target))return null;
    return {authored:{target:copy(intent.target),sourceRevision:after.revision},placement:{state:'attached',sourceRevision:after.revision,target:copy(intent.target)}};
  }
  if(intent.sourceRevision!==before.revision||!validCompoundTarget(before.source,intent.target))return null;
  return {authored:{target:copy(intent.target),sourceRevision:before.revision},placement:mapCompoundPlacement(before.source,after.edits??[],
    {state:'attached',sourceRevision:before.revision,target:intent.target},after.revision)};
}

export function compoundPlanShape(plan){const readPlan=isReadDependencyPlan(plan);return !!(exact(plan,readPlan?readDependencyPlanFields(plan):catalog.planFields)&&(plan.contract===catalog.contract||readPlan)&&plan.invocation&&plan.descriptor&&Array.isArray(plan.sources)&&Array.isArray(plan.annotations));}
export function decideAnnotationPlan(state,plan,checks,readContext=null){
  const refuse=()=>({status:'not-published',state});
  const readPlan=isReadDependencyPlan(plan);
  if(!compoundPlanShape(plan)||plan.sources.length+plan.annotations.length>checks.maxParticipants||
      !gateNames.filter(k=>k!=='versionsAvailable').every(k=>checks[k]))return refuse();
  if(readPlan&&!declaredReadsCurrent(state,plan.reads,checks.readDependencyAuthority,readContext,checks.readDependencyLimits,plan.contract,checks.configurationReadProfile))return refuse();
  if(isSourceScopePlan(plan)&&!declaredWriteScopesCurrent(state,plan,checks.sourceScopeLimits))return refuse();
  const lifecycle=state.documents.some(d=>Object.hasOwn(d,'annotationLifecycle'));
  if(lifecycle&&!validMembershipRoot(state,checks.membershipLimits))return refuse();
  const membership=plan.annotations.some(a=>['create','delete'].includes(a?.kind));if(membership&&!membershipPlanReady(state,plan,checks))return refuse();
  const n=copy(state),changes=[],edited=new Map(),seenSources=new Set(),seenAnnotations=new Set();
  for(const input of plan.sources){
    if(!exact(input,catalog.sourceFields)||input.document.authority!==state.authority||seenSources.has(identity(input.document)))return refuse();seenSources.add(identity(input.document));
    const d=n.documents.find(d=>equal(d.document,input.document)),before=state.documents.find(d=>equal(d.document,input.document));
    if(!d||d.live===false||d.revision!==input.expectedSource||d.annotationSet!==input.expectedAnnotationSet)return refuse();
    if(!Array.isArray(input.edits)||input.edits.some(e=>!exact(e,catalog.editFields)))return refuse();
    try{validateCanonicalBatch(d.source,input.edits);}catch{return refuse();}
    if(input.edits.some(e=>!equal(e.expected,d.source.slice(e.lo,e.hi))))return refuse();
    const next=emitByOrigin(d.source,input.edits);edited.set(identity(input.document),input.edits);
    if(equal(next,d.source)){
      // Mapping has its own semantic projection; equal bytes do not erase it.
      for(const row of d.records)row.placement=mapCompoundPlacement(d.source,input.edits,row.placement,d.revision);
      continue;
    }
    d.source=next;d.revision++;
    for(const row of d.records)row.placement=mapCompoundPlacement(before.source,input.edits,row.placement,d.revision);
    const inverseSequence=applySequentialWithInverses(before.source,input.edits).inverses.reverse();
    changes.push({scope:sourceScope(d.document),kind:'source',document:copy(d.document),before:copy(before.source),after:copy(next),forward:copy(input.edits),inverseSequence});
  }
  for(const input of plan.annotations){
    if(!input||!Object.hasOwn(catalog.annotationFields,input.kind)||!exact(input,catalog.annotationFields[input.kind])||input.document.authority!==state.authority)return refuse();
    const key=JSON.stringify([identity(input.document),input.id,input.incarnation,input.kind]);if(seenAnnotations.has(key))return refuse();seenAnnotations.add(key);
    const d=n.documents.find(d=>equal(d.document,input.document)),beforeDoc=state.documents.find(d=>equal(d.document,input.document));
    if(!d||d.live===false)return refuse();const row=d.records.find(r=>r.id===input.id&&r.incarnation===input.incarnation),old=beforeDoc.records.find(r=>r.id===input.id&&r.incarnation===input.incarnation);
    if(input.kind==='create'||input.kind==='delete'){
      const target=input.kind==='create'?targetFor(input.target,beforeDoc,{...d,edits:edited.get(identity(d.document))??[]},plan.descriptor):null;
      if(input.kind==='create'&&!target||input.kind==='delete'&&!old)return refuse();
      const result=input.kind==='create'?createMembership(d,input,target,checks):deleteMembership(d,input,old);if(!result)return refuse();
      changes.push({scope:annotationScope(d.document,input),kind:'membership',document:copy(d.document),id:input.id,incarnation:input.incarnation,...result,beforeSourceRevision:beforeDoc.revision,afterSourceRevision:d.revision});continue;
    }
    if(!row||!old)return refuse();
    let request,preparedTarget;
    if(input.kind==='body')request={kind:'body',id:input.id,incarnation:input.incarnation,expectedBody:input.expectedBody,body:input.body};
    else if(input.kind==='reattach'){
      preparedTarget=targetFor(input.target,beforeDoc,{...d,edits:edited.get(identity(d.document))??[]},plan.descriptor);if(!preparedTarget)return refuse();
      request={kind:'reattach',id:input.id,incarnation:input.incarnation,expectedTarget:input.expectedTarget,target:preparedTarget.authored};
    }else return refuse();
    const result=changeAnnotation({record:row,retired:[]},request,{authorized:true,bounded:true,notStopped:true,targetCurrent:true});
    if(result.status==='refused')return refuse();
    if(result.status==='unchanged'&&(!preparedTarget||equal(preparedTarget.placement,row.placement)))continue;
    const updated=result.status==='unchanged'?{...copy(row),targetGeneration:row.targetGeneration+1}:result.state.record;
    d.records[d.records.indexOf(row)]=updated;
    const field=input.kind==='body'?'body':'target';
    if(field==='target')updated.placement=copy(preparedTarget.placement);
    changes.push({scope:annotationScope(d.document,row),kind:field,document:copy(d.document),id:row.id,incarnation:row.incarnation,
      before:copy(old[field]),after:copy(updated[field]),beforePlacement:copy(old.placement),afterPlacement:copy(updated.placement),
      beforeSourceRevision:beforeDoc.revision,afterSourceRevision:d.revision});
  }
  for(const d of n.documents){const old=state.documents.find(x=>equal(x.document,d.document));if(!equal(d.records,old.records))d.annotationSet++;}
  if(!changes.length){
    const mappingChanged=n.documents.some((d,i)=>!equal(d.records,state.documents[i].records));
    if(!mappingChanged)return checks.completeIdentity?{status:'unchanged',state}:refuse();
    if(!checks.versionsAvailable||!publicationVersionsValid(state,n))return refuse();
    n.publication++;
    // No source bytes or authored annotation field changed: no authored undo entry.
    if(lifecycle&&!validMembershipRoot(n,checks.membershipLimits))return refuse();
    return {status:'applied',state:n};
  }
  if(!checks.versionsAvailable||!publicationVersionsValid(state,n))return refuse();
  if(state.groups.some(g=>g.id===plan.invocation))return refuse();
  const scopes=[...new Set(changes.map(c=>c.scope))];
  // New work invalidates complete linked redo groups transitively, not one stack suffix.
  const bad=new Set(scopes.flatMap(s=>n.frontiers[s]?.redo??[]));let changed=true;
  while(changed){changed=false;for(const frontier of Object.values(n.frontiers)){
    const boundary=frontier.redo.findLastIndex(id=>bad.has(id));
    for(const id of frontier.redo.slice(0,boundary+1))if(!bad.has(id)){bad.add(id);changed=true;}
  }}
  for(const frontier of Object.values(n.frontiers))frontier.redo=frontier.redo.filter(id=>!bad.has(id));
  for(const group of n.groups)if(bad.has(group.id))group.valid=false;
  for(const scope of scopes){n.frontiers[scope]??={undo:[],redo:[]};n.frontiers[scope].undo.push(plan.invocation);}
  n.groups.push({id:plan.invocation,scopes,changes,valid:true});n.publication++;
  if(lifecycle&&!validMembershipRoot(n,checks.membershipLimits))return refuse();
  return {status:'applied',state:n};
}

export function publishAnnotationPlan(state,plan,checks,readContext=null){
  const result=decideAnnotationPlan(state,plan,checks,readContext);
  if(result.status!=='applied'||result.state.groups.length===state.groups.length)return result;
  const n=result.state,changes=n.groups.at(-1).changes;
  const journal=prepareSourceJournal(state,n,changes.filter(c=>c.kind==='source').map(c=>({document:c.document,batches:[c.forward]})),checks.parserSourceLimits,checks.parserSourceReserved);
  commitSourceJournal(n,journal);
  return result;
}

export function reverseAnnotationGroup(state,id,direction,checks){
  const refuse=()=>({status:'not-published',state});
  if(!['undo','redo'].includes(direction)||!gateNames.every(k=>checks[k]))return refuse();
  const group=state.groups.find(g=>g.id===id);if(!group?.valid||group.scopes.some(s=>state.frontiers[s]?.[direction].at(-1)!==id))return refuse();
  const lifecycle=state.documents.some(d=>Object.hasOwn(d,'annotationLifecycle'));if(lifecycle&&!validMembershipRoot(state,checks.membershipLimits))return refuse();
  const membership=group.changes.some(c=>c.kind==='membership');if(membership&&!membershipReady(state,checks))return refuse();
  const n=copy(state),expected=direction==='undo'?'after':'before',desired=direction==='undo'?'before':'after';
  const ordered=direction==='undo'?[...group.changes].reverse():group.changes;
  const changes=[...ordered.filter(c=>c.kind==='source'),...ordered.filter(c=>c.kind!=='source')];
  // Reversal restores authored fields, never whole historical rows or old numeric versions.
  for(const change of changes){
    const d=n.documents.find(d=>equal(d.document,change.document));if(!d||d.live===false)return refuse();
    if(change.kind==='source'){
      if(!equal(d.source,change[expected]))return refuse();
      const batches=direction==='undo'?change.inverseSequence.map(e=>[e]):[change.forward];d.revision++;
      for(const edits of batches){
        try{validateCanonicalBatch(d.source,edits);}catch{return refuse();}
        const original=copy(d.source);d.source=emitByOrigin(original,edits);
        for(const row of d.records)row.placement=mapCompoundPlacement(original,edits,row.placement,d.revision);
      }
      if(!equal(d.source,change[desired]))return refuse();
    }else if(change.kind==='membership'){
      if(!reverseMembership(d,change,expected,desired,group,checks))return refuse();
    }else if(['body','target'].includes(change.kind)){
      const row=d.records.find(r=>r.id===change.id&&r.incarnation===change.incarnation);if(!row||!equal(row[change.kind],change[expected]))return refuse();
      row[change.kind]=copy(change[desired]);
      if(change.kind==='body')row.bodyVersion++;
      else{
        if(!checks.targetRestorationValid||(!group.changes.some(c=>c.kind==='source'&&equal(c.document,change.document))&&d.revision!==change[expected+'SourceRevision']))return refuse();
        row.targetGeneration++;
        const placement=copy(change[desired+'Placement']);
        // General mapping-history restoration is not proved by this reference.
        if(placement.state!=='detached'&&!validCompoundTarget(d.source,placement.target))return refuse();
        row.placement=placement.state==='detached'?placement:{...placement,sourceRevision:d.revision};}
    }else throw Error('unknown annotation history participant');
  }
  for(const d of n.documents){const old=state.documents.find(x=>equal(x.document,d.document));if(!equal(d.records,old.records))d.annotationSet++;}
  const other=direction==='undo'?'redo':'undo';
  for(const scope of group.scopes){n.frontiers[scope][direction].pop();n.frontiers[scope][other].push(id);}
  n.publication++;
  if(lifecycle&&!validMembershipRoot(n,checks.membershipLimits))return refuse();
  const journal=prepareSourceJournal(state,n,changes.filter(c=>c.kind==='source').map(c=>({document:c.document,batches:direction==='undo'?c.inverseSequence.map(e=>[e]):[c.forward]})),checks.parserSourceLimits,checks.parserSourceReserved);
  commitSourceJournal(n,journal);return {status:'applied',state:n};
}

export function checkAnnotationCompounds(){
  const document={authority:'a',id:'A',incarnation:1},other={authority:'a',id:'B',incarnation:1};
  const target={kind:'range',start:1,end:3};
  const record={id:'note',incarnation:1,body:'old',bodyVersion:1,target:{sourceRevision:1,target:copy(target)},targetGeneration:1,
    placement:{state:'attached',sourceRevision:1,target:copy(target)}};
  const state={authority:'a',publication:0,documents:[{document,source:[97,98,99,100],revision:1,annotationSet:1,records:[record]},
    {document:other,source:[120],revision:1,annotationSet:1,records:[]}],groups:[],frontiers:{}};
  const checks={...Object.fromEntries(gateNames.map(k=>[k,true])),completeIdentity:true,targetRestorationValid:true,maxParticipants:4};
  const source={document,expectedSource:1,expectedAnnotationSet:1,edits:[{lo:1,hi:3,expected:[98,99],ins:[]}]};
  const body={kind:'body',document,id:'note',incarnation:1,expectedBody:1,body:'new'};
  const plan={contract:catalog.contract,invocation:'G',descriptor:'candidate-G',sources:[source],annotations:[body]};
  const controls=[],control=(name,ok)=>{assert(ok,name);controls.push(name);};let gateCases=0;
  for(let mask=0;mask<128;mask++){
    const gate={...checks,...Object.fromEntries(gateNames.map((k,i)=>[k,!!(mask&(1<<i))]))};
    const before=copy({state,plan,gate}),out=publishAnnotationPlan(state,plan,gate);
    assert.equal(out.status,mask===127?'applied':'not-published');if(mask!==127)assert.deepEqual(out.state,state);
    else{assert.deepEqual(out.state.documents[0].source,[97,100]);assert.equal(out.state.documents[0].records[0].body,'new');assert.deepEqual(out.state.documents[0].records[0].placement,{state:'detached'});assert.equal(out.state.groups[0].scopes.length,2);}
    assert.deepEqual({state,plan,gate},before);gateCases++;
  }
  const applied=publishAnnotationPlan(state,plan,checks),undone=reverseAnnotationGroup(applied.state,'G','undo',checks),redone=reverseAnnotationGroup(undone.state,'G','redo',checks);
  control('compound-body-source-roundtrip',undone.status==='applied'&&redone.status==='applied'&&undone.state.documents[0].records[0].body==='old'&&equal(redone.state.documents[0].source,[97,100]));
  control('reversal-versions-fresh',undone.state.documents[0].revision===3&&redone.state.documents[0].revision===4&&redone.state.documents[0].records[0].bodyVersion===4);
  const sourceOnly=publishAnnotationPlan(state,{...plan,annotations:[]},checks),laterBody=publishAnnotationPlan(sourceOnly.state,{...plan,invocation:'H',sources:[],annotations:[body]},checks);
  const preserve=reverseAnnotationGroup(laterBody.state,'G','undo',checks);
  control('source-undo-preserves-later-body',preserve.status==='applied'&&preserve.state.documents[0].records[0].body==='new'&&preserve.state.documents[0].records[0].bodyVersion===2);
  control('mapping-not-authored-frontier',sourceOnly.state.groups[0].scopes.length===1&&!Object.keys(sourceOnly.state.frontiers).some(k=>k.startsWith('annotation:')));
  const laterCompound=publishAnnotationPlan(applied.state,{...plan,invocation:'H',sources:[],annotations:[{...body,expectedBody:2,body:'later'}]},checks);
  control('later-authored-participant-blocks-whole-compound',equal(reverseAnnotationGroup(laterCompound.state,'G','undo',checks),{status:'not-published',state:laterCompound.state}));
  const equalAgain=publishAnnotationPlan(laterCompound.state,{...plan,invocation:'I',sources:[],annotations:[{...body,expectedBody:3,body:'new'}]},checks);
  control('equal-content-does-not-bypass-history-frontier',equal(reverseAnnotationGroup(equalAgain.state,'G','undo',checks),{status:'not-published',state:equalAgain.state}));
  const reject=(name,p=plan,s=state)=>{const before=copy({s,p}),out=publishAnnotationPlan(s,p,checks);assert.deepEqual(out,{status:'not-published',state:s});assert.deepEqual({s,p},before);controls.push(name);};
  reject('source-stale',{...plan,sources:[{...source,expectedSource:2}]});
  reject('private-attachment-set-stale',{...plan,sources:[{...source,expectedAnnotationSet:2}]});
  reject('body-stale-after-source-preparation',{...plan,annotations:[{...body,expectedBody:2}]});
  reject('wrong-owner',{...plan,annotations:[{...body,document:{...document,incarnation:2}}]});
  reject('duplicate-source',{...plan,sources:[source,source]});reject('duplicate-authored-record',{...plan,annotations:[body,body]});
  reject('unknown-operation',{...plan,annotations:[{...body,kind:'evaluate'}]});
  const noop=publishAnnotationPlan(state,{...plan,sources:[],annotations:[{...body,body:'old'}]},checks);
  control('complete-noop-no-history',equal(noop,{status:'unchanged',state}));
  control('noop-needs-no-unused-counter',equal(publishAnnotationPlan(state,{...plan,sources:[],annotations:[{...body,body:'old'}]},
    {...checks,versionsAvailable:false}),{status:'unchanged',state}));
  const equalBytes=publishAnnotationPlan(state,{...plan,sources:[{...source,edits:[{lo:1,hi:3,expected:[98,99],ins:[98,99]}]}],annotations:[]},checks);
  control('equal-bytes-companion-not-classified-noop',equalBytes.status==='applied'&&equalBytes.state.documents[0].revision===1&&equalBytes.state.documents[0].annotationSet===2&&equal(equalBytes.state.groups,state.groups));
  const targetShapes=[{kind:'document'},{kind:'point',offset:1,affinity:'before',profile:'position'},
    {kind:'point',offset:2,affinity:'after',profile:'content'},{kind:'range',start:0,end:2}];let targetCases=0;
  for(const target of targetShapes)for(const basis of ['before','candidate']){
    const intent=basis==='before'?{basis,sourceRevision:1,target}:{basis,descriptor:plan.descriptor,target};
    const reattach={kind:'reattach',document,id:'note',incarnation:1,expectedTarget:1,target:intent};
    const out=publishAnnotationPlan(state,{...plan,annotations:[reattach]},checks);assert.equal(out.status,'applied');
    assert.equal(out.state.documents[0].records[0].targetGeneration,2);assert.equal(out.state.documents[0].records[0].body,'old');targetCases++;
    const undo=reverseAnnotationGroup(out.state,'G','undo',checks),redo=reverseAnnotationGroup(undo.state,'G','redo',checks);
    assert.equal(undo.status,'applied');assert.equal(redo.status,'applied');assert.equal(redo.state.documents[0].records[0].targetGeneration,4);
  }
  reject('candidate-target-wrong-descriptor',{...plan,annotations:[{kind:'reattach',document,id:'note',incarnation:1,expectedTarget:1,target:{basis:'candidate',descriptor:'other',target:{kind:'document'}}}]});
  const laterTarget=publishAnnotationPlan(sourceOnly.state,{...plan,invocation:'T',sources:[],annotations:[{kind:'reattach',document,id:'note',incarnation:1,expectedTarget:1,
    target:{basis:'before',sourceRevision:2,target:{kind:'point',offset:1,affinity:'after',profile:'position'}}}]},checks);
  const targetPreserved=reverseAnnotationGroup(laterTarget.state,'G','undo',checks);
  control('source-undo-preserves-later-authored-target',targetPreserved.status==='applied'&&targetPreserved.state.documents[0].records[0].targetGeneration===2&&equal(targetPreserved.state.documents[0].records[0].target,laterTarget.state.documents[0].records[0].target));
  const utf8=[97,240,159,153,130,98];
  control('target-rejects-utf8-interior',!validCompoundTarget(utf8,{kind:'point',offset:2,affinity:'before',profile:'position'}));
  control('target-rejects-empty-range',!validCompoundTarget(utf8,{kind:'range',start:1,end:1}));
  control('target-document-no-fabricated-range',validCompoundTarget([], {kind:'document'}));
  const newAfterUndo=publishAnnotationPlan(undone.state,{...plan,invocation:'N',sources:[],annotations:[{...body,expectedBody:3,body:'branch'}]},checks);
  control('new-work-retires-complete-linked-redo',newAfterUndo.status==='applied'&&!newAfterUndo.state.groups.find(g=>g.id==='G').valid&&Object.values(newAfterUndo.state.frontiers).every(f=>!f.redo.includes('G')));
  const bSource={document:other,expectedSource:1,expectedAnnotationSet:1,edits:[{lo:0,hi:1,expected:[120],ins:[121]}]};
  const multi=publishAnnotationPlan(state,{...plan,sources:[source,bSource]},checks);
  control('multi-document-source-comment-one-publication',multi.status==='applied'&&multi.state.publication===1&&multi.state.groups[0].scopes.length===3&&equal(multi.state.documents[1].source,[121]));
  const blocked=publishAnnotationPlan(multi.state,{...plan,invocation:'B-later',sources:[{...bSource,expectedSource:2,edits:[{lo:0,hi:1,expected:[121],ins:[122]}]}],annotations:[]},checks);
  control('one-source-frontier-blocks-whole-group',equal(reverseAnnotationGroup(blocked.state,'G','undo',checks),{status:'not-published',state:blocked.state}));
  const independent=publishAnnotationPlan(sourceOnly.state,{...plan,invocation:'independent-B',sources:[bSource],annotations:[]},checks);
  const independentUndo=reverseAnnotationGroup(independent.state,'G','undo',checks);
  control('unrelated-document-history-does-not-block',independentUndo.status==='applied'&&equal(independentUndo.state.documents[1],independent.state.documents[1]));
  const foreign=copy(state);foreign.documents[1].document.authority='foreign';
  reject('cross-authority-even-if-root-present',{...plan,sources:[{...bSource,document:foreign.documents[1].document}]},foreign);
  const utf8Invalid={kind:'reattach',document,id:'note',incarnation:1,expectedTarget:1,target:{basis:'candidate',descriptor:plan.descriptor,target:{kind:'range',start:1,end:9}}};
  reject('candidate-target-outside-result',{...plan,annotations:[utf8Invalid]});
  const sameTarget=publishAnnotationPlan(state,{...plan,sources:[],annotations:[{kind:'reattach',document,id:'note',incarnation:1,expectedTarget:1,
    target:{basis:'before',sourceRevision:1,target:copy(target)}}]},checks);
  control('same-complete-target-is-noop',sameTarget.status==='unchanged'&&equal(sameTarget.state,state));
  const move={kind:'reattach',document,id:'note',incarnation:1,expectedTarget:1,target:{basis:'candidate',descriptor:plan.descriptor,target:{kind:'document'}}};
  let fieldCases=0,fieldState;
  for(const annotations of [[body,move],[move,body]]){
    const out=publishAnnotationPlan(state,{...plan,annotations},checks);assert.equal(out.status,'applied');
    assert.equal(out.state.groups[0].scopes.length,2);assert.equal(out.state.documents[0].annotationSet,2);
    assert.equal(out.state.documents[0].records[0].bodyVersion,2);assert.equal(out.state.documents[0].records[0].targetGeneration,2);
    if(fieldState)assert.deepEqual(out.state.documents,fieldState);fieldState=copy(out.state.documents);
    const undo=reverseAnnotationGroup(out.state,'G','undo',checks);assert.equal(undo.status,'applied');assert.equal(undo.state.documents[0].records[0].body,'old');fieldCases++;
  }
  control('diagnostics-not-authority',equal(applied,publishAnnotationPlan(state,plan,{...checks,diagnosticFailed:true})));
  return {status:'annotation-compound-reference-only',gateCases,targetCases,fieldCases,controls,membershipReversalProved:false,mappingHistoryRestorationProved:false,implementationRefinement:false};
}
export async function checkCompoundMutations(){
  const source=fs.readFileSync(new URL('./annotation-compounds.mjs',import.meta.url),'utf8');
  const mutations=[
    ['omit-private-set-dependency','d.annotationSet!==input.expectedAnnotationSet','false'],
    ['body-is-source-history','scope:annotationScope(d.document,row)','scope:sourceScope(d.document)'],
    ['ignore-body-version','expectedBody:input.expectedBody','expectedBody:row.bodyVersion'],
    ['ignore-participant-frontier',"group.scopes.some(s=>state.frontiers[s]?.[direction].at(-1)!==id)",'false'],
    ['rollback-source-revision','d.revision++;\n      for(const edits','d.revision--;\n      for(const edits'],
    ['restore-old-row-on-source-undo','for(const row of d.records)row.placement=mapCompoundPlacement(original,edits,row.placement,d.revision);',
      "for(const row of d.records){row.body='old';row.placement=mapCompoundPlacement(original,edits,row.placement,d.revision);}"],
    ['ignore-candidate-descriptor','intent.descriptor!==descriptor','false'],
    ['clear-only-one-redo-stack','for(const frontier of Object.values(n.frontiers))frontier.redo=frontier.redo.filter(id=>!bad.has(id));',
      'for(const frontier of Object.values(n.frontiers).slice(0,1))frontier.redo=frontier.redo.filter(id=>!bad.has(id));']
  ];
  const rejected=[];
  for(const [name,from,to] of mutations){assert(source.includes(from),`missing mutation site ${name}`);let changed=source.replace(from,to);
    for(const file of ['text-oracles.mjs','mapping-profiles.mjs','annotation-records.mjs','parser-source.mjs','annotation-targets.mjs','read-dependencies.mjs','source-scopes.mjs','annotation-membership.mjs'])changed=changed.replace(`from './${file}'`,`from '${new URL(file,import.meta.url).href}'`);
    changed=changed.replaceAll("new URL('./annotation-compounds.json',import.meta.url)",`new URL('${new URL('annotation-compounds.json',import.meta.url).href}')`);
    const module=await import(`data:text/javascript;base64,${Buffer.from(changed).toString('base64')}`);
    let failed=false;try{module.checkAnnotationCompounds();}catch(e){assert.equal(e.code,'ERR_ASSERTION',`non-assertion failure ${name}`);failed=true;}
    assert(failed,`surviving mutation ${name}`);rejected.push(name);
  }
  return rejected;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkAnnotationCompounds(),mutations:await checkCompoundMutations()},null,2));
