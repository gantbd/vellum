// Format-neutral owned initial-restoration plans; no storage decoder or provider I/O.
import {isDeepStrictEqual as equal} from 'node:util';
import {membershipReady,validMembershipRoot} from './annotation-membership.mjs';
const owned=new WeakMap(),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const natural=n=>Number.isSafeInteger(n)&&n>=0,positive=n=>natural(n)&&n>0;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const label=s=>typeof s==='string'&&s.length>0&&Buffer.from(s,'utf8').toString('utf8')===s;
const identity=d=>exact(d,['authority','id','incarnation'])&&label(d.authority)&&label(d.id)&&positive(d.incarnation);
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
export const annotationRestorationGates=Object.freeze(['authorized','scope','continuous','live','notStopped','annotationWritable','restorationAllowed','identityRestorable','sourceEvidenceReady','aliasesCurrent','targetEvidenceReady','historyRestorationReady','preparationReserved','resourcesReady','companionsReady','replyReady']);
const allowed=g=>annotationRestorationGates.every(k=>g?.[k]===true);
export const restorationSourceValid=(s,l)=>{try{return Array.isArray(s)&&s.length<=l&&Object.keys(s).length===s.length&&s.every(b=>Number.isInteger(b)&&b>=0&&b<=255)&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Uint8Array.from(s))!==undefined;}catch{return false;}};

export function annotationRestorationTargetBasis(d){return copy({document:d.document,live:d.live,source:d.source,revision:d.revision,annotationSet:d.annotationSet,records:d.records,annotationLifecycle:d.annotationLifecycle,binding:d.binding,ordinaryBindings:d.ordinaryBindings});}
export function annotationRestorationReady(root,authority,document,profile,g){
 if(!allowed(g)||!identity(document)||root?.authority!==authority?.authority||document.authority!==authority.authority||!exact(profile,['native','membership'])||!profile.native||!Object.values(profile.native).every(natural)||!membershipReady(root,{...g,membershipLimits:profile.membership})||size(root)>profile.native.maxRootBytes||root.documents.length>profile.native.maxDocuments||!Array.isArray(root.groups)||!root.groups.every(group=>Array.isArray(group.changes)))return null;
 const ds=root.documents.filter(d=>equal(d.document,document)),d=ds[0];if(ds.length!==1||d.live!==true||d.records.length!==0||d.annotationLifecycle.entries.length!==0||!restorationSourceValid(d.source,profile.native.maxSourceBytes)||root.groups.some(group=>group.changes.some(c=>equal(c.document,document)&&c.kind!=='source')))return null;return d;
}

export function sealAnnotationRestoration(domain,document,context,basis,records,profile,proof,historicalRequired){
 const value={domain,document:copy(document),context:copy(context),basis:copy(basis),records:copy(records),profile:copy(profile),proof:copy(proof),historicalRequired};if(size({...value,domain:{kind:'invocation-domain/1'}})>profile.membership.maxPlanBytes)return null;const plan=Object.freeze({kind:'initial-annotation-restoration/1'});owned.set(plan,freeze(value));return plan;
}
export function annotationRestorationCandidate(domain,root,authority,plan,g){
 const p=owned.get(plan);if(!p||p.domain!==domain||p.context.authority!==authority.authority||p.context.engine!==authority.engine||!allowed(g))return null;const d=annotationRestorationReady(root,authority,p.document,p.profile,g);if(!d||!equal(annotationRestorationTargetBasis(d),p.basis)||!equal(g.restorationBasis,p.proof)||p.historicalRequired&&g.historicalSourceEvidenceReady!==true)return null;
 if(p.records.length===0)return {status:'unchanged',state:root,document:copy(p.document)};
 if(g.versionsAvailable!==true||root.publication===Number.MAX_SAFE_INTEGER||d.annotationSet===Number.MAX_SAFE_INTEGER)return null;
 const next=copy(root),n=next.documents.find(d=>equal(d.document,p.document));n.records=copy(p.records);n.annotationLifecycle.entries=n.records.map(r=>({id:r.id,incarnation:r.incarnation,phase:'present',bodyFloor:r.bodyVersion,targetFloor:r.targetGeneration,retained:null}));n.annotationSet++;next.publication++;
 if(size(next)>p.profile.native.maxRootBytes||!validMembershipRoot(next,p.profile.membership))return null;return {status:'restored',state:next,document:copy(p.document)};
}
export function discardAnnotationRestoration(plan){return owned.delete(plan);}
