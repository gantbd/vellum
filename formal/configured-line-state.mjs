// Owned configured line participant; normalized private product, not a public ABI.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {nativeEngineSettingsBasis,acquireNativeEngineSettingsContext,readNativeEngineSettingsContext,releaseNativeEngineSettingsContext} from './engine-settings-state.mjs';
import {prepareLineEdit,lineInvocationIntent,publishLineEdit} from './line-edits.mjs';
const plans=new WeakMap(),copy=structuredClone;
const configuredInput=Object.freeze({family:'F11',contract:'configured-line-edit/1',operation:'ordinary-edit',variant:'ordinary-edit',profileConstant:'configured-prefix-line/1',fields:Object.freeze(['contract','operation','action','invocation','document','view','sourceRevision','expectedSelection','profile','configuration'])});
export function configuredLineInputForm(){return configuredInput;}
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&Object.keys(v).length===keys.length&&keys.every(k=>Object.hasOwn(v,k));
const yes=(g,keys)=>keys.every(k=>g?.[k]===true),key=t=>JSON.stringify([t.authority,t.id,t.incarnation]);
const currentFacts=['authorized','scope','continuous','live','notStopped','interpretationCurrent','resourcesReady','ownerControlAllowed','settingsReadable','sourceReadable','viewWritable','bounded','companionsReady','sourceWritable','allTargetsWritable'];
function binding(r){return {target:r.configuration.target,keys:r.configuration.keys,document:r.document,view:r.view,action:r.action};}
function allowed(g,c,b){const a=g?.lineConfigurationAuthority;return exact(a,['context','binding','readAllowed','useAllowed'])&&equal(a.context,c)&&equal(a.binding,b)&&a.readAllowed===true&&a.useAllowed===true;}
function resolve(cfg,r){
 const fields=r.action==='insert-newline'?['delimiter','autoIndent']:['unit','tabWidth'];
 if(!['indent-lines','outdent-lines','insert-newline'].includes(r.action)||!exact(r.configuration,['target','keys'])||!exact(r.configuration.keys,fields))return null;
 const target=cfg.targets.find(t=>equal(t.target,r.configuration.target));if(!target)return null;
 const values=cfg.resolved[key(target.target)],out={};
 for(const f of fields){const name=r.configuration.keys[f],schema=cfg.schemas.find(s=>s.key===name);if(typeof name!=='string'||!schema||schema.domain!=='source'||!Object.hasOwn(values,name))return null;Object.defineProperty(out,f,{value:copy(values[name].value),enumerable:true});}
 return out;
}
function footprint(cfg,r){const t=cfg.targets.find(t=>equal(t.target,r.configuration.target));return t?{schema:cfg.schemaGeneration,restrictions:cfg.restrictionGeneration,association:cfg.associationGeneration,source:cfg.domains[key(t.target)].source}:null;}
export function prepareConfiguredLine(owner,domain,ticket,reference,before,envelope,p,c,g,existing=null){
 if(!owner||owner.root!==before||!p?.argumentLimits||!equal(c,owner.context)||!equal(g?.semanticSettingsProfile,p)||!yes(g,currentFacts)||!yes(g,['outsidePublicationGuard','preparationReserved']))return null;
 const normalized=copySettingsData(envelope,p.argumentLimits),bound=copySettingsData(c,p.argumentLimits);if(!normalized||!bound)return null;const e=normalized.value,context=bound.value;
 if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route)||!exact(e.request,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:r,...tuple}=e.request,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
 if(decoded?.descriptor.id!=='F11'||decoded.variant!=='ordinary-edit')return null;
 if(!exact(r,configuredInput.fields)||r.contract!==configuredInput.contract||r.operation!==configuredInput.operation||r.profile!==configuredInput.profileConstant||r.invocation!==reference||!exact(r.configuration,['target','keys']))return null;
 const b=binding(r);if(!allowed(g,context,b))return null;
 if(existing){const old=plans.get(existing);return old&&old.owner===owner&&old.domain===domain&&old.ticket===ticket&&equal(old.value.request,r)&&equal(old.value.context,context)&&equal(old.value.profile,p)?existing:null;}
 const cfg=nativeEngineSettingsBasis(owner,p),settings=cfg&&resolve(cfg,r);if(!settings)return null;
 const {configuration,...rest}=r,ordinary={...rest,contract:'line-edit/1',profile:'explicit-prefix-line-edit/1',settings};
 const inner=prepareLineEdit(before,ordinary,{...g,lineSettings:settings},g.viewLimits,g.lineWorkLimits),intent=inner&&lineInvocationIntent(inner,owner.limits.maxIntentBytes);if(!intent)return null;
 const payload=copySettingsData({request:r,context,profile:p,binding:b,dependency:footprint(cfg,r),inner:intent.value},{...p.argumentLimits,maxRequestBytes:Math.min(p.maxPlanBytes,owner.limits.maxIntentBytes)});if(!payload||Buffer.byteLength(JSON.stringify({kind:'line',plan:payload.value}))>owner.limits.maxIntentBytes)return null;
 const lease=acquireNativeEngineSettingsContext(owner,domain,context,p,g);if(!lease)return null;
 const token=Object.freeze({kind:'native-configured-line-plan/1'});plans.set(token,{owner,domain,ticket,reference,value:payload.value,inner,lease});return token;
}
export function configuredLineIntent(plan,maximum,owner,domain,ticket){const p=plans.get(plan);return p&&p.owner===owner&&p.domain===domain&&p.ticket===ticket&&Number.isSafeInteger(maximum)&&maximum>=0&&Buffer.byteLength(JSON.stringify(p.value))<=maximum?{invocation:p.reference,value:p.value}:null;}
export function publishConfiguredLine(state,plan,g,owner,domain,ticket){
 const p=plans.get(plan);if(!p)return null;const no=()=>({status:'not-published',state});
 if(p.owner!==owner||p.domain!==domain||p.ticket!==ticket||owner.root!==state||!equal(g?.semanticSettingsProfile,p.value.profile)||!yes(g,currentFacts)||!allowed(g,p.value.context,p.value.binding))return no();
 const held=readNativeEngineSettingsContext(owner,domain,p.lease,p.value.context,g),current=held&&nativeEngineSettingsBasis(owner,p.value.profile);
 if(!current||!equal(footprint(current,p.value.request),p.value.dependency))return no();
 const settings=resolve(current,p.value.request);if(!settings||!equal(settings,p.value.inner.settings))return no();
 return publishLineEdit(state,p.inner,{...g,lineSettings:settings},g.viewLimits,g.lineWorkLimits);
}
export function forgetConfiguredLine(plan,owner,domain,ticket,g){const p=plans.get(plan);if(!p||p.owner!==owner||p.domain!==domain||p.ticket!==ticket||!releaseNativeEngineSettingsContext(owner,domain,p.lease,p.value.context,g))return false;return plans.delete(plan);}
