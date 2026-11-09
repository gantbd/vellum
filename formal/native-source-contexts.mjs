// Selected current/retained-source F02 issuer. Native references, not a wire codec.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {captureSourceProfile,readSourceProduct} from './source-products.mjs';
import {logicalLines} from './reference-profile.mjs';
import {singleReadAcquisitionResultValid} from './read-contexts.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./read-contexts.json',import.meta.url)));
const source=JSON.parse(fs.readFileSync(new URL('./source-products.json',import.meta.url)));
const pools=new WeakMap(),natural=n=>Number.isSafeInteger(n)&&n>=0;
const searchLeases=new WeakMap();
const fields=['sourceProfile','maxContexts','maxHolds','maxIssued','maxRootBytes','maxCaptureBytes','maxContextBytes','maxRequestBytes','maxReplyBytes','maxSelectorBytes','maxInputNodes','maxInputDepth','maxHeldBytes'];
const yes=(g,ks)=>ks.every(k=>g?.[k]===true),no=()=>({status:'unavailable'}),token=kind=>Object.freeze({kind});
const exact=(v,ks)=>{if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(d);return keys.length===ks.length&&ks.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable===true);};
const copy=(v,p,maxRequestBytes)=>copySettingsData(v,{maxRequestBytes,maxInputNodes:p.maxInputNodes,maxInputDepth:p.maxInputDepth});
const prep=['authorized','scope','continuous','live','notStopped','sourceReadable','outsidePublicationGuard','resourcesReady','preparationReserved','retentionReady','replyReady','transferReady'];
const release=['authorized','scope','continuous','live','notStopped','sourceReadable','outsidePublicationGuard','resourcesReady','retained','replyReady','transferReady'];
const loss=['authorized','scope','continuous','live','retained','sourceReadable'];
const discharge=['ownerControlAllowed','retentionReleased','deliveryDrained'];
function scope(owner,c,p){const v=copy(c,p,p.maxContextBytes)?.value;if(!v||!exact(v,catalog.scopeFields)||!exact(v.document,['authority','id','incarnation'])||v.document.authority!==owner.context.authority||!natural(v.document.incarnation)||v.document.incarnation<1||!['authority','id'].every(k=>typeof v.document[k]==='string'&&v.document[k].length)||!catalog.scopeFields.filter(k=>k!=='document').every(k=>typeof v[k]==='string'&&v[k].length))return null;return v;}
function pool(owner,domain,handle){const m=pools.get(owner);return m&&m.domain===domain&&m.handle===handle&&!m.closed?m:null;}
function rootLive(owner,document){const ds=owner.root?.documents?.filter(d=>equal(d.document,document));return ds?.length===1&&ds[0].live===true?ds[0]:null;}
function close(ctx,invalidated=false){ctx.stopped=true;ctx.invalidated ||= invalidated;if(ctx.holds===0)ctx.snapshot=null;}
export function openNativeSourceContexts(owner,domain,p,g){
 if(pools.has(owner)||owner.faulted||!exact(p,fields)||!fields.filter(k=>k!=='sourceProfile').every(k=>natural(p[k]))||!exact(p.sourceProfile,source.profileFields)||!source.profileFields.every(k=>natural(p.sourceProfile[k]))||!yes(g,['ownerControlAllowed','resourcesReady','preparationReserved']))return null;
 const captured=copy(p,p,p.maxRequestBytes);if(!captured||!captureSourceProfile(captured.value.sourceProfile))return null;
 // Input/copy workspace is independent of retained context count, including zero.
 // The actual parent separately owns its bounded authoritative/candidate root frames.
 const charge=captured.bytes+2*p.maxRequestBytes+p.maxCaptureBytes+2*p.maxContextBytes+2*p.maxReplyBytes+p.maxContexts*(2*p.maxCaptureBytes+p.maxContextBytes+p.maxReplyBytes)+(p.maxContexts+p.maxHolds)*p.maxSelectorBytes;
 if(!natural(charge)||charge>p.maxHeldBytes||!natural(owner.charge)||charge>owner.limits.maxHeldBytes-owner.charge)return null;
 const handle=token('native-source-context-pool/1');pools.set(owner,{domain,handle,profile:captured.value,charge,contexts:new Map(),holds:new Map(),issued:0,closed:false});owner.charge+=charge;return handle;
}
export function acquireNativeSourceContext(owner,domain,handle,request,c,g){
 const m=pool(owner,domain,handle);if(!m||!exact(request,catalog.acquisition.fields))return no();const p=m.profile,s=scope(owner,c,p),selection=request.selection;
 const retained=exact(selection,['kind','reference'])&&selection.kind==='retained',reference=retained?selection.reference:null;
 const r=copy(retained?{...request,selection:{kind:'retained',reference:null}}:request,p,p.maxRequestBytes)?.value;
 if(!s||!r||r.contract!==catalog.contract||r.operation!=='snapshot-handoff'||!equal(r.document,s.document)||r.profile!==s.profile||!retained&&(!exact(r.selection,['kind'])||r.selection.kind!=='capture-current')||!equal(r.data,['source']))return no();
 // Retained acquisition resolves only an original context in this actual pool.
 // Its stored projection, never current bytes, supplies the new owned capture.
 const original=retained?searchSource(owner,domain,handle,reference,c,g):null;
 if(retained&&!original||owner.faulted||!yes(g,prep)||m.contexts.size>=p.maxContexts||m.issued>=p.maxIssued)return no();
 // The actual owner supplies the root; there is no caller-root/candidate argument.
 if(!retained&&Buffer.byteLength(JSON.stringify(owner.root))>p.maxRootBytes)return no();const d=retained?null:rootLive(owner,s.document);if(!retained&&!d)return no();
 const ref=retained?original.ctx.snapshot.ref:{authority:d.document.authority,document:d.document.id,incarnation:d.document.incarnation,revision:d.revision};
 const captured=copy({ref,source:retained?original.ctx.snapshot.source:d.source,profile:s.profile},p,p.maxCaptureBytes);if(!captured)return no();
 const context=token('native-source-read-context/1'),snapshot={...captured.value,selector:context};
 const probe=readSourceProduct(snapshot,{contract:source.contract,operation:'bounded-range',document:s.document,snapshot:context,profile:s.profile,lo:0,hi:0},{grant:true,scope:true,continuity:true,capacity:true},p.sourceProfile);
 const reply={status:'context',context,data:['source']};
 if(!singleReadAcquisitionResultValid(reply,request,context)){owner.faulted=true;return no();}
 if(probe.status!=='content'||!copy(context,p,p.maxSelectorBytes)||!copy(reply,p,p.maxReplyBytes))return no();
 m.contexts.set(context,{scope:s,snapshot,stopped:false,invalidated:false,holds:0});m.issued++;return Object.freeze(reply);
}
function normalizedQuery(m,r){
 if(!r||typeof r!=='object')return null;const op=Object.getOwnPropertyDescriptor(r,'operation');
 if(!op||!Object.hasOwn(op,'value')||typeof op.value!=='string'||!Object.hasOwn(source.variants,op.value)||!exact(r,[...source.common,...source.variants[op.value]]))return null;
 const v=copy({...r,snapshot:null},m.profile,m.profile.maxRequestBytes)?.value;return v?{...v,snapshot:r.snapshot}:null;
}
function queryShape(ctx,r){
 if(r.contract!==source.contract)return false;const xs=ctx.snapshot.source,boundary=n=>natural(n)&&n<=xs.length&&(n===xs.length||(xs[n]&0xc0)!==0x80);
 return r.operation==='bounded-range'?boundary(r.lo)&&boundary(r.hi)&&r.lo<=r.hi:natural(r.line)&&r.line<logicalLines(xs).length;
}
export function queryNativeSourceContext(owner,domain,handle,request,c,g){
 const m=pool(owner,domain,handle);if(!m||g?.outsidePublicationGuard!==true)return no();const r=normalizedQuery(m,request),s=scope(owner,c,m.profile),ctx=r&&m.contexts.get(r.snapshot);
 if(!ctx||ctx.stopped||!s||!equal(s,ctx.scope)||!equal(r.document,s.document)||r.profile!==s.profile||!queryShape(ctx,r))return no();
 // Envelope and caller binding precede accepted bound-loss observations.
 if(owner.faulted||loss.some(k=>g?.[k]===false)||!rootLive(owner,s.document)){close(ctx,true);return no();}
 if(!yes(g,release))return no();
 const out=readSourceProduct(ctx.snapshot,r,{grant:true,scope:true,continuity:true,capacity:true},m.profile.sourceProfile);
 return copy(out,m.profile,m.profile.maxReplyBytes)?Object.freeze(out):no();
}
export function stopNativeSourceContext(owner,domain,handle,request,c,g){
 const m=pool(owner,domain,handle);if(!m||!exact(request,catalog.close.fields)||request.contract!==catalog.contract||request.operation!=='stop-observation'||g?.ownerControlAllowed!==true)return no();
 const ctx=m.contexts.get(request.context),s=scope(owner,c,m.profile);if(!ctx||!s||!equal(ctx.scope,s))return no();close(ctx);return {status:'stopped'};
}
// Trusted internal lifetime operations. Selector possession is not worker authority.
export function holdNativeSourceContext(owner,domain,handle,reference,g){
 const m=pool(owner,domain,handle),ctx=m?.contexts.get(reference);if(!ctx||ctx.stopped||owner.faulted||!rootLive(owner,ctx.scope.document)||!yes(g,['ownerControlAllowed','retained','resourcesReady'])||m.holds.size>=m.profile.maxHolds||m.issued>=m.profile.maxIssued)return null;
 const ticket=token('native-source-context-hold/1');if(!copy(ticket,m.profile,m.profile.maxSelectorBytes))return null;m.holds.set(ticket,reference);ctx.holds++;m.issued++;return ticket;
}
export function releaseNativeSourceContextHold(owner,domain,handle,ticket,g){
 const m=pool(owner,domain,handle),reference=m?.holds.get(ticket),ctx=m?.contexts.get(reference);if(!ctx||!yes(g,discharge)||m.holds.get(ticket)!==reference||m.contexts.get(reference)!==ctx)return false;m.holds.delete(ticket);ctx.holds--;if(ctx.stopped&&ctx.holds===0)ctx.snapshot=null;return true;
}
export function retireNativeSourceContext(owner,domain,handle,reference,g){
 const m=pool(owner,domain,handle),ctx=m?.contexts.get(reference);if(!ctx?.stopped||ctx.holds!==0||!yes(g,discharge))return false;return m.contexts.delete(reference);
}
export function retireNativeSourceContexts(owner,domain,handle,g){
 const m=pool(owner,domain,handle);if(!m||m.contexts.size||m.holds.size||!yes(g,discharge))return false;m.closed=true;m.profile=null;owner.charge-=m.charge;return true;
}
export function nativeSourceContextUsage(owner){const m=pools.get(owner);return m?{closed:m.closed,contexts:m.contexts.size,holds:m.holds.size,retainedRoots:[...m.contexts.values()].filter(c=>c.snapshot!==null).length,invalidated:[...m.contexts.values()].filter(c=>c.invalidated).length,issued:m.issued,declaredHeldBytes:m.closed?0:m.charge}:null;}
// Private F05 composition. No public pin or caller-root/candidate interface.
export function nativeSourceContextPoolAvailable(owner,domain,handle){return !!pool(owner,domain,handle);}
export function locateNativeSourceContextPool(owner,domain,reference){const m=pools.get(owner);return m&&m.domain===domain&&!m.closed&&m.contexts.has(reference)?m.handle:null;}
export function nativeSourceStopTargetOwned(owner,domain,handle,reference){return locateNativeSourceContextPool(owner,domain,reference)===handle&&handle!==null;}
function searchSource(owner,domain,handle,reference,c,g){
 const m=pool(owner,domain,handle),ctx=m?.contexts.get(reference),s=m&&scope(owner,c,m.profile);
 if(!ctx||ctx.stopped||!s||!equal(ctx.scope,s)||g?.outsidePublicationGuard!==true)return null;
 if(owner.faulted||loss.some(k=>g?.[k]===false)||!rootLive(owner,s.document)){close(ctx,true);return null;}
 return yes(g,release)?{m,ctx}:null;
}
export function reserveNativeSourceSearchLease(owner,domain,handle,reference,c,g){
 const bound=searchSource(owner,domain,handle,reference,c,g);if(!bound)return null;const {m,ctx}=bound;
 if(m.holds.size>=m.profile.maxHolds||m.issued>=m.profile.maxIssued)return null;
 const lease=token('native-source-context-hold/1');if(!copy(lease,m.profile,m.profile.maxSelectorBytes))return null;
 m.holds.set(lease,reference);ctx.holds++;m.issued++;searchLeases.set(lease,{owner,domain,handle,reference,ctx,published:false});return lease;
}
export function inspectNativeSourceSearchContext(owner,domain,handle,reference,c,g){const bound=searchSource(owner,domain,handle,reference,c,g);return bound?{snapshot:bound.ctx.snapshot,sourceProfile:bound.m.profile.sourceProfile}:null;}
export function inspectNativeSourceSearchLease(owner,domain,handle,lease,c,g){
 const held=searchLeases.get(lease);if(!held||held.owner!==owner||held.domain!==domain||held.handle!==handle)return null;
 const bound=searchSource(owner,domain,handle,held.reference,c,g);return bound&&bound.ctx===held.ctx&&bound.m.holds.get(lease)===held.reference?{snapshot:held.ctx.snapshot,sourceProfile:bound.m.profile.sourceProfile}:null;
}
export function publishNativeSourceSearchLease(owner,domain,handle,lease){const h=searchLeases.get(lease),m=pool(owner,domain,handle);if(!h||h.owner!==owner||h.domain!==domain||h.handle!==handle||h.published||h.ctx.stopped||m?.holds.get(lease)!==h.reference)return false;h.published=true;return true;}
function releaseSearchLease(owner,domain,handle,lease,published,g){
 const h=searchLeases.get(lease),m=pool(owner,domain,handle);if(!h||h.owner!==owner||h.domain!==domain||h.handle!==handle||h.published!==published||m?.holds.get(lease)!==h.reference||published&&!yes(g,discharge)||searchLeases.get(lease)!==h||m?.holds.get(lease)!==h.reference)return false;
 m.holds.delete(lease);h.ctx.holds--;if(h.ctx.stopped&&h.ctx.holds===0)h.ctx.snapshot=null;searchLeases.delete(lease);return true;
}
export function discardNativeSourceSearchLease(owner,domain,handle,lease){return releaseSearchLease(owner,domain,handle,lease,false,null);}
export function retireNativeSourceSearchLease(owner,domain,handle,lease,g){return releaseSearchLease(owner,domain,handle,lease,true,g);}
