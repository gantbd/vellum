// Owned view/ordinary-edit reference. Not native decoding or a complete editor engine.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {validateCanonicalBatch,emitByOrigin} from './text-oracles.mjs';
import {mapAttachment} from './mapping-profiles.mjs';
import {logicalLines,byteToLine} from './reference-profile.mjs';
import {publishAnnotationPlan,reverseAnnotationGroup} from './annotation-compounds.mjs';
import {resolveSearchOccurrence} from './search-occurrences.mjs';
import {carrySourceJournal} from './parser-source.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./selection-products.json',import.meta.url),'utf8'));
const copy=structuredClone,plans=new WeakMap();
export function selectionInputForms(){return Object.freeze(Object.entries(catalog.families).flatMap(([family,operations])=>operations.map(operation=>Object.freeze({family,contract:catalog.contract,operation,variant:operation,profileConstant:catalog.profile,fields:Object.freeze([...catalog.common,...catalog.variants[operation]])}))));}
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const bytes=s=>Array.from(Buffer.from(s,'utf8'));
const scalar=s=>typeof s==='string'&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Uint8Array.from(bytes(s)))===s;
const boundary=(s,p)=>Number.isSafeInteger(p)&&p>=0&&p<=s.length&&(p===s.length||(s[p]&0xc0)!==0x80);
const limitsValid=l=>l&&catalog.limits.every(k=>Number.isSafeInteger(l[k])&&l[k]>=0);
const gates=['authorized','sourceReadable','viewWritable','continuous','live','bounded','notStopped','interpretationCurrent','companionsReady'];
const signature=v=>({view:copy(v.view),document:copy(v.document),sourceRevision:v.sourceRevision,selectionGeneration:v.selectionGeneration,profile:v.profile,selections:copy(v.selections),primary:v.primary});
const viewKey=v=>JSON.stringify([v.view.authority,v.view.id,v.view.incarnation]);
const canonicalOrder=(a,b)=>viewKey(a)<viewKey(b)?-1:viewKey(a)>viewKey(b)?1:0;
const attached=(state,document)=>state.views.filter(v=>equal(v.document,document)).map(signature).sort(canonicalOrder);

export function normalizeSelection(source,selections,primary,limits){
  if(!limitsValid(limits)||source.length>limits.maxRootBytes||!Array.isArray(selections)||!selections.length||selections.length>limits.maxInputSelections||
      !Number.isSafeInteger(primary)||primary<0||primary>=selections.length)return null;
  try{validateCanonicalBatch(source,[]);}catch{return null;}
  if(selections.some(s=>!exact(s,catalog.selectionFields)||!boundary(source,s.anchor)||!boundary(source,s.head)||!catalog.affinities.includes(s.affinity)))return null;
  const rows=selections.map((s,i)=>({...s,lo:Math.min(s.anchor,s.head),hi:Math.max(s.anchor,s.head),members:[i]}));
  const ranges=[],carets=new Map();
  for(const row of rows.filter(r=>r.lo<r.hi).sort((a,b)=>a.lo-b.lo||a.hi-b.hi)){
    const last=ranges.at(-1);
    if(last&&row.lo<last.hi){last.hi=Math.max(last.hi,row.hi);last.members.push(...row.members);}else ranges.push(copy(row));
  }
  for(const row of rows.filter(r=>r.lo===r.hi)){
    const key=JSON.stringify([row.lo,row.affinity]);
    if(carets.has(key))carets.get(key).members.push(...row.members);else carets.set(key,copy(row));
  }
  const result=[...ranges,...carets.values()].map(r=>{
    const owner=r.members.includes(primary)?primary:Math.min(...r.members),s=selections[owner];
    return {lo:r.lo,hi:r.hi,primary:r.members.includes(primary),selection:{anchor:s.anchor>s.head?r.hi:r.lo,head:s.anchor>s.head?r.lo:r.hi,affinity:s.affinity}};
  }).sort((a,b)=>a.lo-b.lo||a.hi-b.hi||catalog.affinities.indexOf(a.selection.affinity)-catalog.affinities.indexOf(b.selection.affinity));
  const out={selections:result.map(r=>r.selection),primary:result.findIndex(r=>r.primary)};
  return result.length<=limits.maxSelections&&Buffer.byteLength(JSON.stringify(out),'utf8')<=limits.maxSelectionBytes?out:null;
}

function currentTarget(state,r,g,limits){
  if(!limitsValid(limits)||state.views.length>limits.maxInputViews||!gates.every(k=>g[k])||r.profile!==catalog.profile||r.document.authority!==state.authority)return null;
  const docs=state.documents.filter(d=>equal(d.document,r.document)),views=state.views.filter(v=>equal(v.view,r.view));
  if(docs.length!==1||views.length!==1)return null;const d=docs[0],v=views[0];
  if(d.live===false||!equal(v.document,r.document)||d.revision!==r.sourceRevision||v.sourceRevision!==r.sourceRevision||v.selectionGeneration!==r.expectedSelection||v.profile!==r.profile)return null;
  const n=normalizeSelection(d.source,v.selections,v.primary,limits);if(!n||!equal(n,{selections:v.selections,primary:v.primary}))return null;
  return {d,v};
}
function moved(source,selection,unit,extend){
  let target;
  if(unit==='document-start')target=0;else if(unit==='document-end')target=source.length;
  else{const location=byteToLine(source,selection.head);if(!location)return null;const row=logicalLines(source)[location.line];target=unit==='line-start'?row.start:row.bodyEnd;}
  return {anchor:extend?selection.anchor:target,head:target,affinity:selection.affinity};
}
export function mapSelection(source,edits,s){
  const mapped=mapAttachment(source,edits,{profile:'position',affinity:s.affinity==='before'?'left':'right',lo:Math.min(s.anchor,s.head),hi:Math.max(s.anchor,s.head),evidence:'mapped'}).location;
  return {anchor:s.anchor>s.head?mapped.hi:mapped.lo,head:s.anchor>s.head?mapped.lo:mapped.hi,affinity:s.affinity};
}
export {signature as selectionViewSignature,viewKey as selectionViewKey};

function prepareSelectionProduct(state,request,g,limits,derivedEdits=null){
  if(!request||!Object.hasOwn(catalog.variants,request.operation)||!exact(request,[...catalog.common,...catalog.variants[request.operation]])||
      request.contract!==catalog.contract||typeof request.invocation!=='string'||!request.invocation)return null;
  const target=currentTarget(state,request,g,limits);if(!target)return null;const {d,v}=target;
  let after,edits=[],post=[],required=[signature(v)];
  if(request.operation==='set-selection')after=normalizeSelection(d.source,request.selections,request.primary,limits);
  else if(request.operation==='select-search-match'){
    const occurrence=resolveSearchOccurrence(request.match,g.searchCurrent);
    if(!occurrence||!equal(occurrence.ref,{authority:request.document.authority,document:request.document.id,incarnation:request.document.incarnation,revision:request.sourceRevision})||
        !['forward','backward'].includes(request.direction)||!catalog.affinities.includes(request.affinity)||!catalog.selectionModes.includes(request.selectionMode))return null;
    const {lo,hi}=occurrence.range,s={anchor:request.direction==='forward'?lo:hi,head:request.direction==='forward'?hi:lo,affinity:request.affinity};
    let selections=copy(v.selections),primary=v.primary;
    if(request.selectionMode==='replace-all'){selections=[s];primary=0;}
    else if(request.selectionMode==='replace-primary')selections[primary]=s;
    else{primary=selections.length;selections.push(s);}
    after=normalizeSelection(d.source,selections,primary,limits);
  }
  else if(request.operation==='logical-movement'){
    if(!catalog.units.includes(request.unit)||typeof request.extend!=='boolean')return null;
    const next=v.selections.map(s=>moved(d.source,s,request.unit,request.extend));if(next.some(s=>!s))return null;
    after=normalizeSelection(d.source,next,v.primary,limits);
  }else{
    if(!g.sourceWritable||!g.allTargetsWritable||!scalar(request.text)||!catalog.selectionAfter.includes(request.selectionAfter))return null;
    const ins=bytes(request.text);if(ins.length>limits.maxTextBytes)return null;
    edits=derivedEdits===null?v.selections.map(s=>({lo:Math.min(s.anchor,s.head),hi:Math.max(s.anchor,s.head),expected:d.source.slice(Math.min(s.anchor,s.head),Math.max(s.anchor,s.head)),ins:copy(ins)})):copy(derivedEdits);
    if(edits.some(e=>e.ins.length>limits.maxTextBytes))return null;
    if(edits.reduce((n,e)=>n+e.hi-e.lo+e.ins.length,0)>limits.maxEditBytes)return null;
    try{validateCanonicalBatch(d.source,edits);}catch{return null;}
    const candidate=emitByOrigin(d.source,edits);required=attached(state,request.document);
    if(required.length>limits.maxViews||new Set(required.map(viewKey)).size!==required.length)return null;
    for(const view of required){
      const normalizedBefore=normalizeSelection(d.source,view.selections,view.primary,limits);
      if(view.sourceRevision!==d.revision||view.profile!==catalog.profile||!normalizedBefore||!equal(normalizedBefore,{selections:view.selections,primary:view.primary}))return null;
      let selections=view.selections.map(s=>mapSelection(d.source,edits,s));
      if(equal(view.view,v.view)&&request.selectionAfter==='carets-after'){
        let delta=0;selections=edits.map((e,i)=>{const at=e.lo+delta+e.ins.length;delta+=e.ins.length-(e.hi-e.lo);return {anchor:at,head:at,affinity:v.selections[i].affinity};});
      }
      const normalized=normalizeSelection(candidate,selections,view.primary,limits);if(!normalized)return null;
      post.push({view:copy(view.view),...normalized});
    }
    after=post.find(p=>equal(p.view,v.view));
  }
  if(!after)return null;
  if(request.operation!=='ordinary-edit')post=[{view:copy(v.view),...after}];
  const plan=freeze({operation:request.operation});
  const ownedRequest=copy({...request,...(request.operation==='select-search-match'?{match:null}:{})});
  if(request.operation==='select-search-match')ownedRequest.match=request.match;
  plans.set(plan,{request:ownedRequest,limits:copy(limits),required,annotationSet:d.annotationSet,edits,post});return plan;
}

export function prepareSelectionEdit(state,request,g,limits){return prepareSelectionProduct(state,request,g,limits);}
// Trusted owner cleanup, not a new public editing verb.
export function discardSelectionEdit(plan,g){return ['ownerControlAllowed','retentionReleased','deliveryDrained'].every(k=>g?.[k]===true)&&plans.delete(plan);}
// Trusted bounded invocation composition, never a northbound inspection API.
export function selectionInvocationIntent(plan,maximum){
  const p=plans.get(plan);if(!p||!Number.isSafeInteger(maximum)||maximum<0)return null;
  const value={request:{...p.request,...(p.request.operation==='select-search-match'?{match:null}:{})},limits:p.limits,required:p.required,annotationSet:p.annotationSet,edits:p.edits,post:p.post,
    opaqueBindings:p.request.operation==='select-search-match'?['search-occurrence']:[],...(p.compositionRequest?{compositionRequest:p.compositionRequest}:{})};
  return Buffer.byteLength(JSON.stringify(value),'utf8')<=maximum?{invocation:p.request.invocation,value:freeze(copy(value))}:null;
}
// Trusted command-lowering composition, not an extra public batch verb or plugin ABI.
export function prepareDerivedSelectionEdit(state,request,edits,g,limits){
  if(request?.operation!=='ordinary-edit')return null;
  return prepareSelectionProduct(state,request,g,limits,edits);
}

const compositionForm=freeze({family:'F11',contract:'composition-commit/1',operation:'accepted-composition-commit',variant:'accepted-composition-commit',profileConstant:'primary-composition-replacement/1',fields:['contract','operation','invocation','document','view','sourceRevision','expectedSelection','profile','replacement','text']});
export function compositionInputForm(){return compositionForm;}
// An already accepted explicit replacement, not a native IME/preedit session.
export function prepareCompositionCommit(state,request,g,limits){
  if(!exact(request,compositionForm.fields)||request.contract!==compositionForm.contract||request.operation!==compositionForm.operation||request.profile!==compositionForm.profileConstant||!scalar(request.text)||!exact(request.replacement,['lo','hi','expected']))return null;
  const lowered={contract:catalog.contract,operation:'ordinary-edit',invocation:request.invocation,document:request.document,view:request.view,sourceRevision:request.sourceRevision,expectedSelection:request.expectedSelection,profile:catalog.profile,text:request.text,selectionAfter:'mapped'};
  const target=currentTarget(state,lowered,g,limits);if(!target)return null;
  const edit={...copy(request.replacement),ins:bytes(request.text)};
  if(!equal(edit.expected,target.d.source.slice(edit.lo,edit.hi)))return null;
  const plan=prepareSelectionProduct(state,lowered,g,limits,[edit]);if(!plan)return null;
  const p=plans.get(plan),origin=p.post.find(v=>equal(v.view,request.view));
  const caret=edit.lo+edit.ins.length,primary=target.v.primary;
  // Original primary index, before normalization can merge mapped ranges.
  const mapped=target.v.selections.map(s=>mapSelection(target.d.source,[edit],s));
  mapped[primary]={anchor:caret,head:caret,affinity:target.v.selections[primary].affinity};
  const normalized=normalizeSelection(emitByOrigin(target.d.source,[edit]),mapped,primary,limits);
  if(!normalized){plans.delete(plan);return null;}
  Object.assign(origin,normalized);p.compositionRequest=copy(request);return plan;
}

export function publishSelectionEdit(state,plan,g){
  const p=plans.get(plan),refuse=()=>({status:'not-published',state});if(!p)return refuse();
  const r=p.request,target=currentTarget(state,r,g,p.limits);if(!target)return refuse();
  if(r.operation==='select-search-match'&&!resolveSearchOccurrence(r.match,g.searchCurrent))return refuse();
  const ordinary=r.operation==='ordinary-edit';
  const current=ordinary?attached(state,r.document):[signature(target.v)];
  if(!equal(current,p.required)||ordinary&&(!g.sourceWritable||!g.allTargetsWritable||target.d.annotationSet!==p.annotationSet))return refuse();
  const viewChanged=p.post.some(after=>{const v=state.views.find(v=>equal(v.view,after.view));return !equal({selections:v.selections,primary:v.primary},{selections:after.selections,primary:after.primary});});
  const sourceChanged=ordinary&&!equal(emitByOrigin(target.d.source,p.edits),target.d.source);
  if((sourceChanged||viewChanged)&&!g.versionsAvailable||viewChanged&&!g.viewVersionsAvailable)return refuse();
  let out={status:'unchanged',state};
  if(ordinary)out=publishAnnotationPlan(state,{contract:'owned-annotation-plan/1',invocation:r.invocation,descriptor:r.invocation,
    sources:[{document:r.document,expectedSource:r.sourceRevision,expectedAnnotationSet:p.annotationSet,edits:copy(p.edits)}],annotations:[]},g);
  if(out.status==='not-published')return refuse();
  if(out.status==='unchanged'&&!sourceChanged&&!viewChanged)return {status:'unchanged',state};
  const next=copy(out.state);if(out.status==='unchanged')next.publication++;
  for(const after of p.post){
    const v=next.views.find(v=>equal(v.view,after.view));
    if(!equal({selections:v.selections,primary:v.primary},{selections:after.selections,primary:after.primary}))v.selectionGeneration++;
    v.selections=copy(after.selections);v.primary=after.primary;v.sourceRevision=next.documents.find(d=>equal(d.document,v.document)).revision;
  }
  if(!Number.isSafeInteger(next.publication)||next.publication<0||next.views.some(v=>!Number.isSafeInteger(v.selectionGeneration)||v.selectionGeneration<0))return refuse();
  carrySourceJournal(out.state,next);
  return {status:'applied',state:next,sourceChanged,viewsChanged:viewChanged};
}

export function checkSelectionProducts(){
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  for(const [id,variants] of Object.entries(catalog.families))assert(variants.every(v=>families.find(f=>f.id===id&&f.kind==='localCommand')?.variants.includes(v)));
  assert.deepEqual(Object.values(catalog.families).flat().sort(),Object.keys(catalog.variants).sort());
  const limits={maxRootBytes:128,maxInputViews:8,maxViews:4,maxInputSelections:8,maxSelections:8,maxSelectionBytes:2048,maxTextBytes:32,maxEditBytes:256};
  const document={authority:'a',id:'A',incarnation:1},other={...document,id:'B'},view=id=>({authority:'a',id,incarnation:1});
  const sel=(anchor,head=anchor,affinity='after')=>({anchor,head,affinity});
  const state={authority:'a',publication:0,documents:[{document,source:bytes('ab\r\né🙂'),revision:1,annotationSet:1,records:[]},
    {document:other,source:bytes('notes'),revision:1,annotationSet:1,records:[]}],groups:[],frontiers:{},views:[
      {view:view('origin'),document,sourceRevision:1,selectionGeneration:1,profile:catalog.profile,selections:[sel(0,2)],primary:0,viewport:0},
      {view:view('peer'),document,sourceRevision:1,selectionGeneration:1,profile:catalog.profile,selections:[sel(6)],primary:0,viewport:7},
      {view:view('other'),document:other,sourceRevision:1,selectionGeneration:1,profile:catalog.profile,selections:[sel(1)],primary:0,viewport:9}]};
  const g={...Object.fromEntries(gates.map(k=>[k,true])),sourceWritable:true,allTargetsWritable:true,versionsAvailable:true,viewVersionsAvailable:true,
    historyReady:true,completeIdentity:true,maxParticipants:4,targetRestorationValid:true};
  const common={contract:catalog.contract,invocation:'E',view:view('origin'),document,sourceRevision:1,expectedSelection:1,profile:catalog.profile};
  const set={...common,operation:'set-selection',selections:[sel(2,0)],primary:0};
  const edit={...common,operation:'ordinary-edit',text:'Z',selectionAfter:'carets-after'};
  const move={...common,operation:'logical-movement',unit:'document-end',extend:true};
  const prepare=(r=set,s=state,h=g,l=limits)=>prepareSelectionEdit(s,r,h,l);
  const run=(r=set,s=state,h=g)=>publishSelectionEdit(s,prepare(r,s,h),h);
  const controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  let normalizationCases=0,gateCases=0,movementCases=0;
  // Independent overlap-component oracle: graph connectivity, not the sorted sweep.
  const inputs=[sel(0),sel(0,0,'before'),sel(1),sel(0,1),sel(1,0),sel(1,2),sel(2,1),sel(0,2)];
  for(const a of inputs)for(const b of inputs)for(const c of inputs)for(let primary=0;primary<3;primary++){
    const rows=[a,b,c],groups=[];
    for(let i=0;i<rows.length;i++){
      const r=rows[i],lo=Math.min(r.anchor,r.head),hi=Math.max(r.anchor,r.head);
      if(lo===hi){const same=groups.find(c=>c.point&&c.lo===lo&&c.affinity===r.affinity);if(same)same.members.push(i);else groups.push({point:true,lo,hi,affinity:r.affinity,members:[i]});}
      else groups.push({point:false,lo,hi,members:[i]});
    }
    let changed=true;while(changed){changed=false;outer:for(let i=0;i<groups.length;i++)for(let j=i+1;j<groups.length;j++){
      const x=groups[i],y=groups[j];if(!x.point&&!y.point&&x.lo<y.hi&&y.lo<x.hi){x.lo=Math.min(x.lo,y.lo);x.hi=Math.max(x.hi,y.hi);x.members.push(...y.members);groups.splice(j,1);changed=true;break outer;}
    }}
    const expected=groups.map(c=>{const s=rows[c.members.includes(primary)?primary:Math.min(...c.members)];return {...c,selection:sel(s.anchor>s.head?c.hi:c.lo,s.anchor>s.head?c.lo:c.hi,s.affinity)};})
      .sort((a,b)=>a.lo-b.lo||a.hi-b.hi||catalog.affinities.indexOf(a.selection.affinity)-catalog.affinities.indexOf(b.selection.affinity));
    const result=normalizeSelection(bytes('ab'),rows,primary,limits);
    assert.deepEqual(result,{selections:expected.map(c=>c.selection),primary:expected.findIndex(c=>c.members.includes(primary))});
    assert.deepEqual(normalizeSelection(bytes('ab'),result.selections,result.primary,limits),result);normalizationCases++;
  }
  for(const request of [set,edit])for(let mask=0;mask<512;mask++){
    const h={...g,...Object.fromEntries(gates.map((k,i)=>[k,!!(mask&(1<<i))]))};
    assert.equal(run(request,state,h).status,mask===511?'applied':'not-published');gateCases++;
  }
  for(const head of [0,1,2,4,6,10])for(const unit of catalog.units)for(const extend of [false,true]){
    const s=copy(state);s.views[0].selections=[sel(0,head)];
    const out=run({...move,unit,extend},s);assert(['applied','unchanged'].includes(out.status));
    const expected=unit==='document-start'?0:unit==='document-end'?10:unit==='line-start'?(head<=2?0:4):(head<=2?2:10);
    assert.equal(out.state.views[0].selections[0].head,expected);movementCases++;
  }
  control('direction-and-primary-merge',equal(normalizeSelection(bytes('abcd'),[sel(0,2),sel(3,1,'before')],1,limits),{selections:[sel(3,0,'before')],primary:0}));
  control('adjacent-nonempty-stays-separate',normalizeSelection(bytes('ab'),[sel(0,1),sel(1,2)],0,limits).selections.length===2);
  control('opposing-affinities-not-erased',normalizeSelection(bytes('ab'),[sel(1),sel(1,1,'before')],0,limits).selections.length===2);
  control('caret-inside-range-not-erased',normalizeSelection(bytes('ab'),[sel(0,2),sel(1)],0,limits).selections.length===2);
  control('empty-selection-set-refused',prepare({...set,selections:[]})===null);
  control('scalar-split-refused',prepare({...set,selections:[sel(5)]})===null);
  control('unsupported-profile-refused',prepare({...set,profile:'implicit-ui-default'})===null);
  control('unknown-command-refused',prepare({...set,operation:'eval'})===null);
  control('unknown-field-refused',prepare({...set,extra:true})===null);
  control('missing-affinity-refused',prepare({...set,selections:[{anchor:0,head:0}]})===null);
  control('unsupported-grapheme-not-scalar-fallback',prepare({...move,unit:'next-grapheme'})===null);
  const cr=copy(state);cr.views[0].selections=[sel(3)];
  control('crlf-interior-not-clamped',prepare({...move,unit:'line-end'},cr)===null);
  control('read-only-source-still-allows-view-selection',run(set,state,{...g,sourceWritable:false}).status==='applied');
  control('read-only-edit-refused',prepare(edit,state,{...g,allTargetsWritable:false})===null);
  control('invalid-literal-refused',prepare({...edit,text:'\ud800'})===null);
  const conflict=copy(state);conflict.views[0].selections=normalizeSelection(conflict.documents[0].source,[sel(1),sel(1,1,'before')],0,limits).selections;
  control('coincident-insertion-needs-resolved-plan',prepare(edit,conflict)===null);
  const inside=copy(state);inside.views[0].selections=[sel(0,2),sel(1)];
  control('caret-inside-replacement-needs-resolved-plan',prepare(edit,inside)===null);
  const adjacent=copy(state);adjacent.views[0].selections=[sel(0,1),sel(1,2)];adjacent.views[0].primary=1;
  const multi=run(edit,adjacent);
  control('adjacent-multiedits-one-group',multi.status==='applied'&&equal(multi.state.documents[0].source,bytes('ZZ\r\né🙂'))&&multi.state.groups.length===1&&multi.state.views[0].primary===1);
  const plan=prepare(edit),before=copy(state),applied=publishSelectionEdit(state,plan,g);
  control('source-and-all-view-companions-once',applied.status==='applied'&&applied.state.publication===1&&applied.state.groups.length===1&&
    equal(applied.state.documents[0].source,bytes('Z\r\né🙂'))&&applied.state.views[0].selections[0].head===1&&applied.state.views[1].selections[0].head===5&&applied.state.views[1].sourceRevision===2);
  control('source-input-unchanged',equal(state,before));
  control('pure-selection-no-source-history',run().state.documents[0].revision===1&&run().state.groups.length===0&&run().state.publication===1);
  const noop={...set,selections:copy(state.views[0].selections)};
  control('validated-noop-no-generations',run(noop,state,{...g,versionsAvailable:false,viewVersionsAvailable:false}).status==='unchanged'&&run(noop).state===state);
  control('affinity-is-semantic',run({...set,selections:[sel(0,2,'before')]}).status==='applied');
  const equalText=run({...edit,text:'ab'});
  control('equal-text-view-change-is-applied',equalText.status==='applied'&&!equalText.sourceChanged&&equalText.state.documents[0].revision===1&&equalText.state.publication===1&&equalText.state.groups.length===0);
  control('equal-text-all-identity-is-unchanged',run({...edit,text:'ab',selectionAfter:'mapped'}).status==='unchanged');
  control('view-version-exhaustion-refuses-all',publishSelectionEdit(state,plan,{...g,viewVersionsAvailable:false}).status==='not-published');
  control('publication-rechecks-readonly',publishSelectionEdit(state,plan,{...g,allTargetsWritable:false}).status==='not-published');
  control('publication-rechecks-stop',publishSelectionEdit(state,plan,{...g,notStopped:false}).status==='not-published');
  control('copied-plan-not-owned',publishSelectionEdit(state,copy(plan),g).status==='not-published');
  const movedPeer=copy(state);movedPeer.views[1].selectionGeneration++;
  control('changed-peer-selection-stales-plan',publishSelectionEdit(movedPeer,plan,g).status==='not-published');
  const added=copy(state);added.views.push({...copy(state.views[1]),view:view('new')});
  control('new-view-not-skipped',publishSelectionEdit(added,plan,g).status==='not-published');
  const duplicate=copy(state);duplicate.views.push(copy(state.views[1]));
  control('duplicate-view-owner-refused',prepare(edit,duplicate)===null);
  const otherRequest={...edit,invocation:'Other',document:other,view:view('other')};
  const unrelated=publishSelectionEdit(state,prepare(otherRequest),g).state;unrelated.views[2].viewport=99;
  const preserved=publishSelectionEdit(unrelated,plan,g);
  control('unrelated-current-state-not-restored',preserved.status==='applied'&&equal(preserved.state.documents[1],unrelated.documents[1])&&preserved.state.views[2].viewport===99&&preserved.state.publication===2);
  const cosmetic=copy(state);cosmetic.views[1].viewport=42;
  control('current-viewport-preserved',publishSelectionEdit(cosmetic,plan,g).state.views[1].viewport===42);
  const ann=copy(state);ann.documents[0].annotationSet++;
  control('annotation-dependency-refuses-stale-edit',publishSelectionEdit(ann,plan,g).status==='not-published');
  control('pure-selection-does-not-capture-annotation-set',publishSelectionEdit(ann,prepare(),g).status==='applied');
  const withAnnotation=copy(state),target={kind:'range',start:6,end:10};
  withAnnotation.documents[0].records=[{id:'note',incarnation:1,body:'authored',bodyVersion:1,target:{sourceRevision:1,target:copy(target)},targetGeneration:1,
    placement:{state:'attached',sourceRevision:1,target:copy(target)}}];
  const mapped=run(edit,withAnnotation);
  control('annotations-and-views-compose',mapped.status==='applied'&&mapped.state.documents[0].records[0].body==='authored'&&
    equal(mapped.state.documents[0].records[0].placement,{state:'attached',sourceRevision:2,target:{kind:'range',start:5,end:9}})&&mapped.state.views[1].sourceRevision===2);
  const affected=copy(withAnnotation);affected.documents[0].records[0].target.target={kind:'range',start:0,end:2};affected.documents[0].records[0].placement.target={kind:'range',start:0,end:2};
  control('equal-byte-nonidentity-annotation-not-noop',run({...edit,text:'ab'},affected).status==='applied');
  control('stale-selection-generation-refused',prepare({...set,expectedSelection:2})===null);
  control('stale-source-revision-refused',prepare({...set,sourceRevision:2})===null);
  for(const key of ['maxRootBytes','maxInputViews','maxViews','maxInputSelections','maxSelections','maxSelectionBytes','maxTextBytes','maxEditBytes'])
    control(`limit-${key}`,prepare(edit,state,g,{...limits,[key]:0})===null);
  const undone=reverseAnnotationGroup(applied.state,'E','undo',g);
  control('raw-reversal-is-not-view-aware',undone.status==='applied'&&equal(undone.state.documents[0].source,state.documents[0].source)&&
    undone.state.views[0].sourceRevision!==undone.state.documents[0].revision);
  return {status:'selection-edit-products-only',normalizationCases,gateCases,movementCases,controls,...catalog.scope};
}

export async function checkSelectionMutations(){
  const source=fs.readFileSync(new URL('./selection-products.mjs',import.meta.url),'utf8');
  const mutations=[
    ['merge-adjacent','row.lo<last.hi','row.lo<=last.hi'],
    ['erase-opposing-affinity','JSON.stringify([row.lo,row.affinity])','JSON.stringify([row.lo])'],
    ['ignore-primary-owner','r.members.includes(primary)?primary:Math.min(...r.members)','Math.min(...r.members)'],
    ['ignore-view-dependencies','!equal(current,p.required)','false'],
    ['skip-peer-companions','for(const view of required){','for(const view of required.slice(0,1)){'],
    ['source-equality-is-full-identity',"if(out.status==='unchanged'&&!sourceChanged&&!viewChanged)return",'if(!sourceChanged)return'],
    ['ignore-readonly','!g.sourceWritable||!g.allTargetsWritable','false'],
    ['restore-captured-viewport','v.selections=copy(after.selections);','v.viewport=0;v.selections=copy(after.selections);']
  ];
  const killed=[];
  for(const [name,from,to] of mutations){assert(source.includes(from),name);let code=source.replace(from,to);
    for(const file of ['text-oracles.mjs','mapping-profiles.mjs','reference-profile.mjs','annotation-compounds.mjs','search-occurrences.mjs','parser-source.mjs'])code=code.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    for(const file of ['selection-products.json','operation-families.json'])code=code.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
    const module=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    let failed=false;try{module.checkSelectionProducts();}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkSelectionProducts(),mutations:await checkSelectionMutations()},null,2));
