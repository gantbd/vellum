// Serialized derived-work owner reference. No provider callbacks, clocks or I/O.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {geometryContextCurrent,acceptGeometry,geometryLayoutCurrent,retireGeometry} from './geometry-products.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./geometry-work.json',import.meta.url),'utf8'));
const geometryCatalog=JSON.parse(fs.readFileSync(new URL('./geometry-products.json',import.meta.url),'utf8'));
const owners=new WeakMap(),copy=structuredClone;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const size=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const natural=n=>Number.isSafeInteger(n)&&n>=0;
const identity=v=>exact(v,['authority','id','incarnation'])&&typeof v.authority==='string'&&v.authority.length>0&&typeof v.id==='string'&&v.id.length>0&&natural(v.incarnation);
const origin=o=>exact(o,catalog.originFields)&&Object.values(o).every(s=>typeof s==='string'&&s.length>0);
const originKey=o=>JSON.stringify(catalog.originFields.map(k=>o[k]));
const launchGates=['authorized','disclosureAllowed','continuous','live','notStopped','providerAvailable','capacity','reservationReady'];
const evidenceGates=['authentic','evidenceAllowed','ingressReady'];
const inWork=t=>t.phase==='ready'||t.phase==='submitted'&&!t.quiescent;
const usage=o=>({tasks:o.tasks.size,outstanding:[...o.tasks.values()].filter(inWork).length,issued:o.used.size,
  heldBytes:[...o.tasks.values()].reduce((n,t)=>n+t.charge,0)});
const invalidate=t=>{if(t.layout)retireGeometry(t.layout);};
function contextValid(state,c,g,l){
  if(!identity(c?.view)||!identity(c?.document)||!exact(c,geometryCatalog.contextFields)||c.profile!==geometryCatalog.profile||typeof c.unit!=='string'||!c.unit||
      ['sourceRevision','annotationSet','layoutGeneration','decorationGeneration','projectionGeneration','configurationGeneration','metricsGeneration','viewportGeneration'].some(k=>!natural(c[k]))||
      !exact(c.viewport,['x','y','width','height'])||!Object.values(c.viewport).every(n=>Number.isSafeInteger(n)&&Math.abs(n)<=l.maxCoordinate)||c.viewport.width<=0||c.viewport.height<=0||
      !Number.isSafeInteger(c.pageDistance)||c.pageDistance<=0||c.pageDistance>c.viewport.height||!geometryContextCurrent(state,c,g))return false;
  const v=state.views.find(v=>equal(v.view,c.view));
  return natural(v.viewportGeneration)&&v.viewportGeneration===c.viewportGeneration&&equal(v.viewportContext,Object.fromEntries(['unit','configurationGeneration','metricsGeneration','projectionGeneration'].map(k=>[k,c[k]])));
}
export function newGeometryWorkRegistry(limits,geometryLimits){
  if(!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||!natural(2*limits.maxEnvelopeBytes)||
      !geometryLimits||!geometryCatalog.limits.every(k=>natural(geometryLimits[k])))return null;
  const owner=Object.freeze({profile:catalog.profile});owners.set(owner,{limits:copy(limits),geometryLimits:copy(geometryLimits),used:new Set(),tasks:new Map()});return owner;
}
export function admitGeometryWork(owner,state,request,g){
  const o=owners.get(owner);if(!o||!launchGates.every(k=>g?.[k])||!exact(request,catalog.requestFields)||request.contract!==catalog.requestContract||
      request.operation!=='enabled-provider-measurement'||request.profile!==catalog.profile||!origin(request.origin)||!identity(request.provider)||
      !equal(request.provider,g.provider)||!contextValid(state,request.context,g.geometry,o.geometryLimits)||size(request)>o.limits.maxRequestBytes)return {status:'unavailable'};
  const source=state.documents.find(d=>equal(d.document,request.context.document)).source;
  if(!Array.isArray(source)||source.length>o.geometryLimits.maxRootBytes)return {status:'unavailable'};
  const minimal={contract:catalog.evidenceContract,task:{kind:'owned-geometry-work/1'},origin:request.origin,provider:request.provider,kind:'quiescence'};
  if(size(minimal)>o.limits.maxEnvelopeBytes||catalog.failures.some(reason=>size({...minimal,kind:'failure',reason})>o.limits.maxEnvelopeBytes))return {status:'unavailable'};
  const capture={request:copy(request),source:copy(source)},captureBytes=size(capture),charge=captureBytes+2*o.limits.maxEnvelopeBytes,u=usage(o),key=originKey(request.origin);
  if(captureBytes>o.limits.maxRequestBytes||!natural(charge)||o.used.has(key)||u.issued>=o.limits.maxIssued||u.tasks>=o.limits.maxTasks||u.outstanding>=o.limits.maxOutstanding||charge>o.limits.maxHeldBytes-u.heldBytes)return {status:'unavailable'};
  const task=Object.freeze({kind:'owned-geometry-work/1'});
  o.tasks.set(task,{capture,charge,phase:'ready',cancelled:false,superseded:false,quiescent:false,witnesses:[],contested:false,layout:null});o.used.add(key);
  return {status:'admitted',task};
}
export function submitGeometryWork(owner,task,state,g){
  const o=owners.get(owner),t=o?.tasks.get(task);
  if(!t||t.phase!=='ready'||t.cancelled||t.superseded||!launchGates.every(k=>g?.[k])||!equal(t.capture.request.provider,g.provider)||
      !contextValid(state,t.capture.request.context,g.geometry,o.geometryLimits))return {status:'not-submitted'};
  const doc=state.documents.find(d=>equal(d.document,t.capture.request.context.document));if(!equal(doc.source,t.capture.source))return {status:'not-submitted'};
  t.phase='submitted';return {status:'submitted',task,capture:copy(t.capture)};
}
export function controlGeometryWork(owner,task,operation,g){
  const t=owners.get(owner)?.tasks.get(task);if(!t||!g?.controlAllowed||!catalog.controls.includes(operation))return {status:'unavailable'};
  const field=operation==='cancel'?'cancelled':'superseded';if(t[field])return {status:'unchanged'};
  t[field]=true;if(t.phase==='ready')t.phase='closed';invalidate(t);return {status:'stopped'};
}
export function observeGeometryWork(owner,event,g){
  const o=owners.get(owner);if(!o||!evidenceGates.every(k=>g?.[k]))return {status:'unavailable'};
  const t=o.tasks.get(event?.task);
  // Correlation/authentication precede packet traversal and protected evidence service.
  if(!t||t.phase!=='submitted'||!Object.hasOwn(catalog.evidenceVariants,event.kind)||!exact(event,[...catalog.evidenceCommon,...catalog.evidenceVariants[event.kind]])||
      event.contract!==catalog.evidenceContract||!equal(event.origin,t.capture.request.origin)||!equal(event.provider,t.capture.request.provider)||size(event)>o.limits.maxEnvelopeBytes)return {status:'unavailable'};
  if(event.kind==='quiescence'){
    if(!g.quiescenceAttested)return {status:'unavailable'};
    if(t.quiescent)return {status:'duplicate'};t.quiescent=true;return {status:'quiescent'};
  }
  if(event.kind==='failure'&&!catalog.failures.includes(event.reason))return {status:'unavailable'};
  const witness=copy(event.kind==='result'?{kind:'result',packet:event.packet}:{kind:'failure',reason:event.reason});
  if(t.witnesses.some(w=>equal(w,witness)))return {status:'duplicate'};
  if(t.witnesses.length>=2)return {status:'evidence-full'};
  t.witnesses.push(witness);
  if(t.witnesses.length>1){t.contested=true;invalidate(t);return {status:'conflict'};}
  return {status:'recorded'};
}
export function acceptGeometryWork(owner,task,state,g){
  const o=owners.get(owner),t=o?.tasks.get(task),fail=()=>({status:'unavailable'});
  if(!t||!g?.acceptanceAllowed||!equal(t.capture.request.provider,g.provider)||t.cancelled||t.superseded||t.contested||t.witnesses.length!==1||t.witnesses[0].kind!=='result'||
      !contextValid(state,t.capture.request.context,g.geometry,o.geometryLimits))return fail();
  const packet=t.witnesses[0].packet;
  if(!equal(packet?.context,t.capture.request.context)||!equal(packet?.source,t.capture.source))return fail();
  if(t.layout)return geometryLayoutCurrent(state,t.layout,g.geometry,o.geometryLimits)?{status:'geometry',layout:t.layout}:fail();
  const peers=[...o.tasks.values()].filter(other=>other!==t&&other.layout&&equal(other.capture.request.context.view,t.capture.request.context.view));
  if(peers.some(other=>geometryLayoutCurrent(state,other.layout,g.geometry,o.geometryLimits)))return fail();
  const layout=acceptGeometry(state,packet,g.geometry,o.geometryLimits);if(!layout)return fail();
  for(const peer of peers)invalidate(peer);t.layout=layout;return {status:'geometry',layout};
}
export function retireGeometryWork(owner,task,g){
  const o=owners.get(owner),t=o?.tasks.get(task);
  if(!t||!g?.retirementAllowed||!g.retentionReleased||!g.deliveryDrained||inWork(t))return {status:'unavailable'};
  invalidate(t);o.tasks.delete(task);return {status:'retired'};
}
// Trusted bounded inspection facets for the supervisor/reference checks, not F06 ABI.
export function geometryWorkUsage(owner){const o=owners.get(owner);return o?usage(o):null;}
export function geometryWorkStatus(owner,task){
  const t=owners.get(owner)?.tasks.get(task);return t?{phase:t.phase,cancelled:t.cancelled,superseded:t.superseded,quiescent:t.quiescent,contested:t.contested,witnesses:t.witnesses.length,workHeld:inWork(t),heldBytes:t.charge}:null;
}
