// Serialized normalized analysis owner; no native callback, execution or source writes.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {validateCanonicalBatch} from './text-oracles.mjs';
import {prepareParserSource} from './parser-source.mjs';
import {analysisQueryValueValid} from './analysis-query-values.mjs';
import {copySettingsData} from './semantic-settings.mjs';
import {analysisQueryClassificationBytes,captureAnalysisQueryLimits,normalizeAnalysisQueryArguments,analysisQueryProducerRequest} from './analysis-query-arguments.mjs';
import {beginInvocationAdmissionMutation,endInvocationAdmissionMutation,invocationAuthorityRoot,invocationAuthorityContext,invocationUsage,beginInvocationOwnerAttachment,commitInvocationOwnerAttachment,abortInvocationOwnerAttachment} from './invocation-publication.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./analysis-work.json',import.meta.url),'utf8'));
const parserCatalog=JSON.parse(fs.readFileSync(new URL('./parser-source.json',import.meta.url),'utf8'));
const queryInputForms=Object.freeze(Object.entries(catalog.queries).map(([operation,extra])=>Object.freeze({family:'F08',variant:operation,operation,contract:catalog.queryContract,fields:Object.freeze([...catalog.queryCommon,...extra])})));
const owners=new WeakMap(),snapshots=new WeakMap(),nativeOwners=new WeakMap(),copy=structuredClone;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const size=v=>Buffer.byteLength(JSON.stringify(v),'utf8'),natural=n=>Number.isSafeInteger(n)&&n>=0;
const identity=r=>exact(r,['authority','id','incarnation'])&&typeof r.authority==='string'&&r.authority.length>0&&typeof r.id==='string'&&r.id.length>0&&natural(r.incarnation);
const origin=r=>exact(r,catalog.originFields)&&Object.values(r).every(s=>typeof s==='string'&&s.length>0);
const key=r=>JSON.stringify([r.authority,r.id,r.incarnation]);
const launch=['authorized','sourceReadable','disclosureAllowed','continuous','live','notStopped','providerAvailable','capacity','reservationReady'];
const release=['authorized','scope','continuous','live','retained','notStopped','replyReady'];
const acceptance=['authorized','sourceReadable','disclosureAllowed','continuous','live','notStopped','acceptanceAllowed'];
const strict=(g,keys)=>keys.every(k=>g?.[k]===true);
function nativeRootSupportsAnalysis(state){
 return Array.isArray(state?.documents)&&Array.isArray(state?.views)&&Array.isArray(state?.analysisContexts)&&
  state.analysisContexts.every(r=>exact(r,catalog.recordFields)&&identity(r.analysis)&&identity(r.document)&&identity(r.provider)&&
   r.analysis.authority===state.authority&&r.document.authority===state.authority&&
   r.languageProfile===catalog.languageProfile&&catalog.recordFields.filter(k=>k.endsWith('Generation')).every(k=>natural(r[k])));
}
// A bound owner never treats an equal supplied root as the actual engine root.
// Current permission truth is still a first-party host premise.
function nativeState(o,state,g,keys){
 return !o?.nativeDomain||invocationUsage(o.nativeDomain)?.faulted===false&&
  state===invocationAuthorityRoot(o.nativeDomain)&&nativeRootSupportsAnalysis(state)&&strict(g,keys);
}
const working=t=>t.phase==='ready'||t.phase==='submitted'&&!t.quiescent;
const usage=o=>({tasks:o.tasks.size,outstanding:[...o.tasks.values()].filter(working).length,issued:o.used.size,heldBytes:[...o.tasks.values()].reduce((n,t)=>n+t.charge,0)+o.queryBytes+(o.gatewayBytes??0),leases:o.leases.size,queryBusy:o.queryBusy,queryFaulted:o.queryFaulted,queryBytes:o.queryBytes});
function current(state,c,g){
  if(!g?.contextAllowed||!exact(c,catalog.contextFields)||!identity(c.document)||!identity(c.analysis)||c.document.authority!==state.authority||c.analysis.authority!==state.authority||
      !natural(c.sourceRevision)||c.languageProfile!==catalog.languageProfile||catalog.contextFields.filter(k=>k.endsWith('Generation')).some(k=>!natural(c[k])))return null;
  const docs=state.documents.filter(d=>equal(d.document,c.document)),rows=state.analysisContexts.filter(r=>equal(r.analysis,c.analysis));
  if(docs.length!==1||rows.length!==1||docs[0].live===false||docs[0].revision!==c.sourceRevision||!exact(rows[0],catalog.recordFields)||!identity(rows[0].provider)||!equal(rows[0].provider,g.provider)||
      catalog.recordFields.filter(k=>k!=='provider').some(k=>!equal(rows[0][k],c[k])))return null;
  return docs[0];
}
const parseKey=c=>Object.fromEntries(catalog.parseKeyFields.map(k=>[k,c[k]]));
function baseChain(o,ref){
  const seen=new Set();while(ref!==null){
    const s=snapshots.get(ref);if(!s||s.owner!==o||!s.parseValid||seen.has(ref)||seen.size>=o.tasks.size)return false;
    seen.add(ref);const t=o.tasks.get(s.task);if(!t)return false;ref=t.base;
  }return true;
}
function baseFor(o,ref,c,provider,sameSource,source){
  const s=snapshots.get(ref);return s&&s.owner===o&&baseChain(o,ref)&&equal(s.provider,provider)&&equal(parseKey(s.packet.context),parseKey(c))&&
    (!sameSource||s.packet.context.sourceRevision===c.sourceRevision&&equal(s.packet.source,source))?s:null;
}
function invalidate(t,parse=false){const s=snapshots.get(t.snapshot);if(s){s.queryEnabled=false;if(parse)s.parseValid=false;}}
function snapshotCurrent(o,ref,state,g){
  const s=snapshots.get(ref);if(!s||s.owner!==o||!s.queryEnabled||!baseChain(o,ref))return null;
  const d=current(state,s.packet.context,g);return d&&equal(d.source,s.packet.source)?s:null;
}
const boundary=(s,p)=>natural(p)&&p<=s.length&&(p===s.length||(s[p]&0xc0)!==0x80);
const interval=(r,s,nonempty=true)=>Array.isArray(r)&&r.length===2&&boundary(s,r[0])&&boundary(s,r[1])&&(nonempty?r[0]<r[1]:r[0]<=r[1]);
const text=(s,l)=>typeof s==='string'&&Buffer.byteLength(s,'utf8')<=l.maxTagBytes&&Buffer.from(s,'utf8').toString('utf8')===s;
function packetValid(p,t,l){
  if(!exact(p,catalog.packetFields)||!equal(p.context,t.context)||!equal(p.source,t.input.source)||size(p)>l.maxEnvelopeBytes||
      !['parsed','syntax-errors'].includes(p.parseState)||!['complete-within-profile','partial'].includes(p.coverage)||
      !Array.isArray(p.regions)||p.regions.length>l.maxRegions||!Array.isArray(p.captures))return false;
  let ranges=0,captures=p.captures.length,prior=0;
  for(const row of p.captures){
    if(!Array.isArray(row)||row.length!==4||!['host-block','host-inline','child'].includes(row[0])||!text(row[1],l)||!interval(row.slice(2),p.source)||row[2]<prior)return false;prior=row[3];
  }
  for(const r of p.regions){
    if(!exact(r,catalog.regionFields)||!text(r.label,l)||![null,'javascript'].includes(r.language)||typeof r.closed!=='boolean'||typeof r.hasError!=='boolean'||
        !catalog.regionKinds.includes(r.kind)||r.body!==null&&!interval(r.body,p.source,false)||!Array.isArray(r.ranges)||!Array.isArray(r.captures))return false;
    let end=r.body?.[0]??0;ranges+=r.ranges.length;captures+=r.captures.length;
    if(ranges>l.maxRanges||captures>l.maxCaptures)return false;
    for(const range of r.ranges){if(!r.body||!interval(range,p.source)||range[0]<end||range[0]<r.body[0]||range[1]>r.body[1])return false;end=range[1];}
    if(r.kind==='parsed'&&(!r.closed||r.language!=='javascript'||!r.ranges.length)||r.kind!=='parsed'&&r.captures.length)return false;
    for(const row of r.captures)if(!Array.isArray(row)||row.length!==3||!text(row[0],l)||!interval(row.slice(1),p.source)||!r.ranges.some(([a,b])=>a<=row[1]&&row[2]<=b))return false;
  }
  return captures<=l.maxCaptures&&p.captures.every(([layer,,lo,hi])=>layer!=='child'||p.regions.some(r=>r.kind==='parsed'&&r.ranges.some(([a,b])=>a<=lo&&hi<=b)))&&(!p.regions.some(r=>r.kind==='region-budget')||p.coverage==='partial');
}
export function newAnalysisRegistry(limits,parserLimits){
  if(!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||!natural(3*limits.maxEnvelopeBytes)||
      !exact(parserLimits,parserCatalog.inputLimits)||!Object.values(parserLimits).every(natural)||parserLimits.maxCoordinate>parserCatalog.coordinateMaximum)return null;
  const ref=Object.freeze({profile:catalog.profile});owners.set(ref,{limits:copy(limits),parserLimits:copy(parserLimits),tasks:new Map(),used:new Set(),leases:new Map(),queryBusy:false,queryFaulted:false,queryBytes:0});return ref;
}
// Fixed first-party assembly port, not an F16 activation command or public codec.
// One registry per actual engine domain prevents replacement from resetting its
// consumed origins. Internal logical bounds do not prove global/physical funds.
export function newInvocationAnalysisRegistry(domain,limits,parserLimits,g){
 const attachment=beginInvocationOwnerAttachment(domain,'analysis',limits?.maxHeldBytes,Object.freeze({profile:catalog.profile}));if(!attachment)return null;
 try{
 if(invocationUsage(domain)?.faulted!==false||nativeOwners.has(domain)||!nativeRootSupportsAnalysis(invocationAuthorityRoot(domain))||
   !strict(g,['ownerControlAllowed','resourcesReady','preparationReserved']))return null;
 const owner=newAnalysisRegistry(limits,parserLimits);if(!owner)return null;
 owners.get(owner).nativeDomain=domain;owners.get(owner).parentAttachment=attachment;nativeOwners.set(domain,owner);if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native analysis attachment invariant');return owner;
 }finally{abortInvocationOwnerAttachment(domain,attachment);}
}
export function admitInvocationAnalysisWork(owner,request,g){const o=owners.get(owner);return o?.nativeDomain?admitAnalysisWork(owner,invocationAuthorityRoot(o.nativeDomain),request,g):{status:'unavailable'};}
export function submitInvocationAnalysisWork(owner,task,g){const o=owners.get(owner);return o?.nativeDomain?submitAnalysisWork(owner,task,invocationAuthorityRoot(o.nativeDomain),g):{status:'not-submitted'};}
export function acceptInvocationAnalysisWork(owner,task,g){const o=owners.get(owner);return o?.nativeDomain?acceptAnalysisWork(owner,task,invocationAuthorityRoot(o.nativeDomain),g):{status:'unavailable'};}
export function queryInvocationAnalysis(owner,request,g){const o=owners.get(owner);return o?.nativeDomain?queryAnalysis(owner,invocationAuthorityRoot(o.nativeDomain),request,g):{status:'unavailable'};}
export function analysisQueryInputForms(){return queryInputForms;}
export function queryDecodedInvocationAnalysis(owner,envelope,inputLimits,c,g){
 const o=owners.get(owner),fail=()=>({status:'unavailable'});
 if(!o?.nativeDomain||o.queryBusy||o.queryFaulted||invocationUsage(o.nativeDomain)?.faulted!==false||analysisQueryClassificationBytes>o.limits.maxHeldBytes-usage(o).heldBytes)return fail();
 o.queryBusy=true;o.gatewayBytes=analysisQueryClassificationBytes;
 try{
  const limits=captureAnalysisQueryLimits(inputLimits);if(!limits)return fail();
  const bytes=Math.min(limits.maxRequestBytes,o.limits.maxQueryBytes),frames=7*bytes;
  if(!natural(frames)||frames>o.limits.maxHeldBytes-usage(o).heldBytes)return fail();o.gatewayBytes+=frames;
  const normalized=normalizeAnalysisQueryArguments(envelope,c,limits,o.limits.maxQueryBytes);
  if(!normalized||!equal(normalized.record.context,invocationAuthorityContext(o.nativeDomain)))return fail();
  const facts=copySettingsData({facts:g},{...limits,maxRequestBytes:bytes});if(!facts)return fail();
  const request=analysisQueryProducerRequest(normalized);
  // Foreign opaque objects must not reach the native request's JSON sizing.
  if(snapshots.get(request.snapshot)?.owner!==o)return fail();
  // No caller code runs between this private handoff and the native busy guard.
  // Native query workspace remains independently charged alongside gatewayBytes.
  o.queryBusy=false;
  try{return queryInvocationAnalysis(owner,request,facts.value.facts);}finally{o.queryBusy=true;}
 }catch{o.queryFaulted=true;return fail();}finally{o.gatewayBytes=0;o.queryBusy=false;}
}
export function admitAnalysisWork(owner,state,r,g){
 const admissionDomain=owners.get(owner)?.nativeDomain??null;const admissionSection=admissionDomain?beginInvocationAdmissionMutation(admissionDomain):null;if(admissionDomain&&!admissionSection)return {status:'unavailable'};try{
  if(!nativeState(owners.get(owner),state,g,[...launch,'scope','contextAllowed']))return {status:'unavailable'};
  const o=owners.get(owner);if(!o||o.queryBusy||o.queryFaulted||!launch.every(k=>g?.[k])||!exact(r,catalog.requestFields)||r.contract!==catalog.contract||r.operation!=='enabled-parse-query-work'||r.profile!==catalog.profile||
      !origin(r.origin)||!identity(r.provider)||!equal(r.provider,g.provider)||!catalog.modes.includes(r.mode)||!Array.isArray(r.journals)||r.journals.length>o.parserLimits.maxReceipts||size(r)>o.limits.maxCaptureBytes)return {status:'unavailable'};
  const d=current(state,r.context,g);if(!d||d.source.length>o.limits.maxSourceBytes)return {status:'unavailable'};
  try{validateCanonicalBatch(d.source,[]);}catch{return {status:'unavailable'};}
  const base=r.base===null?null:baseFor(o,r.base,r.context,r.provider,r.mode==='query-only',d.source);
  if(r.base!==null&&!base||r.mode==='query-only'&&(!base||r.journals.length))return {status:'unavailable'};
  const target={document:r.context.document,revision:r.context.sourceRevision,source:d.source};
  const input=r.mode==='query-only'?{profile:parserCatalog.profile,mode:'reuse-parse',fromRevision:target.revision,toRevision:target.revision,source:copy(d.source),edits:[]}:
    prepareParserSource(base?{document:base.packet.context.document,revision:base.packet.context.sourceRevision,source:base.packet.source}:null,target,r.journals,{...g,captureReserved:g.reservationReady},o.parserLimits);
  if(!input)return {status:'unavailable'};
  const minimal={contract:catalog.evidenceContract,task:{kind:'analysis-task/1'},origin:r.origin,provider:r.provider,kind:'quiescence'};
  if(size(minimal)>o.limits.maxEnvelopeBytes||catalog.failures.some(reason=>size({...minimal,kind:'failure',reason})>o.limits.maxEnvelopeBytes))return {status:'unavailable'};
  const captured={context:copy(r.context),origin:copy(r.origin),provider:copy(r.provider),mode:r.mode,input},charge=size(captured)+3*o.limits.maxEnvelopeBytes,u=usage(o),used=JSON.stringify(catalog.originFields.map(k=>r.origin[k]));
  if(size(captured)>o.limits.maxCaptureBytes||!natural(charge)||o.used.has(used)||u.issued>=o.limits.maxIssued||u.tasks>=o.limits.maxTasks||u.outstanding>=o.limits.maxOutstanding||charge>o.limits.maxHeldBytes-u.heldBytes)return {status:'unavailable'};
  const task=Object.freeze({kind:'analysis-task/1'});o.used.add(used);
  o.tasks.set(task,{...captured,base:input.mode==='fresh'?null:r.base,charge,phase:'ready',cancelled:false,superseded:false,quiescent:false,witnesses:[],contested:false,snapshot:null});
  return {status:'admitted',task};

 }finally{if(admissionSection)endInvocationAdmissionMutation(admissionDomain,admissionSection);}
}
export function submitAnalysisWork(owner,task,state,g){
  if(!nativeState(owners.get(owner),state,g,[...launch,'scope','contextAllowed']))return {status:'not-submitted'};
  const o=owners.get(owner),t=o?.tasks.get(task);
  if(!t||o.queryBusy||o.queryFaulted||t.phase!=='ready'||t.cancelled||t.superseded||!launch.every(k=>g?.[k])||!equal(t.provider,g.provider))return {status:'not-submitted'};
  const d=current(state,t.context,g);if(!d||!equal(d.source,t.input.source)||t.base!==null&&!baseFor(o,t.base,t.context,t.provider,t.mode==='query-only',d.source)||o.leases.has(key(t.context.analysis)))return {status:'not-submitted'};
  o.leases.set(key(t.context.analysis),task);t.phase='submitted';return {status:'submitted',task,base:t.base,capture:copy({context:t.context,origin:t.origin,provider:t.provider,mode:t.mode,input:t.input})};
}
export function controlAnalysisWork(owner,task,operation,g){
  if(owners.get(owner)?.nativeDomain&&g?.controlAllowed!==true)return {status:'unavailable'};
  const o=owners.get(owner),t=o?.tasks.get(task);if(!t||o.queryBusy||!g?.controlAllowed||!['cancel','supersede'].includes(operation))return {status:'unavailable'};
  const field=operation==='cancel'?'cancelled':'superseded';if(t[field])return {status:'unchanged'};
  t[field]=true;if(t.phase==='ready')t.phase='closed';invalidate(t);return {status:'stopped'};
}
export function observeAnalysisWork(owner,event,g){
  if(owners.get(owner)?.nativeDomain&&(!strict(g,['authentic','evidenceAllowed','ingressReady'])||event?.kind==='quiescence'&&g?.quiescenceAttested!==true))return {status:'unavailable'};
  const o=owners.get(owner);if(!o||o.queryBusy||!g?.authentic||!g.evidenceAllowed||!g.ingressReady)return {status:'unavailable'};
  const t=o.tasks.get(event?.task);
  if(!t||t.phase!=='submitted'||!Object.hasOwn(catalog.evidenceVariants,event.kind)||!exact(event,[...catalog.evidenceCommon,...catalog.evidenceVariants[event.kind]])||event.contract!==catalog.evidenceContract||
      !equal(event.origin,t.origin)||!equal(event.provider,t.provider)||size(event)>o.limits.maxEnvelopeBytes)return {status:'unavailable'};
  if(event.kind==='quiescence'){
    if(!g.quiescenceAttested)return {status:'unavailable'};if(t.quiescent)return {status:'duplicate'};
    t.quiescent=true;if(o.leases.get(key(t.context.analysis))===event.task)o.leases.delete(key(t.context.analysis));return {status:'quiescent'};
  }
  if(event.kind==='failure'&&!catalog.failures.includes(event.reason))return {status:'unavailable'};
  const witness=copy(event.kind==='result'?{kind:'result',packet:event.packet}:{kind:'failure',reason:event.reason});
  if(t.witnesses.some(w=>equal(w,witness)))return {status:'duplicate'};if(t.witnesses.length>=2)return {status:'evidence-full'};
  t.witnesses.push(witness);if(t.witnesses.length>1){t.contested=true;invalidate(t,true);return {status:'conflict'};}return {status:'recorded'};
}
export function acceptAnalysisWork(owner,task,state,g){
  if(!nativeState(owners.get(owner),state,g,[...acceptance,'scope','contextAllowed']))return {status:'unavailable'};
  const o=owners.get(owner),t=o?.tasks.get(task),fail=()=>({status:'unavailable'});
  if(!t||o.queryBusy||o.queryFaulted||!acceptance.every(k=>g?.[k])||!equal(t.provider,g.provider)||t.cancelled||t.superseded||t.contested||t.witnesses.length!==1||t.witnesses[0].kind!=='result'||!baseChain(o,t.base))return fail();
  const d=current(state,t.context,g);if(!d||!equal(d.source,t.input.source)||!packetValid(t.witnesses[0].packet,t,o.limits))return fail();
  if(t.snapshot)return snapshotCurrent(o,t.snapshot,state,g)?{status:'analysis',snapshot:t.snapshot}:fail();
  const peers=[...o.tasks.values()].filter(p=>p!==t&&p.snapshot&&equal(p.context.analysis,t.context.analysis));
  if(peers.some(p=>snapshotCurrent(o,p.snapshot,state,g)))return fail();
  for(const p of peers)invalidate(p);
  const ref=Object.freeze({kind:'accepted-analysis/1'});snapshots.set(ref,{owner:o,task,provider:copy(t.provider),packet:copy(t.witnesses[0].packet),queryEnabled:true,parseValid:true});t.snapshot=ref;return {status:'analysis',snapshot:ref};
}
export function queryAnalysis(owner,state,r,g){
  if(!nativeState(owners.get(owner),state,g,[...release,'contextAllowed']))return {status:'unavailable'};
  const o=owners.get(owner),fail=()=>({status:'unavailable'});if(!o||o.queryBusy||o.queryFaulted)return fail();
  const queryFrameBytes=o.limits.maxEnvelopeBytes+Math.max(size({status:'captures'}),size({status:'analysis-summary'})),workspace=2*queryFrameBytes;if(!natural(queryFrameBytes)||!natural(workspace)||workspace>o.limits.maxHeldBytes-usage(o).heldBytes)return fail();
  o.queryBusy=true;o.queryBytes=workspace;
  try{
  if(!release.every(k=>g?.[k])||!r||!Object.hasOwn(catalog.queries,r.operation)||!exact(r,[...catalog.queryCommon,...catalog.queries[r.operation]])||r.contract!==catalog.queryContract||size(r)>o.limits.maxQueryBytes)return fail();
  const s=snapshotCurrent(o,r.snapshot,state,g);if(!s||!identity(r.view))return fail();
  const views=state.views.filter(v=>equal(v.view,r.view));if(views.length!==1||!equal(views[0].document,s.packet.context.document)||!equal(views[0].analysis,s.packet.context.analysis)||views[0].sourceRevision!==s.packet.context.sourceRevision)return fail();
  let result;
  if(r.operation==='supplied-evidence-inspection')result={status:'analysis-summary',context:copy(s.packet.context),parseState:s.packet.parseState,coverage:s.packet.coverage};
  else{
    if(!interval(r.range,s.packet.source,false))return fail();
    result={status:'captures',context:copy(s.packet.context),parseState:s.packet.parseState,coverage:s.packet.coverage,captures:s.packet.captures.filter(([, ,lo,hi])=>r.range[0]<r.range[1]&&lo<r.range[1]&&r.range[0]<hi).map(([layer,name,lo,hi])=>[layer,name,Math.max(lo,r.range[0]),Math.min(hi,r.range[1])])};
  }
  if(!(size(result)<=o.limits.maxReplyBytes))return fail();
  if(!analysisQueryValueValid(result,r.operation)||!equal(result.context,s.packet.context)||result.parseState!==s.packet.parseState||result.coverage!==s.packet.coverage)throw Error('native analysis query result invariant');
  return result;
  }catch{o.queryFaulted=true;return fail();}finally{o.queryBytes=0;o.queryBusy=false;}
}
export function retireAnalysisWork(owner,task,g){
  if(owners.get(owner)?.nativeDomain&&!strict(g,['retirementAllowed','retentionReleased','deliveryDrained']))return {status:'unavailable'};
  const o=owners.get(owner),t=o?.tasks.get(task);if(!t||o.queryBusy||!g?.retirementAllowed||!g.retentionReleased||!g.deliveryDrained||working(t)||[...o.tasks.values()].some(p=>p.base!==null&&p.base===t.snapshot))return {status:'unavailable'};
  invalidate(t,true);o.tasks.delete(task);return {status:'retired'};
}
// Trusted supervisor inspection, not F06/public reflection or actual heap accounting.
export function analysisUsage(owner){const o=owners.get(owner);return o?usage(o):null;}
export function analysisTaskStatus(owner,task){const t=owners.get(owner)?.tasks.get(task);return t?{phase:t.phase,cancelled:t.cancelled,superseded:t.superseded,quiescent:t.quiescent,contested:t.contested,witnesses:t.witnesses.length,workHeld:working(t)}:null;}
// Trusted native-host composition guard, not a public reflective query or authentication.
export function analysisResultMatches(owner,task,packet,g){
  if(owners.get(owner)?.nativeDomain&&g?.nativeBindingAllowed!==true)return false;
  const t=owners.get(owner)?.tasks.get(task);return Boolean(g?.nativeBindingAllowed&&t&&!t.cancelled&&!t.superseded&&!t.contested&&t.witnesses.length===1&&t.witnesses[0].kind==='result'&&equal(t.witnesses[0].packet,packet));
}
