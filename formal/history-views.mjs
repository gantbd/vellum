// Private bounded view part composed by the actual F14 owned admission path.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {normalizeSelection,mapSelection,selectionViewSignature,selectionViewKey} from './selection-products.mjs';
import {validateCanonicalBatch,emitByOrigin} from './text-oracles.mjs';
import {carrySourceJournal} from './parser-source.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./history-views.json',import.meta.url),'utf8'));
const selection=JSON.parse(fs.readFileSync(new URL('./selection-products.json',import.meta.url),'utf8'));
const copy=structuredClone,owned=new WeakSet();
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const validViewLimits=l=>l&&selection.limits.every(k=>Number.isSafeInteger(l[k])&&l[k]>=0);
const validWorkLimits=l=>l&&catalog.workLimits.every(k=>Number.isSafeInteger(l[k])&&l[k]>=0);
const within=(charges,l)=>validWorkLimits(l)&&charges.batches<=l.maxViewBatches&&charges.edits<=l.maxViewEdits&&charges.bytes<=l.maxViewBytes&&charges.planBytes<=l.maxViewPlanBytes;
const order=(a,b)=>selectionViewKey(a)<selectionViewKey(b)?-1:selectionViewKey(a)>selectionViewKey(b)?1:0;
function captureViews(state,sources,limits){
  if(!Array.isArray(state.views)||!validViewLimits(limits)||state.views.length>limits.maxInputViews||
      new Set(state.views.map(selectionViewKey)).size!==state.views.length)return null;
  const views=state.views.filter(v=>sources.some(s=>equal(s.document,v.document))).map(selectionViewSignature).sort(order);
  if(views.length>limits.maxViews)return null;
  for(const v of views){
    const s=sources.find(s=>equal(s.document,v.document));
    const normalized=normalizeSelection(s.before,v.selections,v.primary,limits);
    if(v.profile!==catalog.selectionProfile||v.sourceRevision!==s.beforeRevision||!normalized||!equal(normalized,{selections:v.selections,primary:v.primary}))return null;
  }
  return views;
}
function prepareViewTransforms(state,group,direction,viewLimits,workLimits,forwardPublication=false){
  if(forwardPublication?direction!=='forward':!['undo','redo'].includes(direction))return null;
  const changes=(direction==='undo'?[...group.changes].reverse():group.changes).filter(c=>c.kind==='source');
  // A body/target-only group declares no source transform; no source-view profile is needed.
  if(!changes.length){const p=freeze({contract:catalog.contract,direction,sources:[],beforeViews:[],afterViews:[],viewLimits:null,charges:{batches:0,edits:0,bytes:0,planBytes:0},viewChanged:false});owned.add(p);return p;}
  if(!validWorkLimits(workLimits)||!validViewLimits(viewLimits))return null;
  const sources=[],seen=new Set(),charges={batches:0,edits:0,bytes:0,planBytes:0};
  for(const change of changes){
    const key=JSON.stringify([change.document.authority,change.document.id,change.document.incarnation]);if(seen.has(key))return null;seen.add(key);
    const docs=state.documents.filter(d=>equal(d.document,change.document));if(docs.length!==1)return null;const d=docs[0];
    const expected=direction==='undo'?change.after:change.before,desired=direction==='undo'?change.before:change.after;
    if(!equal(d.source,expected)||d.source.length>viewLimits.maxRootBytes||!Array.isArray(change.forward)||!Array.isArray(change.inverseSequence))return null;
    const batches=direction==='undo'?change.inverseSequence.map(e=>[e]):[change.forward];let source=copy(d.source);
    for(const edits of batches){
      if(!Array.isArray(edits)||edits.some(e=>!e||!Array.isArray(e.ins)))return null;
      charges.batches++;charges.edits+=edits.length;charges.bytes+=source.length+edits.reduce((n,e)=>n+e.ins.length+Math.max(0,e.hi-e.lo),0);
      if(!Number.isSafeInteger(charges.bytes)||!within(charges,workLimits))return null;
      try{validateCanonicalBatch(source,edits);}catch{return null;}
      source=emitByOrigin(source,edits);if(source.length>viewLimits.maxRootBytes)return null;
    }
    if(!equal(source,desired))return null;
    const advance=forwardPublication?!equal(source,d.source):true;
    sources.push({document:copy(d.document),before:copy(d.source),after:source,beforeRevision:d.revision,afterRevision:d.revision+(advance?1:0),batches:copy(batches)});
  }
  const beforeViews=captureViews(state,sources,viewLimits);if(!beforeViews)return null;
  const afterViews=[];
  for(const view of beforeViews){
    const s=sources.find(s=>equal(s.document,view.document));let source=copy(s.before),selections=copy(view.selections);
    for(const edits of s.batches){selections=selections.map(selection=>mapSelection(source,edits,selection));source=emitByOrigin(source,edits);}
    // Normalize once, preserving the original primary association through all inverse steps.
    const normalized=normalizeSelection(source,selections,view.primary,viewLimits);if(!normalized)return null;
    afterViews.push({view:copy(view.view),document:copy(view.document),sourceRevision:s.afterRevision,...normalized});
  }
  const viewChanged=afterViews.some(v=>{const old=beforeViews.find(p=>equal(p.view,v.view));return !equal({selections:v.selections,primary:v.primary},{selections:old.selections,primary:old.primary});});
  const p={contract:catalog.contract,direction,sources,beforeViews,afterViews,viewLimits:copy(viewLimits),charges,viewChanged};
  // Close the charge over its own decimal representation with a finite digit bound.
  let settled=false;
  for(let step=0;step<17;step++){const size=Buffer.byteLength(JSON.stringify(p),'utf8');if(size===charges.planBytes){settled=true;break;}charges.planBytes=size;}
  if(!settled||!within(charges,workLimits))return null;
  freeze(p);owned.add(p);return p;
}
export function prepareReversalViews(state,group,direction,viewLimits,workLimits){return prepareViewTransforms(state,group,direction,viewLimits,workLimits);}
export function prepareForwardViews(state,document,sourceRevision,edits,viewLimits,workLimits){
  const docs=state.documents.filter(d=>equal(d.document,document));if(docs.length!==1||docs[0].revision!==sourceRevision)return null;
  const source=docs[0].source;
  try{validateCanonicalBatch(source,edits);}catch{return null;}
  // Empty edits declare no transform, not an unknown view inventory interpreted as empty.
  const changes=edits.length?[{kind:'source',document,before:source,after:emitByOrigin(source,edits),forward:edits,inverseSequence:[]}]:[];
  return prepareViewTransforms(state,{changes},'forward',viewLimits,workLimits,true);
}
export function validateReversalViews(state,part,viewLimits,workLimits,viewVersionsAvailable){
  if(!part||!owned.has(part))return false;
  if(!part.sources.length)return true;
  if(!equal(part.viewLimits,viewLimits)||!within(part.charges,workLimits)||part.viewChanged&&!viewVersionsAvailable)return false;
  if(part.sources.some(s=>!state.documents.some(d=>equal(d.document,s.document)&&d.revision===s.beforeRevision&&equal(d.source,s.before))))return false;
  return equal(captureViews(state,part.sources,part.viewLimits),part.beforeViews);
}
function decideViews(state,part,reversed,viewLimits,workLimits,viewVersionsAvailable,allowUnchanged=false){
  const refuse=()=>({status:'not-published',state});
  if(!validateReversalViews(state,part,viewLimits,workLimits,viewVersionsAvailable)||
      !(reversed.status==='applied'||allowUnchanged&&reversed.status==='unchanged'))return refuse();
  if(part.sources.some(s=>!reversed.state.documents.some(d=>equal(d.document,s.document)&&d.revision===s.afterRevision&&equal(d.source,s.after))))return refuse();
  if(reversed.status==='unchanged'&&!part.viewChanged)return {status:'unchanged',state};
  const next=copy(reversed.state);
  if(reversed.status==='unchanged')next.publication++;
  for(const after of part.afterViews){
    const v=next.views.find(v=>equal(v.view,after.view));
    if(!equal({selections:v.selections,primary:v.primary},{selections:after.selections,primary:after.primary}))v.selectionGeneration++;
    v.selections=copy(after.selections);v.primary=after.primary;v.sourceRevision=after.sourceRevision;
  }
  if(!Number.isSafeInteger(next.publication)||next.publication<0||(next.views??[]).some(v=>!Number.isSafeInteger(v.selectionGeneration)||v.selectionGeneration<0))return refuse();
  return {status:'applied',state:next};
}
function completeViews(state,part,reversed,viewLimits,workLimits,viewVersionsAvailable,allowUnchanged=false){
  const result=decideViews(state,part,reversed,viewLimits,workLimits,viewVersionsAvailable,allowUnchanged);
  if(result.status==='applied'&&result.state!==reversed.state){const next=result.state;
    carrySourceJournal(reversed.state,next);
  }
  return result;
}
export function decideForwardViews(state,part,published,viewLimits,workLimits,viewVersionsAvailable){return decideViews(state,part,published,viewLimits,workLimits,viewVersionsAvailable,true);}
export function completeReversalViews(state,part,reversed,viewLimits,workLimits,viewVersionsAvailable){return completeViews(state,part,reversed,viewLimits,workLimits,viewVersionsAvailable);}
export function completeForwardViews(state,part,published,viewLimits,workLimits,viewVersionsAvailable){return completeViews(state,part,published,viewLimits,workLimits,viewVersionsAvailable,true);}
