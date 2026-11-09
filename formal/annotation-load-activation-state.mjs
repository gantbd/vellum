// Actual annotation read producer and closed local restore/baseline transitions.
import {isDeepStrictEqual as equal} from 'node:util';
import {inspectAnnotationResourceCandidateFromLedger} from './annotation-resource-read.mjs';
import {prepareAnnotationRestoration} from './annotation-restoration.mjs';
import {annotationRestorationCandidate,discardAnnotationRestoration} from './annotation-restoration-state.mjs';
import {annotationBaselineCandidate,prepareNativeAnnotationReadBaseline} from './annotation-native-load.mjs';
import {annotationLoadedBaselineGates,discardAnnotationLoadWriterGuard,readLoadedAnnotationBaseline,completeLoadedAnnotationBaseline,discardLoadedAnnotationBaseline} from './annotation-loaded-baseline-state.mjs';
const handles=new WeakMap(),byTask=new WeakMap(),transitions=new WeakMap(),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const valid=p=>p.manager.domain===p.domain&&p.manager.reader===p.readerOwner&&p.manager.restorations===p.restorations&&p.manager.consumed===p.consumed&&p.ledger.tasks===p.records&&p.records.get(p.task)===p.record&&p.record.phase==='submitted'&&!p.record.contested&&!p.record.refused&&(p.consumed.has(p.task)&&p.restorations.has(p.task)||!p.record.stopped&&p.record.candidate===p.candidate&&p.candidate!==null);
export function sealAnnotationLoadActivationAuthority(owner,manager,ledger,task,domain){const p={owner,manager,ledger,task,domain,readerOwner:manager.reader,restorations:manager.restorations,consumed:manager.consumed,records:ledger?.tasks,record:ledger?.tasks.get(task),candidate:ledger?.tasks.get(task)?.candidate};if(!p.record||!valid(p))return null;const old=byTask.get(task);if(old){const q=handles.get(old);return q&&q.owner===owner&&q.manager===manager&&q.ledger===ledger&&valid(q)?old:null;}const h=Object.freeze({kind:'annotation-load-activation-authority/1'});handles.set(h,p);byTask.set(task,h);return h;}
export function readAnnotationLoadActivationAuthority(handle,domain){const p=handles.get(handle);return p&&p.domain===domain&&valid(p)?p:null;}
export function revokeAnnotationLoadActivationAuthority(task){const h=byTask.get(task);if(!h)return false;byTask.delete(task);return handles.delete(h);}
const cleanup=p=>{if(p?.restoration)discardAnnotationRestoration(p.restoration);if(p?.baseline)discardLoadedAnnotationBaseline(p.baseline);if(p?.guard)discardAnnotationLoadWriterGuard(p.guard);};
export function prepareNativeAnnotationLoad(authority,domain,before,context,request,profile,inspection,g){
 const a=readAnnotationLoadActivationAuthority(authority,domain);if(!a||before?.authority!==context?.authority)return null;let p={a,authority,domain,before,context:copy(context),request:copy(request),profile:copy(profile),inspection,restoration:null,baseline:null,guard:null};
 if(request.phase==='restore'){
  if(a.consumed.has(a.task)||!equal(profile?.snapshot,a.manager.snapshotLimits)||inspection!==null)return null;
  const candidate=inspectAnnotationResourceCandidateFromLedger(a.ledger,a.task,before,g);if(candidate.status!=='annotation-resource-candidate')return null;
  p.retained={candidate:copy(candidate),context:copy(request.context),binding:copy(request.binding),profile:copy(profile),beforePublication:before.publication};if(size(p.retained)>a.manager.limits.maxCandidateBytes)return null;
  p.restoration=prepareAnnotationRestoration(domain,before,context,candidate.capture.request.document,Uint8Array.from(candidate.bytes),request.context,request.binding,profile,g);if(!p.restoration)return null;
  const result=annotationRestorationCandidate(domain,before,context,p.restoration,g);if(!result){cleanup(p);return null;}p.result=result;p.disposition=result.status;p.document=copy(candidate.capture.request.document);
 }else if(request.phase==='adopt-baseline'){
  if(!annotationLoadedBaselineGates.every(k=>g?.[k]===true)||g.dispositionReadable!==true)return null;const r=a.restorations.get(a.task);if(!r||!equal(profile,r.profile))return null;
  const c=prepareNativeAnnotationReadBaseline(a.owner,a.manager,a.task,before,inspection,g);if(!c)return null;p.guard=c.guard;p.baseline=c.token;p.result=c.result;p.disposition=p.result.status;p.document=c.document;
 }else return null;
 p.completion=Object.freeze({status:p.disposition});const token=Object.freeze({kind:'native-annotation-load-transition/1'});transitions.set(token,p);return token;
}
export function readNativeAnnotationLoad(token,domain,before,context,g){const p=transitions.get(token);if(!p||p.domain!==domain||p.before!==before||!equal(p.context,context)||readAnnotationLoadActivationAuthority(p.authority,domain)!==p.a)return null;const a=p.a;
 if(p.request.phase==='restore'){if(a.consumed.has(a.task))return null;const candidate=inspectAnnotationResourceCandidateFromLedger(a.ledger,a.task,before,g),r=annotationRestorationCandidate(domain,before,context,p.restoration,g);return candidate.status==='annotation-resource-candidate'&&equal(candidate,p.retained.candidate)&&r&&r.status===p.disposition&&equal(r.state,p.result.state)?p:null;}
 if(!annotationLoadedBaselineGates.every(k=>g?.[k]===true)||g.dispositionReadable!==true)return null;const c=annotationBaselineCandidate(a.owner,a.manager,a.task,before,g),b=readLoadedAnnotationBaseline(p.baseline,domain);return c&&b&&b.before===before&&b.next===p.result.state&&equal(c.next,p.result.state)&&p.disposition===(c.next===before?'current':'installed')?p:null;
}
export function commitNativeAnnotationLoad(token,domain,before,context,g){const p=readNativeAnnotationLoad(token,domain,before,context,g);if(!p)return null;if(p.request.phase==='restore'){p.a.restorations.set(p.a.task,p.retained);p.a.consumed.add(p.a.task);discardAnnotationRestoration(p.restoration);}else if(!completeLoadedAnnotationBaseline(p.baseline,domain))return null;discardAnnotationLoadWriterGuard(p.guard);transitions.delete(token);return p.completion;}
export function discardNativeAnnotationLoad(token){const p=transitions.get(token);if(!p)return false;cleanup(p);return transitions.delete(token);}
