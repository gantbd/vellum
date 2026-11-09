// Owned native serialization settings participant; no provider I/O.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {sourceSerializationProfileValid as profileValid,sourceSerializationDependencies as deps,sourceSerializationInputForm,scopedSourceSerializationInputForm,decideSerialization,serializationIdentity,serializationLabel} from './source-serialization.mjs';
const plans=new WeakMap(),size=v=>Buffer.byteLength(JSON.stringify(v));
const inputForm=sourceSerializationInputForm();
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
// Owned metadata-only conditional basis, distinct from source read contexts.
const basisPools=new WeakMap(),basisOwners=new WeakMap();
const natural=n=>Number.isSafeInteger(n)&&n>=0,version=n=>natural(n)&&n>0;
const basisFields=['serializationProfile','maxContexts','maxIssuedContexts','maxContextBytes','maxHeldBytes'];
const basisGates=['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','ownerControlAllowed','settingsWritable','serializationBasisAllowed'];
const permitted=(g,keys)=>keys.every(k=>g?.[k]===true);
const basisProfileValid=p=>exact(p,basisFields)&&profileValid(p.serializationProfile)&&basisFields.filter(k=>k!=='serializationProfile').every(k=>natural(p[k]));
const basisData=(value,p)=>copySettingsData(value,{...p.serializationProfile.argumentLimits,maxRequestBytes:p.maxContextBytes});
function basisContext(owner,c,p){return exact(c,['authority','engine','recipient','purpose','destination','profile','continuity'])&&Object.values(c).every(v=>serializationLabel(v,p.serializationProfile))&&c.authority===owner.context.authority&&c.engine===owner.context.engine;}
function metadataDocument(root,document,p){
 if(!serializationIdentity(document,p)||document.authority!==root.authority||!Array.isArray(root.documents))return null;
 const found=root.documents.filter(d=>equal(d.document,document));if(found.length!==1)return null;const d=found[0],s=d.serialization;
 return d.live!==false&&(!d.lifecycle||d.lifecycle.phase==='live')&&exact(s,['generation','profile','bom'])&&version(s.generation)&&s.profile==='strict-utf8-one-bom/1'&&typeof s.bom==='boolean'?d:null;
}
function basisFacts(owner,c,document,p,g){
 if(!equal(g?.sourceSerializationContext,c)||!equal(g.sourceSerializationProfile,p.serializationProfile)||g.sourceSerializationBasis!==owner.root)return null;
 const grant=basisData(g.serializationWriteContext,p)?.value;
 return exact(grant,['context','document','writeAllowed'])&&equal(grant.context,c)&&equal(grant.document,document)&&typeof grant.writeAllowed==='boolean'?grant:null;
}
export function openSerializationBasisPool(owner,domain,p,g){
 if(!owner||owner.faulted||basisPools.has(owner)||!basisProfileValid(p)||!permitted(g,['ownerControlAllowed','resourcesReady','outsidePublicationGuard','preparationReserved'])||!equal(g.sourceSerializationProfile,p.serializationProfile))return false;
 const profile=basisData(p,p),charge=profile?profile.bytes+p.maxContexts*p.maxContextBytes:null;
 if(charge===null||!natural(charge)||charge>p.maxHeldBytes||!natural(owner.charge)||charge>owner.limits.maxHeldBytes-owner.charge)return false;
 const handle=Object.freeze({kind:'source-serialization-basis-pool/1'});owner.charge+=charge;basisPools.set(owner,{domain,handle,profile:profile.value,charge,bases:new Map(),issued:0,closed:false});return true;
}
// First-party identity witness for the actual private pool. Not a facade result.
export function nativeSerializationBasisPoolHandle(owner,domain){const m=basisPools.get(owner);return m?.domain===domain?m.handle:null;}
export function captureSerializationBasis(owner,domain,document,c,g){
 const m=basisPools.get(owner);if(!m||m.closed||m.domain!==domain||owner.faulted||!basisContext(owner,c,m.profile)||!permitted(g,[...basisGates,'outsidePublicationGuard','preparationReserved']))return null;
 const grant=basisFacts(owner,c,document,m.profile,g),d=grant&&grant.writeAllowed&&metadataDocument(owner.root,document,m.profile.serializationProfile);if(!d)return null;
 const capture=basisData({context:c,document,expectedSerialization:d.serialization.generation},m.profile);
 if(!capture||m.bases.size>=m.profile.maxContexts||m.issued>=m.profile.maxIssuedContexts)return null;
 const token=Object.freeze({kind:'source-serialization-basis/1'});m.bases.set(token,{value:capture.value,stopped:false});m.issued++;basisOwners.set(token,{owner,domain});return token;
}
function retainedSerializationBasis(owner,domain,token,c){
 const m=basisPools.get(owner),s=m?.bases.get(token),binding=basisOwners.get(token);
 return m&&!m.closed&&m.domain===domain&&binding?.owner===owner&&binding.domain===domain&&s&&equal(s.value.context,c)?{m,s}:null;
}
function currentSerializationBasis(owner,domain,token,c,g){
 const found=retainedSerializationBasis(owner,domain,token,c);if(!found||found.s.stopped||owner.faulted)return null;
 const {m,s}=found,grant=basisFacts(owner,c,s.value.document,m.profile,g);if(!grant)return null;
 if(['authorized','scope','continuous','live','interpretationCurrent','settingsWritable','serializationBasisAllowed'].some(k=>g?.[k]===false)||grant.writeAllowed===false){s.stopped=true;return null;}
 if(!permitted(g,basisGates))return null;
 const d=metadataDocument(owner.root,s.value.document,m.profile.serializationProfile);if(!d||d.serialization.generation!==s.value.expectedSerialization){s.stopped=true;return null;}
 return found;
}
export function inspectSerializationBasis(owner,domain,token,c,g){return g?.outsidePublicationGuard===true&&currentSerializationBasis(owner,domain,token,c,g)?{status:'source-serialization-basis',availability:'current'}:{status:'unavailable'};}
export function resolveSerializationBasis(owner,domain,token,c,g){if(g?.serializationHostBasisAllowed!==true)return null;const found=currentSerializationBasis(owner,domain,token,c,g);return found?structuredClone({document:found.s.value.document,expectedSerialization:found.s.value.expectedSerialization}):null;}
export function stopSerializationBasis(owner,domain,token,c,g){const found=retainedSerializationBasis(owner,domain,token,c);if(!found||g?.ownerControlAllowed!==true)return false;found.s.stopped=true;return true;}
export function releaseSerializationBasis(owner,domain,token,c,g){const found=retainedSerializationBasis(owner,domain,token,c);if(!found||!permitted(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;found.m.bases.delete(token);basisOwners.delete(token);return true;}
export function retireSerializationBasisPool(owner,domain,g){const m=basisPools.get(owner);if(!m||m.closed||m.domain!==domain||m.bases.size||!permitted(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;m.closed=true;m.profile=null;owner.charge-=m.charge;return true;}
export function serializationBasisUsage(owner){const m=basisPools.get(owner);return m?{contexts:m.bases.size,issued:m.issued,closed:m.closed,declaredHeldBytes:m.closed?0:m.charge}:null;}
export function prepareSourceSerialization(owner,domain,ticket,reference,before,envelope,c,p,h,existing,scoped=null){
 if(!profileValid(p)||owner.context.authority!==c.authority||owner.context.engine!==c.engine||owner.root!==before||h.outsidePublicationGuard!==true||h.preparationReserved!==true)return null;
 const copied=copySettingsData(envelope,p.argumentLimits),context=copySettingsData(c,p.argumentLimits);if(!copied||!context)return null;const e=copied.value;
 if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route)||!exact(e.request,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:r,...tuple}=e.request,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
 if(decoded?.descriptor.id!==inputForm.family||decoded.variant!==inputForm.variant||r?.invocation!==reference)return null;
 const d=decideSerialization(before,r,c,p,h),extra=d&&deps(r,h,p);if(!d||!extra||size(d.state)>owner.limits.maxRootBytes)return null;
 const data=copySettingsData({request:r,context:context.value,profile:p,required:h.requiredDependencies,dependencies:extra,...(scoped?{scoped:{request:scoped.request,basisCount:1}}:{})},{...p.argumentLimits,maxRequestBytes:Math.min(p.maxPlanBytes,owner.limits.maxIntentBytes)});if(!data||size({kind:'settings',plan:data.value})>owner.limits.maxIntentBytes)return null;
 if(existing){const old=plans.get(existing);return old&&old.owner===owner&&old.domain===domain&&old.ticket===ticket&&Boolean(old.scoped)===Boolean(scoped)&&(!scoped||old.scoped.basis===scoped.basis)&&equal(old.value,data.value)?existing:null;}
 const plan=Object.freeze({kind:'native-source-serialization-plan/1'});plans.set(plan,{owner,domain,ticket,value:freeze(data.value),...(scoped?{scoped:{basis:scoped.basis}}:{})});return plan;
}
export function sourceSerializationIntent(plan,max,owner,domain,ticket){const p=plans.get(plan);return p&&p.owner===owner&&p.domain===domain&&p.ticket===ticket&&size(p.value)<=max?{invocation:p.value.request.invocation,value:p.value}:null;}
export function publishSourceSerialization(before,plan,h,owner,domain,ticket){
 const p=plans.get(plan);if(!p)return null;const no={status:'not-published',state:before};
 if(p.owner!==owner||p.domain!==domain||p.ticket!==ticket||!equal(p.value.required,h.requiredDependencies))return no;
 if(p.scoped){const current=lowerScopedSerialization(owner,domain,p.scoped.basis,p.value.scoped.request,p.value.context,p.value.profile,h);if(!current||!equal(current,p.value.request))return no;}
 const d=decideSerialization(before,p.value.request,p.value.context,p.value.profile,h),extra=d&&deps(p.value.request,h,p.value.profile);return d&&extra&&equal(extra,p.value.dependencies)?d:no;
}
export function forgetSourceSerialization(plan,owner,domain,ticket){const p=plans.get(plan);return !!p&&p.owner===owner&&p.domain===domain&&p.ticket===ticket&&plans.delete(plan);}

const scopedInput=scopedSourceSerializationInputForm();
function normalizeScopedSerialization(envelope,c,p){
 if(!profileValid(p))return null;const data=copySettingsData(envelope,p.argumentLimits),context=copySettingsData(c,p.argumentLimits);if(!data||!context)return null;
 const e=data.value;if(!exact(context.value,['authority','engine','recipient','purpose','destination','profile','continuity'])||!Object.values(context.value).every(v=>serializationLabel(v,p))||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route)||!exact(e.request,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:r,...tuple}=e.request,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
 if(decoded?.descriptor.id!==scopedInput.family||decoded.variant!==scopedInput.variant||!exact(r,scopedInput.fields)||r.contract!==scopedInput.contract||r.operation!==scopedInput.operation||!serializationLabel(r.invocation,p)||r.profile!==context.value.profile)return null;
 return {request:r,context:context.value};
}
function lowerScopedSerialization(owner,domain,basis,r,c,p,h){
 if(h?.serializationHostBasisAllowed!==true)return null;const held=retainedSerializationBasis(owner,domain,basis,c);if(!held||held.s.stopped||!equal(held.m.profile.serializationProfile,p))return null;
 const expected={document:held.s.value.document,expectedSerialization:held.s.value.expectedSerialization},request={...r,contract:inputForm.contract,...expected};
 // Bounded complete semantic preflight precedes the stateful condition observer.
 if(!decideSerialization(owner.root,request,c,p,h)||!deps(request,h,p))return null;
 const current=resolveSerializationBasis(owner,domain,basis,c,h);return current&&equal(current,expected)?request:null;
}
export function prepareScopedSourceSerialization(owner,domain,ticket,reference,before,envelope,basis,c,p,h,existing){
 if(h?.outsidePublicationGuard!==true||h.preparationReserved!==true)return null;
 const input=normalizeScopedSerialization(envelope,c,p);if(!input||input.request.invocation!==reference)return null;
 const request=lowerScopedSerialization(owner,domain,basis,input.request,input.context,p,h);if(!request)return null;
 return prepareSourceSerialization(owner,domain,ticket,reference,before,{route:'typed',request:{family:15,variant:'semantic-settings',version:1,arguments:request}},input.context,p,h,existing,{request:input.request,basis});
}
