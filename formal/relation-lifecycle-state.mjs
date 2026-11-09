// Closed logical container birth/retirement, not UI allocation or document disposal.
import {isDeepStrictEqual as equal} from 'node:util';
import {nativeSourceReadProfileValid} from './source-read-state.mjs';
const plans=new WeakMap(),copy=structuredClone,natural=n=>Number.isSafeInteger(n)&&n>=0,size=v=>Buffer.byteLength(JSON.stringify(v));
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const label=(s,l)=>typeof s==='string'&&s.length>0&&Buffer.from(s).toString()===s&&Buffer.byteLength(s)<=l;
const identity=(v,l)=>exact(v,['authority','id','incarnation'])&&label(v.authority,l)&&label(v.id,l)&&natural(v.incarnation);
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
export const relationLifecycleGates=Object.freeze(['authorized','scope','continuous','live','notStopped','resourcesReady','companionsReady','versionsAvailable','replyReady','ownerControlAllowed','preparationReserved','relationCoverageComplete']);
export function relationLifecycleRequiredGates(r){return r?.action==='create'?[...relationLifecycleGates,'creationAllowed']:r?.action==='retire'?[...relationLifecycleGates,'retirementAllowed','retainedConsumersSafe']:null;}
function rowsValid(root,l){const ids=xs=>new Set(xs.map(x=>JSON.stringify([x.authority,x.id,x.incarnation]))).size===xs.length;
 return label(root.authority,l.maxTagBytes)&&root.views.every(v=>identity(v.view,l.maxTagBytes)&&v.view.authority===root.authority&&(v.document===null||identity(v.document,l.maxTagBytes))&&['attachmentGeneration','selectionGeneration','viewportGeneration','sourceRevision','primary'].every(k=>natural(v[k]))&&Array.isArray(v.selections)&&v.profile==='explicit-positional-selection/1')&&ids(root.views.map(v=>v.view))&&root.workspaces.every(w=>identity(w.workspace,l.maxTagBytes)&&w.workspace.authority===root.authority&&typeof w.live==='boolean'&&natural(w.generation)&&Array.isArray(w.members)&&w.members.length<=l.maxRelations&&w.members.every(d=>identity(d,l.maxTagBytes))&&ids(w.members))&&ids(root.workspaces.map(w=>w.workspace));
}
function candidate(before,r,profile){const l=profile?.lifecycle;if(!nativeSourceReadProfileValid(before,profile)||!rowsValid(before,l)||!['view','workspace'].includes(r?.kind)||!['create','retire'].includes(r?.action)||!exact(r,r.action==='create'?['kind','action']:['kind','action','target','expected'])||size(r)>l.maxRequestBytes||!natural(before.publication)||before.publication===Number.MAX_SAFE_INTEGER)return null;
 const next=copy(before),table=r.kind==='view'?'views':'workspaces';let target;
 if(r.action==='create'){
  const a=before.identityAllocation;if(!exact(a,['namespace','next','last'])||!label(a.namespace,l.maxTagBytes)||!natural(a.next)||a.next===Number.MAX_SAFE_INTEGER||!natural(a.last)||a.next>a.last||before[table].length>=l.maxRelations)return null;
  target={authority:before.authority,id:`${a.namespace}:${a.next}`,incarnation:1};if(!identity(target,l.maxTagBytes)||[...before.documents.map(d=>d.document),...before.retiredDocuments,...before.views.map(v=>v.view),...before.workspaces.map(w=>w.workspace)].some(x=>equal(x,target)))return null;
  next[table].push(r.kind==='view'?{view:copy(target),document:null,sourceRevision:0,attachmentGeneration:1,selectionGeneration:1,viewportGeneration:1,selections:[],primary:0,profile:'explicit-positional-selection/1'}:{workspace:copy(target),live:true,generation:1,members:[]});next.identityAllocation.next++;
 }else{
  if(!identity(r.target,l.maxTagBytes)||r.target.authority!==before.authority)return null;const i=before[table].findIndex(row=>equal(row[r.kind],r.target));if(i<0)return null;const row=before[table][i];
  if(r.kind==='view'){if(!exact(r.expected,['attachmentGeneration','selectionGeneration','viewportGeneration'])||!Object.keys(r.expected).every(k=>natural(r.expected[k])&&r.expected[k]===row[k]))return null;}
  else if(!exact(r.expected,['generation'])||!natural(r.expected.generation)||r.expected.generation!==row.generation||row.live!==true)return null;
  target=copy(r.target);next[table].splice(i,1);
 }
 next.publication++;if(!nativeSourceReadProfileValid(next,profile)||!rowsValid(next,l))return null;
 return {next,result:{status:r.action==='create'?'created':'retired',kind:r.kind,target:copy(target)}};
}
export function prepareNativeRelationLifecycle(owner,domain,before,r,profile,g){if(!relationLifecycleRequiredGates(r)?.every(k=>g?.[k]===true))return null;const p=candidate(before,r,profile);if(!p||size(p.result)>owner.limits.maxResultBytes)return null;const token=Object.freeze({kind:'native-relation-lifecycle/1'});plans.set(token,{...p,owner,domain,before,request:freeze(copy(r)),profile:freeze(copy(profile))});return token;}
export function readNativeRelationLifecycle(token,owner,domain,before,g){const p=plans.get(token);if(!p||p.owner!==owner||p.domain!==domain||p.before!==before||!relationLifecycleRequiredGates(p.request)?.every(k=>g?.[k]===true))return null;const current=candidate(before,p.request,p.profile);return current&&equal(current.next,p.next)&&equal(current.result,p.result)&&size(p.result)<=owner.limits.maxResultBytes?p:null;}
export function commitNativeRelationLifecycle(token){const p=plans.get(token);if(!p)return null;const result=copy(p.result);plans.delete(token);return result;}
export function discardNativeRelationLifecycle(token){return plans.delete(token);}
