// Pure participants on an explicitly activated normalized lifecycle inventory.
// Fresh issuer, policy, retention and target-restoration truth remain supplied premises.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {compoundTargetShape,validCompoundTarget} from './annotation-targets.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-membership.json',import.meta.url))),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort()),natural=n=>Number.isSafeInteger(n)&&n>=0,version=n=>natural(n)&&n>0;
const scalar=s=>typeof s==='string'&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Buffer.from(s))===s,label=s=>scalar(s)&&s.length>0;
const fields=r=>({body:r.body,target:copy(r.target),placement:copy(r.placement)}),sameAuthored=(r,v)=>r.body===v.body&&equal(r.target,v.target);
const allowed=(g,ks)=>ks.every(k=>g?.[k]===true),member=r=>['create','delete'].includes(r?.kind),slotFor=(d,id,inc)=>d.annotationLifecycle.entries.find(s=>s.id===id&&s.incarnation===inc);
const placement=p=>exact(p,['state'])&&p.state==='detached'||exact(p,['state','sourceRevision','target'])&&['attached','affected'].includes(p.state)&&version(p.sourceRevision)&&compoundTargetShape(p.target);
function record(r,l){return exact(r,['id','incarnation','body','bodyVersion','target','targetGeneration','placement'])&&label(r.id)&&version(r.incarnation)&&scalar(r.body)&&version(r.bodyVersion)&&version(r.targetGeneration)&&exact(r.target,['sourceRevision','target'])&&version(r.target.sourceRevision)&&compoundTargetShape(r.target.target)&&placement(r.placement)&&size(r)<=l.maxRecordBytes;}
export function validMembershipRoot(state,l){
 if(!exact(l,catalog.limits)||!Object.values(l).every(natural)||!state||!natural(state.publication)||size(state)>l.maxRootBytes||!Array.isArray(state.documents)||state.documents.length>l.maxDocuments)return false;
 let count=0,retained=0;const documents=[];
 for(const d of state.documents){if(documents.some(document=>equal(document,d.document))||!Array.isArray(d.records)||!exact(d.annotationLifecycle,['entries'])||!Array.isArray(d.annotationLifecycle.entries)||!version(d.revision)||!version(d.annotationSet))return false;documents.push(d.document);
  const slots=new Set(),live=new Set();for(const r of d.records){if(!record(r,l)||live.has(r.id))return false;live.add(r.id);if(r.placement.state!=='detached'&&(r.placement.sourceRevision!==d.revision||!validCompoundTarget(d.source,r.placement.target)))return false;}
  for(const s of d.annotationLifecycle.entries){const identity=JSON.stringify([s.id,s.incarnation]);if(!exact(s,catalog.slotFields)||!label(s.id)||!version(s.incarnation)||!catalog.phases.includes(s.phase)||!version(s.bodyFloor)||!version(s.targetFloor)||slots.has(identity))return false;slots.add(identity);count++;
   const rows=d.records.filter(r=>r.id===s.id&&r.incarnation===s.incarnation);
   if(s.phase==='present'){if(rows.length!==1||s.retained!==null||s.bodyFloor>rows[0].bodyVersion||s.targetFloor>rows[0].targetGeneration)return false;}
   else{if(rows.length)return false;if(s.phase==='retired'){if(s.retained!==null)return false;}else{if(!record(s.retained,l)||s.retained.id!==s.id||s.retained.incarnation!==s.incarnation||s.retained.bodyVersion!==s.bodyFloor||s.retained.targetGeneration!==s.targetFloor)return false;retained+=size(s.retained);}}
  }
  if(d.records.some(r=>!d.annotationLifecycle.entries.some(s=>s.id===r.id&&s.incarnation===r.incarnation&&s.phase==='present')))return false;
 }
 return natural(count)&&count<=l.maxEntries&&natural(retained)&&retained<=l.maxRetainedBytes;
}
export function membershipPlanReady(state,plan,g){
 if(!allowed(g,catalog.gates)||!validMembershipRoot(state,g.membershipLimits)||size(plan)>g.membershipLimits.maxPlanBytes||plan.annotations.some(x=>!x||typeof x.kind!=='string'))return false;
 const creates=plan.annotations.filter(x=>x.kind==='create').map(x=>({document:x.document,id:x.id,incarnation:x.incarnation}));
 if(creates.length&&(g.allocationCurrent!==true||!equal(g.membershipAllocationPlan,plan)||!equal(g.membershipAllocations,creates)))return false;
 for(const r of plan.annotations.filter(member))if(plan.annotations.filter(x=>equal(x.document,r.document)&&x.id===r.id&&x.incarnation===r.incarnation).length!==1)return false;
 return true;
}
export function membershipReady(state,g){return allowed(g,catalog.gates)&&validMembershipRoot(state,g.membershipLimits);}
// An explicit inventory profile is not permission to infer legacy rows as empty.
export function membershipInventoryActive(state,g){return Object.hasOwn(g??{},'membershipLimits')||Array.isArray(state?.documents)&&state.documents.some(d=>d&&Object.hasOwn(d,'annotationLifecycle'));}
export function membershipInventoryValid(state,g){return Array.isArray(state?.documents)&&(!membershipInventoryActive(state,g)||validMembershipRoot(state,g?.membershipLimits));}
export function createMembership(d,input,target,g){
 if(!label(input.id)||!version(input.incarnation)||!scalar(input.body)||slotFor(d,input.id,input.incarnation)||d.records.some(r=>r.id===input.id))return null;
 const row={id:input.id,incarnation:input.incarnation,body:input.body,bodyVersion:1,target:copy(target.authored),targetGeneration:1,placement:copy(target.placement)};
 if(!record(row,g.membershipLimits))return null;
 d.records.push(row);d.annotationLifecycle.entries.push({id:row.id,incarnation:row.incarnation,phase:'present',bodyFloor:1,targetFloor:1,retained:null});return {before:null,after:fields(row)};
}
export function deleteMembership(d,input,original){
 const row=d.records.find(r=>r.id===input.id&&r.incarnation===input.incarnation),s=slotFor(d,input.id,input.incarnation);
 if(!row||!s||s.phase!=='present'||row.bodyVersion!==input.expectedBody||row.targetGeneration!==input.expectedTarget)return null;
 s.phase='retained-absent';s.bodyFloor=row.bodyVersion;s.targetFloor=row.targetGeneration;s.retained=copy(row);d.records.splice(d.records.indexOf(row),1);return {before:fields(original),after:null};
}
export function reverseMembership(d,change,expected,desired,group,g){
 const s=slotFor(d,change.id,change.incarnation),row=d.records.find(r=>r.id===change.id&&r.incarnation===change.incarnation),wish=change[desired],prior=change[expected];
 if(!s||s.phase==='retired')return false;
 if(wish===null){
  if(prior===null||!row||s.phase!=='present'||!sameAuthored(row,prior))return false;
  s.phase='retained-absent';s.bodyFloor=row.bodyVersion;s.targetFloor=row.targetGeneration;s.retained=copy(row);d.records.splice(d.records.indexOf(row),1);return true;
 }
 if(prior!==null||row||s.phase!=='retained-absent'||!s.retained||!sameAuthored(s.retained,wish)||d.records.some(r=>r.id===change.id)||g.targetRestorationValid!==true)return false;
 const source=group.changes.find(c=>c.kind==='source'&&equal(c.document,d.document));
 if(source?!equal(d.source,source[desired]):d.revision!==change[desired+'SourceRevision'])return false;
 if(wish.placement.state!=='detached'&&!validCompoundTarget(d.source,wish.placement.target))return false;
 const bodyVersion=s.bodyFloor+1,targetGeneration=s.targetFloor+1;if(!version(bodyVersion)||!version(targetGeneration))return false;
 const restored={id:s.id,incarnation:s.incarnation,body:wish.body,bodyVersion,target:copy(wish.target),targetGeneration,placement:wish.placement.state==='detached'?{state:'detached'}:{...copy(wish.placement),sourceRevision:d.revision}};
 if(!record(restored,g.membershipLimits))return false;
 d.records.push(restored);s.phase='present';s.bodyFloor=bodyVersion;s.targetFloor=targetGeneration;s.retained=null;return true;
}
export function retireMembership(state,document,id,incarnation,g){
 const fail=()=>({status:'not-published',state});if(!allowed(g,catalog.retirementGates)||!validMembershipRoot(state,g.membershipLimits)||state.publication===Number.MAX_SAFE_INTEGER)return fail();
 const d=state.documents.find(d=>equal(d.document,document)),s=d&&slotFor(d,id,incarnation);if(!s||d.live===false||s.phase!=='retained-absent')return fail();
 if(!Array.isArray(state.groups)||state.groups.some(group=>group.changes.some(c=>equal(c.document,document)&&c.id===id&&c.incarnation===incarnation)))return fail();
 const next=copy(state),slot=slotFor(next.documents.find(d=>equal(d.document,document)),id,incarnation);slot.phase='retired';slot.retained=null;next.publication++;
 if(!validMembershipRoot(next,g.membershipLimits))return fail();return {status:'applied',state:next};
}
