// Initial native role activation candidates, not root installation or annotation I/O.
import {isDeepStrictEqual as equal} from 'node:util';
import {pairCatalog as catalog,pairExact as exact,pairNatural as natural,pairSize as size,pairIdentity as identity,pairKey as key,pairContext as context,pairRootValid,pairRegistration,pairDocumentRegistrationsCurrent,ordinarySourceBindingReady} from './binding-pair-profile.mjs';
import {sourceSaveRegistryAuthority,sourceSaveDestinationResolved} from './source-save.mjs';
const owned=new WeakMap(),copy=structuredClone;
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const basis=(s,d)=>({authority:s.authority,document:copy(d.document),source:copy(d.binding),inventory:Object.hasOwn(d,'ordinaryBindings')?copy(d.ordinaryBindings):null,owners:copy(s.resourceOwners),registrations:copy(s.writeRegistrations)});
function candidate(state,r,h){
 if(!pairRootValid(state,h?.limits)||sourceSaveRegistryAuthority(h.owner)!==state.authority||!catalog.gates.every(k=>h[k]===true)||!r||r.contract!==catalog.contract||!['initialize','bind-annotations'].includes(r.operation)||!exact(r,r.operation==='initialize'?['contract','operation','document','expectedSourceBinding']:['contract','operation','document','expectedSourceBinding','expectedPair','provider','resource','expectedRegistration'])||size(r)>h.limits.maxPlanBytes||!identity(r.document)||r.document.authority!==state.authority)return null;
 const ds=state.documents.filter(d=>equal(d.document,r.document)),d=ds[0];if(ds.length!==1||d.live!==true||!Object.hasOwn(d,'binding')||!natural(r.expectedSourceBinding)||(d.binding?.generation??0)!==r.expectedSourceBinding||!pairDocumentRegistrationsCurrent(state,d,h.limits))return null;
 let next,annotation=null;
 if(r.operation==='initialize'){
  if(Object.hasOwn(d,'ordinaryBindings')||h.annotationBindingAbsentAttested!==true||state.resourceOwners.some(row=>row.artifact==='annotations'&&equal(row.document,d.document)))return null;
  next=copy(state);next.documents.find(x=>equal(x.document,r.document)).ordinaryBindings={generation:1,annotations:null};
 }else{
  if(!d.ordinaryBindings||d.ordinaryBindings.generation!==r.expectedPair||!natural(r.expectedPair)||!identity(r.provider)||!identity(r.resource)||!natural(r.expectedRegistration)||!equal(h.ordinaryPairBasis,{document:d.document,source:d.binding,inventory:d.ordinaryBindings}))return null;
  const row=pairRegistration(state.writeRegistrations,r.provider,r.resource,r.expectedRegistration,h.limits);if(!row)return null;
  const old=d.ordinaryBindings.annotations;
  if(old){if(!equal(Object.fromEntries(Object.keys(context(row)).map(k=>[k,old[k]])),context(row))||!ordinarySourceBindingReady(state,d,d.binding,{...h,ordinaryBindingLimits:h.limits})||size(state)>h.limits.maxCandidateBytes)return null;return {status:'unchanged',state};}
  if(r.expectedPair===Number.MAX_SAFE_INTEGER||state.resourceOwners.some(owner=>key(owner)===key(row))||!sourceSaveDestinationResolved(h.owner,d.document,row,h))return null;
  annotation={generation:r.expectedPair+1,...context(row)};next=copy(state);const n=next.documents.find(x=>equal(x.document,r.document));n.ordinaryBindings={generation:r.expectedPair+1,annotations:annotation};next.resourceOwners.push({identityDomain:row.identityDomain,identityKey:row.identityKey,document:copy(d.document),bindingGeneration:annotation.generation,artifact:'annotations'});
  if(!ordinarySourceBindingReady(next,n,n.binding,{...h,ordinaryBindingLimits:h.limits,ordinaryPairBasis:{document:n.document,source:n.binding,inventory:n.ordinaryBindings}}))return null;
 }
 if(h.versionsAvailable!==true||state.publication===Number.MAX_SAFE_INTEGER)return null;next.publication++;
 return pairRootValid(next,h.limits)&&size(next)<=h.limits.maxCandidateBytes?{status:'applied',state:next}:null;
}
export function prepareOrdinaryBinding(state,request,host){const result=candidate(state,request,host);if(!result)return null;const d=state.documents.find(d=>equal(d.document,request.document)),value={request:copy(request),basis:basis(state,d),limits:copy(host.limits)};if(size(value)>host.limits.maxPlanBytes)return null;const plan=Object.freeze({kind:'ordinary-binding-plan/1'});owned.set(plan,{owner:host.owner,value:freeze(value)});return plan;}
export function publishOrdinaryBinding(state,plan,host){const p=owned.get(plan),fail=()=>({status:'not-published',state});if(!p||p.owner!==host?.owner||!equal(p.value.limits,host.limits)||!Array.isArray(state?.documents))return fail();const ds=state.documents.filter(d=>equal(d.document,p.value.request.document));if(ds.length!==1||!equal(p.value.basis,basis(state,ds[0])))return fail();return candidate(state,p.value.request,host)??fail();}
