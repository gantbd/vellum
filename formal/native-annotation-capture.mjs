// Internal prepared F02 projections only, not installed contexts or public queries.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {captureSourceProfile} from './source-products.mjs';
const pools=new WeakMap(),fields=['sourceProfile','maxPrepared','maxIssued','maxRows','maxRootBytes','maxCaptureBytes','maxContextBytes','maxRequestBytes','maxSelectorBytes','maxInputNodes','maxInputDepth','maxHeldBytes'];
const scopeFields=['document','recipient','purpose','destination','policy','continuity','profile'],requestFields=['contract','operation','document','selection','data','profile'];
const gates=['authorized','scope','continuous','live','notStopped','annotationReadable','visibilityComplete','projectionOrderAttested','outsidePublicationGuard','resourcesReady','preparationReserved'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,yes=(g,ks)=>ks.every(k=>g?.[k]===true);
const exact=(v,ks)=>{if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(d);return keys.length===ks.length&&ks.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable);};
const copy=(v,p,n)=>copySettingsData(v,{maxRequestBytes:n,maxInputNodes:p.maxInputNodes,maxInputDepth:p.maxInputDepth});
const token=kind=>Object.freeze({kind}),size=v=>Buffer.byteLength(JSON.stringify(v));
function pool(owner,domain,handle){const m=pools.get(owner);return m&&m.domain===domain&&m.handle===handle&&!m.closed?m:null;}
export function openNativeAnnotationCapture(owner,domain,p,g){
 if(pools.has(owner)||owner.faulted||!exact(p,fields)||!fields.filter(k=>k!=='sourceProfile').every(k=>natural(p[k]))||!yes(g,['ownerControlAllowed','resourcesReady','preparationReserved']))return null;
 const captured=copy(p,p,p.maxRequestBytes);if(!captured||!captureSourceProfile(captured.value.sourceProfile))return null;
 const charge=captured.bytes+2*p.maxRequestBytes+2*p.maxCaptureBytes+2*p.maxContextBytes+p.maxPrepared*(2*p.maxCaptureBytes+p.maxContextBytes+p.maxSelectorBytes);
 if(!natural(charge)||charge>p.maxHeldBytes||!natural(owner.charge)||charge>owner.limits.maxHeldBytes-owner.charge)return null;
 const handle=token('native-annotation-preparation-pool/1');pools.set(owner,{domain,handle,profile:captured.value,charge,cuts:new Map(),issued:0,closed:false});owner.charge+=charge;return handle;
}
export function prepareNativeAnnotationCapture(owner,domain,handle,request,scope,grants,g){
 const m=pool(owner,domain,handle);if(!m||owner.faulted||!yes(g,gates))return null;const p=m.profile,v=copy({request,scope,grants},p,p.maxRequestBytes)?.value;if(!v)return null;
 const r=v.request,s=v.scope,rows=v.grants;
 if(!exact(r,requestFields)||r.contract!=='read-context/1'||r.operation!=='snapshot-handoff'||!exact(r.selection,['kind'])||r.selection.kind!=='capture-current'||!Array.isArray(r.data)||!r.data.includes('annotations')||r.data.length>2||new Set(r.data).size!==r.data.length||r.data.some(k=>!['source','annotations'].includes(k))||!exact(s,scopeFields)||!exact(s.document,['authority','id','incarnation'])||s.document.authority!==owner.context.authority||!natural(s.document.incarnation)||s.document.incarnation<1||!['authority','id'].every(k=>typeof s.document[k]==='string'&&s.document[k].length)||!scopeFields.filter(k=>k!=='document').every(k=>typeof s[k]==='string'&&s[k].length)||!equal(r.document,s.document)||r.profile!==s.profile||!copy(s,p,p.maxContextBytes)||r.data.includes('source')&&g?.sourceReadable!==true||!Array.isArray(rows)||rows.length>p.maxRows||m.cuts.size>=p.maxPrepared||m.issued>=p.maxIssued)return null;
 if(size(owner.root)>p.maxRootBytes)return null;
 const ds=owner.root.documents.filter(d=>equal(d.document,s.document));if(ds.length!==1||ds[0].live!==true||!Array.isArray(ds[0].records)||ds[0].records.length>p.maxRows)return null;const d=ds[0],keys=new Set();
 for(const row of rows){if(!exact(row,['id','incarnation','visible','body','target'])||typeof row.id!=='string'||!row.id||!natural(row.incarnation)||row.incarnation<1||!['visible','body','target'].every(k=>typeof row[k]==='boolean')||!row.visible&&(row.body||row.target))return null;const key=JSON.stringify([row.id,row.incarnation]);if(keys.has(key))return null;keys.add(key);}
 if(rows.length!==d.records.length||d.records.some(row=>!keys.has(JSON.stringify([row.id,row.incarnation])))||new Set(d.records.map(row=>JSON.stringify([row.id,row.incarnation]))).size!==d.records.length)return null;
 // Grant rows choose facets, never bytes, versions or placement. All values
 // below come from the one actual root; internal identities are not public F03 selectors.
 const annotations=[];for(const row of d.records){const grant=rows.find(x=>x.id===row.id&&x.incarnation===row.incarnation);if(!grant.visible)continue;annotations.push({identity:{id:row.id,incarnation:row.incarnation},...(grant.body?{content:{body:row.body,version:row.bodyVersion}}:{}),...(grant.target?{location:{placement:row.placement,targetGeneration:row.targetGeneration}}:{})});}
 if(r.data.includes('source')&&(d.source.length>p.sourceProfile.maxRootBytes||d.source.length>p.sourceProfile.maxInputBytes))return null;
 const value=copy({document:s.document,data:[...r.data].sort(),annotations,...(r.data.includes('source')?{source:{ref:{authority:d.document.authority,document:d.document.id,incarnation:d.document.incarnation,revision:d.revision},bytes:d.source,profile:p.sourceProfile}}:{})},p,p.maxCaptureBytes)?.value;if(!value)return null;
 const cut=token('native-annotation-prepared-cut/1');if(!copy(cut,p,p.maxSelectorBytes))return null;
 m.cuts.set(cut,{scope:s,value,invalidated:false});m.issued++;return cut;
}
// Selected outside-guard handoff preflight, not installation or recipient release.
// projectionCurrent is accepted current authority for the entire captured facet
// domain; it is not inferred from source permission or a matching scope label.
export function validateNativeAnnotationCapture(owner,domain,handle,cut,c,g){
 const m=pool(owner,domain,handle),held=m?.cuts.get(cut);if(!held||held.invalidated||g?.outsidePublicationGuard!==true)return false;
 const scope=copy(c,m.profile,m.profile.maxContextBytes)?.value;if(!scope||!exact(scope,scopeFields)||!equal(scope,held.scope))return false;
 const required=['authorized','scope','continuous','live','notStopped','annotationReadable','projectionCurrent','retained'],source=held.value.data.includes('source');
 const ds=owner.root.documents.filter(d=>equal(d.document,held.scope.document));
 if(owner.faulted||ds.length!==1||ds[0].live!==true||required.some(k=>g?.[k]===false)||source&&g?.sourceReadable===false){held.invalidated=true;return false;}
 return yes(g,[...required,'resourcesReady','replyReady','transferReady'])&&(!source||g?.sourceReadable===true);
}
// Trusted owner inspection for integration/evidence, never recipient disclosure.
// Request binding only: no grant/currentness observation and no projection copy.
export function nativeAnnotationCutMatchesRequest(owner,domain,handle,cut,request){
 const m=pool(owner,domain,handle),held=m?.cuts.get(cut);if(!held||!exact(request,requestFields))return false;const r=copy(request,m.profile,m.profile.maxRequestBytes)?.value;
 return !!r&&r.contract==='read-context/1'&&r.operation==='snapshot-handoff'&&exact(r.selection,['kind'])&&r.selection.kind==='capture-current'&&equal(r.document,held.value.document)&&r.profile===held.scope.profile&&equal(r.data,held.value.data);
}
export function inspectNativeAnnotationCapture(owner,domain,handle,cut,g){const m=pool(owner,domain,handle),held=m?.cuts.get(cut);return held&&g?.ownerControlAllowed===true?structuredClone(held.value):null;}
// Internal one-time transfer; the caller must reserve/install its bounded owner
// in the same serialized decision. This does not refund the preparation pool.
export function consumeNativeAnnotationCapture(owner,domain,handle,cut,c,g){if(!validateNativeAnnotationCapture(owner,domain,handle,cut,c,g))return null;const m=pool(owner,domain,handle),held=m.cuts.get(cut);m.cuts.delete(cut);return {scope:held.scope,value:held.value};}
export function discardNativeAnnotationCapture(owner,domain,handle,cut,g){const m=pool(owner,domain,handle);return !!m&&yes(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])&&m.cuts.delete(cut);}
export function retireNativeAnnotationCapture(owner,domain,handle,g){const m=pool(owner,domain,handle);if(!m||m.cuts.size||!yes(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;m.closed=true;m.profile=null;owner.charge-=m.charge;return true;}
export function nativeAnnotationCaptureUsage(owner){const m=pools.get(owner);return m?{closed:m.closed,prepared:m.cuts.size,invalidated:[...m.cuts.values()].filter(c=>c.invalidated).length,issued:m.issued,declaredHeldBytes:m.closed?0:m.charge}:null;}
