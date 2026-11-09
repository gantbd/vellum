// Existing F02 registry/native F05 composition only; not a native context issuer.
import {copySettingsData} from './semantic-settings.mjs';
import {captureReadContextSearchSource} from './read-contexts.mjs';
import {literalSearchInputForm,searchLiteral} from './literal-search.mjs';
const form=literalSearchInputForm(),known=['literal-matches','next-match','previous-match'];
if(form.family!=='F05'||form.contract!=='literal-search/1'||form.operations.length!==3||new Set(form.operations).size!==3||form.operations.some(k=>!known.includes(k)))throw Error('unsupported search declaration source');
const profileFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
function profileData(p){if(!p||typeof p!=='object'||![Object.prototype,null].includes(Object.getPrototypeOf(p)))return null;const d=Object.getOwnPropertyDescriptors(p),keys=Reflect.ownKeys(d);if(keys.length!==3||keys.some(k=>typeof k!=='string'||!profileFields.includes(k)||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true||!Number.isSafeInteger(d[k].value)||d[k].value<0))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
export function searchCapturedReadContext(registry,request,current,searchCurrent,searchLimits,inputLimits){
 const no=()=>({state:registry,result:{status:'unavailable'}}),bounds=profileData(inputLimits);if(!bounds)return no();
 const copied=copySettingsData({request,current,searchCurrent,searchLimits},bounds);if(!copied)return no();const {request:r,current:c,searchCurrent:queryCurrent,searchLimits:queryLimits}=copied.value;
 if(!r||typeof r!=='object'||Array.isArray(r)||!c||typeof c!=='object'||Array.isArray(c)||!queryCurrent||typeof queryCurrent!=='object'||!queryLimits||typeof queryLimits!=='object')return no();
 if(Object.keys(r).length!==form.fields.length||form.fields.some(k=>!Object.hasOwn(r,k))||r.contract!==form.contract||!form.operations.includes(r.operation))return no();
 // Native sorting assumes normalized numeric ranges; copying data alone is not normalization.
 if(!Array.isArray(r.scopes)||r.scopes.some(s=>!s||typeof s!=='object'||Array.isArray(s)||Object.keys(s).length!==2||!Object.hasOwn(s,'lo')||!Object.hasOwn(s,'hi')||!Number.isSafeInteger(s.lo)||s.lo<0||!Number.isSafeInteger(s.hi)||s.hi<0))return no();
 if(!['grant','scope','continuity','capacity','retained','notStopped'].every(k=>queryCurrent[k]===true))return no();
 const captured=captureReadContextSearchSource(registry,r.context,c);if(!captured)return no();
 const effectiveLimits={...queryLimits,maxRootBytes:Math.min(queryLimits.maxRootBytes,captured.sourceProfile.maxRootBytes),maxInputBytes:Math.min(queryLimits.maxInputBytes,captured.sourceProfile.maxInputBytes),maxReplyBytes:Math.min(queryLimits.maxReplyBytes,captured.sourceProfile.maxReplyBytes)};
 // Native query guards retain exact ref/profile/scope/completeness semantics.
 const result=searchLiteral(captured.snapshot,r,queryCurrent,effectiveLimits);
 return {state:registry,result};
}
