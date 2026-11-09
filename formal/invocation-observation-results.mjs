// Closed selected native F06 values, not public wire codes or policy truth.
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const forms=freeze({execution:['status','phase','observation'],outcome:['status','observation','outcome'],waiting:['status','wait'],observed:['status','result'],'observation-ended':['status'],unavailable:['status']}),outcomeFields=Object.freeze(['observation','publication','control','effect','reasons','participants','count','correlation','recovery']),reasons=Object.freeze(['invalid','denied','missing','stale','conflict','capacity','fault']),recovery=Object.freeze(['observe-existing','prepare-new-proposal','retry-protected-effect','reconcile','inspect']);
const vocabulary=freeze({phases:['pending','terminal'],publication:['applied','unchanged','notApplied','unknown','withheld'],control:['done','stopped','rejected','faulted','observationEnded','withheld'],effect:['none','confirmed','failed','unknown','withheld']}),bounds=freeze({reasons:2,participants:2,recovery:1}),declarations=freeze({forms,outcomeFields,reasons,recovery,vocabulary,bounds});
function exact(v,keys){if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const ds=Object.getOwnPropertyDescriptors(v),own=Reflect.ownKeys(ds);return own.length===keys.length&&keys.every(k=>Object.hasOwn(ds,k)&&Object.hasOwn(ds[k],'value')&&ds[k].enumerable===true);}
function dense(a,max){if(!Array.isArray(a)||Object.getPrototypeOf(a)!==Array.prototype||a.length>max)return false;const ds=Object.getOwnPropertyDescriptors(a),own=Reflect.ownKeys(ds);if(own.length!==a.length+1)return false;for(let i=0;i<a.length;i++)if(!Object.hasOwn(ds,i)||!Object.hasOwn(ds[i],'value')||ds[i].enumerable!==true)return false;return true;}
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
export function invocationObservationResultForms(){return forms;}
export function invocationObservationResultDeclarations(){return declarations;}
export function nativeObservationOutcomeValid(v){
 if(!exact(v,outcomeFields)||v.observation!=='available'||!vocabulary.publication.includes(v.publication)||!vocabulary.control.includes(v.control)||!vocabulary.effect.includes(v.effect)||v.publication==='applied'&&v.control==='rejected')return false;
 if(!dense(v.reasons,bounds.reasons)||v.reasons.length<1)return false;
 if(v.reasons.includes('unavailable')){if(v.reasons.length!==1)return false;}else for(let i=0;i<v.reasons.length;i++)if(!reasons.includes(v.reasons[i])||i>0&&reasons.indexOf(v.reasons[i-1])>=reasons.indexOf(v.reasons[i]))return false;
 if(!dense(v.participants,bounds.participants)||!v.participants.every(x=>typeof x==='string')||!dense(v.recovery,bounds.recovery)||!v.recovery.every(x=>recovery.includes(x)))return false;
 return v.count===null?v.participants.length===0&&v.correlation===null:Number.isSafeInteger(v.count)&&v.count===v.participants.length&&typeof v.correlation==='string';
}
function execution(v,operation,observation){
 const ds=object(v)?Object.getOwnPropertyDescriptors(v):null,has=ds&&Object.hasOwn(ds,'outcome');
 if(!exact(v,[...forms.execution,...(has?['outcome']:[])])||v.status!=='execution'||!vocabulary.phases.includes(v.phase)||!object(v.observation)||v.observation!==observation)return false;
 return !has||operation==='status'&&v.phase==='terminal'&&nativeObservationOutcomeValid(v.outcome);
}
export function invocationObservationResultValid(v,operation,observation=null,wait=null){
 if(!['status','retained-outcome','minimal-control-status','bounded-observation-wait'].includes(operation)||!object(v))return false;
 const status=Object.getOwnPropertyDescriptor(v,'status')?.value;if(typeof status!=='string'||!Object.hasOwn(forms,status))return false;
 if(status==='execution')return ['status','minimal-control-status'].includes(operation)&&execution(v,operation,observation);
 if(!exact(v,forms[status]))return false;if(status==='unavailable')return true;
 if(status==='outcome')return operation==='retained-outcome'&&object(v.observation)&&v.observation===observation&&nativeObservationOutcomeValid(v.outcome);
 if(operation!=='bounded-observation-wait')return false;
 if(status==='waiting')return object(v.wait)&&v.wait===wait;
 if(status==='observed')return execution(v.result,'status',observation);
 return status==='observation-ended';
}
