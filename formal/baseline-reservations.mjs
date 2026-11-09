// Private ledger attached by the actual invocation owner; no public grant/token API.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {prepareAnnotationBaselineEnvelope,annotationBaselineWithinEnvelope} from './annotation-baseline-capacity.mjs';
import {ordinarySourceBindingReady} from './binding-pair-profile.mjs';
import {readAnnotationWriteClaim,readAnnotationWriteClaimControl,readPreparedAnnotationWriteClaim,dischargeIneligibleAnnotationWriteClaim,bindAnnotationWriteClaim,dischargeAnnotationWriteClaim,annotationInstallationGates,readAnnotationBaselineInstallation,completeAnnotationBaselineInstallation,annotationWriteClaimLive} from './annotation-write-claims.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./baseline-reservations.json',import.meta.url))),pools=new WeakMap(),stages=new WeakMap(),copy=structuredClone;
const natural=n=>Number.isSafeInteger(n)&&n>=0,size=v=>Buffer.byteLength(JSON.stringify(v));
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const allowed=(g,ks)=>ks.every(k=>g?.[k]===true),unavailable=()=>({status:'unavailable'});
const key=d=>JSON.stringify([d.authority,d.id,d.incarnation]);
const admissions=new WeakMap();
function rootFloor(o,root,rows){
 if(root?.authority!==o.context.authority||!natural(root.publication)||!Array.isArray(root.documents)||size(root)>o.maxRootBytes||rows.length>Number.MAX_SAFE_INTEGER-root.publication)return null;
 if(rows.length===0)return size(root);
 const next=copy(root),groups=new Map();for(const r of rows){const k=key(r.document);if(!groups.has(k))groups.set(k,[]);groups.get(k).push(r);}
 let growth=0;for(const rs of groups.values()){
  const ds=next.documents.filter(d=>equal(d.document,rs[0].document)),d=ds[0];if(ds.length!==1||d.live!==true||rs.some(r=>!equal(d.ordinaryBindings?.annotations,r.binding)))return null;
  const slot=Math.max(size(Object.hasOwn(d,'annotationBaseline')?d.annotationBaseline:null),...rs.map(r=>r.envelope.baselineBytes));growth+=slot-size(null);if(!natural(growth))return null;d.annotationBaseline=null;
 }
 next.publication=Number.MAX_SAFE_INTEGER;const bytes=size(next)+growth;return natural(bytes)&&bytes<=o.maxRootBytes?bytes:null;
}
export function openBaselineReservationPool(context,root,maxRootBytes,limits,g){
 if(!allowed(g,catalog.openGates)||!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||!natural(maxRootBytes)||root?.authority!==context?.authority)return null;
 const baseBytes=size({context,limits,maxRootBytes,issued:Number.MAX_SAFE_INTEGER,stopped:true,heldBytes:Number.MAX_SAFE_INTEGER})+limits.maxReservations*size({reservation:{kind:'annotation-baseline-reservation/1'},charge:Number.MAX_SAFE_INTEGER})+2*limits.maxIssued*size({claim:{kind:'annotation-write-reservation-claim/1'}});
 if(!natural(baseBytes)||baseBytes>limits.maxHeldBytes)return null;const o={context:copy(context),limits:copy(limits),maxRootBytes,baseBytes,heldBytes:baseBytes,issued:0,stopped:false,holds:new Map(),producerPins:new Set()};if(rootFloor(o,root,[])===null)return null;
 const pool=Object.freeze({kind:'annotation-baseline-reservation-pool/1'});pools.set(pool,o);return pool;
}
function admitBaselineCapacity(pool,root,document,envelope,g,claim=null,domain=null){
 const o=pools.get(pool);if(!o||o.stopped||!allowed(g,catalog.reserveGates)||o.holds.size>=o.limits.maxReservations||o.issued>=o.limits.maxIssued||!document||root?.authority!==o.context.authority||!Array.isArray(root.documents))return unavailable();
 const ls={maxRootBytes:o.maxRootBytes,maxBaselineBytes:o.limits.maxBaselineBytes,maxTagBytes:o.limits.maxTagBytes},actual=prepareAnnotationBaselineEnvelope(envelope?.template,ls);if(!actual||!equal(actual,envelope))return unavailable();
 const ds=root.documents.filter(d=>equal(d.document,document)),d=ds[0],b=d?.ordinaryBindings?.annotations,t=envelope.template;
 if(ds.length!==1||d.live!==true||!b||!ordinarySourceBindingReady(root,d,d.binding,g)||t.annotationBindingGeneration!==b.generation||!equal(t.provider,b.provider)||t.sourceRevision!==d.revision||t.annotationSet!==d.annotationSet||t.context.authority!==o.context.authority||t.context.engine!==o.context.engine)return unavailable();
 const value={document:copy(document),binding:copy(b),envelope:copy(envelope),...(claim?{claim}:{})},charge=o.limits.maxRecordBytes+envelope.baselineBytes;
 if(size(value)>o.limits.maxRecordBytes||!natural(charge)||charge>o.limits.maxHeldBytes-o.heldBytes||rootFloor(o,root,[...o.holds.values(),value])===null)return unavailable();
 const reservation=Object.freeze({kind:'annotation-baseline-reservation/1'});o.holds.set(reservation,{...value,charge});if(claim&&!bindAnnotationWriteClaim(claim,domain,pool,reservation)){o.holds.delete(reservation);return unavailable();}o.heldBytes+=charge;o.issued++;return {status:'reserved',reservation};
}
export function reserveBaselineCapacity(pool,root,document,envelope,g){return admitBaselineCapacity(pool,root,document,envelope,g);}
export function reserveBaselineWriteCapacity(pool,root,domain,claim,g,allowNew=true){
 const o=pools.get(pool);if(!o||!allowed(g,catalog.reserveGates))return unavailable();const s=readAnnotationWriteClaim(claim);if(!s||s.domain!==domain||s.released)return unavailable();
 if(s.link){return s.link.pool===pool&&s.link.domain===domain&&o.holds.get(s.link.reservation)?.claim===claim?{status:'reserved',reservation:s.link.reservation}:unavailable();}
 // Fixed first-party lookup mode cannot create a promise if caller facts reenter
 // and discharge the formerly retained hold before this lookup completes.
 if(allowNew!==true)return unavailable();
 if(s.phase!=='queued'||s.stopped||s.maxTagBytes!==o.limits.maxTagBytes||!Array.isArray(root?.documents)||!Array.isArray(root.writeRegistrations))return unavailable();const ds=root.documents.filter(d=>equal(d.document,s.document)),rows=root.writeRegistrations.filter(r=>equal(r.provider,s.registration.provider)&&equal(r.resource,s.registration.resource));
 if(ds.length!==1||!equal(ds[0].ordinaryBindings?.annotations,s.binding)||rows.length!==1||!equal(rows[0],s.registration))return unavailable();const envelope=prepareAnnotationBaselineEnvelope(s.draft,{maxRootBytes:o.maxRootBytes,maxBaselineBytes:o.limits.maxBaselineBytes,maxTagBytes:o.limits.maxTagBytes});if(!envelope)return unavailable();return admitBaselineCapacity(pool,root,s.document,envelope,g,claim,domain);
}
export function baselineReservationsAllow(pool,root){const o=pools.get(pool);return !!o&&rootFloor(o,root,[...o.holds.values()])!==null;}
export function prepareOrdinaryBaselineCapacity(pool,root,domain,claim,g){
 const o=pools.get(pool),s=readPreparedAnnotationWriteClaim(claim);if(!o||!s||s.domain!==domain||s.maxTagBytes!==o.limits.maxTagBytes||!allowed(g,catalog.reserveGates))return null;
 const ds=root?.documents?.filter(d=>equal(d.document,s.document)),rows=root?.writeRegistrations?.filter(r=>equal(r.provider,s.registration.provider)&&equal(r.resource,s.registration.resource));if(ds?.length!==1||!equal(ds[0].ordinaryBindings?.annotations,s.binding)||rows?.length!==1||!equal(rows[0],s.registration))return null;
 const envelope=prepareAnnotationBaselineEnvelope(s.draft,{maxRootBytes:o.maxRootBytes,maxBaselineBytes:o.limits.maxBaselineBytes,maxTagBytes:o.limits.maxTagBytes});if(!envelope)return null;
 // Run the common capacity law against a private ledger copy, never the live pool.
 const shadow=Object.freeze({kind:'annotation-baseline-reservation-pool/1'}),trial={...o,holds:new Map(o.holds)};pools.set(shadow,trial);const r=admitBaselineCapacity(shadow,root,s.document,envelope,g);pools.delete(shadow);if(r.status!=='reserved')return null;
 const row={...trial.holds.get(r.reservation),claim};
 // Match the common charged record, excluding only its separately charged number.
 const {charge,...value}=row;if(size(value)>o.limits.maxRecordBytes)return null;
 const token=Object.freeze({kind:'ordinary-annotation-capacity-stage/1'});admissions.set(token,{pool,root,domain,claim,reservation:r.reservation,row,issued:o.issued,heldBytes:o.heldBytes,count:o.holds.size});return token;
}
export function validateOrdinaryBaselineCapacity(pool,root,domain,claim,token,g){
 const o=pools.get(pool),s=admissions.get(token),c=readPreparedAnnotationWriteClaim(claim)??readAnnotationWriteClaim(claim);return !!o&&!!s&&!!c&&s.pool===pool&&s.root===root&&s.domain===domain&&s.claim===claim&&c.domain===domain&&c.phase==='queued'&&!c.stopped&&!c.link&&!c.released&&!o.stopped&&o.issued===s.issued&&o.heldBytes===s.heldBytes&&o.holds.size===s.count&&allowed(g,catalog.reserveGates)&&rootFloor(o,root,[...o.holds.values(),s.row])!==null;
}
export function commitOrdinaryBaselineCapacity(pool,root,domain,claim,token,g){
 const o=pools.get(pool),s=admissions.get(token);if(!validateOrdinaryBaselineCapacity(pool,root,domain,claim,token,g)||!bindAnnotationWriteClaim(claim,domain,pool,s.reservation))return null;o.holds.set(s.reservation,s.row);o.heldBytes+=s.row.charge;o.issued++;admissions.delete(token);return s.reservation;
}
export function discardOrdinaryBaselineCapacity(token){return admissions.delete(token);}
export function stageAnnotationBaselineInstallation(pool,domain,before,claim,token,g){
 const o=pools.get(pool);if(!o||!allowed(g,annotationInstallationGates))return null;const c=readAnnotationBaselineInstallation(token,claim,domain,pool);if(!c||c.before!==before)return null;
 const hold=c.mode==='refresh'?null:o.holds.get(c.reservation);if(c.mode!=='refresh'&&(!hold||hold.claim!==claim))return null;
 const ds=before?.documents?.filter(d=>equal(d.document,c.document)),d=ds?.[0],rows=before?.writeRegistrations?.filter(r=>equal(r.provider,c.registration.provider)&&equal(r.resource,c.registration.resource));
 if(ds?.length!==1||d.live!==true||!equal(d.ordinaryBindings?.annotations,c.binding)||rows?.length!==1||!equal(rows[0],c.registration)||!ordinarySourceBindingReady(before,d,d.binding,g))return null;
 const next=c.next,changed=c.mode==='publish'||c.mode==='refresh';if(changed){
  if(next===before||g.versionsAvailable!==true||!natural(before.publication)||!natural(next?.publication)||next.publication!==before.publication+1||!Array.isArray(next.documents))return null;const ns=next.documents.filter(d=>equal(d.document,c.document));if(ns.length!==1||!equal(ns[0].annotationBaseline,c.baseline))return null;
  const frame=copy(next),fd=frame.documents.find(d=>equal(d.document,c.document));if(Object.hasOwn(d,'annotationBaseline'))fd.annotationBaseline=copy(d.annotationBaseline);else delete fd.annotationBaseline;frame.publication=before.publication;if(!equal(frame,before))return null;
  if(hold&&!annotationBaselineWithinEnvelope(hold.envelope,c.baseline,{maxRootBytes:o.maxRootBytes,maxBaselineBytes:o.limits.maxBaselineBytes,maxTagBytes:o.limits.maxTagBytes}))return null;
 }else if(next!==before||!equal(d.annotationBaseline,c.baseline))return null;
 const remaining=[...o.holds.entries()].filter(([key])=>key!==c.reservation).map(([,value])=>value);if(rootFloor(o,next,remaining)===null)return null;
 if(c.mode!=='publish'&&!o.producerPins.has(c.producerClaim))return null;const pins=new Set(o.producerPins);pins.add(c.producerClaim);if(pins.size>o.issued)return null;
 const stage=Object.freeze({next,document:copy(c.document),status:changed?'installed':'transferred'});stages.set(token,{stage,pool,domain,before,claim,hold,reservation:c.reservation,pins});return stage;
}
export function commitAnnotationBaselineInstallation(pool,domain,before,claim,token,stage){
 const o=pools.get(pool),s=stages.get(token);if(!o||!s||s.stage!==stage||s.pool!==pool||s.domain!==domain||s.before!==before||s.claim!==claim||s.hold&&o.holds.get(s.reservation)!==s.hold||!completeAnnotationBaselineInstallation(token,claim,domain,pool))return false;
 o.producerPins=s.pins;if(s.hold){o.holds.delete(s.reservation);o.heldBytes-=s.hold.charge;}stages.delete(token);return true;
}
export function releaseBaselineCapacity(pool,reservation,g){const o=pools.get(pool),r=o?.holds.get(reservation);if(!r||!allowed(g,catalog.releaseGates)||r.claim&&!dischargeAnnotationWriteClaim(r.claim,pool,reservation))return unavailable();o.holds.delete(reservation);o.heldBytes-=r.charge;return {status:'released'};}
export function releaseIneligibleBaselineCapacity(pool,domain,claim,g){
 const o=pools.get(pool),s=readAnnotationWriteClaimControl(claim);if(!o||!s||s.domain!==domain||s.link?.domain!==domain||s.link.pool!==pool||!allowed(g,catalog.releaseGates))return unavailable();const reservation=s.link.reservation,r=o.holds.get(reservation);if(!r||r.claim!==claim||!dischargeIneligibleAnnotationWriteClaim(claim,domain,pool,reservation))return unavailable();o.holds.delete(reservation);o.heldBytes-=r.charge;return {status:'released'};
}
export function stopBaselineReservations(pool,g){const o=pools.get(pool);if(!o||g?.ownerControlAllowed!==true)return unavailable();o.stopped=true;return {status:'stopped'};}
export function baselineReservationUsage(pool){const o=pools.get(pool);return o?{issued:o.issued,reservations:o.holds.size,stopped:o.stopped,baseBytes:o.baseBytes,heldBytes:o.heldBytes,reservedPublicationUnits:o.holds.size,producerPins:o.producerPins.size,liveProducerPins:[...o.producerPins].filter(annotationWriteClaimLive).length,profile:copy(o.limits)}:null;}
