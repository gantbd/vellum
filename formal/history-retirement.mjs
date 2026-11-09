// Trusted normalized supervision candidates; no native root installation or reclamation.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {validMembershipRoot} from './annotation-membership.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./history-retirement.json',import.meta.url))),owned=new WeakMap(),copy=structuredClone;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const natural=n=>Number.isSafeInteger(n)&&n>=0,size=v=>Buffer.byteLength(JSON.stringify(v));
const label=s=>typeof s==='string'&&s.length>0&&Buffer.from(s).toString('utf8')===s;
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const distinct=xs=>new Set(xs).size===xs.length;
const identity=d=>exact(d,['authority','id','incarnation'])&&label(d.authority)&&label(d.id)&&natural(d.incarnation);
const key=d=>JSON.stringify({authority:d.authority,id:d.id,incarnation:d.incarnation});
const historyBasis=s=>({authority:s.authority,groups:copy(s.groups),frontiers:copy(s.frontiers)});
function hostValid(h){return h&&h.owner&&typeof h.owner==='object'&&label(h.recipient)&&label(h.profile)&&exact(h.limits,catalog.limits)&&Object.values(h.limits).every(natural)&&catalog.gates.every(k=>h[k]===true);}
function registry(state,l,membershipLimits){
 if(!state||!label(state.authority)||!natural(state.publication)||size(state)>l.maxRootBytes||!Array.isArray(state.documents)||state.documents.length>l.maxDocuments||!Array.isArray(state.groups)||state.groups.length>l.maxGroups||!state.frontiers||typeof state.frontiers!=='object'||Array.isArray(state.frontiers)||Object.keys(state.frontiers).length>l.maxScopes)return null;
 if(state.documents.some(d=>!identity(d.document)||d.document.authority!==state.authority)||new Set(state.documents.map(d=>key(d.document))).size!==state.documents.length)return null;
 if(state.documents.some(d=>Object.hasOwn(d,'annotationLifecycle'))&&!validMembershipRoot(state,membershipLimits))return null;
 const groups=new Map();let changes=0,entries=0;const seen=new Map(),edges=new Map();
 for(const g of state.groups){
  if(!exact(g,['id','scopes','changes','valid'])||!label(g.id)||groups.has(g.id)||typeof g.valid!=='boolean'||!Array.isArray(g.scopes)||g.scopes.length===0||g.scopes.length>l.maxParticipants||!g.scopes.every(label)||!distinct(g.scopes)||!Array.isArray(g.changes)||g.changes.length===0||(changes+=g.changes.length)>l.maxChanges)return null;
  for(const c of g.changes){if(!c||!['source','body','target','membership'].includes(c.kind)||!identity(c.document)||c.document.authority!==state.authority||!state.documents.some(d=>equal(d.document,c.document))||!label(c.scope))return null;
   const expected=c.kind==='source'?`source:${key(c.document)}`:label(c.id)&&natural(c.incarnation)?`annotation:${key(c.document)}:${c.id}:${c.incarnation}`:null;if(c.scope!==expected)return null;
  }
  if(!equal([...new Set(g.changes.map(c=>c.scope))].sort(),g.scopes.toSorted()))return null;
  groups.set(g.id,g);seen.set(g.id,[]);
 }
 const positions=new Map(state.groups.map((g,i)=>[g.id,i]));
 for(const [scope,f]of Object.entries(state.frontiers)){
  if(!label(scope)||!exact(f,['undo','redo'])||!Array.isArray(f.undo)||!Array.isArray(f.redo))return null;
  if(f.undo.length&&f.redo.length&&positions.get(f.undo.at(-1))>=positions.get(f.redo.at(-1)))return null;
  for(const direction of ['undo','redo']){
   const stack=f[direction];if(!Array.isArray(stack)||(entries+=stack.length)>l.maxFrontierEntries||!distinct(stack))return null;
   for(let i=0;i<stack.length;i++){
    const id=stack[i],g=groups.get(id);if(!g?.valid||!g.scopes.includes(scope))return null;
    if(i&&!(direction==='undo'?positions.get(stack[i-1])<positions.get(id):positions.get(stack[i-1])>positions.get(id)))return null;
    seen.get(id).push({scope,direction});
    // To reach a lower entry one must first traverse the higher one.
    if(i)edges.set(JSON.stringify([id,stack[i-1]]),[id,stack[i-1]]);
   }
  }
 }
 for(const [id,g]of groups){const rows=seen.get(id);if(g.valid?(rows.length!==g.scopes.length||new Set(rows.map(x=>x.scope)).size!==g.scopes.length||new Set(rows.map(x=>x.direction)).size!==1):rows.length!==0)return null;}
 return {groups,edges};
}
function evaluate(state,seeds,h){
 const l=h.limits,r=registry(state,l,h.membershipLimits),facts=h.retention;if(!r||!Array.isArray(seeds)||seeds.length>l.maxSeeds||!distinct(seeds)||seeds.some(id=>!r.groups.has(id))||!exact(facts,['edges','pins','floors','allowed'])||!Array.isArray(facts.edges)||facts.edges.length>l.maxEdges||!Array.isArray(facts.pins)||facts.pins.length>l.maxPins||!distinct(facts.pins)||facts.pins.some(id=>!r.groups.has(id))||!Array.isArray(facts.allowed)||facts.allowed.length>l.maxGroups||!distinct(facts.allowed)||facts.allowed.some(id=>!r.groups.has(id))||!Array.isArray(facts.floors)||facts.floors.length>l.maxFloors)return null;
 const extras=new Set();for(const edge of facts.edges){if(!Array.isArray(edge)||edge.length!==2||edge.some(id=>!r.groups.has(id))||extras.has(JSON.stringify(edge)))return null;extras.add(JSON.stringify(edge));r.edges.set(JSON.stringify(edge),copy(edge));}
 if(r.edges.size>l.maxEdges)return null;
 const protectedGroups=new Set(facts.pins),floorKeys=new Set();
 for(const floor of facts.floors){if(!exact(floor,['scope','direction','through'])||!['undo','redo'].includes(floor.direction)||!label(floor.scope)||!r.groups.has(floor.through)||floorKeys.has(JSON.stringify(floor)))return null;floorKeys.add(JSON.stringify(floor));
  const stack=state.frontiers[floor.scope]?.[floor.direction],at=stack?.indexOf(floor.through);if(!stack||at<0)return null;for(const id of stack.slice(at))protectedGroups.add(id);
 }
 const outgoing=new Map([...r.groups.keys()].map(id=>[id,[]]));for(const [from,to]of r.edges.values())outgoing.get(from).push(to);
 const removed=new Set(seeds),queue=[...seeds];let steps=0;
 for(let i=0;i<queue.length;i++){
  if(steps>=l.maxClosureSteps)return null;steps++;
  for(const to of outgoing.get(queue[i])){if(steps>=l.maxClosureSteps)return null;steps++;if(!removed.has(to)){removed.add(to);queue.push(to);}}
 }
 if([...removed].some(id=>protectedGroups.has(id)||!facts.allowed.includes(id)))return null;
 if(removed.size===0){const result={status:'unchanged',state,retention:copy(facts)};return size(result)<=l.maxCandidateBytes?{removed:[],result}:null;}
 if(h.versionsAvailable!==true||state.publication===Number.MAX_SAFE_INTEGER)return null;
 const next=copy(state);next.groups=next.groups.filter(g=>!removed.has(g.id));for(const f of Object.values(next.frontiers))for(const direction of ['undo','redo'])f[direction]=f[direction].filter(id=>!removed.has(id));next.publication++;
 const retention={edges:facts.edges.filter(([from,to])=>!removed.has(from)&&!removed.has(to)).map(edge=>copy(edge)),pins:copy(facts.pins),floors:copy(facts.floors),allowed:facts.allowed.filter(id=>!removed.has(id))},result={status:'applied',state:next,retention};
 if(!registry(next,l,h.membershipLimits)||size(result)>l.maxCandidateBytes)return null;
 return {removed:state.groups.filter(g=>removed.has(g.id)).map(g=>g.id),result};
}
export function prepareHistoryRetirement(state,request,host){
 if(!hostValid(host)||!exact(request,catalog.requestFields)||request.contract!==catalog.contract||request.operation!=='retire-complete-groups'||request.authority!==state?.authority)return null;
 const checked=evaluate(state,request.seeds,host);if(!checked)return null;
 const value={request:copy(request),recipient:host.recipient,profile:host.profile,limits:copy(host.limits),membershipLimits:copy(host.membershipLimits),basis:historyBasis(state),retention:copy(host.retention),removed:checked.removed};
 if(size(value)>host.limits.maxPlanBytes)return null;const plan=Object.freeze({kind:'history-retirement-plan/1'});owned.set(plan,{owner:host.owner,value:freeze(value)});return plan;
}
export function publishHistoryRetirement(state,plan,host){
 const p=owned.get(plan),fail=()=>({status:'not-published',state});if(!p||p.owner!==host?.owner||!hostValid(host)||state?.authority!==p.value.request.authority||!Array.isArray(state.groups)||!state.frontiers||p.value.recipient!==host.recipient||p.value.profile!==host.profile||!equal(p.value.limits,host.limits)||!equal(p.value.membershipLimits,host.membershipLimits)||!equal(p.value.retention,host.retention)||!equal(p.value.basis,historyBasis(state)))return fail();
 const checked=evaluate(state,p.value.request.seeds,host);return checked&&equal(checked.removed,p.value.removed)?checked.result:fail();
}
