// View-owner intent publication only; not native scrolling, shaping or presentation.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {geometryTargetProjection} from './geometry-products.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./viewport-products.json',import.meta.url),'utf8'));
const copy=structuredClone,plans=new WeakMap();
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const size=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const validLimits=l=>l&&catalog.limits.every(k=>Number.isSafeInteger(l[k])&&l[k]>=0);
const coord=(n,l)=>Number.isSafeInteger(n)&&Math.abs(n)<=l.maxCoordinate;
const viewport=(v,l)=>exact(v,['x','y','width','height'])&&Object.values(v).every(n=>coord(n,l))&&v.width>0&&v.height>0;
const identity=r=>exact(r,['authority','id','incarnation'])&&typeof r.authority==='string'&&r.authority.length>0&&typeof r.id==='string'&&r.id.length>0&&Number.isSafeInteger(r.incarnation)&&r.incarnation>=0;
const context=c=>exact(c,catalog.contextFields)&&typeof c.unit==='string'&&c.unit.length>0&&catalog.contextFields.slice(1).every(k=>Number.isSafeInteger(c[k])&&c[k]>=0);
const gates=['authorized','sourceReadable','viewWritable','continuous','live','bounded','notStopped','interpretationCurrent','companionsReady'];
function current(state,r,g,l){
  if(!gates.every(k=>g?.[k])||!validLimits(l)||!Array.isArray(state?.views)||!Array.isArray(state?.documents)||!context(r.context)||
      !equal(r.context,g.viewportContext)||!identity(r.document)||!identity(r.view)||r.document.authority!==state.authority||r.view.authority!==state.authority)return null;
  const docs=state.documents.filter(d=>equal(d.document,r.document)),views=state.views.filter(v=>equal(v.view,r.view));
  if(docs.length!==1||views.length!==1)return null;const d=docs[0],v=views[0];
  if(d.live===false||!equal(v.document,r.document)||d.revision!==r.sourceRevision||v.sourceRevision!==r.sourceRevision||
      !Number.isSafeInteger(v.viewportGeneration)||v.viewportGeneration<0||v.viewportGeneration!==r.expectedViewport||
      !equal(v.viewportContext,r.context)||!viewport(v.viewport,l))return null;
  return {d,v};
}
const signature=v=>({view:copy(v.view),document:copy(v.document),sourceRevision:v.sourceRevision,viewport:copy(v.viewport),viewportGeneration:v.viewportGeneration,viewportContext:copy(v.viewportContext)});
const floorHalf=n=>n>=0n?n/2n:-((-n+1n)/2n);
function alignAxis(position,extent,start,end,margin,alignment){
  const p=BigInt(position),e=BigInt(extent),a=BigInt(start),b=BigInt(end),m=BigInt(margin),first=a-m,last=b-e+m;
  if(alignment==='start')return first;if(alignment==='end')return last;if(alignment==='center')return floorHalf(a+b-e);
  // Nearest keeps the current viewport if either target or viewport is contained.
  const lo=first<last?first:last,hi=first>last?first:last;return p<lo?lo:p>hi?hi:p;
}
function finiteNumber(n,l){return n < -BigInt(l.maxCoordinate)||n>BigInt(l.maxCoordinate)?null:Number(n);}
function resolveReveal(state,r,g,geometryLimits){
  const i=r.intent,projection=geometryTargetProjection(state,i.layout,i.target,g.geometry,geometryLimits);if(!projection)return null;
  if(!equal(projection.context.view,r.view)||!equal(projection.context.document,r.document)||projection.context.sourceRevision!==r.sourceRevision||
      projection.context.viewportGeneration!==r.expectedViewport||!catalog.contextFields.every(k=>projection.context[k]===r.context[k]))return null;
  return projection;
}
export function prepareViewportEdit(state,request,g,limits,geometryLimits){
  if(!request||!Object.hasOwn(catalog.variants,request.operation)||!exact(request,[...catalog.common,...catalog.variants[request.operation]])||
      request.contract!==catalog.contract||request.profile!==catalog.profile||typeof request.invocation!=='string'||!request.invocation)return null;
  const target=current(state,request,g,limits);if(!target||size(request)>limits.maxRequestBytes)return null;
  const before=signature(target.v);let after=copy(before.viewport),projection=null;
  if(request.operation==='scroll'){
    if(!exact(request.delta,['x','y'])||!Object.values(request.delta).every(n=>coord(n,limits)))return null;
    after.x=finiteNumber(BigInt(after.x)+BigInt(request.delta.x),limits);after.y=finiteNumber(BigInt(after.y)+BigInt(request.delta.y),limits);
  }else{
    const i=request.intent;
    if(i?.kind==='set'){
      if(!exact(i,['kind','viewport'])||!viewport(i.viewport,limits))return null;after=copy(i.viewport);
    }else if(i?.kind==='reveal'){
      if(!exact(i,['kind','layout','target','xAlign','yAlign','margin'])||!catalog.alignments.includes(i.xAlign)||!catalog.alignments.includes(i.yAlign)||
          !exact(i.margin,['x','y'])||!Object.values(i.margin).every(n=>Number.isSafeInteger(n)&&n>=0)||
          2n*BigInt(i.margin.x)>=BigInt(after.width)||2n*BigInt(i.margin.y)>=BigInt(after.height))return null;
      projection=resolveReveal(state,request,g,geometryLimits);if(!projection)return null;
      after.x=finiteNumber(alignAxis(after.x,after.width,projection.rect.left,projection.rect.right,i.margin.x,i.xAlign),limits);
      after.y=finiteNumber(alignAxis(after.y,after.height,projection.rect.top,projection.rect.bottom,i.margin.y,i.yAlign),limits);
    }else return null;
  }
  if(!viewport(after,limits))return null;
  const captured=copy({...request,...(projection?{intent:{...request.intent,layout:null,target:null}}:{})});
  if(projection){captured.intent.layout=request.intent.layout;captured.intent.target=request.intent.target;}
  const payload={request:captured,before,after,projection},planBytes=size(payload);if(planBytes>limits.maxPlanBytes)return null;
  const plan=Object.freeze({contract:catalog.contract});plans.set(plan,{...payload,planBytes,requestBytes:size(request)});return plan;
}
export function publishViewportEdit(state,plan,g,limits,geometryLimits){
  const p=plans.get(plan),refuse=()=>({status:'not-published',state});if(!p)return refuse();
  const target=current(state,p.request,g,limits);if(!target||!equal(signature(target.v),p.before)||p.requestBytes>limits.maxRequestBytes||p.planBytes>limits.maxPlanBytes||!viewport(p.after,limits))return refuse();
  if(p.projection&&!equal(resolveReveal(state,p.request,g,geometryLimits),p.projection))return refuse();
  if(equal(p.before.viewport,p.after))return {status:'unchanged',state};
  if(!g.versionsAvailable||!Number.isSafeInteger(target.v.viewportGeneration+1)||!Number.isSafeInteger(state.publication+1))return refuse();
  const next=copy(state),v=next.views.find(v=>equal(v.view,p.request.view));v.viewport=copy(p.after);v.viewportGeneration++;next.publication++;
  return {status:'applied',state:next};
}
