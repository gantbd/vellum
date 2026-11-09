// Owned native created-reference participant. Public wire/Rust encoding is not selected.
import {isDeepStrictEqual as equal} from 'node:util';
import {nativeEngineSettingsIntent,publishNativeEngineSettings,captureNativeEngineSettingsWriteBasis} from './engine-settings-state.mjs';
import {releaseOutcome} from './outcome-projection.mjs';
const holds=new WeakMap(),references=new WeakMap(),reservations=new WeakMap(),stages=new WeakMap(),size=v=>Buffer.byteLength(JSON.stringify(v)),copy=structuredClone;
const natural=n=>Number.isSafeInteger(n)&&n>=0,allowed=(g,keys)=>keys.every(k=>g?.[k]===true);
const releaseGates=['authorized','scope','continuous','live','notStopped','settingsCreatedResultReadable','settingsCreatedResultContextCurrent','historicalSettingsLayersReadable'];
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
function bound(hold,owner,domain,ticket,c){const s=holds.get(hold);return s&&!s.closed&&s.owner===owner&&s.domain===domain&&s.ticket===ticket&&equal(s.data.context,c)?s:null;}
function summary(s,policy){const entry=s.owner.slots.get(s.ticket);if(entry?.phase!=='done'||entry.settingsPrepared!==s.plan)return null;const result=releaseOutcome(entry.prepared,policy);return result.observation==='available'&&['applied','unchanged'].includes(result.publication)?result:null;}
export function reserveCreatedSettingsResult(owner,domain,ticket,plan,c,ceiling){
 const captured=nativeEngineSettingsIntent(plan,owner.limits.maxIntentBytes,owner,domain,ticket),entry=owner.slots.get(ticket);
 if(!captured||entry?.phase!=='issued'||entry.settingsPrepared!==plan||!equal(entry.context,c)||owner.faulted)return null;
 const previous=reservations.get(ticket);if(previous){const existing=bound(previous,owner,domain,ticket,c);return existing&&existing.plan===plan&&existing.ceiling===(ceiling===true)?previous:null;}
 const {request,profile}=captured.value;if(request.contract!=='engine-settings-layers/1')return null;
 const created=request.create.map((row,index)=>({index,layer:copy(row.layer),referenceKind:'created-settings-layer-reference/1'}));
 const data=freeze({context:copy(c),profile:copy(profile),invocation:request.invocation,created});
 const frame={status:'result',result:{invocation:request.invocation,status:'unchanged',created:created.map(row=>({index:row.index,reference:{kind:row.referenceKind}}))}};
 // Durable result data captures no source/root. One-turn stages share the common
 // before/candidate roots; reserve the configuration staging envelope as well.
 const charge=size(data)+size(frame)+profile.maxStateBytes;
 if(size(data)>owner.limits.maxResultBytes||size(frame)>owner.limits.maxResultBytes||!natural(charge)||charge>owner.limits.maxHeldBytes-owner.charge)return null;
 owner.charge+=charge;const token=Object.freeze({kind:'settings-created-result-hold/1'});holds.set(token,{owner,domain,ticket,plan,data,ceiling:ceiling===true,charge,closed:false,committed:false,refs:null,status:null});reservations.set(ticket,token);return token;
}
export function releaseCreatedSettingsResult(hold,owner,domain,ticket,c,policy,g){
 const s=bound(hold,owner,domain,ticket,c);if(!s?.committed||!s.ceiling||!summary(s,policy)||!allowed(g,releaseGates))return {status:'unavailable'};
 const result={status:'result',result:{invocation:s.data.invocation,status:s.status,created:s.refs.map((reference,index)=>({index,reference}))}};
 return size(result)<=owner.limits.maxResultBytes?result:{status:'unavailable'};
}
export function captureCreatedSettingsWriteBasis(hold,owner,domain,ticket,reference,c,policy,g){
 const r=references.get(reference),s=bound(hold,owner,domain,ticket,c);if(!s||r?.hold!==hold||releaseCreatedSettingsResult(hold,owner,domain,ticket,c,policy,g).status!=='result')return null;
 return captureNativeEngineSettingsWriteBasis(owner,domain,s.data.created[r.index].layer,c,s.data.profile,g);
}
export function dischargeCreatedSettingsResult(hold,owner,domain,ticket,c,g){
 const s=bound(hold,owner,domain,ticket,c);if(!s||!allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;
 for(const reference of s.refs??[])references.delete(reference);s.closed=true;s.data=null;s.refs=null;owner.charge-=s.charge;return true;
}
export function createdSettingsResultUsage(hold,owner,domain,ticket,c){const s=bound(hold,owner,domain,ticket,c);return s?{declaredHeldBytes:s.charge,committed:s.committed,references:s.refs?.length??0}:null;}

// The common publisher stages and commits this result in one synchronous turn.
export function admitCreatedSettingsResult(hold,owner,domain,ticket,c,g){const s=bound(hold,owner,domain,ticket,c),entry=owner.slots.get(ticket);if(!s||s.committed||s.admitted||entry?.phase!=='issued'||entry.settingsPrepared!==s.plan)return false;s.ceiling=s.ceiling&&g?.settingsCreatedResultReferencesAllowed===true;s.admitted=true;return true;}
export function stageCreatedSettingsPublication(hold,owner,domain,ticket,before,result,c,g){
 const s=bound(hold,owner,domain,ticket,c),entry=owner.slots.get(ticket);if(!s?.admitted||s.committed||entry?.phase!=='running'||entry.settingsCreatedHold!==hold||entry.settingsPrepared!==s.plan||owner.root!==before||!['applied','unchanged'].includes(result.status))return null;
 const expected=publishNativeEngineSettings(before,s.plan,g,owner,domain,ticket);if(expected.status!==result.status||!equal(expected.state,result.state))return null;
 for(const row of s.data.created){const found=result.state.engineSettings.layers.filter(layer=>equal(layer.layer,row.layer));if(found.length!==1||before.engineSettings.layers.some(layer=>equal(layer.layer,row.layer)))return null;}
 const refs=s.data.created.map(row=>Object.freeze({kind:row.referenceKind}));
 const frame={status:'result',result:{invocation:s.data.invocation,status:result.status,created:refs.map((reference,index)=>({index,reference}))}};
 if(size(frame)>owner.limits.maxResultBytes||size(s.data)>owner.limits.maxResultBytes)return null;
 const stage=Object.freeze({kind:'created-settings-publication-stage/1'});stages.set(stage,{hold,owner,domain,ticket,before,next:result.state,status:result.status,refs});return stage;
}
export function commitCreatedSettingsPublication(stage,hold,owner,domain,ticket,before,result,c,g){
 const t=stages.get(stage),s=bound(hold,owner,domain,ticket,c),entry=owner.slots.get(ticket);
 if(!t||!s||t.hold!==hold||t.owner!==owner||t.domain!==domain||t.ticket!==ticket||t.before!==before||owner.root!==before||entry?.phase!=='running'||entry.settingsCreatedHold!==hold||t.next!==result.state||t.status!==result.status)return false;
 const current=publishNativeEngineSettings(before,s.plan,g,owner,domain,ticket);if(current.status!==result.status||!equal(current.state,result.state))return false;
 s.refs=t.refs;s.status=t.status;s.committed=true;for(let index=0;index<s.refs.length;index++)references.set(s.refs[index],{hold,index});return true;
}
export function discardCreatedSettingsPublication(stage,faulted=false){const t=stages.get(stage);if(!t)return false;const s=holds.get(t.hold);if(faulted&&s){for(const reference of s.refs??[])references.delete(reference);s.refs=null;s.status=null;s.committed=false;}return stages.delete(stage);}
