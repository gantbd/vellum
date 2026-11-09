// Fixed first-party SaveAs local activation, no provider invocation or callback.
import {nativeLifecycleReceiptValid} from './lifecycle-results.mjs';
import {isDeepStrictEqual as equal} from 'node:util';
import {sourceSaveAsInvocationForm} from './lifecycle-activation-declarations.mjs';
import {prepareNativeSourceSaveAsActivation} from './source-save.mjs';
import {readSourceDestinationTransition,commitSourceDestinationTransition,discardSourceDestinationTransition} from './source-destination-state.mjs';
import {captureLoadInvocation,stageLoadInvocation,commitLoadInvocation,discardLoadInvocationStage,forgetLoadInvocation} from './load-invocation-state.mjs';
const plans=new WeakMap(),stages=new WeakMap(),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort()),freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
export const saveAsInvocationProfile=sourceSaveAsInvocationForm.profile;
const nativeStatus=status=>status==='activated'?'applied':status;
export function prepareSaveAsInvocation(owner,domain,ticket,reference,before,request,profile,authority,g){
 if(!exact(request,sourceSaveAsInvocationForm.fields)||request.contract!=='lifecycle-invocation/1'||request.operation!=='change-document-lifecycle'||request.profile!==saveAsInvocationProfile||request.invocation!==reference||!exact(request.request,sourceSaveAsInvocationForm.nested.fields))return null;
 const value={request:copy(request),profile:copy(profile)};if(size({kind:'lifecycle',plan:value})>owner.limits.maxIntentBytes)return null;
 const token=prepareNativeSourceSaveAsActivation(authority,domain,before,profile,request.request.probe,g);if(!token)return null;const native=readSourceDestinationTransition(token,domain,before,g);discardSourceDestinationTransition(token);if(!native)return null;
 const plan=Object.freeze({kind:'owned-saveas-invocation/1'});plans.set(plan,{owner,domain,ticket,invocation:reference,authority,value:freeze(value)});return plan;
}
export function captureSaveAsInvocation(plan,owner,domain,ticket,max){const p=plans.get(plan);if(!p)return captureLoadInvocation(plan,owner,domain,ticket,max);return p&&p.owner===owner&&p.domain===domain&&p.ticket===ticket&&size(p.value)<=max?{invocation:p.invocation,inspection:p.authority,value:copy(p.value)}:null;}
const current=(p,h)=>h.lifecycleInterpretationCurrent===true&&equal(h.lifecycleProfile,p.value.profile)&&h.lifecycleSaveAsAuthority===p.authority;
export function stageSaveAsInvocation(plan,owner,domain,ticket,before,host){
 const p=plans.get(plan);if(!p)return stageLoadInvocation(plan,owner,domain,ticket,before,host);if(p.owner!==owner||p.domain!==domain||p.ticket!==ticket)throw Error('foreign SaveAs invocation');const refuse=()=>({stage:null,result:{status:'not-published',state:before},receipt:null});if(!current(p,host))return refuse();
 const token=prepareNativeSourceSaveAsActivation(p.authority,domain,before,p.value.profile,p.value.request.request.probe,host);if(!token)return refuse();const native=readSourceDestinationTransition(token,domain,before,host);if(!native){discardSourceDestinationTransition(token);return refuse();}
 const receipt={invocation:p.invocation,status:nativeStatus(native.plan.status),operation:'source-save-as',document:copy(native.plan.capture.request.document)};if(!nativeLifecycleReceiptValid(receipt)){discardSourceDestinationTransition(token);throw Error('native lifecycle receipt protocol fault');};if(size({status:'result',result:receipt})>owner.limits.maxResultBytes){discardSourceDestinationTransition(token);return refuse();}
 const stage=Object.freeze({kind:'saveas-invocation-stage/1'});stages.set(stage,{plan,owner,domain,ticket,before,token,native,receipt:freeze(copy(receipt))});return {stage,result:{status:nativeStatus(native.plan.status),state:native.next},receipt:copy(receipt),scratch:null};
}
export function commitSaveAsInvocation(stage,plan,owner,domain,ticket,before,result,receipt,host){
 const s=stages.get(stage),p=plans.get(plan);if(!p)return commitLoadInvocation(stage,plan,owner,domain,ticket,before,result,receipt,host);if(!s||s.plan!==plan||s.owner!==owner||s.domain!==domain||s.ticket!==ticket||s.before!==before||!current(p,host)||result.status!==nativeStatus(s.native.plan.status)||result.state!==s.native.next||!equal(receipt,s.receipt)||!readSourceDestinationTransition(s.token,domain,before,host))return false;
 const committed=commitSourceDestinationTransition(s.token);if(!committed||committed.status!==s.native.plan.status)return false;stages.delete(stage);return true;
}
export function discardSaveAsInvocationStage(stage){const s=stages.get(stage);if(!s)return discardLoadInvocationStage(stage);discardSourceDestinationTransition(s.token);return stages.delete(stage);}
export function forgetSaveAsInvocation(plan){return plans.delete(plan)||forgetLoadInvocation(plan);}
