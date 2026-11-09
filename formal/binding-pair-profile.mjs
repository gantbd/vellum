// Trusted normalized activated profile. Legacy references are not silently migrated.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
export const pairCatalog=JSON.parse(fs.readFileSync(new URL('./binding-pairs.json',import.meta.url)));
const writeCatalog=JSON.parse(fs.readFileSync(new URL('./source-save.json',import.meta.url)));
export const pairExact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
export const pairNatural=n=>Number.isSafeInteger(n)&&n>=0;
export const pairSize=v=>Buffer.byteLength(JSON.stringify(v));
const label=v=>typeof v==='string'&&v.length>0&&Buffer.from(v).toString('utf8')===v;
export const pairIdentity=d=>pairExact(d,['authority','id','incarnation'])&&label(d.authority)&&label(d.id)&&pairNatural(d.incarnation);
export const pairKey=b=>JSON.stringify([b.identityDomain,b.identityKey]);
export const pairContext=row=>({provider:structuredClone(row.provider),resource:structuredClone(row.resource),registrationGeneration:row.generation,identityDomain:row.identityDomain,identityKey:row.identityKey,locator:row.locator});
export function pairRegistration(rows,p,r,g,l){const found=rows.filter(row=>equal(row.provider,p)&&equal(row.resource,r)),row=found[0];return found.length===1&&pairExact(row,writeCatalog.registrationFields)&&pairIdentity(row.provider)&&pairIdentity(row.resource)&&pairNatural(row.generation)&&row.generation===g&&row.identityKnown===true&&row.logicalContinuity===true&&['syncSupported','conditionalWrite','causalReceipts','nonReuseTokens','exclusiveCreate'].every(k=>typeof row[k]==='boolean')&&[row.identityDomain,row.identityKey,row.locator,row.procedure].every(s=>label(s)&&Buffer.byteLength(s)<=l.maxTagBytes)?row:null;}
const binding=b=>b&&pairNatural(b.generation)&&pairIdentity(b.provider)&&pairIdentity(b.resource)&&pairNatural(b.registrationGeneration)&&[b.identityDomain,b.identityKey,b.locator].every(label);
export function pairLimitsValid(l){return pairExact(l,pairCatalog.limits)&&Object.values(l).every(pairNatural);}
export function pairRootValid(state,l){
 if(!pairLimitsValid(l)||!label(state?.authority)||!pairNatural(state.publication)||pairSize(state)>l.maxRootBytes||!Array.isArray(state.documents)||state.documents.length>l.maxDocuments||!state.documents.every(d=>pairIdentity(d.document)&&d.document.authority===state.authority)||new Set(state.documents.map(d=>JSON.stringify([d.document.authority,d.document.id,d.document.incarnation]))).size!==state.documents.length||!Array.isArray(state.resourceOwners)||state.resourceOwners.length>l.maxOwnerRows||!state.resourceOwners.every(r=>r&&label(r.identityDomain)&&label(r.identityKey)&&pairIdentity(r.document)&&pairNatural(r.bindingGeneration)&&(r.artifact===undefined||r.artifact==='annotations')&&state.documents.some(d=>equal(d.document,r.document)))||!Array.isArray(state.writeRegistrations)||state.writeRegistrations.length>l.maxRegistrations||new Set(state.resourceOwners.map(pairKey)).size!==state.resourceOwners.length)return false;
 for(const d of state.documents){if(!Object.hasOwn(d,'ordinaryBindings'))continue;const p=d.ordinaryBindings;if(!pairExact(p,['generation','annotations'])||!pairNatural(p.generation)||p.generation<1||!Object.hasOwn(d,'binding')||d.binding!==null&&!binding(d.binding)||p.annotations!==null&&(!pairExact(p.annotations,['generation','provider','resource','registrationGeneration','identityDomain','identityKey','locator'])||!binding(p.annotations)))return false;
  if(d.binding){const rows=state.resourceOwners.filter(r=>pairKey(r)===pairKey(d.binding));if(rows.length!==1||rows[0].artifact!==undefined||!equal(rows[0].document,d.document)||rows[0].bindingGeneration!==d.binding.generation)return false;}
  if(p.annotations){const a=p.annotations,rows=state.resourceOwners.filter(r=>pairKey(r)===pairKey(a));if(a.generation<1||a.generation>p.generation||d.binding&&pairKey(d.binding)===pairKey(a)||rows.length!==1||rows[0].artifact!=='annotations'||!equal(rows[0].document,d.document)||rows[0].bindingGeneration!==a.generation)return false;}
 }
 for(const row of state.resourceOwners.filter(r=>r.artifact==='annotations')){const ds=state.documents.filter(d=>equal(d.document,row.document)),a=ds[0]?.ordinaryBindings?.annotations;if(ds.length!==1||!a||pairKey(a)!==pairKey(row)||a.generation!==row.bindingGeneration)return false;}
 return true;
}
export function ordinaryAnnotationOwner(d,row){const p=d?.ordinaryBindings,a=p?.annotations;return pairExact(p,['generation','annotations'])&&pairNatural(p.generation)&&p.generation>0&&binding(a)&&a.generation>0&&a.generation<=p.generation&&row?.artifact==='annotations'&&equal(row.document,d.document)&&pairKey(a)===pairKey(row)&&a.generation===row.bindingGeneration;}
export function pairDocumentRegistrationsCurrent(state,d,l){return [d.binding,d.ordinaryBindings?.annotations??null].every(b=>{if(b===null)return true;if(!binding(b))return false;const row=pairRegistration(state.writeRegistrations,b.provider,b.resource,b.registrationGeneration,l);return row&&equal(Object.fromEntries(Object.keys(pairContext(row)).map(k=>[k,b[k]])),pairContext(row));});}
export function ordinarySourceBindingReady(state,d,source,g){
 // Existing single-artifact reference fixtures remain historical, not evidence of
 // native v1 migration. Every activated root takes the full pair check below.
 if(!Object.hasOwn(d??{},'ordinaryBindings'))return true;
 if(!pairRootValid(state,g?.ordinaryBindingLimits)||!pairDocumentRegistrationsCurrent(state,d,g.ordinaryBindingLimits)||g.ordinaryBindingsCurrent!==true||g.ordinaryIdentityCurrent!==true||!equal(g.ordinaryPairBasis,{document:d.document,source:d.binding,inventory:d.ordinaryBindings}))return false;
 const a=d.ordinaryBindings.annotations;if(!source||!a)return true;const e=g.ordinaryDistinction;
 return binding(source)&&pairKey(source)!==pairKey(a)&&pairExact(e,['document','source','annotations','relation'])&&e.relation==='distinct'&&equal(e.document,d.document)&&equal(e.source,source)&&equal(e.annotations,a);
}
// Identity/pair continuity does not grant or require source-write capability.
export function ordinaryBindingIdentityReady(state,d,g){if(!Object.hasOwn(d??{},'ordinaryBindings')||!pairRootValid(state,g?.ordinaryBindingLimits)||g?.ordinaryBindingsCurrent!==true||g.ordinaryIdentityCurrent!==true||!equal(g.ordinaryPairBasis,{document:d.document,source:d.binding,inventory:d.ordinaryBindings}))return false;const source=d.binding,a=d.ordinaryBindings.annotations,e=g.ordinaryDistinction;return !source||!a||binding(source)&&pairKey(source)!==pairKey(a)&&pairExact(e,['document','source','annotations','relation'])&&e.relation==='distinct'&&equal(e.document,d.document)&&equal(e.source,source)&&equal(e.annotations,a);}
