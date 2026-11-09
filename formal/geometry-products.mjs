// Normalized accepted geometry and current-view consumer; no native measurement or I/O.
// Presentation-owned reference behavior, not a symbolic engine facade dependency.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {validateCanonicalBatch} from './text-oracles.mjs';
import {prepareSelectionEdit,publishSelectionEdit} from './selection-products.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./geometry-products.json',import.meta.url),'utf8'));
const layouts=new WeakMap(),targets=new WeakMap(),plans=new WeakMap(),copy=structuredClone;
const gates=['authorized','scope','continuous','capacity','live','retained','notStopped'];
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const size=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const validLimits=l=>l&&catalog.limits.every(k=>Number.isSafeInteger(l[k])&&l[k]>=0);
const coordinate=(v,l)=>Number.isSafeInteger(v)&&Math.abs(v)<=l.maxCoordinate;
const point=(v,source)=>exact(v,catalog.pointFields)&&Number.isSafeInteger(v.position)&&v.position>=0&&v.position<=source.length&&
  (v.position===source.length||(source[v.position]&0xc0)!==0x80)&&['before','after'].includes(v.affinity);
const stopPoint=s=>({position:s.position,affinity:s.affinity});
const rect=(r,l)=>exact(r,['left','right','top','bottom'])&&Object.values(r).every(v=>coordinate(v,l))&&r.left<r.right&&r.top<r.bottom;
const inside=(r,x,y)=>r.left<=x&&x<r.right&&r.top<=y&&y<r.bottom;
const unavailable=()=>({status:'unavailable'});
function currentContext(state,context,current){
  if(!gates.every(k=>current?.[k])||!equal(context,current.context)||!Array.isArray(state?.views)||!Array.isArray(state?.documents))return false;
  const docs=state.documents.filter(d=>equal(d.document,context.document)),views=state.views.filter(v=>equal(v.view,context.view));
  return context.document.authority===state.authority&&context.view.authority===state.authority&&docs.length===1&&docs[0].live!==false&&views.length===1&&equal(views[0].document,context.document)&&docs[0].revision===context.sourceRevision&&
    docs[0].annotationSet===context.annotationSet&&views[0].sourceRevision===context.sourceRevision&&equal(views[0].viewport,context.viewport)&&
    (!Object.hasOwn(views[0],'viewportGeneration')||views[0].viewportGeneration===context.viewportGeneration);
}
function bounded(packet,l){
  if(!validLimits(l)||!exact(packet,['context','source','rows','hits','atStart','atEnd'])||!exact(packet.context,catalog.contextFields)||
      !Array.isArray(packet.source)||packet.source.length>l.maxRootBytes||typeof packet.atStart!=='boolean'||typeof packet.atEnd!=='boolean'||
      !Array.isArray(packet.rows)||!packet.rows.length||packet.rows.length>l.maxRows||!Array.isArray(packet.hits)||packet.hits.length>l.maxHitRegions||size(packet)>l.maxLayoutBytes)return false;
  const c=packet.context;
  if(c.profile!==catalog.profile||typeof c.unit!=='string'||!c.unit||!exact(c.viewport,['x','y','width','height'])||
      !Object.values(c.viewport).every(v=>coordinate(v,l))||c.viewport.width<=0||c.viewport.height<=0||
      !coordinate(c.pageDistance,l)||c.pageDistance<=0||c.pageDistance>c.viewport.height||
      ['sourceRevision','annotationSet','layoutGeneration','decorationGeneration','projectionGeneration','configurationGeneration','metricsGeneration','viewportGeneration'].some(k=>!Number.isSafeInteger(c[k])||c[k]<0))return false;
  try{validateCanonicalBatch(packet.source,[]);}catch{return false;}
  const stops=[];
  for(let ri=0;ri<packet.rows.length;ri++){
    const row=packet.rows[ri];
    if(!exact(row,['top','bottom','stops'])||!coordinate(row.top,l)||!coordinate(row.bottom,l)||row.top>=row.bottom||
        ri>0&&packet.rows[ri-1].bottom!==row.top||!Array.isArray(row.stops)||!row.stops.length)return false;
    for(let si=0;si<row.stops.length;si++){
      const s=row.stops[si];if(!exact(s,['position','affinity','x','left','right'])||!point(stopPoint(s),packet.source)||!coordinate(s.x,l)||si>0&&row.stops[si-1].x>s.x)return false;
      stops.push(s);if(stops.length>l.maxStops)return false;
    }
  }
  for(let i=0;i<stops.length;i++)for(const direction of ['left','right']){
    const link=stops[i][direction];
    if(!link||!['edge','unavailable','stop'].includes(link.kind)||!exact(link,link.kind==='stop'?['kind','index']:['kind']))return false;
    if(link.kind==='stop'&&(!Number.isSafeInteger(link.index)||link.index<0||link.index>=stops.length||link.index===i||
        !equal(stops[link.index][direction==='left'?'right':'left'],{kind:'stop',index:i})))return false;
  }
  let work=0;
  for(let i=0;i<stops.length;i++)for(const direction of ['left','right']){
    const seen=new Set([i]);let at=i;
    while(stops[at][direction].kind==='stop'){
      if(++work>l.maxValidationSteps)return false;
      at=stops[at][direction].index;if(!Number.isSafeInteger(at)||at<0||at>=stops.length||seen.has(at))return false;seen.add(at);
    }
  }
  for(const hit of packet.hits){
    if(!exact(hit,['rect','target'])||!rect(hit.rect,l))return false;
    const t=hit.target;
    if(t?.kind==='source'){if(!exact(t,['kind','index'])||!Number.isSafeInteger(t.index)||t.index<0||t.index>=stops.length)return false;}
    else if(t?.kind==='virtual'){if(!exact(t,['kind','id'])||typeof t.id!=='string'||!t.id)return false;}
    else return false;
  }
  // Conditional caret/hit inverse only where one unambiguous source hit is declared.
  const flat=flatten(packet);
  for(const s of flat){
    const hit=[];for(const h of packet.hits){if(++work>l.maxValidationSteps)return false;if(inside(h.rect,s.x,packet.rows[s.row].top))hit.push(h.target);}
    const unique=[...new Map(hit.map(t=>[JSON.stringify(t),t])).values()];
    if(unique.length===1&&unique[0].kind==='source'&&!equal(stopPoint(stops[unique[0].index]),stopPoint(s)))return false;
  }
  return true;
}
export function acceptGeometry(state,packet,current,limits){
  if(!packet||!exact(packet.context,catalog.contextFields)||!currentContext(state,packet.context,current)||!bounded(packet,limits))return null;
  const d=state.documents.find(d=>equal(d.document,packet.context.document));if(!equal(d.source,packet.source))return null;
  const ticket=Object.freeze({profile:catalog.profile});layouts.set(ticket,{packet:copy(packet),cache:new Map()});return ticket;
}
function active(state,ticket,current,limits){
  const p=layouts.get(ticket);
  return p&&currentContext(state,p.packet.context,current)&&!p.retired&&p.cache.size<=limits?.maxTargets&&[...p.cache.keys()].every(k=>Buffer.byteLength(k,'utf8')<=limits.maxRequestBytes)&&bounded(p.packet,limits)&&
    equal(state.documents.find(d=>equal(d.document,p.packet.context.document)).source,p.packet.source)?p:null;
}
// Trusted composition facets, not public verbs or an in-process security boundary.
export function geometryContextCurrent(state,context,current){
  return !!(context?.document&&context?.view&&exact(context,catalog.contextFields)&&currentContext(state,context,current));
}
export function geometryLayoutCurrent(state,ticket,current,limits){return !!active(state,ticket,current,limits);}
export function retireGeometry(ticket){const p=layouts.get(ticket);if(!p)return false;p.retired=true;return true;}
const flatten=packet=>packet.rows.flatMap((r,row)=>r.stops.map(s=>({...s,row})));
function answerSource(record,ticket,key,result,origin,steps,limits){
  let ref=record.cache.get(key);
  if(!ref){
    if(record.cache.size>=limits.maxTargets)return unavailable();
    ref=Object.freeze({kind:'owned-visual-target/1'});
    // A withheld reply must not install an undisclosed persistent target.
    if(size({...result,reference:ref})>limits.maxReplyBytes)return unavailable();
    targets.set(ref,{layout:ticket,index:result.index,point:copy(result.point),origin:copy(origin),steps});record.cache.set(key,ref);
  }
  return {...result,reference:ref};
}
function choose(candidates,choice){
  const unique=[...new Map(candidates.map(c=>[JSON.stringify(c),c])).values()];
  if(choice!==null)return unique.find(c=>equal(c,choice))??null;
  return unique.length===1?unique[0]:unique.length?{kind:'ambiguous',choices:unique}:null;
}
export function queryGeometry(state,ticket,request,current,limits){
  const p=active(state,ticket,current,limits);
  if(!p||!request||!Object.hasOwn(catalog.variants,request.operation)||!exact(request,[...catalog.common,...catalog.variants[request.operation]])||request.contract!==catalog.contract||size(request)>limits.maxRequestBytes)return unavailable();
  const {packet}=p,stops=flatten(packet),key=JSON.stringify(request);let out;
  if(request.operation==='bounded-layout'){
    if(!Number.isSafeInteger(request.start)||request.start<0||request.start>packet.rows.length||!Number.isSafeInteger(request.count)||request.count<0||request.count>limits.maxRows||!Number.isSafeInteger(request.start+request.count))return unavailable();
    const rows=packet.rows.slice(request.start,request.start+request.count).map(r=>({top:r.top,bottom:r.bottom,stops:r.stops.map(s=>({...stopPoint(s),x:s.x}))}));
    out={status:'layout',unit:packet.context.unit,rows,coverage:{atStart:packet.atStart&&request.start===0,atEnd:packet.atEnd&&request.start+rows.length===packet.rows.length}};
  }else if(request.operation==='caret-hit-conversion'){
    const r=request.request;
    if(r?.kind==='caret'){
      if(!exact(r,['kind','point'])||!point(r.point,packet.source))return unavailable();
      out={status:'carets',carets:stops.flatMap((s,index)=>equal(stopPoint(s),r.point)?[{index,x:s.x,top:packet.rows[s.row].top,bottom:packet.rows[s.row].bottom}]:[])};
      if(!out.carets.length)return unavailable();
    }else if(r?.kind==='hit'){
      if(!exact(r,['kind','x','y','choice'])||!coordinate(r.x,limits)||!coordinate(r.y,limits))return unavailable();
      const candidates=packet.hits.filter(h=>inside(h.rect,r.x,r.y)).map(h=>h.target.kind==='source'?{kind:'source',index:h.target.index,point:stopPoint(stops[h.target.index])}:{kind:'virtual',id:h.target.id});
      const selected=choose(candidates,r.choice);if(!selected)return unavailable();
      out=selected.kind==='source'?answerSource(p,ticket,key,{status:'source',point:selected.point,index:selected.index},null,0,limits):{status:selected.kind,...(selected.kind==='virtual'?{id:selected.id}:{choices:selected.choices})};
    }else return unavailable();
  }else{
    if(!Number.isSafeInteger(request.origin)||request.origin<0||request.origin>=stops.length||!catalog.units.includes(request.unit)||
        !Number.isSafeInteger(request.count)||request.count<1||request.count>limits.maxSteps||!Array.isArray(request.choices)||request.choices.length!==request.count)return unavailable();
    const origin=stops[request.origin];let index=request.origin,goal=origin.x,pageY=BigInt(packet.rows[origin.row].top);
    if(request.goal!==null){if(!exact(request.goal,['layout','x'])||request.goal.layout!==ticket||!coordinate(request.goal.x,limits))return unavailable();goal=request.goal.x;}
    for(let step=0;step<request.count;step++){
      const s=stops[index],row=packet.rows[s.row];
      if(['left','right'].includes(request.unit)){
        if(request.choices[step]!==null)return unavailable();
        const link=s[request.unit];if(link.kind==='unavailable')return unavailable();if(link.kind==='stop')index=link.index;
      }else if(['row-start','row-end'].includes(request.unit)){
        if(request.choices[step]!==null)return unavailable();
        const list=stops.map((s,i)=>({...s,index:i})).filter(s=>s.row===origin.row);index=list[request.unit==='row-start'?0:list.length-1].index;
      }else{
        const down=request.unit.endsWith('down'),page=request.unit.startsWith('page'),targetY=page?(pageY+=BigInt(down?1:-1)*BigInt(packet.context.pageDistance)):null;
        if(page&&(targetY < -BigInt(limits.maxCoordinate)||targetY>BigInt(limits.maxCoordinate)))return unavailable();
        let targetRow=page?packet.rows.findIndex(r=>r.top<=targetY&&targetY<r.bottom):s.row+(down?1:-1);
        if(page&&targetRow<0)targetRow=targetY<packet.rows[0].top?-1:packet.rows.length;
        if(targetRow<0){if(!packet.atStart)return unavailable();targetRow=0;}
        if(targetRow>=packet.rows.length){if(!packet.atEnd)return unavailable();targetRow=packet.rows.length-1;}
        const list=stops.map((s,i)=>({...s,index:i})).filter(s=>s.row===targetRow),distances=list.map(s=>{const d=BigInt(s.x)-BigInt(goal);return d<0n?-d:d;}),distance=distances.reduce((a,b)=>a<b?a:b);
        const selected=choose(list.filter((s,i)=>distances[i]===distance).map(s=>({kind:'source',index:s.index,point:stopPoint(s)})),request.choices[step]);
        if(!selected)return unavailable();if(selected.kind==='ambiguous'){const reply={status:'ambiguous',step,choices:selected.choices};return size(reply)<=limits.maxReplyBytes?reply:unavailable();}index=selected.index;
      }
    }
    const point=stopPoint(stops[index]);out=answerSource(p,ticket,key,{status:'source',point,index,goal:{layout:ticket,x:goal}},stopPoint(origin),request.count,limits);
  }
  return size(out)<=limits.maxReplyBytes?out:unavailable();
}
export function prepareVisualSelection(state,request,g,selectionLimits,limits){
  if(!exact(request,catalog.selectionFields)||request.contract!==catalog.selectionContract||request.operation!=='supported-visual-movement'||request.profile!==catalog.profile||
      !catalog.selectionModes.includes(request.mode)||typeof request.extend!=='boolean'||!Array.isArray(request.targets)||request.targets.length>limits?.maxTargets)return null;
  const layout=active(state,request.layout,g.geometry,limits);if(!layout||size(request)>limits.maxRequestBytes||!equal(layout.packet.context.view,request.view)||!equal(layout.packet.context.document,request.document)||layout.packet.context.sourceRevision!==request.sourceRevision)return null;
  const view=state.views.find(v=>equal(v.view,request.view)),resolved=request.targets.map(ref=>targets.get(ref));
  if(!resolved.length||resolved.some(t=>!t||t.layout!==request.layout)||resolved.reduce((n,t)=>n+t.steps,0)>limits.maxSteps)return null;
  const represented=flatten(layout.packet),uniquePoint=p=>represented.filter(s=>equal(stopPoint(s),p)).length===1;
  if(resolved.some(t=>!uniquePoint(t.point)))return null;
  let selections=copy(view.selections),primary=view.primary;
  const moved=(s,t)=>({anchor:request.extend?s.anchor:t.point.position,head:t.point.position,affinity:t.point.affinity});
  if(request.mode==='move-heads'){
    if(resolved.length!==selections.length||resolved.some((t,i)=>!uniquePoint(t.origin)||!equal(t.origin,{position:selections[i].head,affinity:selections[i].affinity})))return null;
    selections=selections.map((s,i)=>moved(s,resolved[i]));
  }else{
    if(resolved.length!==1)return null;
    const s=moved(selections[primary],resolved[0]);
    if(request.mode==='replace-all'){selections=[s];primary=0;}else selections[primary]=s;
  }
  const setter={contract:'selection-edit/1',operation:'set-selection',invocation:request.invocation,view:request.view,document:request.document,
    sourceRevision:request.sourceRevision,expectedSelection:request.expectedSelection,profile:'explicit-positional-selection/1',selections,primary};
  const core=prepareSelectionEdit(state,setter,g,selectionLimits);if(!core)return null;
  const plan=Object.freeze({contract:catalog.selectionContract});plans.set(plan,{core,layout:request.layout,selectionLimits:copy(selectionLimits),steps:resolved.reduce((n,t)=>n+t.steps,0),requestBytes:size(request)});return plan;
}
export function publishVisualSelection(state,plan,g,selectionLimits,limits){
  const p=plans.get(plan),refuse=()=>({status:'not-published',state});
  if(!p||!equal(p.selectionLimits,selectionLimits)||!active(state,p.layout,g.geometry,limits)||p.steps>limits.maxSteps||p.requestBytes>limits.maxRequestBytes)return refuse();
  return publishSelectionEdit(state,p.core,g);
}
// Trusted F15 composition facet, not a public reflection or arbitrary rectangle verb.
export function geometryTargetProjection(state,layout,reference,current,limits){
  const p=active(state,layout,current,limits),t=targets.get(reference);if(!p||!t||t.layout!==layout)return null;
  const stops=flatten(p.packet),s=stops[t.index];if(!s||!equal(stopPoint(s),t.point))return null;
  const row=p.packet.rows[s.row];
  return {context:copy(p.packet.context),point:copy(t.point),index:t.index,rect:{left:s.x,right:s.x,top:row.top,bottom:row.bottom}};
}
