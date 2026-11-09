// Package-private producer facet. Only source-save's checked constructor mints
// these claims; they are not an untrusted factory or a public token protocol.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {prepareAnnotationBaselineEnvelope,annotationBaselineWithinEnvelope} from './annotation-baseline-capacity.mjs';
const claims=new WeakMap(),installations=new WeakMap(),copy=structuredClone;
export const annotationInstallationGates=JSON.parse(fs.readFileSync(new URL('./annotation-baseline-installation.json',import.meta.url))).gates;
export const annotationClaimGates=['ownerControlAllowed','preparationReserved','historicalArtifactReadable'];
export function createAnnotationWriteClaim(owner,task,writer,ticket,draft,domain){const claim=Object.freeze({kind:'annotation-write-reservation-claim/1'});claims.set(claim,{owner,task,writer,ticket,draft:copy(draft),domain,link:null,released:false});return claim;}
function claimState(claim){const r=claims.get(claim);if(!r||r.writer.tasks.get(r.task)!==r.ticket)return null;const t=r.ticket;return {owner:r.owner,task:r.task,domain:r.domain,phase:t.phase,stopped:t.stopped,quiescent:t.quiescent,disposition:t.disposition,application:t.knowledge.application,contested:t.contested,destinationFaulted:r.writer.faults.has(t.key),destinationReleased:t.reservationReleased,released:r.released,transfer:r.transfer??null,link:r.link?{...r.link}:null};}
export function readAnnotationWriteClaimControl(claim){return claimState(claim);}
export function readAnnotationWriteClaim(claim){const r=claims.get(claim),s=claimState(claim);if(!s)return null;const t=r.ticket;return {...s,document:copy(t.capture.request.document),binding:copy(t.capture.exportState.originalAnnotationBinding),registration:copy(t.capture.registration),draft:copy(r.draft),maxTagBytes:r.writer.limits.maxTagBytes};}
// Only the checked synchronous ordinary-save producer creates unissued records.
export function readPreparedAnnotationWriteClaim(claim){const r=claims.get(claim),t=r?.ticket;if(!r||!t.annotation?.ordinary||r.writer.tasks.has(r.task)||r.link||r.released||t.phase!=='queued'||t.stopped)return null;return {owner:r.owner,task:r.task,domain:r.domain,phase:t.phase,stopped:t.stopped,link:null,released:false,document:copy(t.capture.request.document),binding:copy(t.capture.exportState.originalAnnotationBinding),registration:copy(t.capture.registration),draft:copy(r.draft),maxTagBytes:r.writer.limits.maxTagBytes};}
export function bindAnnotationWriteClaim(claim,domain,pool,reservation){const r=claims.get(claim),s=claimState(claim);if(!r||!s||s.domain!==domain||s.link||s.released||s.phase!=='queued'||s.stopped)return false;r.link={domain,pool,reservation};return true;}
export function annotationWriteClaimDispatchReady(claim,owner,task){const s=claimState(claim);return !!s&&s.owner===owner&&s.task===task&&!!s.link&&!s.released;}
export function annotationWriteClaimBound(claim){return !!claims.get(claim)?.link;}
export function annotationWriteClaimLive(claim){return claimState(claim)!==null;}
export function dischargeAnnotationWriteClaim(claim,pool,reservation){const r=claims.get(claim),s=claimState(claim);if(!r||!s||s.link?.pool!==pool||s.link.reservation!==reservation||!s.destinationReleased)return false;const safe=s.phase==='queued'?s.stopped:s.phase==='dispatched'&&s.application==='not-written'&&s.disposition&&s.quiescent&&!s.contested&&!s.destinationFaulted;if(!safe)return false;r.link=null;r.released=true;return true;}
// Release only an impossible future qualified-baseline promise, not write evidence.
export function dischargeIneligibleAnnotationWriteClaim(claim,domain,pool,reservation){
 const r=claims.get(claim),s=claimState(claim);if(!r||!s||s.domain!==domain||s.link?.domain!==domain||s.link.pool!==pool||s.link.reservation!==reservation||!s.destinationReleased||s.phase!=='dispatched'||s.application!=='written'||!s.disposition||!s.quiescent||s.contested||s.destinationFaulted)return false;
 if(acknowledgment(r.ticket)!=='synchronized'||!['failed','not-requested'].includes(r.ticket.knowledge.synchronization))return false;
 r.link=null;r.released=true;r.transfer='terminal-ineligible';return true;
}
export function retireAnnotationWriteClaim(claim){if(annotationWriteClaimBound(claim))return false;return claims.delete(claim);}
const acknowledgment=t=>t.capture.request.acknowledgment==='synchronized'||t.capture.request.acknowledgment==='ordinary'&&t.capture.registration.syncSupported?'synchronized':'written';
const qualified=(w,t)=>!!t?.annotation&&t.phase==='dispatched'&&t.knowledge.application==='written'&&(acknowledgment(t)==='written'||t.knowledge.synchronization==='succeeded')&&!t.contested&&!w.faults.has(t.key);
function newestProducer(r,task){const xs=[...r.writer.tasks.entries()].filter(([,t])=>qualified(r.writer,t)&&equal(t.capture.request.document,r.ticket.capture.request.document)&&equal(t.capture.exportState?.originalAnnotationBinding,r.ticket.capture.exportState.originalAnnotationBinding)&&equal(t.capture.registration,r.ticket.capture.registration));if(!xs.length)return false;const serial=Math.max(...xs.map(([,t])=>t.serial)),latest=xs.filter(([,t])=>t.serial===serial);return latest.length===1&&latest[0][0]===task;}
function producerBaseline(r,p,b,allowTokenless){
 const pc=claims.get(p?.annotation?.reservationClaim);if(!pc||pc.writer!==r.writer||pc.ticket!==p||pc.domain!==r.domain||!qualified(r.writer,p)||!equal(p.capture.registration,r.ticket.capture.registration)||!equal(pc.draft.context,r.draft.context))return false;
 const l={maxRootBytes:r.writer.limits.maxRootBytes,maxBaselineBytes:r.writer.limits.maxBaselineBytes,maxTagBytes:r.writer.limits.maxTagBytes},e=prepareAnnotationBaselineEnvelope(pc.draft,l);
 return !!e&&annotationBaselineWithinEnvelope(e,b,l)&&b.dispatchSerial===p.serial&&b.acknowledgment===acknowledgment(p)&&(equal(b.resourceVersion,p.knowledge.resourceVersion)||allowTokenless&&b.resourceVersion===null);
}
// Ephemeral certificates never escape the synchronous checked writer/native call.
// This factory has the same package-private confinement as claim construction.
export function sealAnnotationBaselineInstallation(owner,task,claim,domain,pool,before,next,baseline,producerTask,mode){
 const r=claims.get(claim);if(!r||r.owner!==owner||r.task!==task||r.domain!==domain||r.writer.tasks.get(task)!==r.ticket||!['publish','current','superseded','refresh'].includes(mode))return null;
 const p=r.writer.tasks.get(producerTask);if(!qualified(r.writer,r.ticket)||!producerBaseline(r,p,baseline,mode==='current'||mode==='superseded')||['publish','refresh'].includes(mode)&&!newestProducer(r,task))return null;
 if(mode==='refresh'?!r.released||r.link||!['baseline-installed','baseline-current'].includes(r.transfer):r.link?.domain!==domain||r.link.pool!==pool)return null;
 const same=producerTask===task;if(mode==='superseded'?(same||p.serial<=r.ticket.serial):!same)return null;
 const token=Object.freeze({kind:'annotation-baseline-installation/1'});installations.set(token,{owner,task,claim,domain,pool,before,next,baseline:copy(baseline),producerTask,producer:p,mode,reservation:r.link?.reservation??null,targetKnowledge:r.ticket.knowledge,producerKnowledge:p.knowledge});return token;
}
export function readAnnotationBaselineInstallation(token,claim,domain,pool){
 const c=installations.get(token),r=claims.get(claim);if(!c||!r||c.claim!==claim||c.domain!==domain||c.pool!==pool||r.domain!==domain||r.writer.tasks.get(c.task)!==r.ticket||r.writer.tasks.get(c.producerTask)!==c.producer||r.ticket.knowledge!==c.targetKnowledge||c.producer.knowledge!==c.producerKnowledge||!qualified(r.writer,r.ticket)||!producerBaseline(r,c.producer,c.baseline,c.mode==='current'||c.mode==='superseded')||['publish','refresh'].includes(c.mode)&&!newestProducer(r,c.task))return null;
 if(c.mode==='superseded'?(c.producerTask===c.task||c.producer.serial<=r.ticket.serial):c.producerTask!==c.task)return null;
 if(c.mode==='refresh'?!r.released||r.link||!['baseline-installed','baseline-current'].includes(r.transfer):r.link?.pool!==pool||r.link.reservation!==c.reservation||r.link.domain!==domain)return null;
 if(c.mode==='refresh'){const ds=c.before?.documents?.filter(d=>equal(d.document,r.ticket.capture.request.document));if(ds?.length!==1||!producerBaseline(r,c.producer,ds[0].annotationBaseline,true))return null;}
 return {...c,producerClaim:c.producer.annotation.reservationClaim,document:copy(r.ticket.capture.request.document),binding:copy(r.ticket.capture.exportState.originalAnnotationBinding),registration:copy(r.ticket.capture.registration),baseline:copy(c.baseline)};
}
export function completeAnnotationBaselineInstallation(token,claim,domain,pool){const c=readAnnotationBaselineInstallation(token,claim,domain,pool),r=claims.get(claim);if(!c||!r)return false;if(c.mode!=='refresh'){r.link=null;r.released=true;r.transfer=c.mode==='publish'?'baseline-installed':c.mode==='superseded'?'baseline-superseded':'baseline-current';}installations.delete(token);return true;}
export function discardAnnotationBaselineInstallation(token){return installations.delete(token);}
