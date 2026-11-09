// Selected synchronous F05 child-result owner; no asynchronous worker protocol.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {literalSearchInputForm,searchLiteral,replacementResultFootprint,discardLiteralSearchResult} from './literal-search.mjs';
import {nativeSourceContextPoolAvailable,locateNativeSourceContextPool,inspectNativeSourceSearchContext,reserveNativeSourceSearchLease,inspectNativeSourceSearchLease,publishNativeSourceSearchLease,discardNativeSourceSearchLease,retireNativeSourceSearchLease} from './native-source-contexts.mjs';
import {nativeAnnotationContextPoolAvailable,locateNativeAnnotationContextPool,inspectNativeCombinedSearchContext,reserveNativeCombinedSearchLease,inspectNativeCombinedSearchLease,publishNativeCombinedSearchLease,discardNativeCombinedSearchLease,retireNativeCombinedSearchLease} from './native-annotation-contexts.mjs';
import {nativeVersionSetContextPoolAvailable,locateNativeVersionSetContextPool,inspectNativeVersionSetSearchContext,reserveNativeVersionSetSearchLease,inspectNativeVersionSetSearchLease,publishNativeVersionSetSearchLease,discardNativeVersionSetSearchLease,retireNativeVersionSetSearchLease} from './native-version-set-contexts.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./literal-search.json',import.meta.url))),form=literalSearchInputForm(),pools=new WeakMap();
const fields=['limits','maxResults','maxIssued','maxRequestBytes','maxCaptureBytes','maxSelectorBytes','maxInputNodes','maxInputDepth','maxHeldBytes'];
const resultOwners=new WeakMap();
const occurrenceResults=new WeakMap();
// Closed first-party issuer sum, not an extensible provider/callback interface.
const issuers=Object.freeze({
 source:{locate:locateNativeSourceContextPool,inspect:inspectNativeSourceSearchContext,reserve:reserveNativeSourceSearchLease,inspectLease:inspectNativeSourceSearchLease,publish:publishNativeSourceSearchLease,discard:discardNativeSourceSearchLease,retire:retireNativeSourceSearchLease},
 combined:{locate:locateNativeAnnotationContextPool,inspect:inspectNativeCombinedSearchContext,reserve:reserveNativeCombinedSearchLease,inspectLease:inspectNativeCombinedSearchLease,publish:publishNativeCombinedSearchLease,discard:discardNativeCombinedSearchLease,retire:retireNativeCombinedSearchLease},
 versionSet:{locate:locateNativeVersionSetContextPool,inspect:inspectNativeVersionSetSearchContext,reserve:reserveNativeVersionSetSearchLease,inspectLease:inspectNativeVersionSetSearchLease,publish:publishNativeVersionSetSearchLease,discard:discardNativeVersionSetSearchLease,retire:retireNativeVersionSetSearchLease}
});
function locateIssuer(owner,domain,reference){for(const kind of ['source','combined','versionSet']){const handle=issuers[kind].locate(owner,domain,reference);if(handle)return {kind,handle};}return null;}
function inspectLease(owner,domain,issuer,lease,c,g){return issuers[issuer.kind].inspectLease(owner,domain,issuer.handle,lease,c,g);}
const flags=['grant','scope','continuity','capacity','retained','notStopped'],loss=['grant','scope','continuity','retained','notStopped'],discharge=['ownerControlAllowed','retentionReleased','deliveryDrained'];
export function nativeSourceSearchCurrentForm(){return Object.freeze({fields:Object.freeze(['request',...flags]),requestFields:form.fields});}
const exact=(v,ks)=>{if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),all=Reflect.ownKeys(d);return all.length===ks.length&&ks.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable===true);};
const natural=n=>Number.isSafeInteger(n)&&n>=0,yes=(g,ks)=>ks.every(k=>g?.[k]===true),no=()=>({status:'unavailable'}),token=kind=>Object.freeze({kind});
const copy=(v,p,maximum=p.maxRequestBytes)=>copySettingsData(v,{maxRequestBytes:maximum,maxInputNodes:p.maxInputNodes,maxInputDepth:p.maxInputDepth});
function pool(owner,domain,handle){const m=pools.get(owner);return m&&m.domain===domain&&m.handle===handle&&!m.closed?m:null;}
export function openNativeSourceSearch(owner,domain,sourceHandle,p,g){
 if(pools.has(owner)||owner.faulted||!exact(p,fields)||!fields.filter(k=>k!=='limits').every(k=>natural(p[k]))||!exact(p.limits,catalog.limits)||!catalog.limits.every(k=>natural(p.limits[k]))||!yes(g,['ownerControlAllowed','resourcesReady','preparationReserved'])||!(nativeSourceContextPoolAvailable(owner,domain,sourceHandle)||nativeAnnotationContextPoolAvailable(owner,domain,sourceHandle)||nativeVersionSetContextPoolAvailable(owner,domain,sourceHandle)))return null;
 const captured=copy(p,p);if(!captured||!exact(captured.value.limits,catalog.limits)||!Object.values(captured.value.limits).every(natural)||captured.value.limits.chunkBytes===0)return null;
 const l=captured.value.limits,charge=captured.bytes+2*p.maxRequestBytes+2*p.maxCaptureBytes+3*p.maxResults*(p.maxCaptureBytes+p.maxRequestBytes+l.maxReplyBytes+l.maxMatches*p.maxSelectorBytes);
 if(!natural(charge)||charge>p.maxHeldBytes||!natural(owner.charge)||charge>owner.limits.maxHeldBytes-owner.charge)return null;
 const handle=token('native-source-search-pool/1');pools.set(owner,{domain,sourceHandle,handle,profile:captured.value,charge,results:new Map(),issued:0,closed:false});owner.charge+=charge;return handle;
}
function packet(m,r,q){
 if(!exact(r,form.fields)||!exact(q,['request',...flags])||!exact(q.request,form.fields)||q.request.context!==r.context)return null;
 const v=copy({request:{...r,context:null},current:{...q,request:{...q.request,context:null}}},m.profile)?.value;if(!v)return null;
 const a=v.request;if(a.contract!==form.contract||!form.operations.includes(a.operation)||!equal(a,v.current.request)||!natural(a.generation)||!Array.isArray(a.scopes)||a.scopes.some(s=>!exact(s,['lo','hi'])||!natural(s.lo)||!natural(s.hi)||s.lo>s.hi))return null;
 return {request:{...a,context:r.context},current:{...v.current,request:{...v.current.request,context:r.context}}};
}
export function queryNativeSourceSearch(owner,domain,handle,r,c,q,g){
 const m=pool(owner,domain,handle);if(!m||owner.faulted||g?.outsidePublicationGuard!==true)return no();const v=packet(m,r,q);
 const capturedScope=copy(c,m.profile,m.profile.maxCaptureBytes)?.value;
 if(!v||!capturedScope||!exact(v.request.ref,['authority','document','incarnation','revision'])||!natural(v.request.ref.revision)||v.request.ref.authority!==capturedScope.document?.authority||v.request.ref.document!==capturedScope.document?.id||v.request.ref.incarnation!==capturedScope.document?.incarnation)return no();
 const issuer=locateIssuer(owner,domain,r.context);if(!issuer||!issuers[issuer.kind].inspect(owner,domain,issuer.handle,r.context,c,g)||!copy(r.context,m.profile,m.profile.maxSelectorBytes)||!yes(v.current,flags)||m.results.size>=m.profile.maxResults||m.issued>=m.profile.maxIssued)return no();
 const lease=issuers[issuer.kind].reserve(owner,domain,issuer.handle,r.context,c,g);if(!lease)return no();let published=false;
 try{
  const captured=inspectLease(owner,domain,issuer,lease,c,g);if(!captured||!copy(captured.snapshot,m.profile,m.profile.maxCaptureBytes))return no();
  const limits={...m.profile.limits,maxRootBytes:Math.min(m.profile.limits.maxRootBytes,captured.sourceProfile.maxRootBytes),maxInputBytes:Math.min(m.profile.limits.maxInputBytes,captured.sourceProfile.maxInputBytes),maxReplyBytes:Math.min(m.profile.limits.maxReplyBytes,captured.sourceProfile.maxReplyBytes)};
  m.issued++;const result=searchLiteral(captured.snapshot,v.request,v.current,limits);
  if(!replacementResultFootprint(result))return result;
  if(result.matches.some(row=>!copy(row.reference,m.profile,m.profile.maxSelectorBytes)))return {status:'exhausted'};
  if(!issuers[issuer.kind].publish(owner,domain,issuer.handle,lease))return no();
  const held={issuer,lease,request:v.request,scope:capturedScope,stopped:false};m.results.set(result,held);resultOwners.set(result,handle);for(const match of result.matches)occurrenceResults.set(match.reference,result);published=true;return result;
 }catch(e){owner.faulted=true;throw e;}finally{if(!published)issuers[issuer.kind].discard(owner,domain,issuer.handle,lease);}
}
export function releaseNativeSourceSearchResult(owner,domain,handle,result,c,q,g){
 const m=pool(owner,domain,handle),held=m?.results.get(result);if(!held||held.stopped||g?.outsidePublicationGuard!==true)return no();
 const s=copy(c,m.profile,m.profile.maxCaptureBytes)?.value,v=packet(m,held.request,q);if(!s||!equal(s,held.scope)||!v)return no();
 const source=inspectLease(owner,domain,held.issuer,held.lease,c,g);
 if(loss.some(k=>v.current[k]===false))held.stopped=true;
 return source&&!held.stopped&&yes(v.current,flags)?result:no();
}
export function retireNativeSourceSearchResult(owner,domain,handle,result,g){
 const m=pool(owner,domain,handle),held=m?.results.get(result);if(!held)return false;
 const facts=Object.fromEntries(discharge.map(k=>[k,g?.[k]===true]));
 if(!yes(facts,discharge)||m.results.get(result)!==held||!issuers[held.issuer.kind].retire(owner,domain,held.issuer.handle,held.lease,facts))return false;
 if(!discardLiteralSearchResult(result,facts)){owner.faulted=true;throw Error('native search result retirement lost its capture');}
 m.results.delete(result);return true;
}
export function retireNativeSourceSearch(owner,domain,handle,g){const m=pool(owner,domain,handle);if(!m||m.results.size||!yes(g,discharge))return false;m.closed=true;m.profile=null;owner.charge-=m.charge;return true;}
export function nativeSourceSearchUsage(owner){const m=pools.get(owner);return m?{closed:m.closed,results:m.results.size,invalidated:[...m.results.values()].filter(r=>r.stopped).length,issued:m.issued,declaredHeldBytes:m.closed?0:m.charge}:null;}
// Internal admission composition only. Existing unmanaged reference producers
// retain their own checks; they do not acquire native source-context guarantees.
export function nativeSourceSearchAllowsPreparation(owner,domain,result,q,g){
 const issuer=resultOwners.get(result);if(!issuer)return true;
 const m=pool(owner,domain,issuer);if(!m||!m.results.has(result))return false;
 const held=m.results.get(result);return releaseNativeSourceSearchResult(owner,domain,m.handle,result,held.scope,q,g)===result;
}
export function nativeSourceSearchAllowsSelectionPreparation(owner,domain,occurrence,q,g){
 const result=occurrenceResults.get(occurrence);return result===undefined||nativeSourceSearchAllowsPreparation(owner,domain,result,q,g);
}
