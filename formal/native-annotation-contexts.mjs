// Selected native F02/F03 owner; not a public wire codec or async result owner.
import {isDeepStrictEqual as equal} from 'node:util';
import fs from 'node:fs';
import {copySettingsData} from './semantic-settings.mjs';
import {readSourceProduct} from './source-products.mjs';
import {logicalLines} from './reference-profile.mjs';
import {singleReadAcquisitionResultValid} from './read-contexts.mjs';
import {captureAnnotations,readAnnotations,annotationQueryResultValid} from './annotation-queries.mjs';
import {validateNativeAnnotationCapture,inspectNativeAnnotationCapture,consumeNativeAnnotationCapture} from './native-annotation-capture.mjs';
const pools=new WeakMap(),fields=['annotationProfile','maxContexts','maxSelectors','maxHolds','maxIssued','maxCaptureBytes','maxContextBytes','maxRequestBytes','maxReplyBytes','maxSelectorBytes','maxSelectorRecordBytes','maxInputNodes','maxInputDepth','maxHeldBytes'];
const annotationFields=['maxInputRows','maxCaptureBytes','maxRowBytes','maxPageRows'];
const queryCatalog=JSON.parse(fs.readFileSync(new URL('./annotation-queries.json',import.meta.url)));
const sourceCatalog=JSON.parse(fs.readFileSync(new URL('./source-products.json',import.meta.url)));
const natural=n=>Number.isSafeInteger(n)&&n>=0,yes=(g,ks)=>ks.every(k=>g?.[k]===true),discharge=['ownerControlAllowed','retentionReleased','deliveryDrained'],no=()=>({status:'unavailable'});
const exact=(v,ks)=>{if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(d);return keys.length===ks.length&&ks.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable);};
const copy=(v,p,n)=>copySettingsData(v,{maxRequestBytes:n,maxInputNodes:p.maxInputNodes,maxInputDepth:p.maxInputDepth});
const token=kind=>Object.freeze({kind});
function pool(owner,domain,handle){const m=pools.get(owner);return m&&m.domain===domain&&m.handle===handle&&!m.closed?m:null;}
export function openNativeAnnotationContexts(owner,domain,p,g){
 if(pools.has(owner)||owner.faulted||!exact(p,fields)||!fields.filter(k=>k!=='annotationProfile').every(k=>natural(p[k]))||!yes(g,['ownerControlAllowed','resourcesReady','preparationReserved']))return null;
 const captured=copy(p,p,p.maxRequestBytes);if(!captured||!exact(captured.value.annotationProfile,annotationFields)||!annotationFields.every(k=>natural(captured.value.annotationProfile[k])))return null;
 const charge=captured.bytes+2*p.maxRequestBytes+3*p.maxCaptureBytes+2*p.maxContextBytes+3*p.maxReplyBytes+p.maxContexts*(2*p.maxCaptureBytes+p.maxContextBytes+p.maxReplyBytes)+(2*p.maxSelectors+p.maxHolds)*(p.maxSelectorBytes+p.maxSelectorRecordBytes);
 if(!natural(charge)||charge>p.maxHeldBytes||!natural(owner.charge)||charge>owner.limits.maxHeldBytes-owner.charge)return null;
 const handle=token('native-annotation-context-pool/1');pools.set(owner,{domain,handle,profile:captured.value,charge,contexts:new Map(),selectors:new Map(),holds:new Map(),issued:0,closed:false});owner.charge+=charge;return handle;
}
export function installNativeAnnotationContext(owner,domain,handle,capturePool,cut,c,g){
 const m=pool(owner,domain,handle);if(!m)return no();const p=m.profile,facts=copy(g,p,p.maxRequestBytes)?.value;if(!facts||facts.outsidePublicationGuard!==true)return no();g=facts;
 if(!validateNativeAnnotationCapture(owner,domain,capturePool,cut,c,g))return no();
 if(owner.faulted||!yes(g,['preparationReserved','resourcesReady'])||m.contexts.size>=p.maxContexts||m.selectors.size>=p.maxSelectors||m.issued>=p.maxIssued)return no();
 const captured=inspectNativeAnnotationCapture(owner,domain,capturePool,cut,{ownerControlAllowed:true}),value=captured&&copy(captured,p,p.maxCaptureBytes)?.value,scope=copy(c,p,p.maxContextBytes)?.value;if(!value||!scope)return no();
 const context=token('native-annotation-read-context/1'),record={kind:'context',context},reply={status:'context',context,data:[...value.data]};
 if(!singleReadAcquisitionResultValid(reply,{contract:'read-context/1',data:value.data,selection:{kind:'capture-current'}},context)){owner.faulted=true;return no();}
 if(!copy(context,p,p.maxSelectorBytes)||!copy(record,p,p.maxSelectorRecordBytes)||!copy(reply,p,p.maxReplyBytes))return no();
 if(value.source){const source=value.source,probe=readSourceProduct({ref:source.ref,source:source.bytes,selector:context,profile:scope.profile},{contract:'source-query/1',operation:'bounded-range',document:scope.document,snapshot:context,profile:scope.profile,lo:0,hi:0},{grant:true,scope:true,continuity:true,capacity:true},source.profile);if(probe.status!=='content')return no();}
 const width=String(value.annotations.length).length,world=value.annotations.map((row,index)=>({reference:String(index).padStart(width,'0'),visible:true,bodyVisible:Object.hasOwn(row,'content'),targetVisible:Object.hasOwn(row,'location'),body:row.content?.body,contentVersion:row.content?.version,location:row.location})),privateScope={...scope,snapshot:String(m.issued+1)},annotations=captureAnnotations(world,privateScope,p.annotationProfile);
 if(!annotations||!copy(annotations,p,p.maxCaptureBytes))return no();
 // Stage complete registry candidates before consuming the private cut. Inert
 // current facts and a frozen reply leave only local nonallocating assignments
 // after consumption under this synchronous reference's no-reentrancy premise.
 Object.freeze(reply);
 try{
  const candidateContexts=new Map(m.contexts),candidateSelectors=new Map(m.selectors);
  candidateContexts.set(context,{scope,value,annotations,records:new Map(),continuations:new Map(),holds:0,stopped:false});candidateSelectors.set(context,record);
  const consumed=consumeNativeAnnotationCapture(owner,domain,capturePool,cut,c,g);if(!consumed)return no();
  m.contexts=candidateContexts;m.selectors=candidateSelectors;m.issued++;return reply;
 }catch(error){owner.faulted=true;throw error;}
}
// Existing F02 retained-selection lowering. No caller root or provider callback.
export function acquireNativeRetainedAnnotationContext(owner,domain,handle,request,c,g){
 const m=pool(owner,domain,handle);if(!m||!exact(request,['contract','operation','document','selection','data','profile'])||!exact(request.selection,['kind','reference'])||request.selection.kind!=='retained')return no();
 const p=m.profile,reference=request.selection.reference,r=copy({...request,selection:{kind:'retained',reference:null}},p,p.maxRequestBytes)?.value,facts=copy(g,p,p.maxRequestBytes)?.value,original=m.contexts.get(reference);
 if(!r||!facts||facts.outsidePublicationGuard!==true||!original||original.stopped||r.contract!=='read-context/1'||r.operation!=='snapshot-handoff'||!equal(r.document,original.scope.document)||r.profile!==original.scope.profile||!Array.isArray(r.data)||r.data.length<1||r.data.length>2||new Set(r.data).size!==r.data.length||r.data.some(k=>!original.value.data.includes(k)))return no();
 // Original whole-projection authority precedes new admission capacity. A
 // permitted narrowing never supplies facets absent from the retained value.
 if(!currentContext(owner,m,original,c,facts))return no();
 if(!yes(facts,['preparationReserved','resourcesReady'])||m.contexts.size>=p.maxContexts||m.selectors.size>=p.maxSelectors||m.issued>=p.maxIssued)return no();
 const selected={...original.value,data:[...r.data].sort()};if(!r.data.includes('source'))delete selected.source;if(!r.data.includes('annotations'))delete selected.annotations;
 const value=copy(selected,p,p.maxCaptureBytes)?.value,scope=copy(c,p,p.maxContextBytes)?.value,annotations=r.data.includes('annotations')?copy({...original.annotations,scope:{...original.annotations.scope,snapshot:String(m.issued+1)}},p,p.maxCaptureBytes)?.value:null;
 if(!value||!scope||r.data.includes('annotations')&&!annotations)return no();
 const context=token('native-annotation-read-context/1'),record={kind:'context',context},reply={status:'context',context,data:[...value.data]};
 if(!singleReadAcquisitionResultValid(reply,{contract:'read-context/1',data:value.data,selection:request.selection},context)){owner.faulted=true;return no();}
 if(!copy(context,p,p.maxSelectorBytes)||!copy(record,p,p.maxSelectorRecordBytes)||!copy(reply,p,p.maxReplyBytes))return no();Object.freeze(reply);
 try{const contexts=new Map(m.contexts),selectors=new Map(m.selectors);contexts.set(context,{scope,value,annotations,records:new Map(),continuations:new Map(),holds:0,stopped:false});selectors.set(context,record);m.contexts=contexts;m.selectors=selectors;m.issued++;return reply;}catch(error){owner.faulted=true;throw error;}
}
// Trusted owner inspection only, not a recipient F03 product.
export function inspectNativeAnnotationContext(owner,domain,handle,context,g){const m=pool(owner,domain,handle),ctx=m?.contexts.get(context);return ctx&&g?.ownerControlAllowed===true?structuredClone(ctx.value):null;}
export function stopNativeAnnotationContext(owner,domain,handle,r,c,g){const m=pool(owner,domain,handle);if(!m||!exact(r,['contract','operation','context'])||r.contract!=='read-context/1'||r.operation!=='stop-observation'||g?.ownerControlAllowed!==true)return no();const ctx=m.contexts.get(r.context),scope=copy(c,m.profile,m.profile.maxContextBytes)?.value;if(!ctx||!scope||!equal(scope,ctx.scope))return no();ctx.stopped=true;return {status:'stopped'};}
export function retireNativeAnnotationContext(owner,domain,handle,context,g){const m=pool(owner,domain,handle),ctx=m?.contexts.get(context);if(!ctx?.stopped||ctx.holds!==0||!yes(g,discharge))return false;for(const [key,row]of m.selectors)if(row.context===context)m.selectors.delete(key);return m.contexts.delete(context);}
export function retireNativeAnnotationContexts(owner,domain,handle,g){const m=pool(owner,domain,handle);if(!m||m.contexts.size||m.selectors.size||!yes(g,discharge))return false;m.closed=true;m.profile=null;owner.charge-=m.charge;return true;}
export function nativeAnnotationContextUsage(owner){const m=pools.get(owner);return m?{closed:m.closed,contexts:m.contexts.size,selectors:m.selectors.size,holds:m.holds.size,stopped:[...m.contexts.values()].filter(c=>c.stopped).length,issued:m.issued,declaredHeldBytes:m.closed?0:m.charge}:null;}
function normalizedAnnotationQuery(m,r){
 if(!r||typeof r!=='object')return null;const op=Object.getOwnPropertyDescriptor(r,'operation');if(!op||!Object.hasOwn(op,'value')||typeof op.value!=='string'||!Object.hasOwn(queryCatalog.variants,op.value)||!exact(r,[...Object.keys(queryCatalog.common),...Object.keys(queryCatalog.variants[op.value])]))return null;
 const opaque=op.value==='enumerate'?'continuation':'annotation',value=copy({...r,snapshot:null,[opaque]:null},m.profile,m.profile.maxRequestBytes)?.value;return value?{...value,snapshot:r.snapshot,[opaque]:r[opaque]}:null;
}
function currentContext(owner,m,ctx,c,g){
 const scope=copy(c,m.profile,m.profile.maxContextBytes)?.value;if(!scope||!equal(scope,ctx.scope))return false;
 const required=ctx.value.data.includes('annotations')?['authorized','scope','continuous','live','notStopped','annotationReadable','projectionCurrent','retained']:['authorized','scope','continuous','live','notStopped','retained'],source=!!ctx.value.source,ds=owner.root.documents.filter(d=>equal(d.document,ctx.scope.document));
 if(owner.faulted||ds.length!==1||ds[0].live!==true||required.some(k=>g?.[k]===false)||source&&g?.sourceReadable===false){ctx.stopped=true;return false;}
 return yes(g,[...required,'resourcesReady','replyReady','transferReady'])&&(!source||g?.sourceReadable===true);
}
export function queryNativeAnnotationContext(owner,domain,handle,request,c,g){
 const m=pool(owner,domain,handle);if(!m||g?.outsidePublicationGuard!==true)return no();const r=normalizedAnnotationQuery(m,request),ctx=r&&m.contexts.get(r.snapshot);if(!ctx||ctx.stopped||!ctx.annotations||r.contract!==queryCatalog.contract||!equal(r.document,ctx.scope.document)||r.profile!==ctx.scope.profile)return no();
 let selected=null;
 if(r.operation==='enumerate'){if(!natural(r.limit)||r.limit<1||r.limit>m.profile.annotationProfile.maxPageRows)return no();if(r.continuation!==null){selected=m.selectors.get(r.continuation);if(selected?.kind!=='continuation'||selected.context!==r.snapshot||selected.limit!==r.limit)return no();}}
 else{selected=m.selectors.get(r.annotation);if(selected?.kind!=='record'||selected.context!==r.snapshot)return no();}
 if(!currentContext(owner,m,ctx,c,g))return no();
 const internal={...r,snapshot:ctx.annotations.scope.snapshot,...(r.operation==='enumerate'?{continuation:selected?r.continuation:null}:{annotation:selected.reference})},cursors=selected?.kind==='continuation'?[{selector:r.continuation,scope:ctx.annotations.scope,limit:selected.limit,next:selected.next}]:[];
 const out=readAnnotations(ctx.annotations,internal,{scope:ctx.annotations.scope,granted:true,retained:true,live:true,continuous:true,transferReady:true},cursors);if(out.status==='unavailable')return no();
 const pending=[],records=new Map(),cursorKey=out.status==='page'&&out.next!==null?JSON.stringify([r.limit,out.next]):null;
 if(out.status==='page')for(const row of out.records){const existing=ctx.records.get(row.reference);if(existing)records.set(row.reference,existing);else{const selector=token('native-annotation-record/1'),record={kind:'record',context:r.snapshot,reference:row.reference};pending.push({selector,record});records.set(row.reference,selector);}}
 let continuation=cursorKey===null?null:ctx.continuations.get(cursorKey);
 if(cursorKey!==null&&!continuation){continuation=token('native-annotation-continuation/1');pending.push({selector:continuation,record:{kind:'continuation',context:r.snapshot,limit:r.limit,next:out.next},cursorKey});}
 if(pending.length>m.profile.maxSelectors-m.selectors.size||pending.length>m.profile.maxIssued-m.issued||pending.some(p=>!copy(p.selector,m.profile,m.profile.maxSelectorBytes)||!copy(p.record,m.profile,m.profile.maxSelectorRecordBytes)))return no();
 const reply=out.status==='page'?{status:'page',snapshot:r.snapshot,records:out.records.map(row=>({...row,reference:records.get(row.reference)})),continuation:continuation??null}:out.status==='record'?{status:'record',snapshot:r.snapshot,record:{...out.record,reference:r.annotation}}:{status:'location',snapshot:r.snapshot,reference:r.annotation,location:out.location};
 if(!annotationQueryResultValid(reply,r)||out.status==='page'&&(reply.continuation!==(continuation??null)||reply.records.some((row,i)=>row.reference!==records.get(out.records[i].reference)))){owner.faulted=true;return no();}
 if(!copy(reply,m.profile,m.profile.maxReplyBytes))return no();
 try{for(const p of pending){m.selectors.set(p.selector,p.record);if(p.record.kind==='record')ctx.records.set(p.record.reference,p.selector);else ctx.continuations.set(p.cursorKey,p.selector);}m.issued+=pending.length;return Object.freeze(reply);}catch(error){owner.faulted=true;throw error;}
}
export function queryNativeCombinedSource(owner,domain,handle,request,c,g){
 const m=pool(owner,domain,handle);if(!m||g?.outsidePublicationGuard!==true||!request||typeof request!=='object')return no();
 const op=Object.getOwnPropertyDescriptor(request,'operation');if(!op||!Object.hasOwn(op,'value')||typeof op.value!=='string'||!Object.hasOwn(sourceCatalog.variants,op.value)||!exact(request,[...sourceCatalog.common,...sourceCatalog.variants[op.value]]))return no();
 const value=copy({...request,snapshot:null},m.profile,m.profile.maxRequestBytes)?.value,r=value&&{...value,snapshot:request.snapshot},ctx=r&&m.contexts.get(r.snapshot),source=ctx?.value.source;
 if(!ctx||ctx.stopped||!source||r.contract!==sourceCatalog.contract||!equal(r.document,ctx.scope.document)||r.profile!==ctx.scope.profile)return no();
 const xs=source.bytes,boundary=n=>natural(n)&&n<=xs.length&&(n===xs.length||(xs[n]&0xc0)!==0x80);
 if(r.operation==='bounded-range'?!(boundary(r.lo)&&boundary(r.hi)&&r.lo<=r.hi):!(natural(r.line)&&r.line<logicalLines(xs).length))return no();
 if(!currentContext(owner,m,ctx,c,g))return no();
 const out=readSourceProduct({ref:source.ref,source:xs,selector:r.snapshot,profile:ctx.scope.profile},r,{grant:true,scope:true,continuity:true,capacity:true},source.profile);
 return copy(out,m.profile,m.profile.maxReplyBytes)?Object.freeze(out):no();
}
// Fixed first-party F05 ownership ports, not public pin/provider callbacks.
export function nativeAnnotationContextPoolAvailable(owner,domain,handle){return !!pool(owner,domain,handle);}
export function locateNativeAnnotationContextPool(owner,domain,reference){const m=pools.get(owner);return m&&m.domain===domain&&!m.closed&&m.contexts.has(reference)?m.handle:null;}
export function nativeAnnotationStopTargetOwned(owner,domain,handle,reference){return locateNativeAnnotationContextPool(owner,domain,reference)===handle&&handle!==null;}
export function inspectNativeCombinedSearchContext(owner,domain,handle,reference,c,g){const m=pool(owner,domain,handle),ctx=m?.contexts.get(reference);if(!ctx||ctx.stopped||!ctx.value.source||g?.outsidePublicationGuard!==true||!currentContext(owner,m,ctx,c,g))return null;const source=ctx.value.source;return {snapshot:{ref:source.ref,source:source.bytes,selector:reference,profile:ctx.scope.profile},sourceProfile:source.profile};}
export function reserveNativeCombinedSearchLease(owner,domain,handle,reference,c,g){if(!inspectNativeCombinedSearchContext(owner,domain,handle,reference,c,g))return null;const m=pool(owner,domain,handle),ctx=m.contexts.get(reference);if(m.holds.size>=m.profile.maxHolds||m.issued>=m.profile.maxIssued)return null;const lease=token('native-combined-search-hold/1');if(!copy(lease,m.profile,m.profile.maxSelectorBytes)||!copy({context:reference,published:false},m.profile,m.profile.maxSelectorRecordBytes))return null;m.holds.set(lease,{context:reference,ctx,published:false});ctx.holds++;m.issued++;return lease;}
export function inspectNativeCombinedSearchLease(owner,domain,handle,lease,c,g){const m=pool(owner,domain,handle),held=m?.holds.get(lease);return held&&m.contexts.get(held.context)===held.ctx?inspectNativeCombinedSearchContext(owner,domain,handle,held.context,c,g):null;}
export function publishNativeCombinedSearchLease(owner,domain,handle,lease){const m=pool(owner,domain,handle),held=m?.holds.get(lease);if(!held||held.published||held.ctx.stopped||owner.faulted||m.contexts.get(held.context)!==held.ctx)return false;held.published=true;return true;}
function dischargeSearchLease(owner,domain,handle,lease,published,g){const m=pool(owner,domain,handle),held=m?.holds.get(lease);if(!held||held.published!==published||published&&!yes(g,discharge)||m.holds.get(lease)!==held)return false;m.holds.delete(lease);held.ctx.holds--;return true;}
export function discardNativeCombinedSearchLease(owner,domain,handle,lease){return dischargeSearchLease(owner,domain,handle,lease,false,null);}
export function retireNativeCombinedSearchLease(owner,domain,handle,lease,g){return dischargeSearchLease(owner,domain,handle,lease,true,g);}
