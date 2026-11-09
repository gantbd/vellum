// Normalized owned clipboard boundary, not a system clipboard or production adapter.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {normalizeSelection,prepareDerivedSelectionEdit,publishSelectionEdit} from './selection-products.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./clipboard-products.json',import.meta.url))),selection=JSON.parse(fs.readFileSync(new URL('./selection-products.json',import.meta.url)));
const owners=new WeakMap(),invocations=new WeakMap(),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v)),natural=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const scalar=s=>typeof s==='string'&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Buffer.from(s,'utf8'))===s;
const identity=v=>exact(v,['authority','id','incarnation'])&&scalar(v.authority)&&v.authority.length>0&&scalar(v.id)&&v.id.length>0&&natural(v.incarnation);
const allowed=(g,ks)=>ks.every(k=>g?.[k]===true),unavailable=()=>({status:'unavailable'});
const originKey=r=>JSON.stringify(r.origin),held=t=>t.phase==='dispatched'&&!t.quiescent;
function registration(o,state,r){
 if(state?.authority!==o.authority||!Array.isArray(state.clipboardRegistrations)||state.clipboardRegistrations.length>o.limits.maxRegistryRows)return null;
 const rows=state.clipboardRegistrations.filter(x=>equal(x.provider,r.provider)&&equal(x.capability,r.capability));if(rows.length!==1)return null;const row=rows[0];
 return exact(row,catalog.registrationFields)&&identity(row.provider)&&identity(row.capability)&&row.generation===r.expectedRegistration&&natural(row.generation)&&catalog.destinations.includes(row.destination)&&typeof row.readable==='boolean'&&typeof row.writable==='boolean'&&scalar(row.acknowledgmentProfile)&&row.acknowledgmentProfile.length>0&&Buffer.byteLength(row.acknowledgmentProfile)<=o.limits.maxTagBytes?row:null;
}
function target(o,state,r){
 if(state?.authority!==o.authority||size(state)>o.limits.maxRootBytes||!Array.isArray(state.documents)||!Array.isArray(state.views)||state.documents.length>o.limits.maxRegistryRows||state.views.length>o.viewLimits.maxInputViews)return null;
 const ds=state.documents.filter(d=>equal(d.document,r.document)),vs=state.views.filter(v=>equal(v.view,r.view));if(ds.length!==1||vs.length!==1)return null;const d=ds[0],v=vs[0];
 if(d.live===false||!equal(v.document,d.document)||d.revision!==r.sourceRevision||v.sourceRevision!==r.sourceRevision||v.selectionGeneration!==r.expectedSelection||r.selectionProfile!==selection.profile||v.profile!==r.selectionProfile)return null;
 const n=normalizeSelection(d.source,v.selections,v.primary,o.viewLimits);return n&&equal(n,{selections:v.selections,primary:v.primary})?{d,v}:null;
}
const selectionCapture=v=>({selections:copy(v.selections),primary:v.primary});
function currentTarget(o,state,t){const x=target(o,state,t.request);return x&&equal(selectionCapture(x.v),t.target)?x:null;}
function payloadValid(o,p){
 if(!exact(p,['format','fragments'])||!['plain','structured'].includes(p.format)||!Array.isArray(p.fragments)||!p.fragments.length||p.fragments.length>o.viewLimits.maxSelections||p.format==='plain'&&p.fragments.length!==1||p.fragments.some(s=>!scalar(s))||size(p)>o.limits.maxPayloadBytes)return false;return true;
}
const editRequest=r=>({contract:selection.contract,operation:'ordinary-edit',invocation:r.invocation,view:r.view,document:r.document,sourceRevision:r.sourceRevision,expectedSelection:r.expectedSelection,profile:r.selectionProfile,text:'',selectionAfter:'carets-after'});
export function newClipboardRegistry(authority,limits,viewLimits){
 if(!scalar(authority)||!authority||!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||Buffer.byteLength(authority)>limits.maxTagBytes||!exact(viewLimits,selection.limits)||!selection.limits.every(k=>natural(viewLimits?.[k])))return null;
 const charge=limits.maxCaptureBytes+(1+limits.maxConflicts)*(limits.maxPayloadBytes+limits.maxEnvelopeBytes)+limits.maxEnvelopeBytes;if(!natural(charge)||!natural(charge*limits.maxTasks)||!natural(4*limits.maxRootBytes))return null;
 const owner=Object.freeze({profile:catalog.profile});owners.set(owner,{authority,limits:copy(limits),viewLimits:copy(viewLimits),charge,tasks:new Map(),used:new Set(),structured:new WeakMap()});return owner;
}
export function admitClipboard(owner,state,r,g){
 const o=owners.get(owner);if(!o||!allowed(g,catalog.admissionGates)||!exact(r,catalog.requestFields)||r.contract!==catalog.contract||!catalog.operations.includes(r.operation)||!scalar(r.invocation)||!r.invocation||!exact(r.origin,['operation','effect','attempt'])||Object.values(r.origin).some(s=>!scalar(s)||!s)||!identity(r.document)||r.document.authority!==o.authority||!identity(r.view)||!identity(r.provider)||!identity(r.capability)||![r.sourceRevision,r.expectedSelection,r.expectedRegistration].every(natural)||!['plain','structured'].includes(r.format)||size(r)>o.limits.maxCaptureBytes||!g.sourceReadable)return unavailable();
 const row=registration(o,state,r),x=target(o,state,r),u=clipboardUsage(owner);if(!row||!x||(r.operation==='paste-request'?!row.readable:!row.writable))return unavailable();
 if(r.format==='structured'&&row.destination!=='session-clipboard'||r.operation!=='copy'&&(!g.sourceWritable||!g.allTargetsWritable||!g.viewWritable)||o.used.has(originKey(r))||u.tasks>=o.limits.maxTasks||u.issued>=o.limits.maxIssued||o.charge>o.limits.maxHeldBytes-u.heldBytes)return unavailable();
 let payload=null;if(r.operation!=='paste-request'){
  const fragments=x.v.selections.map(s=>x.d.source.slice(Math.min(s.anchor,s.head),Math.max(s.anchor,s.head))).map(b=>new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Uint8Array.from(b)));
  if(fragments.every(s=>s.length===0))return {status:'unchanged'};
  payload={format:r.format,fragments:r.format==='plain'?[fragments.join('\n')]:fragments};if(!payloadValid(o,payload)||size({status:'clipboard-payload',payload})>o.limits.maxEnvelopeBytes)return unavailable();
 }
 const capture={request:copy(r),registration:copy(row),target:selectionCapture(x.v),payload};if(size(capture)>o.limits.maxCaptureBytes)return unavailable();
 const receiptCapture={document:copy(r.document),view:copy(r.view),sourceRevision:r.sourceRevision,selectionGeneration:r.expectedSelection,format:r.format,acknowledgmentProfile:row.acknowledgmentProfile};
 const minimal={contract:catalog.evidenceContract,task:{kind:'clipboard-task/1'},origin:r.origin,provider:r.provider,capability:r.capability,capture:receiptCapture,kind:'result',outcome:'not-applied',payload:null};if(size(minimal)>o.limits.maxEnvelopeBytes)return unavailable();
 const task=Object.freeze({kind:'clipboard-task/1'});o.tasks.set(task,{...capture,receiptCapture,phase:'queued',stopped:false,quiescent:false,contested:false,result:null,conflicts:[],published:false,localStatus:null,localIntent:null,stage:null});o.used.add(originKey(r));return {status:'admitted',task};
}
export function dispatchClipboard(owner,task,state,g){
 const o=owners.get(owner),t=o?.tasks.get(task);if(!t||t.phase!=='queued'||t.stopped||!allowed(g,catalog.dispatchGates)||size(state)>o.limits.maxRootBytes||!equal(registration(o,state,t.request),t.registration)||clipboardUsage(owner).outstanding>=o.limits.maxOutstanding)return {status:'not-dispatched'};
 if(t.request.operation==='paste-request'&&!currentTarget(o,state,t))return {status:'not-dispatched'};
 if(t.request.operation!=='paste-request'&&!g.disclosureAllowed)return {status:'not-dispatched'};
 t.phase='dispatched';return {status:'dispatched',task,origin:copy(t.request.origin),provider:copy(t.registration.provider),capability:copy(t.registration.capability),operation:t.request.operation,format:t.request.format,acknowledgmentProfile:t.registration.acknowledgmentProfile,receiptCapture:copy(t.receiptCapture)};
}
export function readClipboardExport(owner,task,g){
 const o=owners.get(owner),t=o?.tasks.get(task);if(!t||t.phase!=='dispatched'||t.stopped||t.request.operation==='paste-request'||!allowed(g,catalog.releaseGates)||!g.disclosureAllowed||!equal(g.provider,t.registration.provider)||!equal(g.capability,t.registration.capability))return unavailable();return {status:'clipboard-payload',payload:copy(t.payload)};
}
// Session-local owned fragments are never reconstructed from external text equality.
export function retainClipboardFragments(owner,task,g){
 const o=owners.get(owner),t=o?.tasks.get(task);if(!t||t.registration.destination!=='session-clipboard'||t.payload?.format!=='structured'||t.stopped||!allowed(g,catalog.releaseGates)||!g.sessionRetentionAllowed)return unavailable();
 if(!t.fragmentHandle){t.fragmentHandle=Object.freeze({kind:'session-fragments/1'});o.structured.set(t.fragmentHandle,t);}return {status:'retained',fragments:t.fragmentHandle};
}
export function observeClipboard(owner,e,g){
 const o=owners.get(owner),t=o?.tasks.get(e?.task);if(!t||!allowed(g,['authentic','evidenceAllowed','ingressReady'])||size(e)>o.limits.maxEnvelopeBytes||!exact(e,['contract','task','origin','provider','capability','capture','kind',...(e.kind==='result'?['outcome','payload']:[])])||e.contract!==catalog.evidenceContract||t.phase!=='dispatched'||!equal(e.origin,t.request.origin)||!equal(e.provider,t.registration.provider)||!equal(e.capability,t.registration.capability)||!equal(e.capture,t.receiptCapture)||!['result','quiescence'].includes(e.kind))return unavailable();
 if(e.kind==='quiescence'){if(!g.quiescenceAttested)return unavailable();if(t.quiescent)return {status:'duplicate'};t.quiescent=true;return {status:'recorded'};}
 if(!g.resultAttested||!['accepted','not-applied'].includes(e.outcome))return unavailable();let payload=null;
 if(t.request.operation==='paste-request'&&e.outcome==='accepted'){
  if(t.request.format==='structured'){
   const source=o.structured.get(e.payload);if(!source||source.stopped||!o.tasks.has([...o.tasks.entries()].find(([,v])=>v===source)?.[0])||!g.sessionRetentionAllowed||!allowed(g,catalog.releaseGates)||!equal(source.registration.provider,t.registration.provider)||!equal(source.registration.capability,t.registration.capability)||!equal(source.registration,t.registration))return unavailable();payload=copy(source.payload);
  }else{if(!payloadValid(o,e.payload)||e.payload.format!=='plain')return unavailable();payload=copy(e.payload);}
 }else if(e.payload!==null)return unavailable();
 const result={outcome:e.outcome,payload};if(t.result){if(equal(result,t.result)||t.conflicts.some(x=>equal(x,result)))return {status:'duplicate'};t.contested=true;if(t.conflicts.length>=o.limits.maxConflicts)return {status:'evidence-full'};t.conflicts.push(result);return {status:'conflict'};}t.result=result;return {status:'recorded'};
}
function clipboardCandidate(owner,task,state,g){
 const o=owners.get(owner),t=o?.tasks.get(task),fail=()=>({status:'not-published',state});if(!t||t.request.operation==='copy'||t.stopped||t.contested||t.result?.outcome!=='accepted'||!allowed(g,catalog.publicationGates))return fail();
 if(t.published)return {status:'already-published',state};if(t.localStatus!==null)return fail();
 const x=currentTarget(o,state,t);if(!x||!equal(registration(o,state,t.request),t.registration))return fail();
 const cut=t.request.operation==='cut-export-stage',payload=cut?null:t.result.payload;if(!cut&&(!payloadValid(o,payload)||payload.format==='structured'&&payload.fragments.length!==x.v.selections.length))return fail();
 const edits=x.v.selections.map((s,i)=>{const lo=Math.min(s.anchor,s.head),hi=Math.max(s.anchor,s.head),text=cut?'':payload.fragments[payload.format==='plain'?0:i];return {lo,hi,expected:x.d.source.slice(lo,hi),ins:Array.from(Buffer.from(text,'utf8'))};});
 const plan=prepareDerivedSelectionEdit(state,editRequest(t.request),edits,g,o.viewLimits);if(!plan)return fail();const out=publishSelectionEdit(state,plan,g);if(!['applied','unchanged'].includes(out.status))return fail();
 if(!natural(out.state.publication)||out.state.publication>Number.MAX_SAFE_INTEGER||out.state.documents.some(d=>!natural(d.revision))||out.state.views.some(v=>!natural(v.selectionGeneration))||size(out.state)>o.limits.maxRootBytes)return fail();
 return {...out,clipboardStage:cut?'cut':'pasted'};
}
export function publishClipboardEdit(owner,task,state,g){
 const t=owners.get(owner)?.tasks.get(task);if(t?.localIntent||t?.stage)return {status:'not-published',state};
 const out=clipboardCandidate(owner,task,state,g);if(['applied','unchanged'].includes(out.status)){t.published=true;t.localStatus=out.status;}return out;
}
// Trusted composition facets, not northbound capabilities. A continuation retains the
// original request/receipt and may be claimed by exactly one local ledger ticket.
const localValue=t=>({request:t.request,registration:t.registration,target:t.target,payload:t.payload,receipt:t.result});
const localBinding=plan=>{const p=invocations.get(plan);return p&&owners.get(p.owner)?.tasks.get(p.task)===p.t&&p.t.localIntent===plan&&p.t.result===p.receipt?p:null;};
export function prepareClipboardInvocation(owner,task,maximum){
 const o=owners.get(owner),t=o?.tasks.get(task);if(!t||!natural(maximum)||t.request.operation==='copy'||t.result?.outcome!=='accepted'||t.contested||t.localStatus!==null||t.stage||size(localValue(t))>maximum)return null;
 if(t.localIntent)return t.localIntent;const plan=Object.freeze({kind:'clipboard-local-intent/1'});t.localIntent=plan;invocations.set(plan,{owner,task,t,receipt:t.result,claim:null});return plan;
}
export function clipboardInvocationIntent(plan,maximum){const p=localBinding(plan);return p&&natural(maximum)&&size(localValue(p.t))<=maximum?{invocation:p.t.request.invocation,value:copy(localValue(p.t))}:null;}
export function claimClipboardInvocation(plan,ticket){const p=localBinding(plan);if(!p||p.claim!==null&&p.claim!==ticket)return false;p.claim=ticket;return true;}
export function releaseClipboardInvocation(plan,ticket,g){const p=localBinding(plan);if(!p||p.claim!==ticket||p.claim!==null&&p.t.localStatus===null||p.t.stage||!allowed(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))return false;p.t.localIntent=null;invocations.delete(plan);return true;}
export function clipboardInvocationEffect(plan){const p=localBinding(plan);return p&&!p.t.contested?'confirmed':'unknown';}
const stageEligibility=g=>Object.fromEntries([...new Set([...catalog.publicationGates,'sourceReadable','sourceWritable','allTargetsWritable','viewWritable','bounded','interpretationCurrent','viewVersionsAvailable','versionsAvailable','completeIdentity','targetRestorationValid','invocationFresh','maxParticipants','parserSourceLimits','parserSourceReserved'])].map(k=>[k,copy(g?.[k])]));
export function stageClipboardInvocation(plan,ticket,state,g,stopped=false){
 const p=localBinding(plan),o=p&&owners.get(p.owner);if(!p||p.claim!==ticket||p.t.stage||p.t.localStatus!==null)return null;
 const extra=4*o.limits.maxRootBytes,eligibility=stageEligibility(g);let result={status:'not-published',state},full=false;
 if(!stopped&&size(eligibility)<=o.limits.maxEnvelopeBytes&&extra<=o.limits.maxHeldBytes-clipboardUsage(p.owner).heldBytes){result=clipboardCandidate(p.owner,p.task,state,g);full=['applied','unchanged'].includes(result.status);}
 const token=Object.freeze({kind:'clipboard-local-stage/1'}),record={token,receipt:p.receipt,status:result.status,full};
 if(full){record.before=state;record.beforeBytes=JSON.stringify(state);record.result=result;record.candidate=result.state;record.resultBytes=JSON.stringify(result);record.eligibility=eligibility;}
 p.t.stage=record;return {stage:token,result};
}
export function discardClipboardStage(plan,ticket,stage){const p=localBinding(plan);if(!p||p.claim!==ticket||p.t.stage?.token!==stage)return false;p.t.stage=null;return true;}
export function validateClipboardStage(plan,ticket,stage,before,result,g){
 const p=localBinding(plan),s=p?.t.stage;if(!p||p.claim!==ticket||!s||s.token!==stage||p.t.localStatus!==null||s.receipt!==p.t.result)return false;
 if(!s.full)return result.status==='not-published'&&result.state===before;
 return !p.t.stopped&&!p.t.contested&&s.before===before&&s.result===result&&s.candidate===result.state&&s.beforeBytes===JSON.stringify(before)&&s.resultBytes===JSON.stringify(result)&&equal(s.eligibility,stageEligibility(g));
}
export function commitClipboardStage(plan,ticket,stage,before,result,g){
 if(!validateClipboardStage(plan,ticket,stage,before,result,g))return false;const t=localBinding(plan).t;
 t.localStatus=result.status==='not-published'?'not-applied':result.status;t.published=['applied','unchanged'].includes(result.status);t.stage=null;return true;
}
export function faultClipboardInvocation(plan,ticket){const p=localBinding(plan);if(!p||p.claim!==ticket)return false;p.t.stage=null;if(!p.t.published)p.t.localStatus='unknown';return true;}
export function controlClipboard(owner,task,g){const t=owners.get(owner)?.tasks.get(task);if(!t||!allowed(g,['authorized','scope','controlAllowed']))return unavailable();t.stopped=true;return {status:'stopped'};}
export function inspectClipboard(owner,task,g){
 const t=owners.get(owner)?.tasks.get(task);if(!t||!allowed(g,catalog.releaseGates))return unavailable();return {status:'clipboard-evidence',phase:t.phase,stopped:t.stopped,quiescent:t.quiescent,contested:t.contested,outcome:t.result?.outcome??'unknown',localPublication:t.localStatus??'not-applied',localDecision:t.localStatus??'pending',cutDisposition:t.request.operation==='cut-export-stage'&&t.result?.outcome==='accepted'&&!t.contested&&!t.published&&t.localStatus!=='unknown'?'copied-not-cut':null};
}
export function retireClipboard(owner,task,g){const o=owners.get(owner),t=o?.tasks.get(task);if(!t||!allowed(g,['retirementAllowed','retentionReleased','deliveryDrained','sessionReferencesReleased'])||t.localIntent||t.stage||t.localStatus==='unknown'||t.phase==='queued'&&!t.stopped||t.phase==='dispatched'&&(!t.quiescent||!t.result||t.contested)||t.request.operation!=='copy'&&!t.published&&t.localStatus!=='not-applied'&&!t.stopped&&t.result?.outcome==='accepted')return unavailable();if(t.fragmentHandle)o.structured.delete(t.fragmentHandle);o.tasks.delete(task);return {status:'retired'};}
export function clipboardUsage(owner){const o=owners.get(owner);return o?{tasks:o.tasks.size,issued:o.used.size,outstanding:[...o.tasks.values()].filter(held).length,heldBytes:o.tasks.size*o.charge+[...o.tasks.values()].filter(t=>t.stage?.full).length*4*o.limits.maxRootBytes}:null;}
