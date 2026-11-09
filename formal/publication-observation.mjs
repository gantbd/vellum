// Serialized recipient observer reference, outside publication's critical path.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {captureAnnotations} from './annotation-queries.mjs';
import {prepareOutcome,releaseOutcome} from './outcome-projection.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./publication-observation.json',import.meta.url))),owners=new WeakMap(),visibilityPackets=new WeakMap(),copy=structuredClone;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort()),size=v=>Buffer.byteLength(JSON.stringify(v)),natural=n=>Number.isSafeInteger(n)&&n>=0;
const scalar=s=>typeof s==='string'&&s.length>0&&new TextDecoder('utf-8',{fatal:true}).decode(Buffer.from(s))===s;
const identity=v=>exact(v,['authority','id','incarnation'])&&scalar(v.authority)&&scalar(v.id)&&natural(v.incarnation);
const identityKey=r=>JSON.stringify([r.authority,r.id,r.incarnation]);
const allowed=g=>catalog.gates.filter(k=>k!=='resourcesReady').every(k=>g?.[k]===true),unavailable=()=>({status:'unavailable'});
const outcomeFlags=['publication','control','effect','reasons','details','recovery'];
const annotationKey=(document,id,incarnation)=>JSON.stringify([document.authority,document.id,document.incarnation,id,incarnation]);
function contextValid(c,l){
 if(!exact(c,catalog.contextFields)||!['authority','recipient','purpose','destination','policy','continuity','profile'].every(k=>scalar(c[k]))||c.profile!==catalog.profile||!exact(c.outcomePolicy,catalog.outcomeFields)||!outcomeFlags.every(k=>typeof c.outcomePolicy[k]==='boolean')||c.outcomePolicy.details!==false||c.outcomePolicy.grant!==true||!natural(c.outcomePolicy.epoch)||c.outcomePolicy.recipient!==c.recipient||!Array.isArray(c.documents)||!c.documents.length||c.documents.length>l.maxDocuments)return false;
 const ids=[];for(const d of c.documents){if(!exact(d,catalog.documentFields)||!identity(d.document)||d.document.authority!==c.authority||typeof d.source!=='boolean'||!Array.isArray(d.annotations)||d.annotations.length>l.maxRows||!Array.isArray(d.views)||d.views.length>l.maxViews)return false;
  ids.push(identityKey(d.document));if(d.collection!==null&&(!exact(d.collection,['body','target'])||typeof d.collection.body!=='boolean'||typeof d.collection.target!=='boolean'||d.annotations.length)||d.annotations.some(a=>!exact(a,catalog.annotationFields)||!scalar(a.id)||!natural(a.incarnation)||!scalar(a.reference)||typeof a.body!=='boolean'||typeof a.target!=='boolean')||new Set(d.annotations.map(a=>a.reference)).size!==d.annotations.length||new Set(d.annotations.map(a=>JSON.stringify([a.id,a.incarnation]))).size!==d.annotations.length||d.views.some(v=>!exact(v,catalog.viewFields)||!identity(v.view)||v.view.authority!==c.authority||!scalar(v.reference))||new Set(d.views.map(v=>v.reference)).size!==d.views.length||new Set(d.views.map(v=>identityKey(v.view))).size!==d.views.length)return false;
 }return new Set(ids).size===ids.length&&size(c)<=l.maxProjectionBytes;
}
function current(o,c,g){if(!o||o.stopped||!allowed(g)||!contextValid(c,o.limits)||c.outcomePolicy.epoch!==o.context.outcomePolicy.epoch)return false;const fixed=copy(c);fixed.outcomePolicy=copy(o.context.outcomePolicy);return equal(fixed,o.context);}
// Native facade composition only: first-party lowered caller claims are not
// trusted loss evidence. No public payload/private-identity encoding is selected.
// Current scope/facts below remain independently attributed trusted host ports.
function originalClaim(o,claimed){
 if(!o||!contextValid(claimed,o.limits)||!(claimed.outcomePolicy.epoch===o.context.outcomePolicy.epoch))return false;
 return catalog.contextFields.every(key=>key==='outcomePolicy'||equal(claimed[key],o.context[key]));
}
// First-party native wrapper predicate, not a release or authority grant.
export function publicationObserverClaimMatches(owner,claimed){return originalClaim(owners.get(owner),claimed);}
function invalidate(o){if(o){o.stopped=true;o.queue=[];o.baseline=null;}return unavailable();}
export function prepareCollectionVisibility(root,c,rows,l,g){
 if(!contextValid(c,l)||root?.authority!==c.authority||!natural(root.publication)||!Array.isArray(root.documents)||size(root)>l.maxRootBytes||root.documents.length>l.maxRows||!Array.isArray(rows)||rows.length>l.maxDocuments*l.maxRows||size(rows)>l.maxRootBytes||!allowed(g)||!g.resourcesReady||!g.preparationReserved||!g.visibilityAttested||!g.visibilityComplete||!g.projectionOrderAttested)return null;
 const expected=[];for(const scope of c.documents.filter(d=>d.collection!==null)){const ds=root.documents.filter(d=>equal(d.document,scope.document));if(ds.length!==1||ds[0].live===false||!Array.isArray(ds[0].records)||ds[0].records.length>l.maxRows)return null;for(const r of ds[0].records)expected.push({document:scope.document,id:r.id,incarnation:r.incarnation});}
 if(rows.length!==expected.length||rows.some(r=>!exact(r,['document','id','incarnation','visible','body','target'])||!identity(r.document)||!scalar(r.id)||!natural(r.incarnation)||!['visible','body','target'].every(k=>typeof r[k]==='boolean')||!r.visible&&(r.body||r.target)))return null;
 const key=r=>annotationKey(r.document,r.id,r.incarnation);if(new Set(expected.map(key)).size!==expected.length||new Set(rows.map(key)).size!==rows.length||!expected.every(r=>rows.some(x=>key(x)===key(r))))return null;
 const handle=Object.freeze({kind:'collection-visibility/1'});visibilityPackets.set(handle,{root,publication:root.publication,signature:JSON.stringify(root),context:copy(c),rows:copy(rows)});return handle;
}
function collectionCurrent(root,c,packet){const v=visibilityPackets.get(packet);if(!v)return null;const scope=copy(c);scope.outcomePolicy=copy(v.context.outcomePolicy);return v.root===root&&v.publication===root.publication&&v.signature===JSON.stringify(root)&&v.context.outcomePolicy.epoch===c.outcomePolicy.epoch&&equal(v.context,scope)?v:null;}
export function retireCollectionVisibility(packet,g){if(!visibilityPackets.has(packet)||!g?.retentionReleased)return unavailable();visibilityPackets.delete(packet);return {status:'retired'};}
const draft=o=>({refs:new Map(o.refs),issued:o.issued});
function collectionReference(stage,key,l){if(stage.refs.has(key))return stage.refs.get(key);if(stage.refs.size>=l.maxBindings||stage.issued>=l.maxIssued)return null;const ref='annotation-'+(stage.refs.size+1),rows=[...stage.refs,[key,ref]];if(size(rows)>l.maxBindingBytes)return null;stage.refs.set(key,ref);stage.issued++;return ref;}
function project(root,c,l,stage,packet){
 if(root?.authority!==c.authority||!Array.isArray(root.documents)||!Array.isArray(root.views)||root.documents.length>l.maxRows||root.views.length>l.maxViews||size(root)>l.maxRootBytes)return null;
 const visibility=c.documents.some(d=>d.collection!==null)?collectionCurrent(root,c,packet):null;if(c.documents.some(d=>d.collection!==null)&&!visibility)return null;if(visibility)visibilityPackets.delete(packet);
 const documents=[];for(const d of c.documents){const rows=root.documents.filter(x=>equal(x.document,d.document));if(rows.length!==1||rows[0].live===false)return null;const source=rows[0];if(!Array.isArray(source.records)||source.records.length>l.maxRows)return null;
  const world=[];for(const a of d.annotations){const matches=source.records.filter(r=>r.id===a.id&&r.incarnation===a.incarnation);if(matches.length>1)return null;if(matches.length)world.push({reference:a.reference,visible:true,bodyVisible:a.body,targetVisible:a.target,body:matches[0].body,contentVersion:matches[0].bodyVersion,location:{placement:copy(matches[0].placement),targetGeneration:matches[0].targetGeneration}});}
  if(d.collection!==null){for(const fact of visibility.rows.filter(r=>equal(r.document,d.document)&&r.visible)){const r=source.records.find(r=>r.id===fact.id&&r.incarnation===fact.incarnation);if(!r)return null;const reference=collectionReference(stage,annotationKey(d.document,r.id,r.incarnation),l);if(!reference)return null;world.push({reference,visible:true,bodyVisible:d.collection.body&&fact.body,targetVisible:d.collection.target&&fact.target,body:r.body,contentVersion:r.bodyVersion,location:{placement:copy(r.placement),targetGeneration:r.targetGeneration}});}}
  const annotations=captureAnnotations(world,{recipient:c.recipient,policy:c.policy,profile:c.profile},{maxInputRows:l.maxRows,maxCaptureBytes:l.maxProjectionBytes,maxRowBytes:l.maxProjectionBytes,maxPageRows:l.maxRows});if(!annotations)return null;
  const views=[];for(const entry of d.views){const vs=root.views.filter(v=>equal(v.view,entry.view));if(vs.length!==1||!equal(vs[0].document,d.document)||vs[0].sourceRevision!==source.revision)return null;const v=vs[0];views.push({reference:entry.reference,selectionGeneration:v.selectionGeneration,profile:v.profile,selections:copy(v.selections),primary:v.primary});}views.sort((a,b)=>a.reference<b.reference?-1:a.reference>b.reference?1:0);
  documents.push({document:copy(d.document),...(d.source?{source:{revision:source.revision,bytes:copy(source.source)}}:{}),annotations:copy(annotations.rows),views});
 }const projection={documents};return size(projection)<=l.maxProjectionBytes?projection:null;
}
const token=()=>Object.freeze({kind:'recipient-generation/1'});
// Trusted internal F02 preparation only. Never a raw northbound snapshot product.
export function capturePublicationRead(owner,root,document,data,c,g,packet=null){
 const o=owners.get(owner);if(!current(o,c,g)||o.gap||!g.resourcesReady||!g.releaseAllowed||!g.retained||!g.transferReady||!Array.isArray(data)||!data.length||data.length>2||new Set(data).size!==data.length||data.some(k=>!['source','annotations'].includes(k)))return null;
 const scope=o.context.documents.find(d=>equal(d.document,document));if(!scope||data.includes('source')&&!scope.source||data.includes('annotations')&&scope.collection===null)return null;
 const stage=draft(o),projection=project(root,o.context,o.limits,stage,packet);if(!projection)return null;
 const selected=projection.documents.find(d=>equal(d.document,document)),source=root.documents.find(d=>equal(d.document,document));
 return {scope:{document:copy(document),recipient:c.recipient,purpose:c.purpose,destination:c.destination,policy:c.policy,continuity:c.continuity,profile:c.profile},
  root:{document:copy(document),sourceRevision:source.revision,source:copy(source.source),records:selected.annotations.map(r=>({reference:r.reference,visible:true,bodyVisible:Object.hasOwn(r,'content'),targetVisible:Object.hasOwn(r,'location'),body:r.content?.body,contentVersion:r.content?.version,location:copy(r.location)}))}};
}
const gap=o=>{o.gap=true;o.queue=[];return {status:'gap',baseline:'required'};};
function frameFits(o,p){return size({status:'projection',before:{kind:'recipient-generation/1'},after:{kind:'recipient-generation/1'},projection:p})<=o.limits.maxReplyBytes;}
export function openPublicationObserver(root,context,limits,g,packet=null){
 if(!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||!contextValid(context,limits)||!g?.resourcesReady||!allowed(g)||limits.maxIssued<1||limits.maxQueue<1)return null;
 const charge=2*limits.maxProjectionBytes+limits.maxQueue*limits.maxReplyBytes+(limits.maxOutcomes+limits.maxIssued)*limits.maxOutcomeBytes+2*limits.maxBindingBytes;if(!natural(charge)||charge>limits.maxHeldBytes)return null;
 const o={context:copy(context),limits:copy(limits),charge,baseline:null,generation:token(),issued:1,refs:new Map(),gap:false,stopped:false,queue:[],outcomes:new Map(),used:new Set()},stage=draft(o),baseline=project(root,context,limits,stage,packet);if(!baseline||!frameFits(o,baseline))return null;o.baseline=baseline;o.refs=stage.refs;o.issued=stage.issued;const owner=Object.freeze({profile:catalog.profile});owners.set(owner,o);return {owner,baseline:{status:'baseline',generation:o.generation,projection:copy(baseline)}};
}
export function observePublicationRoot(owner,root,c,g,packet=null){
 const o=owners.get(owner);if(!current(o,c,g))return invalidate(o);if(!g.resourcesReady){gap(o);return unavailable();}if(o.gap)return {status:'gap',baseline:'required'};const stage=draft(o),p=project(root,o.context,o.limits,stage,packet);if(!p)return gap(o);if(equal(p,o.baseline))return {status:'unchanged'};
 if(o.queue.length>=o.limits.maxQueue||stage.issued>=o.limits.maxIssued||!frameFits(o,p))return gap(o);const after=token();o.queue.push({before:o.generation,after,projection:p});o.baseline=p;o.generation=after;o.refs=stage.refs;o.issued=stage.issued+1;return {status:'queued'};
}
export function releasePublicationEvent(owner,c,g){
 const o=owners.get(owner);if(!current(o,c,g))return invalidate(o);if(!g.resourcesReady||!g.releaseAllowed||!g.retained||!g.transferReady)return unavailable();if(o.gap)return {status:'gap',baseline:'required'};const entry=o.queue.shift();return entry?{status:'projection',before:entry.before,after:entry.after,projection:copy(entry.projection)}:{status:'empty'};
}
export function releaseBoundPublicationEvent(owner,claimed,currentContext,g){
 if(!originalClaim(owners.get(owner),claimed))return unavailable();
 return releasePublicationEvent(owner,currentContext,g);
}
export function resyncPublicationObserver(owner,root,c,g,packet=null){
 const o=owners.get(owner);if(!current(o,c,g)||!g.resourcesReady||!g.releaseAllowed||!g.retained||!g.transferReady||!g.baselineAllowed)return unavailable();const stage=draft(o),p=project(root,o.context,o.limits,stage,packet);if(!p||!frameFits(o,p)||stage.issued>=o.limits.maxIssued)return unavailable();o.queue=[];o.baseline=p;o.generation=token();o.refs=stage.refs;o.issued=stage.issued+1;o.gap=false;return {status:'baseline',generation:o.generation,projection:copy(p)};
}
export function capturePublicationOutcome(owner,before,result,invocation,c,g){
 const o=owners.get(owner);if(!current(o,c,g)||!g.resourcesReady||!g.outcomeCaptureAllowed||!scalar(invocation)||!['applied','unchanged','not-published'].includes(result?.status)||!result.state||before?.authority!==o.context.authority||result.state.authority!==o.context.authority||size(before)>o.limits.maxRootBytes||size(result.state)>o.limits.maxRootBytes||o.used.has(invocation)||o.outcomes.size>=o.limits.maxOutcomes||o.issued>=o.limits.maxIssued)return null;
 if(result.status==='applied'?result.state===before||!natural(before.publication)||result.state.publication!==before.publication+1:result.state!==before)return null;
 const facts={publication:result.status==='not-published'?'notApplied':result.status,control:result.status==='not-published'?'rejected':'done',effect:'none',basis:'none',causes:[],participants:[],publicCorrelation:invocation};
 const prepared=prepareOutcome(facts,o.context.outcomePolicy);if(size(prepared)>o.limits.maxOutcomeBytes||size({status:'outcome',outcome:releaseOutcome(prepared,o.context.outcomePolicy)})>o.limits.maxReplyBytes)return null;const handle=Object.freeze({kind:'publication-outcome/1'});o.outcomes.set(handle,prepared);o.used.add(invocation);o.issued++;return handle;
}
export function releasePublicationOutcome(owner,handle,c,g){const o=owners.get(owner),p=o?.outcomes.get(handle);if(!current(o,c,g))return invalidate(o);if(!p||!g.resourcesReady||!g.releaseAllowed||!g.retained||!g.transferReady)return unavailable();const out={status:'outcome',outcome:releaseOutcome(p,c.outcomePolicy)};return size(out)<=o.limits.maxReplyBytes?out:unavailable();}
// Trusted producer facet only. Native composition supplies facts from an actual
// terminal invocation; no northbound caller supplies these authoritative facts.
export function captureNativePublicationFacts(owner,facts,invocation,c,g){
 // This existing observer profile carries a local publication summary, not
 // origin-recipient reason/participant classifications or generic effect truth.
 if(!facts||!['applied','unchanged','notApplied'].includes(facts.publication)||facts.effect!=='none'||facts.basis!=='none'||!Array.isArray(facts.causes)||facts.causes.length||!Array.isArray(facts.participants)||facts.participants.length)return null;
 const o=owners.get(owner);if(!current(o,c,g)||g.resourcesReady!==true||g.outcomeCaptureAllowed!==true||!scalar(invocation)||facts?.publicCorrelation!==invocation||o.used.has(invocation)||o.outcomes.size>=o.limits.maxOutcomes||o.issued>=o.limits.maxIssued)return null;
 const prepared=prepareOutcome(facts,o.context.outcomePolicy);if(size(prepared)>o.limits.maxOutcomeBytes||size({status:'outcome',outcome:releaseOutcome(prepared,o.context.outcomePolicy)})>o.limits.maxReplyBytes)return null;
 const handle=Object.freeze({kind:'publication-outcome/1'});o.outcomes.set(handle,prepared);o.used.add(invocation);o.issued++;return handle;
}
export function releaseBoundPublicationOutcome(owner,handle,claimed,currentContext,g){
 if(!originalClaim(owners.get(owner),claimed))return unavailable();
 return releasePublicationOutcome(owner,handle,currentContext,g);
}
export function retirePublicationOutcome(owner,handle,g){const o=owners.get(owner);if(!o?.outcomes.has(handle)||!g.retentionReleased||!g.deliveryDrained)return unavailable();o.outcomes.delete(handle);return {status:'retired'};}
export function stopPublicationObserver(owner,g){const o=owners.get(owner);if(!o||!g.ownerControlAllowed)return unavailable();o.stopped=true;o.queue=[];o.baseline=null;return {status:'stopped'};}
export function publicationObserverUsage(owner){const o=owners.get(owner);return o?{queued:o.queue.length,outcomes:o.outcomes.size,issued:o.issued,bindings:o.refs.size,gap:o.gap,stopped:o.stopped,declaredHeldBytes:o.charge}:null;}
