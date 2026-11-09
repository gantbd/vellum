// First-party conditional basis authority. Neither a read selector nor a mutation.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {invocationAuthorityRoot,invocationAuthorityContext,invocationUsage,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment,releaseInvocationOwnerAttachment,beginInvocationAdmissionMutation,endInvocationAdmissionMutation} from './invocation-publication.mjs';
import {nativeResourceCatalogValid} from './resource-registration-state.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./registration-bases.json',import.meta.url))),owners=new WeakMap(),domains=new WeakMap(),copy=structuredClone;
const size=v=>Buffer.byteLength(JSON.stringify(v)),natural=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const gates=(g,ks)=>ks.every(k=>g?.[k]===true),no=()=>({status:'unavailable'}),token=kind=>Object.freeze({kind});
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const label=(s,max)=>typeof s==='string'&&s.length>0&&Buffer.byteLength(s)<=max&&Buffer.from(s).toString('utf8')===s;
const identity=(v,l)=>exact(v,['authority','id','incarnation'])&&label(v.authority,l)&&label(v.id,l)&&natural(v.incarnation);
const healthy=m=>invocationUsage(m.domain)?.faulted===false;
function scopeValid(m,c){return exact(c,catalog.contextFields)&&Object.values(c).every(v=>label(v,m.limits.maxTagBytes))&&size(c)<=m.limits.maxScopeBytes&&c.profile===catalog.profile&&c.authority===m.authority&&c.engine===m.engine;}
function slotValid(m,s){return exact(s,['table','provider','resource'])&&['read','write'].includes(s.table)&&identity(s.provider,m.limits.maxTagBytes)&&identity(s.resource,m.limits.maxTagBytes)&&size(s)<=m.limits.maxSlotBytes;}
function rootFor(m,g){if(!healthy(m)||!gates(g,catalog.currentGates)||g?.registrationBasisInterpretationCurrent!==true||!equal(g.registrationBasisProfile,m.nativeProfile))return null;const root=invocationAuthorityRoot(m.domain);return nativeResourceCatalogValid(root,m.nativeProfile)&&size(root)<=m.limits.maxRootBytes?root:null;}
const rowAt=(root,s)=>root[s.table==='read'?'resourceRegistrations':'writeRegistrations'].find(r=>equal(r.provider,s.provider)&&equal(r.resource,s.resource))??null;
const condition=row=>row===null?{state:'absent'}:{state:'present',generation:row.generation};
const useValue=p=>({slot:copy(p.slot),expected:copy(p.expected),row:copy(p.row)});
const publicReply=(basis,p)=>({status:'registration-basis',basis,condition:p.expected.state});
export function registrationBasisCharge(l){if(!exact(l,catalog.limits)||!Object.values(l).every(natural))return null;const n=l.maxProfileBytes+l.maxBases*(2*l.maxBasisBytes+l.maxUseBytes+2*l.maxReplyBytes);return natural(n)&&n<=l.maxHeldBytes?n:null;}
export function openNativeRegistrationBasisRegistry(domain,nativeProfile,limits,g){
 const attachment=beginInvocationOwnerAttachment(domain,'registration-bases',limits?.maxHeldBytes,token('registration-basis-registry/1'));if(!attachment)return null;
 try{const c=invocationAuthorityContext(domain),charge=registrationBasisCharge(limits);if(!c||invocationUsage(domain)?.faulted!==false||domains.has(domain)||charge===null||!gates(g,['ownerControlAllowed','resourcesReady','preparationReserved'])||!nativeResourceCatalogValid(invocationAuthorityRoot(domain),nativeProfile)||size(nativeProfile)>limits.maxProfileBytes)return null;const owner=token('registration-basis-registry/1');owners.set(owner,{domain,parentAttachment:attachment,authority:c.authority,engine:c.engine,nativeProfile:freeze(copy(nativeProfile)),limits:freeze(copy(limits)),charge,bases:new Map(),issued:0});domains.set(domain,owner);if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native registration basis attachment invariant');return owner;}
 finally{abortInvocationOwnerAttachment(domain,attachment);}
}
export function captureNativeRegistrationBasis(owner,c,slot,g){const m=owners.get(owner);if(!m||!scopeValid(m,c)||!slotValid(m,slot))return no();const root=rootFor(m,g);if(!root)return no();const row=rowAt(root,slot),p={scope:copy(c),slot:copy(slot),expected:condition(row),row:copy(row)};if(size(p)>m.limits.maxBasisBytes||size(useValue(p))>m.limits.maxUseBytes)return no();
 for(const [basis,old]of m.bases)if(equal(old,p))return publicReply(basis,p);
 // An identical retained basis is service, not a new obligation. Only issuance
 // enters ordinary admission; captured caller facts cannot reopen that fence.
 const admissionSection=beginInvocationAdmissionMutation(m.domain);if(!admissionSection)return no();try{
 if(owners.get(owner)!==m||invocationAuthorityRoot(m.domain)!==root)return no();
 if(m.bases.size>=m.limits.maxBases||m.issued>=m.limits.maxIssued)return no();
 const basis=token(catalog.profile),reply=publicReply(basis,p);if(size(reply)>m.limits.maxReplyBytes)return no();m.bases.set(basis,freeze(p));m.issued++;return reply;
 }finally{endInvocationAdmissionMutation(m.domain,admissionSection);}
}
function current(owner,basis,domain,c,g){const m=owners.get(owner),p=m?.bases.get(basis);if(!p||m.domain!==domain||!scopeValid(m,c)||!equal(p.scope,c))return null;const root=rootFor(m,g);if(!root)return null;const row=rowAt(root,p.slot);return equal(row,p.row)&&equal(condition(row),p.expected)?{m,p}:null;}
export function inspectNativeRegistrationBasis(owner,basis,domain,c,g){const found=current(owner,basis,domain,c,g);if(!found)return no();const reply={status:'registration-basis',condition:found.p.expected.state,availability:'current'};return size(reply)<=found.m.limits.maxReplyBytes?reply:no();}
// Private first-party consumer only. Not an ordinary recipient release surface.
// A later owned mutation must revalidate and use the existing native commit gate.
export function resolveNativeRegistrationBasis(owner,basis,domain,c,g){if(g?.registrationHostBasisAllowed!==true)return null;const found=current(owner,basis,domain,c,g);if(!found)return null;const value=useValue(found.p);return size(value)<=found.m.limits.maxUseBytes?freeze(value):null;}
// First-party scope validation also covers an empty batch and post-commit receipts.
// It does not assert that any particular captured row condition remains current.
export function nativeRegistrationBasisScopeCurrent(owner,domain,c,profile,before,g){const m=owners.get(owner);if(!m||m.domain!==domain||!scopeValid(m,c)||!equal(profile,m.nativeProfile)||g?.registrationHostBasisAllowed!==true)return false;const root=rootFor(m,g);return root!==null&&root===before;}
export function retireNativeRegistrationBasis(owner,basis,g){const m=owners.get(owner);if(!m?.bases.has(basis)||!gates(g,catalog.retirementGates))return no();m.bases.delete(basis);return {status:'retired'};}
export function retireNativeRegistrationBasisRegistry(owner,g){const m=owners.get(owner);if(!m||m.bases.size||!gates(g,catalog.retirementGates))return no();if(!releaseInvocationOwnerAttachment(m.domain,m.parentAttachment,g))return no();owners.delete(owner);return {status:'retired'};}
// Trusted supervision, never ordinary metric labels or a recipient epoch oracle.
export function nativeRegistrationBasisUsage(owner){const m=owners.get(owner);return m?{bases:m.bases.size,issued:m.issued,declaredHeldBytes:m.charge}:null;}
