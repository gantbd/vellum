// Closed logical movement/deletion over owned exact-source segmentation evidence.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {resolveBoundaries} from './unicode-boundaries.mjs';
import {prepareSelectionEdit,prepareDerivedSelectionEdit,publishSelectionEdit,selectionInvocationIntent} from './selection-products.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./boundary-commands.json',import.meta.url),'utf8'));
const plans=new WeakMap(),copy=structuredClone;
export function boundaryInputForms(){return Object.freeze(Object.entries(catalog.families).flatMap(([family,operations])=>operations.map(operation=>Object.freeze({family,contract:catalog.contract,operation,variant:operation,profileConstant:catalog.profile,fields:Object.freeze([...catalog.common])}))));}
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const limitsValid=l=>l&&['maxRootBytes','maxBoundaries','maxWords','maxBoundaryBytes','maxSteps','maxDerivedBytes'].every(k=>Number.isSafeInteger(l[k])&&l[k]>=0);
function capture(state,r,g,limits){
  if(!Array.isArray(state?.documents)||!Array.isArray(state?.views)||!g.boundaryLive||!g.boundaryRetained||g.boundaryProfile!=='unicode18-uax29/1')return null;
  const d=state.documents.find(d=>equal(d.document,r.document));if(!d)return null;
  return resolveBoundaries(r.boundary,{...r.document,revision:r.sourceRevision},d.source,limits);
}
function advance(vector,at,direction){
  return direction==='forward'?(vector.find(p=>p>at)??vector.at(-1)):([...vector].reverse().find(p=>p<at)??vector[0]);
}
export function prepareBoundaryCommand(state,request,g,selectionLimits,limits){
  if(!limitsValid(limits)||!exact(request,catalog.common)||request.contract!==catalog.contract||request.profile!==catalog.profile||
      !['logical-movement','ordinary-edit'].includes(request.operation)||!catalog.units.includes(request.unit)||!catalog.directions.includes(request.direction)||
      !Number.isSafeInteger(request.count)||request.count<1||request.count>limits.maxSteps||typeof request.extend!=='boolean'||request.operation==='ordinary-edit'&&request.extend)return null;
  const table=capture(state,request,g,limits);if(!table)return null;
  const common={contract:'selection-edit/1',operation:'logical-movement',invocation:request.invocation,view:request.view,document:request.document,
    sourceRevision:request.sourceRevision,expectedSelection:request.expectedSelection,profile:'explicit-positional-selection/1',unit:'document-start',extend:false};
  if(!prepareSelectionEdit(state,common,g,selectionLimits))return null;
  const d=state.documents.find(d=>equal(d.document,request.document)),v=state.views.find(v=>equal(v.view,request.view)),[graphemes,,words]=table;
  const vector=request.unit==='grapheme'?graphemes:[...new Set([0,...words.map(w=>w[request.unit==='word-start'?0:1]),d.source.length])].sort((a,b)=>a-b);
  if(request.unit!=='grapheme'&&vector.some(p=>!graphemes.includes(p)))return null;
  if(request.count*v.selections.length>limits.maxSteps)return null;
  const after=[],edits=[];
  for(const s of v.selections){
    const lo=Math.min(s.anchor,s.head),hi=Math.max(s.anchor,s.head);let target=s.head,steps=request.count;
    if(request.operation==='ordinary-edit'&&lo===hi&&!graphemes.includes(s.head))return null;
    if(lo!==hi&&!request.extend){target=request.direction==='forward'?hi:lo;steps--;}
    if(request.operation==='ordinary-edit'&&lo!==hi)target=request.direction==='forward'?hi:lo;
    else for(let i=0;i<steps;i++)target=advance(vector,target,request.direction);
    if(request.operation==='logical-movement')after.push({anchor:request.extend?s.anchor:target,head:target,affinity:s.affinity});
    else{
      const from=lo!==hi?lo:Math.min(s.head,target),to=lo!==hi?hi:Math.max(s.head,target);
      if(lo===hi&&!graphemes.includes(target))return null;
      edits.push({lo:from,hi:to,expected:d.source.slice(from,to),ins:[]});
    }
  }
  const derivedBytes=Buffer.byteLength(JSON.stringify({after,edits}),'utf8');if(derivedBytes>limits.maxDerivedBytes)return null;
  let core;
  if(request.operation==='logical-movement'){
    const setter={...common,operation:'set-selection',selections:after,primary:v.primary};delete setter.unit;delete setter.extend;
    core=prepareSelectionEdit(state,setter,g,selectionLimits);
  }else{
    const edit={...common,operation:'ordinary-edit',text:'',selectionAfter:'carets-after'};delete edit.unit;delete edit.extend;
    core=prepareDerivedSelectionEdit(state,edit,edits,g,selectionLimits);
  }
  if(!core)return null;
  const plan=Object.freeze({contract:catalog.contract,operation:request.operation});
  plans.set(plan,{request:{...copy({...request,boundary:null}),boundary:request.boundary},core,selectionLimits:copy(selectionLimits),steps:request.count*v.selections.length,derivedBytes});return plan;
}
export function boundaryInvocationIntent(plan,maximum){
  const p=plans.get(plan);if(!p||!Number.isSafeInteger(maximum)||maximum<0)return null;
  const core=selectionInvocationIntent(p.core,maximum);if(!core)return null;
  const value={request:{...copy({...p.request,boundary:null})},selectionLimits:copy(p.selectionLimits),steps:p.steps,derivedBytes:p.derivedBytes,core:core.value,opaqueBindings:['unicode-boundary']};
  return Buffer.byteLength(JSON.stringify(value),'utf8')<=maximum?{invocation:p.request.invocation,value}:null;
}
export function discardBoundaryCommand(plan,g){return ['ownerControlAllowed','retentionReleased','deliveryDrained'].every(k=>g?.[k]===true)&&plans.delete(plan);}
export function publishBoundaryCommand(state,plan,g,selectionLimits,limits){
  const p=plans.get(plan),refuse=()=>({status:'not-published',state});
  if(!p||!limitsValid(limits)||!equal(p.selectionLimits,selectionLimits)||p.steps>limits.maxSteps||p.derivedBytes>limits.maxDerivedBytes||!capture(state,p.request,g,limits))return refuse();
  return publishSelectionEdit(state,p.core,g);
}
