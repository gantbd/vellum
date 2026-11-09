// Private normalized initial restoration; file data is not identity authority.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {decodeSnapshot,stageSnapshot} from './annotation-snapshot.mjs';
import {annotationRestorationGates,annotationRestorationReady,annotationRestorationTargetBasis,restorationSourceValid,sealAnnotationRestoration,discardAnnotationRestoration} from './annotation-restoration-state.mjs';
import {invocationAuthorityRoot,invocationAuthorityContext,invocationUsage,publishInvocationAnnotationRestoration} from './invocation-publication.mjs';
export {annotationRestorationGates};
import {validCompoundTarget} from './annotation-targets.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-artifact.json',import.meta.url))),copy=structuredClone;
const size=v=>Buffer.byteLength(JSON.stringify(v)),natural=n=>Number.isSafeInteger(n)&&n>=0,positive=n=>natural(n)&&n>0;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const label=s=>typeof s==='string'&&s.length>0&&Buffer.from(s,'utf8').toString('utf8')===s;
const decimal=s=>typeof s==='string'&&/^(?:0|[1-9][0-9]*)$/.test(s)&&BigInt(s)<=BigInt(Number.MAX_SAFE_INTEGER)?Number(s):null;
function target(t,sources){const source=sources.find(s=>equal(s.evidence,t.source));if(!source)return null;const out=t.kind==='document'?{kind:'document'}:t.kind==='point'?{kind:'point',offset:decimal(t.offset),affinity:t.affinity,profile:t.profile}:{kind:'range',start:decimal(t.start),end:decimal(t.end)};return validCompoundTarget(source.bytes,out)?{sourceRevision:source.revision,target:out}:null;}
export function prepareAnnotationRestoration(domain,root,authority,document,input,c,binding,profile,g){
 try{
  if(!exact(profile,['native','snapshot','membership'])||!exact(profile.native,catalog.limits)||!Object.values(profile.native).every(natural))return null;const nativeProfile={native:profile.native,membership:profile.membership},d=annotationRestorationReady(root,authority,document,nativeProfile,g);if(!d||!exact(c,catalog.contextFields)||!Object.values(c).every(label)||c.authority!==authority.authority||c.engine!==authority.engine||!exact(binding,['context','document','sourceRevision','annotationSet','association','identities','sources'])||!equal(binding.context,c)||!equal(binding.document,document)||binding.sourceRevision!==d.revision||binding.annotationSet!==d.annotationSet||size(binding)>profile.native.maxBindingBytes)return null;
  const decoded=decodeSnapshot(input,profile.snapshot);if(decoded.status!=='decoded')return null;const v=copy(decoded.value),l=profile.native;if(input.length>l.maxArtifactBytes||size({status:'unchanged'})>l.maxReplyBytes||size({status:'restored'})>l.maxReplyBytes)return null;
  if(!exact(binding.association,['namespace','value'])||!Object.values(binding.association).every(label)||!Array.isArray(binding.identities)||binding.identities.length!==v.records.length||binding.identities.length>l.maxAliases||!Array.isArray(binding.sources)||binding.sources.length>l.maxAliases||v.records.length>l.maxRecords)return null;
  const nativeIds=new Set(),aliases=new Set(),revisions=new Set(),evidence=new Set();for(const a of binding.identities){const key=JSON.stringify([a.namespace,a.value]);if(!exact(a,['id','incarnation','namespace','value'])||![a.id,a.namespace,a.value].every(label)||!positive(a.incarnation)||nativeIds.has(a.id)||aliases.has(key))return null;nativeIds.add(a.id);aliases.add(key);}
  for(const s of binding.sources){const key=JSON.stringify([s.evidence?.domain,s.evidence?.token]);if(!exact(s,['revision','evidence','bytes'])||!positive(s.revision)||!exact(s.evidence,['domain','token','byteLength'])||!label(s.evidence.domain)||!label(s.evidence.token)||!restorationSourceValid(s.bytes,l.maxSourceBytes)||s.evidence.byteLength!==String(s.bytes.length)||revisions.has(s.revision)||evidence.has(key))return null;revisions.add(s.revision);evidence.add(key);}
  const source=binding.sources.find(s=>s.revision===d.revision);if(!source||!equal(source.bytes,d.source)||!equal(source.evidence,v.source)||binding.sources.some(s=>s.revision!==d.revision)&&g.historicalSourceEvidenceReady!==true)return null;
  const staged=stageSnapshot(input,profile.snapshot,{document:binding.association,source:source.evidence,bytes:Uint8Array.from(d.source),annotationSet:String(d.annotationSet)},{authorized:true,resources:true,notStopped:true,targetEvidence:true,identityRestorable:true,annotationSetCurrent:true});if(staged.status!=='staged')return null;
  const proof={context:c,document,binding,snapshot:v};if(!equal(g.restorationBasis,proof))return null;
  const records=[],used=new Set([d.revision]);for(const r of v.records){const a=binding.identities.find(a=>a.namespace===r.identity.namespace&&a.value===r.identity.value),incarnation=decimal(r.identity.incarnation),bodyVersion=decimal(r.contentVersion),targetGeneration=decimal(r.targetGeneration),authored=target(r.authoredTarget,binding.sources);if(!a||incarnation!==a.incarnation||!positive(incarnation)||!positive(bodyVersion)||!positive(targetGeneration)||!authored)return null;used.add(authored.sourceRevision);let placement={state:'detached'};if(r.placement.state!=='detached'){const located=target(r.placement.target,binding.sources);if(!located||located.sourceRevision!==d.revision)return null;placement={state:r.placement.state,...located};used.add(located.sourceRevision);}records.push({id:a.id,incarnation,body:r.body,bodyVersion,target:authored,targetGeneration,placement});}
  if(used.size!==binding.sources.length)return null;
  return sealAnnotationRestoration(domain,document,c,annotationRestorationTargetBasis(d),records,nativeProfile,proof,binding.sources.some(s=>s.revision!==d.revision));
 }catch{return null;}
}

// Edge composition: serialized data ends at the typed native participant boundary.
export function restoreInvocationAnnotations(domain,document,input,c,binding,profile,g){
 if(invocationUsage(domain)?.faulted!==false)return {status:'unavailable'};const plan=prepareAnnotationRestoration(domain,invocationAuthorityRoot(domain),invocationAuthorityContext(domain),document,input,c,binding,profile,g);if(!plan)return {status:'unavailable'};try{return publishInvocationAnnotationRestoration(domain,plan,g);}finally{discardAnnotationRestoration(plan);}
}
