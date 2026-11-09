// Bounded document serialization metadata, independent of configuration schemas.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {mergeDependencies} from './annotation-operations.mjs';
export const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const natural=v=>Number.isSafeInteger(v)&&v>=0,version=v=>natural(v)&&v>0;
export const sourceSerializationArgumentFields=Object.freeze(['maxDependencies','maxRequestBytes','maxIdentifierBytes','maxInputNodes','maxInputDepth']);
const inputForm=Object.freeze({family:'F16',contract:'source-serialization/1',operation:'semantic-settings',variant:'semantic-settings',fields:Object.freeze(['contract','operation','invocation','profile','document','expectedSerialization','serialization','dependencies'])});
export function sourceSerializationInputForm(){return Object.freeze({...inputForm,fields:Object.freeze([...inputForm.fields])});}
const scopedForm=Object.freeze({family:'F16',contract:'scoped-source-serialization/1',operation:'semantic-settings',variant:'semantic-settings',fields:Object.freeze(['contract','operation','invocation','profile','serialization','dependencies'])});
export function scopedSourceSerializationInputForm(){return Object.freeze({...scopedForm,fields:Object.freeze([...scopedForm.fields])});}
export function sourceSerializationProfileValid(p){return exact(p,['argumentLimits','maxPlanBytes'])&&exact(p.argumentLimits,sourceSerializationArgumentFields)&&Object.values(p.argumentLimits).every(natural)&&natural(p.maxPlanBytes);}
export const serializationLabel=(v,p)=>typeof v==='string'&&v.length>0&&Buffer.byteLength(v)<=p.argumentLimits.maxIdentifierBytes&&Buffer.from(v).toString('utf8')===v;
export const serializationIdentity=(v,p)=>exact(v,['authority','id','incarnation'])&&serializationLabel(v.authority,p)&&serializationLabel(v.id,p)&&version(v.incarnation);
const validDependency=(d,p)=>exact(d,['domain','identity','generation'])&&serializationLabel(d.domain,p)&&serializationLabel(d.identity,p)&&version(d.generation);
export function sourceSerializationDependencies(request,h,p){
 if(!sourceSerializationProfileValid(p)||!Array.isArray(request?.dependencies)||request.dependencies.length>p.argumentLimits.maxDependencies||!Array.isArray(h?.requiredDependencies)||h.requiredDependencies.length>p.argumentLimits.maxDependencies||!Array.isArray(h.contexts)||h.contexts.length>p.argumentLimits.maxInputNodes)return null;
 const input=copySettingsData({required:h.requiredDependencies,extra:request.dependencies,current:h.contexts},{...p.argumentLimits,maxRequestBytes:p.maxPlanBytes});
 if(!input||[...input.value.required,...input.value.extra,...input.value.current].some(d=>!validDependency(d,p))||!mergeDependencies(input.value.required,input.value.extra,input.value.current,p.argumentLimits.maxDependencies))return null;
 return [...new Map([...input.value.required,...input.value.extra].map(d=>[JSON.stringify([d.domain,d.identity]),d])).values()].sort((a,b)=>JSON.stringify(a).localeCompare(JSON.stringify(b)));
}

const nat=natural,profile='strict-utf8-one-bom/1';
const gates=['authorized','scope','continuous','live','notStopped','interpretationCurrent','companionsReady','settingsWritable','resourcesReady'];
export function decideSerialization(before,request,c,p,g){
 if(!sourceSerializationProfileValid(p)||!exact(c,['authority','engine','recipient','purpose','destination','profile','continuity'])||!Object.values(c).every(v=>serializationLabel(v,p)))return null;
 const normalized=copySettingsData(request,p.argumentLimits);if(!normalized)return null;const r=normalized.value;
 if(!exact(r,inputForm.fields)||r.contract!==inputForm.contract||r.operation!==inputForm.operation||!serializationLabel(r.invocation,p)||r.profile!==c.profile||!serializationIdentity(r.document,p)||r.document.authority!==before.authority||c.authority!==before.authority||!version(r.expectedSerialization)||!exact(r.serialization,['profile','bom'])||r.serialization.profile!==profile||typeof r.serialization.bom!=='boolean'||!Array.isArray(r.dependencies)||r.dependencies.length>p.argumentLimits.maxDependencies||r.dependencies.some(d=>!exact(d,['domain','identity','generation'])||!serializationLabel(d.domain,p)||!serializationLabel(d.identity,p)||!version(d.generation)))return null;
 if(!gates.every(k=>g?.[k]===true)||!equal(g.sourceSerializationContext,c)||!equal(g.sourceSerializationProfile,p)||g.sourceSerializationBasis!==before)return null;
 const grant=copySettingsData(g.serializationWriteContext,p.argumentLimits)?.value;
 if(!exact(grant,['context','document','writeAllowed'])||!equal(grant.context,c)||!equal(grant.document,r.document)||grant.writeAllowed!==true)return null;
 const found=before.documents.filter(d=>equal(d.document,r.document));if(found.length!==1)return null;const d=found[0],s=d.serialization;
 if(d.live===false||!exact(s,['generation','profile','bom'])||!version(s.generation)||s.profile!==profile||typeof s.bom!=='boolean'||s.generation!==r.expectedSerialization)return null;
 if(s.bom===r.serialization.bom)return {status:'unchanged',state:before};
 if(g.versionsAvailable!==true||!nat(before.publication)||before.publication===Number.MAX_SAFE_INTEGER||s.generation===Number.MAX_SAFE_INTEGER)return null;
 const next=structuredClone(before),target=next.documents.find(d=>equal(d.document,r.document));
 target.serialization={generation:s.generation+1,...r.serialization};next.publication++;
 return {status:'applied',state:next};
}
export const sourceSerializationGates=Object.freeze(gates);
