// Private current-cut preparation only, not installed member/query authority.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {prepareVersionSet} from './version-set-contexts.mjs';
import {captureSourceProfile} from './source-products.mjs';
const pools=new WeakMap(),fields=['limits','maxPrepared','maxIssued','maxRootBytes','maxCaptureBytes','maxContextBytes','maxRequestBytes','maxSelectorBytes','maxInputNodes','maxInputDepth','maxHeldBytes'],limitFields=['groups','participants','inputRoots','retainedCuts','contexts','units','selectors','holds','issuance','sourceBytes','source','annotation'],scopeFields=['recipient','purpose','destination','policy','continuity','profile'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,yes=(g,ks)=>ks.every(k=>g?.[k]===true),exact=(v,ks)=>{if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(d);return keys.length===ks.length&&ks.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable);},copy=(v,p,n)=>copySettingsData(v,{maxRequestBytes:n,maxInputNodes:p.maxInputNodes,maxInputDepth:p.maxInputDepth}),token=kind=>Object.freeze({kind});
function pool(owner,domain,handle){const m=pools.get(owner);return m&&m.domain===domain&&m.handle===handle&&!m.closed?m:null;}
export function openNativeVersionSetCapture(owner,domain,p,g){
 if(pools.has(owner)||owner.faulted||!exact(p,fields)||!fields.filter(k=>k!=='limits').every(k=>natural(p[k]))||!yes(g,['ownerControlAllowed','resourcesReady','preparationReserved']))return null;
 const captured=copy(p,p,p.maxRequestBytes),l=captured?.value.limits;if(!captured||!exact(l,limitFields)||!limitFields.filter(k=>!['source','annotation'].includes(k)).every(k=>natural(l[k]))||!captureSourceProfile(l.source)||!exact(l.annotation,['maxInputRows','maxCaptureBytes','maxRowBytes','maxPageRows'])||!Object.values(l.annotation).every(natural))return null;
 // Measuring the current root can precede refusal at the smaller local input
 // ceiling. Reserve against the actual parent's root bound, not that threshold.
 const rootWorkspace=owner.limits.maxRootBytes;if(!natural(rootWorkspace))return null;
 const charge=captured.bytes+rootWorkspace+2*p.maxRequestBytes+3*p.maxCaptureBytes+2*p.maxContextBytes+p.maxPrepared*(p.maxCaptureBytes+p.maxContextBytes+p.maxSelectorBytes);
 if(!natural(charge)||charge>p.maxHeldBytes||!natural(owner.charge)||charge>owner.limits.maxHeldBytes-owner.charge)return null;
 const handle=token('native-version-set-preparation-pool/1');pools.set(owner,{domain,handle,profile:captured.value,charge,cuts:new Map(),issued:0,closed:false});owner.charge+=charge;return handle;
}
export function prepareNativeVersionSetCapture(owner,domain,handle,request,host,g){
 const m=pool(owner,domain,handle);if(!m||owner.faulted)return null;const p=m.profile,v=copy({request,host,facts:g},p,p.maxRequestBytes)?.value;if(!v||!yes(v.facts,['outsidePublicationGuard','resourcesReady','preparationReserved']))return null;const r=v.request,h=v.host;
 if(!exact(r,['contract','operation','authority','participants','selection','profile'])||r.contract!=='read-version-set/1'||r.operation!=='snapshot-handoff'||r.authority!==owner.context.authority||!exact(r.selection,['kind'])||r.selection.kind!=='capture-current'||!Array.isArray(r.participants)||!r.participants.length||r.participants.length>p.limits.participants||!h||!Array.isArray(h.members)||h.members.length!==r.participants.length||!scopeFields.every(k=>typeof h[k]==='string'&&h[k].length)||h.profile!==r.profile||!yes(h,['granted','live','continuous','transferReady','captureComplete','preparationReserved'])||m.cuts.size>=p.maxPrepared||m.issued>=p.maxIssued)return null;
 // Capture the actual published root once. No caller root/candidate or per-member
 // recapture function participates in the loop below.
 if(h.members.some(f=>!f||typeof f!=='object'||Array.isArray(f)||!exact(f.document,['authority','id','incarnation'])))return null;
 const published=owner.root;if(Buffer.byteLength(JSON.stringify(published))>p.maxRootBytes||published.documents.length>p.limits.inputRoots)return null;
 const roots=[],nativeFacets=[],keys=new Set();
 for(const participant of r.participants){
  if(!exact(participant,['document','data'])||!exact(participant.document,['authority','id','incarnation'])||participant.document.authority!==owner.context.authority||!natural(participant.document.incarnation)||participant.document.incarnation<1||!['authority','id'].every(k=>typeof participant.document[k]==='string'&&participant.document[k].length)||!Array.isArray(participant.data)||!participant.data.length||participant.data.length>2||new Set(participant.data).size!==participant.data.length||participant.data.some(k=>!['source','annotations'].includes(k)))return null;
  const key=JSON.stringify([participant.document.authority,participant.document.id]);if(keys.has(key))return null;keys.add(key);
  const ds=published.documents.filter(d=>equal(d.document,participant.document)),hs=h.members.filter(f=>equal(f.document,participant.document));if(ds.length!==1||ds[0].live!==true||hs.length!==1)return null;const d=ds[0],facts=hs[0],source=participant.data.includes('source'),annotations=participant.data.includes('annotations');
  if(!scopeFields.every(k=>equal(facts[k],h[k]))||!yes(facts,['granted','live','continuous','transferReady','captureComplete','preparationReserved'])||source&&facts.sourceReadable!==true||annotations&&!yes(facts,['annotationReadable','visibilityComplete','projectionOrderAttested']))return null;
  const records=[],bindings=[];
  if(annotations){const rows=facts.rows;if(!Array.isArray(d.records)||d.records.length>p.limits.annotation.maxInputRows||!Array.isArray(rows)||rows.length!==d.records.length)return null;const granted=new Map();
   for(const row of rows){if(!exact(row,['id','incarnation','visible','body','target'])||typeof row.id!=='string'||!row.id||!natural(row.incarnation)||row.incarnation<1||!['visible','body','target'].every(k=>typeof row[k]==='boolean')||!row.visible&&(row.body||row.target))return null;const key=JSON.stringify([row.id,row.incarnation]);if(granted.has(key))return null;granted.set(key,row);}
   const seen=new Set(),width=String(d.records.length).length;for(const [index,row]of d.records.entries()){const key=JSON.stringify([row.id,row.incarnation]),grant=granted.get(key);if(!grant||seen.has(key))return null;seen.add(key);const reference=String(index).padStart(width,'0');records.push({reference,visible:grant.visible,bodyVisible:grant.body,targetVisible:grant.target,...(grant.body?{body:row.body,contentVersion:row.bodyVersion}:{}),...(grant.target?{location:{placement:row.placement,targetGeneration:row.targetGeneration}}:{})});if(grant.visible)bindings.push({reference,identity:{id:row.id,incarnation:row.incarnation},body:grant.body,target:grant.target});}
  }
  if(annotations)nativeFacets.push({document:d.document,records:bindings});
  roots.push({document:d.document,...(source?{source:d.source,sourceRevision:d.revision}:{}),records});
 }
 const normalized=copy(roots,p,p.maxCaptureBytes)?.value;if(!normalized)return null;
 const candidate=prepareVersionSet({id:owner.context.authority,current:{identity:token('native-version-set-cut/1'),roots:normalized},retained:[]},r,h,p.limits),value=candidate&&copy({...candidate,nativeFacets},p,p.maxCaptureBytes)?.value,metadata=value&&copy({scope:value.scope,invalidated:false},p,p.maxContextBytes)?.value;if(!value||!metadata)return null;
 const cut=token('native-version-set-prepared/1');if(!copy(cut,p,p.maxSelectorBytes))return null;m.cuts.set(cut,{...metadata,value});m.issued++;return cut;
}
export function validateNativeVersionSetCapture(owner,domain,handle,cut,host,g){
 const m=pool(owner,domain,handle);return m?validateNativeVersionSetHeld(owner,m.cuts.get(cut),m.profile,host,g):false;
}
// Fixed first-party currentness port for private prepared/installed owner rows.
export function validateNativeVersionSetHeld(owner,held,p,host,g){
 if(!held||held.invalidated)return false;const v=copy({host,facts:g},p,p.maxRequestBytes)?.value;if(!v||v.facts?.outsidePublicationGuard!==true)return false;const h=v.host;
 if(!h||!scopeFields.every(k=>equal(h[k],held.scope[k]))||!Array.isArray(h.members)||h.members.length!==held.value.members.length||h.members.some(f=>!f||typeof f!=='object'||Array.isArray(f)||!exact(f.document,['authority','id','incarnation'])))return false;
 const paired=[];for(const member of held.value.members){const hs=h.members.filter(f=>equal(f.document,member.document));if(hs.length!==1||!Object.hasOwn(hs[0],'stamp')||!scopeFields.every(k=>equal(hs[0][k],h[k])))return false;paired.push({member,facts:hs[0]});}
 const required=['granted','live','continuous','notStopped','retained'],published=owner.root;
 const lost=owner.faulted||required.some(k=>h[k]===false)||paired.some(({member,facts})=>{const ds=published.documents.filter(d=>equal(d.document,member.document)),classes=member.candidate.data,keys=[...required,...(classes.includes('source')?['sourceReadable']:[]),...(classes.includes('annotations')?['annotationReadable','projectionCurrent']:[])];return ds.length!==1||ds[0].live!==true||!equal(facts.stamp,member.stamp)||keys.some(k=>facts[k]===false);});
 if(lost){held.invalidated=true;return false;}
 return yes(v.facts,['resourcesReady'])&&yes(h,[...required,'replyReady','transferReady','retentionReady'])&&paired.every(({member,facts})=>yes(facts,[...required,...(member.candidate.data.includes('source')?['sourceReadable']:[]),...(member.candidate.data.includes('annotations')?['annotationReadable','projectionCurrent']:[])]));
}
// Private once-only handoff port. Installation must stage its complete owner
// before this call; returning the already allocated row adds no transfer frame.
export function consumeNativeVersionSetCapture(owner,domain,handle,cut,host,g){if(!validateNativeVersionSetCapture(owner,domain,handle,cut,host,g))return null;const m=pool(owner,domain,handle),held=m.cuts.get(cut);m.cuts.delete(cut);return held;}
export function inspectNativeVersionSetCapture(owner,domain,handle,cut,g){const m=pool(owner,domain,handle),held=m?.cuts.get(cut);return held&&g?.ownerControlAllowed===true?structuredClone(held.value):null;}
// Canonical request binding only. Current host facts and one-time installation
// remain separate; this port never returns the retained projection.
export function nativeVersionSetCutMatchesRequest(owner,domain,handle,cut,request){
 const m=pool(owner,domain,handle),held=m?.cuts.get(cut);if(!held||!exact(request,['contract','operation','authority','participants','selection','profile']))return false;const r=copy(request,m.profile,m.profile.maxRequestBytes)?.value;
 return !!r&&r.contract==='read-version-set/1'&&r.operation==='snapshot-handoff'&&r.authority===owner.context.authority&&r.profile===held.scope.profile&&exact(r.selection,['kind'])&&r.selection.kind==='capture-current'&&Array.isArray(r.participants)&&r.participants.length===held.value.members.length&&r.participants.every((p,i)=>exact(p,['document','data'])&&equal(p.document,held.value.members[i].document)&&equal(p.data,held.value.members[i].candidate.data));
}
export function discardNativeVersionSetCapture(owner,domain,handle,cut,g){const m=pool(owner,domain,handle);return !!m&&yes(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])&&m.cuts.delete(cut);}
export function retireNativeVersionSetCapture(owner,domain,handle,g){const m=pool(owner,domain,handle);if(!m||m.cuts.size||!yes(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;m.closed=true;m.profile=null;owner.charge-=m.charge;return true;}
export function nativeVersionSetCaptureUsage(owner){const m=pools.get(owner);return m?{closed:m.closed,prepared:m.cuts.size,invalidated:[...m.cuts.values()].filter(c=>c.invalidated).length,issued:m.issued,declaredHeldBytes:m.closed?0:m.charge}:null;}
