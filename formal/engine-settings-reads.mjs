// Native F07 projection of actual engine settings; host visibility truth is a premise.
// Plain bounded normalized data, not a hostile decoder, wire codec or policy engine.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {nativeEngineSettingsBasis} from './engine-settings-state.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {settingsQueryResultFrameValid,settingsQueryResultValid,settingsQueryOriginBindingsValid} from './settings-query-results.mjs';
import {settingsQueryInputForms} from './settings-query-declarations.mjs';
import {normalizeSettingsQueryArguments,settingsQueryProducerEnvelope,captureSettingsQueryArgumentLimits,settingsQueryArgumentClassificationBytes} from './settings-query-arguments.mjs';
export {settingsQueryInputForms} from './settings-query-declarations.mjs';
const pools=new WeakMap(),natural=n=>Number.isSafeInteger(n)&&n>=0;
const fields=['engineProfile','maxPackets','maxContexts','maxOrigins','maxIssued','maxPacketBytes','maxCaptureBytes','maxReplyBytes','maxRequestBytes','maxContextBytes','maxSelectorBytes','maxInputNodes','maxInputDepth','maxHeldBytes'];
const scopeFields=['authority','engine','recipient','purpose','destination','policy','continuity','profile'];
const profileName='native-engine-settings-read/1',contract='configuration-query/1';
const queryForms=settingsQueryInputForms();
const exact=(v,ks)=>{if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const ds=Object.getOwnPropertyDescriptors(v),all=Reflect.ownKeys(ds);return all.length===ks.length&&ks.every(k=>Object.hasOwn(ds,k)&&Object.hasOwn(ds[k],'value')&&ds[k].enumerable===true);};
const yes=(g,ks)=>ks.every(k=>g?.[k]===true),no=()=>({status:'unavailable'}),token=kind=>Object.freeze({kind});
const put=(o,k,v)=>Object.defineProperty(o,k,{value:v,enumerable:true,writable:true,configurable:true});
const key=r=>JSON.stringify([r.authority,r.id,r.incarnation]);
const prep=['authorized','scope','continuous','live','notStopped','ownerControlAllowed','outsidePublicationGuard','resourcesReady','preparationReserved','visibilityAttested','visibilityComplete','targetReadable'];
const read=['authorized','scope','continuous','live','notStopped','outsidePublicationGuard','resourcesReady','retained','transferReady','replyReady'];
function validProfile(p){return exact(p,fields)&&p.engineProfile&&typeof p.engineProfile==='object'&&fields.filter(k=>k!=='engineProfile').every(k=>natural(p[k]));}
function copied(v,p,maximum){return copySettingsData(v,{maxRequestBytes:maximum,maxInputNodes:p.maxInputNodes,maxInputDepth:p.maxInputDepth});}
function scope(owner,c,p){const v=copied(c,p,p.maxContextBytes)?.value;return v&&exact(v,scopeFields)&&Object.values(v).every(s=>typeof s==='string'&&s.length>0)&&v.profile===profileName&&v.authority===owner.context.authority&&v.engine===owner.context.engine?v:null;}
function retained(owner,domain){const m=pools.get(owner);return m&&!m.closed&&m.domain===domain?m:null;}
function active(owner,domain){const m=owner.faulted?null:retained(owner,domain);return m?.decoderFaulted?null:m;}
function invalidate(ctx){ctx.invalidated=true;ctx.data=null;ctx.refs.clear();}
function origins(r){return Array.isArray(r.origins)?r.origins:[...new Set(Object.values(r.origins).flat())].sort();}
function factsValid(cfg,f){
 if(!exact(f,['target','keys','capabilities'])||typeof f.capabilities!=='boolean'||!Array.isArray(f.keys)||f.keys.length!==cfg.schemas.length)return false;
 const t=cfg.targets.find(t=>equal(t.target,f.target));if(!t)return false;
 const names=new Set();for(const row of f.keys){
  if(!exact(row,['key','visible','value','origins','declaration','originIds'])||typeof row.key!=='string'||names.has(row.key)||!cfg.schemas.some(s=>s.key===row.key)||!['visible','value','origins','declaration'].every(k=>typeof row[k]==='boolean')||!Array.isArray(row.originIds)||row.originIds.some(s=>typeof s!=='string')||new Set(row.originIds).size!==row.originIds.length||!row.visible&&(row.value||row.origins||row.declaration||row.originIds.length)||!row.origins&&row.originIds.length)return false;
  const actual=origins(cfg.resolved[key(t.target)][row.key]);if(row.originIds.some(id=>!actual.includes(id)))return false;names.add(row.key);
 }
 return true;
}
function projection(cfg,f,ceiling=null){
 const values=cfg.resolved[key(f.target)],out=[];for(const old of ceiling??cfg.schemas.map(s=>f.keys.find(r=>r.key===s.key)).filter(r=>r.visible)){
  const grant=f.keys.find(r=>r.key===old.key);if(!grant?.visible||old.value&&!grant.value||old.origins&&!grant.origins||old.declaration&&!grant.declaration)return null;
  const row={key:old.key,value:old.value,origins:old.origins,declaration:old.declaration},actual=values[old.key];
  if(old.value)row.data=actual.value;
  if(old.origins){row.originIds=ceiling?old.originIds:grant.originIds.slice().sort();const ids=origins(actual);if(row.originIds.some(id=>!grant.originIds.includes(id)||!ids.includes(id)))return null;
   const select=ids=>ids.filter(id=>row.originIds.includes(id));row.originData=Array.isArray(actual.origins)?select(actual.origins):Object.fromEntries(Object.entries(actual.origins).map(([k,v])=>[k,select(v)]));
  }
  if(old.declaration){const s=cfg.schemas.find(s=>s.key===old.key);row.description=Object.fromEntries(['key','type','domain','scopes','composition','sensitivity'].map(k=>[k,s[k]]));}
  out.push(row);
 }
 return {rows:out,capabilities:f.capabilities};
}
// Opening charges the actual shared parent, not a second self-certified budget.
export function openNativeSettingsReads(owner,domain,p,g){
 if(pools.has(owner)||owner.faulted||!validProfile(p)||!yes(g,['ownerControlAllowed','resourcesReady','preparationReserved']))return null;
 const profile=copied(p,p,p.maxPacketBytes);if(!profile||!nativeEngineSettingsBasis(owner,p.engineProfile))return null;
 const charge=profile.bytes+2*p.maxPackets*p.maxPacketBytes+p.maxContexts*(2*p.maxCaptureBytes+p.maxContextBytes+(p.maxOrigins+2)*p.maxSelectorBytes);
 if(!natural(charge)||charge>p.maxHeldBytes||!natural(owner.charge)||charge>owner.limits.maxHeldBytes-owner.charge)return null;
 const handle=token('engine-settings-read-pool/1');pools.set(owner,{domain,handle,profile:profile.value,charge,packets:new Map(),contexts:new Map(),issued:0,closed:false});owner.charge+=charge;return handle;
}
export function prepareNativeSettingsRead(owner,domain,handle,c,facts,basis,g){
 const m=active(owner,domain);if(!m||m.handle!==handle||!yes(g,prep)||basis!==owner.root.engineSettings||m.packets.size>=m.profile.maxPackets||m.issued>=m.profile.maxIssued)return null;
 const p=m.profile,s=scope(owner,c,p),cfg=nativeEngineSettingsBasis(owner,p.engineProfile),data=s&&cfg&&copied({scope:s,facts,cfg},p,p.maxPacketBytes);if(!data||!factsValid(data.value.cfg,data.value.facts))return null;
 const packet=token('engine-settings-read-cut/1');m.packets.set(packet,{...data.value,basis});m.issued++;return packet;
}
function take(owner,m,packet,c){const v=m.packets.get(packet);if(!v||v.basis!==owner.root.engineSettings||!equal(v.scope,c))return null;m.packets.delete(packet);return v;}
function projectPacket(m,p,old=null){
 if(old&&old.capabilities&&!p.facts.capabilities)return null;
 const data=projection(p.cfg,p.facts,old?.rows??null);if(!data)return null;
 if(old)data.capabilities=old.capabilities;
 return copied(data,m.profile,m.profile.maxCaptureBytes)?.value??null;
}
export function acquireNativeSettingsRead(owner,domain,handle,c,packet,g){
 const m=active(owner,domain);if(!m||m.handle!==handle||!yes(g,read)||!scope(owner,c,m.profile)||m.contexts.size>=m.profile.maxContexts)return no();
 const p=take(owner,m,packet,c),data=p&&projectPacket(m,p);if(!data)return no();
 const ids=[...new Set(data.rows.flatMap(r=>r.originIds??[]))].sort(),cost=ids.length+2;
 if(ids.length>m.profile.maxOrigins||cost>m.profile.maxIssued-m.issued)return no();
 const reference=token('engine-settings-read-context/1'),target=token('engine-settings-read-target/1'),refs=new Map(ids.map(id=>[id,token('engine-settings-origin/1')]));
 if([...refs.values(),reference,target].some(ref=>!copied(ref,m.profile,m.profile.maxSelectorBytes)))return no();
 const reply={status:'context',context:reference,target,coverage:'captured-permitted'};if(!copied(reply,m.profile,m.profile.maxReplyBytes))return no();
 m.contexts.set(reference,{scope:p.scope,target,targetIdentity:p.facts.target,data,refs,stopped:false,invalidated:false});m.issued+=cost;return reply;
}
function request(m,envelope){
 // Opaque context/target references are checked before copying the remaining data.
 if(!exact(envelope,['route','request'])||!['typed','dynamic'].includes(envelope.route))return null;const q=envelope.request,tag=envelope.route==='typed'?'family':'name';
 if(!exact(q,[tag,'variant','version','arguments']))return null;const tuple=Object.fromEntries([tag,'variant','version'].map(k=>[k,q[k]]));
 const decoded=envelope.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F07'||!['effective-configuration','permitted-origin','capabilities'].includes(decoded.variant))return null;
 const a=q.arguments,form=queryForms.find(f=>f.variant===decoded.variant);if(!form||!exact(a,form.fields)||a.contract!==form.contract||a.operation!==form.operation||a.profile!==profileName)return null;
 const data=copied({...a,context:null,target:null},m.profile,m.profile.maxRequestBytes);if(!data||decoded.variant!=='capabilities'&&(typeof a.key!=='string'||!a.key.length))return null;
 return {...data.value,context:a.context,target:a.target};
}
export function queryNativeSettingsRead(owner,domain,handle,envelope,c,packet,g){
 const m=active(owner,domain);if(!m||m.handle!==handle||g?.outsidePublicationGuard!==true||!scope(owner,c,m.profile))return no();const a=request(m,envelope),ctx=a&&m.contexts.get(a.context);
 if(!ctx||ctx.stopped||ctx.invalidated||ctx.target!==a.target||!equal(ctx.scope,c))return no();
 // Only a bound trusted observation can invalidate; malformed/foreign requests cannot.
 if(['authorized','scope','continuous','live','retained'].some(k=>g?.[k]===false)){invalidate(ctx);return no();}
 if(!yes(g,read))return no();
 try{return releaseSettingsQuery(owner,m,ctx,a,c,packet);}
 catch{invalidate(ctx);return no();}
}
function releaseSettingsQuery(owner,m,ctx,a,c,packet){
 const basis=nativeEngineSettingsBasis(owner,m.profile.engineProfile);if(!basis)return no();if(!basis.targets.some(t=>equal(t.target,ctx.targetIdentity))){invalidate(ctx);return no();}
 const p=take(owner,m,packet,c);if(!p||!equal(p.facts.target,ctx.targetIdentity))return no();const current=projectPacket(m,p,ctx.data);if(!current||!equal(current,ctx.data)){invalidate(ctx);return no();}
 let out;
 if(a.operation==='capabilities'){if(!ctx.data.capabilities)return no();out={status:'capabilities',context:a.context,target:a.target,coverage:'permitted-only',capabilities:ctx.data.rows.filter(r=>r.declaration).map(r=>r.description)};}
 else{const row=ctx.data.rows.find(r=>r.key===a.key);if(!row)return no();if(a.operation==='effective-configuration'){if(!row.value)return no();out={status:'value',context:a.context,target:a.target,key:a.key,value:row.data};}
  else{if(!row.origins)return no();const refs=ids=>ids.map(id=>ctx.refs.get(id));const data=Array.isArray(row.originData)?refs(row.originData):Object.fromEntries(Object.entries(row.originData).map(([k,v])=>[k,refs(v)]));out={status:'origins',context:a.context,target:a.target,key:a.key,coverage:'permitted-only',origins:data};}}
 // Preserve issued reference identity while returning newly owned bounded data.
 const resultLimits={maxRequestBytes:m.profile.maxReplyBytes,maxInputNodes:m.profile.maxInputNodes,maxInputDepth:m.profile.maxInputDepth};
 // Result frame checkpoint, separate from validator mutation targets.
 if(!settingsQueryResultFrameValid(out,a.operation,a.context,a.target,a.key))throw Error('settings result invariant');
 if(out.status==='value'&&!equal(out.value,ctx.data.rows.find(r=>r.key===a.key).data))throw Error('settings value binding invariant');
 if(out.status==='origins'&&!settingsQueryOriginBindingsValid(out.origins,ctx.data.rows.find(r=>r.key===a.key).originData,ctx.refs))throw Error('settings origin binding invariant');
 if(out.status==='capabilities'&&!equal(out.capabilities,ctx.data.rows.filter(r=>r.declaration).map(r=>r.description)))throw Error('settings declaration binding invariant');
 if(!copied(out,m.profile,m.profile.maxReplyBytes))return no();
 if(out.status==='value')out.value=copied(out.value,m.profile,m.profile.maxReplyBytes).value;
 if(out.status==='capabilities')out.capabilities=copied(out.capabilities,m.profile,m.profile.maxReplyBytes).value;
 // Released result checkpoint, separate from validator mutation targets.
 if(!settingsQueryResultValid(out,a.operation,a.context,a.target,a.key,resultLimits))throw Error('settings released result invariant');
 if(out.status==='origins'&&!settingsQueryOriginBindingsValid(out.origins,ctx.data.rows.find(r=>r.key===a.key).originData,ctx.refs))throw Error('settings released origin binding invariant');
 if(out.status==='value'&&!equal(out.value,ctx.data.rows.find(r=>r.key===a.key).data))throw Error('settings released value binding invariant');
 if(out.status==='capabilities'&&!equal(out.capabilities,ctx.data.rows.filter(r=>r.declaration).map(r=>r.description)))throw Error('settings released declaration binding invariant');
 return Object.freeze(out);
}
export function discardNativeSettingsReadCut(owner,domain,handle,packet,g){const m=retained(owner,domain);return !!(m&&m.handle===handle&&yes(g,['ownerControlAllowed','retentionReleased'])&&m.packets.delete(packet));}
// Ordinary F07 decoding is charged to the actual shared parent. No protected verb.
export function queryDecodedNativeSettingsRead(owner,domain,handle,envelope,inputLimits,c,packet,g){
 const m=active(owner,domain);if(!m||m.handle!==handle||m.decoding||settingsQueryArgumentClassificationBytes>owner.limits.maxHeldBytes-owner.charge)return no();
 let charged=settingsQueryArgumentClassificationBytes;m.decoding=true;m.decodeCharge=charged;owner.charge+=charged;
 try{
  // Settings decoder classification checkpoint.
  const p=captureSettingsQueryArgumentLimits(inputLimits);if(!p)return no();
  // Seven logical frames: envelope, authority, canonical, limits, descriptor,
  // current facts and fixed native producer envelope.
  const maximum=p.maxRequestBytes,workspace=7*maximum;if(!natural(workspace)||workspace>owner.limits.maxHeldBytes-owner.charge)return no();
  charged+=workspace;m.decodeCharge=charged;owner.charge+=workspace;
  // Settings decoder normalization checkpoint.
  const n=normalizeSettingsQueryArguments(envelope,c,p,maximum);if(!n||!scope(owner,n.record.context,m.profile))return no();
  const current=copySettingsData({facts:g},{...p,maxRequestBytes:maximum});if(!current)return no();
  // Settings decoder dispatch checkpoint.
  return queryNativeSettingsRead(owner,domain,handle,settingsQueryProducerEnvelope(n),n.record.context,packet,current.value.facts);
 }catch{m.decoderFaulted=true;return no();}
 finally{owner.charge-=charged;m.decodeCharge=0;m.decoding=false;}
}
export function nativeSettingsReadDecodeUsage(owner,domain,handle){const m=retained(owner,domain);return m&&m.handle===handle?{busy:m.decoding===true,declaredWorkspaceBytes:m.decodeCharge??0,faulted:m.decoderFaulted===true}:null;}
export function stopNativeSettingsRead(owner,domain,handle,reference,c,g){const m=retained(owner,domain),s=m?.contexts.get(reference),sc=m&&scope(owner,c,m.profile);if(!s||m.handle!==handle||!sc||!equal(s.scope,sc)||g?.ownerControlAllowed!==true)return false;s.stopped=true;s.data=null;s.refs.clear();return true;}
export function nativeSettingsStopTargetOwned(owner,domain,handle,reference){const m=retained(owner,domain);return Boolean(m&&m.handle===handle&&m.contexts.has(reference));}
export function retireNativeSettingsRead(owner,domain,handle,reference,g){const m=retained(owner,domain),s=m?.contexts.get(reference);return !!((s?.stopped||s?.invalidated)&&!m.decoding&&m.handle===handle&&yes(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])&&m.contexts.delete(reference));}
export function retireNativeSettingsReads(owner,domain,handle,g){const m=retained(owner,domain);if(!m||m.handle!==handle||m.decoding||m.packets.size||m.contexts.size||!yes(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;m.closed=true;m.profile=null;owner.charge-=m.charge;return true;}
// Trusted supervision counts, not an ordinary recipient existence oracle.
export function nativeSettingsReadUsage(owner){const m=pools.get(owner);return m?{closed:m.closed,packets:m.packets.size,contexts:m.contexts.size,invalidated:[...m.contexts.values()].filter(s=>s.invalidated).length,issued:m.issued,declaredHeldBytes:m.closed?0:m.charge}:null;}
