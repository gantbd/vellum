// Current serialized-root lifecycle reference, not a filesystem or physical reclaimer.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {utf8Step,utf8AtEnd} from './source-codec.mjs';
import {normalizeSelection} from './selection-products.mjs';
import {sourceSaveDestinationResolved,sourceSaveRegistryAuthority} from './source-save.mjs';
import {pairRootValid,pairExact,pairNatural,pairIdentity} from './binding-pair-profile.mjs';
import {annotationLoadedBaselineShape} from './annotation-loaded-baseline-state.mjs';
import {membershipInventoryActive,membershipInventoryValid} from './annotation-membership.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./lifecycle-products.json',import.meta.url),'utf8'));
// Selected public F17 forms from the same field lists used by the native guard.
// Retained binding/provenance discharge is private, not an additional public verb.
export function documentLifecycleInputForms(){return Object.freeze([
 ['create-scratch','create-scratch'],['attach-detach-view','attach-view'],
 ['attach-detach-view','detach-view'],['membership','membership'],['dispose','dispose']
].map(([variant,operation])=>Object.freeze({family:'F17',variant,contract:catalog.contract,operation,fields:Object.freeze([...catalog.common,...catalog.variants[operation]])})));}
const annotationBaselines=JSON.parse(fs.readFileSync(new URL('./annotation-baselines.json',import.meta.url),'utf8'));
const copy=structuredClone,natural=n=>Number.isSafeInteger(n)&&n>=0,size=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const scalar=s=>typeof s==='string'&&Buffer.from(s,'utf8').toString('utf8')===s;
const identity=r=>exact(r,['authority','id','incarnation'])&&scalar(r.authority)&&r.authority.length>0&&scalar(r.id)&&r.id.length>0&&natural(r.incarnation);
const identityKey=r=>JSON.stringify([r.authority,r.id,r.incarnation]);
const allowed=(g,ks)=>ks.every(k=>g?.[k]);
const lifecycle=d=>d.lifecycle??(d.live===true?{generation:0,phase:'live',retainedBinding:null}:null);
const safeIncrement=n=>natural(n)&&n<Number.MAX_SAFE_INTEGER;
const validBytes=(xs,l)=>{if(!Array.isArray(xs)||xs.length>l.maxSourceBytes||Object.keys(xs).length!==xs.length||!Object.keys(xs).every((k,i)=>k===String(i)))return false;let s='start';for(const b of xs){s=utf8Step(s,b);if(s==='failed')return false;}return utf8AtEnd(s);};
function rootValid(state,l){return scalar(state?.authority)&&Array.isArray(state.documents)&&state.documents.every(d=>identity(d.document)&&d.document.authority===state.authority&&typeof d.live==='boolean')&&new Set(state.documents.map(d=>identityKey(d.document))).size===state.documents.length&&Array.isArray(state.retiredDocuments)&&state.retiredDocuments.every(identity)&&Array.isArray(state.resourceOwners)&&Array.isArray(state.views)&&Array.isArray(state.workspaces)&&state.documents.length<=l.maxDocuments&&[state.retiredDocuments,state.resourceOwners,state.views,state.workspaces].every(xs=>xs.length<=l.maxRelations)&&size(state)<=l.maxRootBytes;}
export function lifecyclePublicationRootValid(state,l,g){return exact(l,catalog.limits)&&Object.values(l).every(natural)&&rootValid(state,l)&&membershipInventoryValid(state,g);}
function relationRows(state,l){return state.workspaces.every(w=>identity(w.workspace)&&w.workspace.authority===state.authority&&typeof w.live==='boolean'&&natural(w.generation)&&Array.isArray(w.members)&&w.members.length<=l.maxRelations&&w.members.every(identity)&&new Set(w.members.map(identityKey)).size===w.members.length)&&new Set(state.workspaces.map(w=>identityKey(w.workspace))).size===state.workspaces.length&&state.views.every(v=>identity(v.view)&&v.view.authority===state.authority&&natural(v.attachmentGeneration)&&natural(v.selectionGeneration)&&natural(v.viewportGeneration)&&(v.document===null||identity(v.document)))&&new Set(state.views.map(v=>identityKey(v.view))).size===state.views.length;}
function finish(state,next,l,g){if(equal(state,next))return {status:'unchanged',state};if(!safeIncrement(state.publication))return {status:'unavailable',state};next.publication++;return rootValid(next,l)&&membershipInventoryValid(next,g)?{status:'applied',state:next}:{status:'unavailable',state};}
function fresh(state,l){const a=state.identityAllocation;if(!exact(a,['namespace','next','last'])||!scalar(a.namespace)||!a.namespace||Buffer.byteLength(a.namespace)>l.maxTagBytes||!safeIncrement(a.next)||!natural(a.last)||a.next>a.last||state.documents.length>=l.maxDocuments)return null;const doc={authority:state.authority,id:`${a.namespace}:${a.next}`,incarnation:1};return identity(doc)&&!state.documents.some(d=>equal(d.document,doc))&&!state.retiredDocuments.some(d=>equal(d,doc))?doc:null;}
function detach(v){if(!safeIncrement(v.attachmentGeneration)||!safeIncrement(v.selectionGeneration)||!safeIncrement(v.viewportGeneration))return false;v.document=null;v.sourceRevision=0;v.selections=[];v.primary=0;v.attachmentGeneration++;v.selectionGeneration++;v.viewportGeneration++;return true;}
export function applyDocumentLifecycle(state,r,g,l,saveOwner=null){
  const fail=()=>({status:'unavailable',state});if(!exact(l,catalog.limits)||!Object.values(l).every(natural)||!rootValid(state,l)||!membershipInventoryValid(state,g)||!allowed(g,catalog.gates)||!r||!Object.hasOwn(catalog.variants,r.operation)||!exact(r,[...catalog.common,...catalog.variants[r.operation]])||r.contract!==catalog.contract||size(r)>l.maxRequestBytes)return fail();
  if(r.operation==='create-scratch'){
    if(!g.creationAllowed||!validBytes(r.source,l)||!exact(r.serialization,['profile','bom'])||r.serialization.profile!=='strict-utf8-one-bom/1'||typeof r.serialization.bom!=='boolean'||!['LF','CRLF','CR'].includes(r.insertionLineBreak))return fail();const document=fresh(state,l);if(!document)return fail();
    const next=copy(state);next.documents.push({document,live:true,revision:1,source:copy(r.source),serialization:{generation:1,...copy(r.serialization)},insertionLineBreak:r.insertionLineBreak,annotationSet:1,records:[],...(membershipInventoryActive(state,g)?{annotationLifecycle:{entries:[]}}:{}),binding:null,sourceBaseline:null,lifecycle:{generation:1,phase:'live',retainedBinding:null}});next.identityAllocation.next++;const out=finish(state,next,l,g);return out.status==='applied'?{...out,document:copy(document)}:out;
  }
  if(!identity(r.document)||r.document.authority!==state.authority)return fail();const docs=state.documents.filter(d=>equal(d.document,r.document));if(docs.length!==1)return fail();const d=docs[0],life=lifecycle(d);if(!life||!exact(life,['generation','phase','retainedBinding'])||!natural(r.expectedLifecycle)||life.generation!==r.expectedLifecycle||!['live','disposed-retained','provenance-released'].includes(life.phase)||life.phase==='live'&&d.live!==true||life.phase!=='live'&&d.live!==false)return fail();
  const next=copy(state),n=next.documents.find(x=>equal(x.document,r.document));
  if(['attach-view','detach-view','membership'].includes(r.operation)){
    if(d.live!==true||!relationRows(state,l))return fail();
    if(r.operation==='membership'){
      if(!g.membershipWritable||!g.workspaceReadable||!identity(r.workspace)||!natural(r.expectedWorkspace)||typeof r.present!=='boolean')return fail();const rows=next.workspaces.filter(w=>equal(w.workspace,r.workspace));if(rows.length!==1||!rows[0].live||rows[0].generation!==r.expectedWorkspace)return fail();const w=rows[0],present=w.members.some(x=>equal(x,r.document));if(present===r.present)return {status:'unchanged',state};if(!safeIncrement(w.generation)||r.present&&w.members.length>=l.maxRelations)return fail();w.members=r.present?[...w.members,copy(r.document)]:w.members.filter(x=>!equal(x,r.document));w.generation++;return finish(state,next,l,g);
    }
    if(!g.viewWritable||!identity(r.view)||!natural(r.expectedAttachment))return fail();const rows=next.views.filter(v=>equal(v.view,r.view));if(rows.length!==1||rows[0].attachmentGeneration!==r.expectedAttachment)return fail();const v=rows[0];
    if(r.operation==='detach-view'){if(!equal(v.document,r.document))return fail();if(!detach(v))return fail();return finish(state,next,l,g);}
    if(!g.sourceReadable||r.sourceRevision!==d.revision||v.document!==null)return fail();const normalized=normalizeSelection(d.source,r.selections,r.primary,g.selectionLimits);if(!normalized||!safeIncrement(v.attachmentGeneration)||!safeIncrement(v.selectionGeneration)||!safeIncrement(v.viewportGeneration))return fail();v.document=copy(r.document);v.sourceRevision=d.revision;v.profile='explicit-positional-selection/1';v.selections=normalized.selections;v.primary=normalized.primary;v.attachmentGeneration++;v.selectionGeneration++;v.viewportGeneration++;return finish(state,next,l,g);
  }
  if(!safeIncrement(life.generation))return fail();
  if(r.operation==='dispose'){
    if(d.live!==true||!g.disposeAllowed||!relationRows(state,l)||d.revision!==r.expectedSource||d.annotationSet!==r.expectedAnnotations||d.serialization?.generation!==r.expectedSerialization||(d.binding?.generation??0)!==r.expectedBinding||!['retain','discard'].includes(r.sourceDisposition)||!['retain','discard'].includes(r.annotationDisposition)||!g[r.sourceDisposition==='retain'?'sourceRetentionReady':'sourceDiscardAllowed']||!g[r.annotationDisposition==='retain'?'annotationRetentionReady':'annotationDiscardAllowed']||!allowed(g,['retentionComplete','effectsRetained','historyRetained','snapshotsRetained','providersRetained','viewsDetachable','membershipsDetachable','observerScopesFenced','bindingCoverageComplete'])||state.retiredDocuments.some(x=>equal(x,r.document)))return fail();
    for(const v of next.views.filter(v=>equal(v.document,r.document)))if(!detach(v))return fail();for(const w of next.workspaces)if(w.members.some(x=>equal(x,r.document))){if(!safeIncrement(w.generation))return fail();w.members=w.members.filter(x=>!equal(x,r.document));w.generation++;}
    n.live=false;n.lifecycle={generation:life.generation+1,phase:'disposed-retained',retainedBinding:copy(d.binding??null)};next.retiredDocuments.push(copy(r.document));return finish(state,next,l,g);
  }
  if(life.phase!=='disposed-retained')return fail();
  if(r.operation==='release-binding'){
    if(!g.bindingReleaseAllowed||!g.bindingCoverageComplete||!g.writerCoverageComplete||(d.binding?.generation??0)!==r.expectedBinding)return fail();if(!d.binding)return {status:'unchanged',state};const b=d.binding,rows=state.resourceOwners.filter(x=>x.identityDomain===b.identityDomain&&x.identityKey===b.identityKey);
    if(rows.length!==1||!equal(rows[0].document,r.document)||rows[0].bindingGeneration!==b.generation||rows[0].exportOrigins?.length||!sourceSaveDestinationResolved(saveOwner,r.document,b,g))return fail();next.resourceOwners=next.resourceOwners.filter(x=>!equal(x,rows[0]));n.binding=null;n.lifecycle={...copy(life),generation:life.generation+1};return finish(state,next,l,g);
  }
  if(d.binding!==null||Object.hasOwn(d,'ordinaryBindings')&&d.ordinaryBindings.annotations!==null||!allowed(g,['retentionDischargeAllowed','historyEvidenceReleased','snapshotEvidenceReleased','readEvidenceReleased','writeEvidenceReleased','providerEvidenceReleased']))return fail();n.lifecycle={generation:life.generation+1,phase:'provenance-released',retainedBinding:null};return finish(state,next,l,g);
}

// Internal disposal substep, not live unbind/rebind or native root installation.
export const disposedAnnotationBindingReleaseGates=Object.freeze([...catalog.gates,'bindingReleaseAllowed','bindingCoverageComplete','writerCoverageComplete','dispositionReadable','annotationHistoryRetained','annotationHistoryCoverageComplete']);
const retainedAnnotationBinding=b=>pairExact(b,['generation','provider','resource','registrationGeneration','identityDomain','identityKey','locator'])&&pairNatural(b.generation)&&b.generation>0&&pairIdentity(b.provider)&&pairIdentity(b.resource)&&pairNatural(b.registrationGeneration)&&['identityDomain','identityKey','locator'].every(k=>scalar(b[k])&&b[k].length>0);
const retainedAnnotationBaseline=b=>b===null||annotationLoadedBaselineShape(b)||b?.kind==='acknowledged-annotation-write'&&pairExact(b,annotationBaselines.baselineFields);
export function releaseDisposedAnnotationBinding(state,r,g,l,saveOwner){
 const fail=()=>({status:'unavailable',state});
 if(!exact(l,catalog.limits)||!Object.values(l).every(natural)||!rootValid(state,l)||!membershipInventoryValid(state,g)||!pairRootValid(state,g?.ordinaryBindingLimits)||sourceSaveRegistryAuthority(saveOwner)!==state.authority||!disposedAnnotationBindingReleaseGates.every(k=>g?.[k]===true)||!exact(r,['contract','document','expectedLifecycle','expectedPair','expectedAnnotationBinding'])||r.contract!=='disposed-annotation-binding-release/1'||!identity(r.document)||r.document.authority!==state.authority||size(r)>l.maxRequestBytes)return fail();
 const d=state.documents.find(d=>equal(d.document,r.document)),life=d?.lifecycle,p=d?.ordinaryBindings;
 if(!d||d.live!==false||!exact(life,['generation','phase','retainedBinding'])||life.phase!=='disposed-retained'||!natural(life.generation)||!natural(r.expectedLifecycle)||r.expectedLifecycle!==life.generation||!p||!natural(r.expectedPair)||r.expectedPair!==p.generation||!natural(r.expectedAnnotationBinding)||(p.annotations?.generation??0)!==r.expectedAnnotationBinding)return fail();
 const history=Object.hasOwn(d,'annotationBindingHistory')?d.annotationBindingHistory:g.annotationHistoryAbsentAttested===true?[]:null;
 if(!Array.isArray(history)||history.length>l.maxRelations||history.some(h=>!exact(h,['binding','annotationBaseline'])||!retainedAnnotationBinding(h.binding)||h.binding.generation>=(p.annotations?.generation??p.generation)||!retainedAnnotationBaseline(h.annotationBaseline))||new Set(history.map(h=>h.binding.generation)).size!==history.length)return fail();
 if(!p.annotations){if(Object.hasOwn(d,'annotationBaseline')?d.annotationBaseline!==null:g.annotationBaselineAbsentAttested!==true)return fail();return {status:'unchanged',state};}
 const b=p.annotations,rows=state.resourceOwners.filter(row=>row.identityDomain===b.identityDomain&&row.identityKey===b.identityKey),baseline=Object.hasOwn(d,'annotationBaseline')?d.annotationBaseline:g.annotationBaselineAbsentAttested===true?null:undefined;
 if(!retainedAnnotationBaseline(baseline)||history.length===l.maxRelations||!safeIncrement(life.generation)||!safeIncrement(p.generation)||rows.length!==1||rows[0].artifact!=='annotations'||!equal(rows[0].document,r.document)||rows[0].bindingGeneration!==b.generation||Object.hasOwn(rows[0],'exportOrigins')&&(!Array.isArray(rows[0].exportOrigins)||rows[0].exportOrigins.length!==0)||!sourceSaveDestinationResolved(saveOwner,r.document,b,g))return fail();
 const next=copy(state),n=next.documents.find(d=>equal(d.document,r.document));
 n.annotationBindingHistory=[...copy(history),{binding:copy(b),annotationBaseline:copy(baseline)}];n.annotationBaseline=null;n.ordinaryBindings={generation:p.generation+1,annotations:null};n.lifecycle={...copy(life),generation:life.generation+1};next.resourceOwners=next.resourceOwners.filter(row=>row.identityDomain!==b.identityDomain||row.identityKey!==b.identityKey);
 const out=finish(state,next,l,g);return out.status==='applied'&&pairRootValid(out.state,g.ordinaryBindingLimits)?out:fail();
}
