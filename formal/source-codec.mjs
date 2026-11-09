// Serialized source fidelity reference, not resource I/O or a production UTF-8 backend.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {captureNativeAnnotationArtifact} from './annotation-artifact.mjs';
import {invocationAuthorityRoot,invocationUsage} from './invocation-publication.mjs';
import {sealSourceDestinationArtifact,sealAnnotationDestinationArtifact} from './source-destination-state.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./source-codec.json',import.meta.url),'utf8'));
const annotationRetention=JSON.parse(fs.readFileSync(new URL('./annotation-retention.json',import.meta.url),'utf8'));
const owners=new WeakMap(),candidates=new WeakMap(),annotationPreparations=new WeakMap(),copy=structuredClone,bomBytes=[239,187,191];
const artifactCatalog=JSON.parse(fs.readFileSync(new URL('./annotation-artifact.json',import.meta.url),'utf8'));
export const annotationPreparationGates=Object.freeze([...artifactCatalog.gates,'ownerControlAllowed']);
const pending=o=>(o.annotationPreparation?1:0)+(o.sourcePreparation?1:0);
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const natural=n=>Number.isSafeInteger(n)&&n>=0,size=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const string=s=>typeof s==='string'&&Buffer.from(s,'utf8').toString('utf8')===s;
const identity=r=>exact(r,['authority','id','incarnation'])&&string(r.authority)&&r.authority.length>0&&string(r.id)&&r.id.length>0&&natural(r.incarnation);
const bytes=xs=>Array.isArray(xs)&&Object.keys(xs).length===xs.length&&Object.keys(xs).every((key,i)=>key===String(i))&&xs.every(b=>Number.isInteger(b)&&b>=0&&b<=255);
const startGates=['authorized','sourceReadable','scope','continuous','live','notStopped','resourcesReady','contextCurrent','coherentInput'];
const captureGates=['authorized','sourceReadable','scope','continuous','live','notStopped','resourcesReady'];
const readGates=['authorized','scope','continuous','live','notStopped','retained','replyReady'];
const allowed=(g,ks)=>ks.every(k=>g?.[k]);
const strictlyAllowed=(g,ks)=>ks.every(k=>g?.[k]===true);
const unavailable=()=>({status:'unavailable'});
const leadingBom=source=>bomBytes.every((b,i)=>source[i]===b);
const boundary=(source,p)=>natural(p)&&p<=source.length&&(p===source.length||(source[p]&0xc0)!==0x80);
export function utf8Step(state,byte){if(!natural(byte)||byte>255)return 'failed';const edge=catalog.utf8States[state]?.find(([a,b])=>a<=byte&&byte<=b);return edge?.[2]??'failed';}
export const utf8AtEnd=state=>state==='start';
const validUtf8=source=>{let state='start';for(const b of source){state=utf8Step(state,b);if(state==='failed')return false;}return utf8AtEnd(state);};
function contextValid(c,l){return exact(c,catalog.contextFields)&&c.profile===catalog.profile&&identity(c.provider)&&identity(c.resource)&&natural(c.bindingGeneration)&&
  exact(c.resourceVersion,['epoch','token'])&&string(c.resourceVersion.epoch)&&c.resourceVersion.epoch.length>0&&string(c.resourceVersion.token)&&
  exact(c.origin,['operation','effect','attempt'])&&Object.values(c.origin).every(s=>string(s)&&s.length>0)&&size(c)<=l.maxContextBytes;}
export function newSourceCodecRegistry(limits){
  if(!exact(limits,catalog.limits)||!Object.values(limits).every(natural)||!natural(limits.maxInputBytes+limits.maxCandidateBytes+limits.maxContextBytes))return null;
  const ref=Object.freeze({profile:catalog.profile});owners.set(ref,{limits:copy(limits),imports:new Map(),artifacts:new Map(),issued:0,held:0});return ref;
}
export function beginSourceImport(owner,context,g){
  const o=owners.get(owner);if(!o||!allowed(g,startGates)||!contextValid(context,o.limits)||o.imports.size>=o.limits.maxImports||o.issued>=o.limits.maxIssued-pending(o))return unavailable();
  const charge=o.limits.maxInputBytes+o.limits.maxCandidateBytes+size(context);if(charge>o.limits.maxHeldBytes-o.held)return unavailable();
  const ref=Object.freeze({kind:'source-import/1'});o.imports.set(ref,{phase:'open',context:copy(context),charge,source:[],prefix:[],bom:null,utf8:'start',inputBytes:0,chunks:0,candidate:null});o.issued++;o.held+=charge;return {status:'open',import:ref};
}
function feed(s,b,l){
  s.utf8=utf8Step(s.utf8,b);if(s.utf8==='failed'){s.phase='failed';return false;}
  if(s.source.length>=l.maxSourceBytes||s.source.length>=l.maxCoordinate){s.phase='failed';return false;}
  s.source.push(b);return true;
}
function consume(s,b,l){
  if(s.bom!==null)return feed(s,b,l);
  s.prefix.push(b);
  if(s.prefix.every((value,i)=>value===bomBytes[i])){if(s.prefix.length===3){s.bom=true;s.prefix=[];}return true;}
  s.bom=false;const prefix=s.prefix;s.prefix=[];return prefix.every(value=>feed(s,value,l));
}
export function appendSourceChunk(owner,ref,context,chunk,g){
  const o=owners.get(owner);if(!o||!allowed(g,startGates))return unavailable();const s=o.imports.get(ref);if(!s||s.phase!=='open'||!equal(context,s.context))return unavailable();
  if(!exact(chunk,['sequence','offset','bytes'])||!natural(chunk.sequence)||!natural(chunk.offset)||chunk.sequence!==s.chunks||chunk.offset!==s.inputBytes)return unavailable();
  const l=o.limits;if(!Array.isArray(chunk.bytes)||chunk.bytes.length>l.maxChunkBytes||!bytes(chunk.bytes)||s.chunks>=l.maxChunks||chunk.bytes.length>l.maxInputBytes-s.inputBytes||chunk.bytes.length>l.maxCoordinate-s.inputBytes){s.phase='failed';return {status:'failed'};}
  s.chunks++;s.inputBytes+=chunk.bytes.length;
  for(const b of chunk.bytes)if(!consume(s,b,l))return {status:'failed'};
  return {status:'open'};
}
export function finishSourceImport(owner,ref,context,g){
  const o=owners.get(owner);if(!o||!allowed(g,startGates))return unavailable();const s=o.imports.get(ref);if(!s||!equal(context,s.context))return unavailable();
  if(s.phase==='complete')return {status:'candidate',candidate:s.candidate};if(s.phase!=='open')return unavailable();
  if(s.bom===null){s.bom=false;const prefix=s.prefix;s.prefix=[];if(!prefix.every(b=>feed(s,b,o.limits)))return {status:'failed'};}
  if(!utf8AtEnd(s.utf8)){s.phase='failed';return {status:'failed'};}
  const payload={context:s.context,source:s.source,bom:s.bom,rawByteLength:s.inputBytes};
  if(size(payload)>o.limits.maxCandidateBytes){s.phase='failed';return {status:'failed'};}
  const candidate=Object.freeze({kind:'source-candidate/1'});candidates.set(candidate,{owner:o,stream:s});s.candidate=candidate;s.phase='complete';return {status:'candidate',candidate};
}
function candidateAt(o,ref){const c=candidates.get(ref);return c&&c.owner===o&&c.stream.phase==='complete'?c.stream:null;}
// Actual codec records for fixed first-party activation, never content release.
export function sourceCandidateActivationWitness(owner,ref,context){const o=owners.get(owner),stream=o&&candidateAt(o,ref);return stream&&equal(stream.context,context)?{codecOwner:owner,codec:o,stream,candidate:ref}:null;}
export function inspectSourceCandidate(owner,ref,context,g){
  const o=owners.get(owner);if(!o||!allowed(g,readGates))return unavailable();const s=candidateAt(o,ref);if(!s||!equal(s.context,context))return unavailable();
  const result={status:'source-candidate',context:s.context,source:s.source,bom:s.bom,rawByteLength:s.inputBytes};return size(result)<=o.limits.maxReplyBytes?copy(result):unavailable();
}
export function mapSourceFilePosition(owner,ref,context,direction,position,g){
  const o=owners.get(owner);if(!o||!allowed(g,readGates))return unavailable();const s=candidateAt(o,ref);if(!s||!equal(s.context,context)||!['document-to-file','file-to-document'].includes(direction)||!natural(position))return unavailable();
  const prefix=s.bom?3:0,doc=direction==='document-to-file'?position:position-prefix;
  if(!boundary(s.source,doc))return unavailable();const out={status:'position',position:direction==='document-to-file'?doc+prefix:doc};return size(out)<=o.limits.maxReplyBytes?out:unavailable();
}
export function abortSourceImport(owner,ref,g){const s=owners.get(owner)?.imports.get(ref);if(!s||!g?.controlAllowed||s.phase==='retired')return unavailable();if(s.phase==='aborted')return {status:'unchanged'};s.phase='aborted';return {status:'aborted'};}
export function retireSourceImport(owner,ref,g){const o=owners.get(owner),s=o?.imports.get(ref);if(!s||!g?.retentionReleased||!g.deliveryDrained)return unavailable();s.phase='retired';s.source=[];s.prefix=[];s.context=null;s.candidate=null;o.held-=s.charge;o.imports.delete(ref);return {status:'retired'};}
export function captureSerializedSource(owner,state,r,g){return sourceCapture(owner,state,r,g,false);}
export function prepareSourceDestinationArtifact(owner,state,r,g){return sourceCapture(owner,state,r,g,true);}
function sourceCapture(owner,state,r,g,prepare){
  const o=owners.get(owner);if(!o||!allowed(g,captureGates)||!exact(r,catalog.captureFields)||r.contract!==catalog.captureContract||r.operation!=='capture-source'||!identity(r.document)||r.document.authority!==state.authority||!natural(r.expectedSource)||!natural(r.expectedSerialization)||o.artifacts.size>=o.limits.maxArtifacts-pending(o)||o.issued>=o.limits.maxIssued-pending(o)||o.limits.maxArtifactBytes>o.limits.maxHeldBytes-o.held)return unavailable();
  const ds=state.documents.filter(d=>equal(d.document,r.document));if(ds.length!==1)return unavailable();const d=ds[0],l=o.limits,m=d.serialization;
  if(d.live===false||d.revision!==r.expectedSource||!exact(m,catalog.serializationFields)||m.generation!==r.expectedSerialization||m.profile!==catalog.profile||typeof m.bom!=='boolean'||!Array.isArray(d.source)||d.source.length>l.maxSourceBytes||!bytes(d.source)||!validUtf8(d.source)||!m.bom&&leadingBom(d.source))return unavailable();
  const prefix=m.bom?bomBytes:[];if(d.source.length>l.maxOutputBytes-prefix.length||d.source.length>l.maxCoordinate-prefix.length)return unavailable();
  const payload={document:copy(d.document),sourceRevision:d.revision,serialization:copy(m),source:copy(d.source),bytes:[...prefix,...d.source]},charge=size(payload);
  if(charge>l.maxArtifactBytes)return unavailable();if(prepare){const preparation=sealSourceDestinationArtifact(owner,o,payload);return preparation?{status:'prepared',preparation}:unavailable();}const ref=Object.freeze({kind:'serialized-source/1'});o.artifacts.set(ref,{payload,charge});o.held+=charge;o.issued++;return {status:'captured',artifact:ref};
}
export function readSerializedSource(owner,ref,range,g){
  const o=owners.get(owner);if(!o||!allowed(g,readGates))return unavailable();const a=o.artifacts.get(ref)?.payload;
  if(o.artifacts.get(ref)?.annotation)return unavailable();
  if(!a||!Array.isArray(range)||range.length!==2||!natural(range[0])||!natural(range[1])||range[0]>range[1]||range[1]>a.bytes.length||range[1]-range[0]>o.limits.maxChunkBytes)return unavailable();
  const out={status:'serialized-bytes',document:a.document,sourceRevision:a.sourceRevision,serialization:a.serialization,range:copy(range),bytes:a.bytes.slice(...range)};return size(out)<=o.limits.maxReplyBytes?copy(out):unavailable();
}
export function compareSerializedSources(owner,first,second,g){
  const o=owners.get(owner);if(!o||!allowed(g,readGates))return unavailable();const a=o.artifacts.get(first)?.payload,b=o.artifacts.get(second)?.payload;
  if(o.artifacts.get(first)?.annotation||o.artifacts.get(second)?.annotation)return unavailable();
  if(!a||!b||!equal(a.document,b.document)||a.bytes.length>o.limits.maxCompareBytes-b.bytes.length)return unavailable();
  const out={status:'compared',comparison:equal(a.bytes,b.bytes)?'equal':'different',first:{sourceRevision:a.sourceRevision,serializationGeneration:a.serialization.generation},second:{sourceRevision:b.sourceRevision,serializationGeneration:b.serialization.generation}};return size(out)<=o.limits.maxReplyBytes?out:unavailable();
}
export function retireSerializedSource(owner,ref,g){const o=owners.get(owner),a=o?.artifacts.get(ref);if(!a||a.annotation||!g?.retentionReleased||!g.deliveryDrained)return unavailable();o.held-=a.charge;o.artifacts.delete(ref);return {status:'retired'};}
// Trusted normalized storage composition. The owner/quotas are the existing codec's.
function annotationCaptureValue(o,domain,document,c,binding,profile,g){
  const captured=captureNativeAnnotationArtifact(domain,document,c,binding,profile,g);if(captured.status!=='annotation-artifact')return unavailable();
  const payload={basis:copy(captured.basis),bytes:copy(captured.bytes)},charge=size(payload),l=o.limits;
  if(size(payload.basis.context)>l.maxContextBytes||payload.bytes.length>l.maxOutputBytes||payload.bytes.length>l.maxCoordinate||charge>l.maxArtifactBytes)return unavailable();
  const artifact=Object.freeze({kind:'serialized-annotation/1'}),basis=copy(payload.basis);Object.freeze(basis.context);Object.freeze(basis.document);Object.freeze(basis);
  const result={status:'captured',artifact,basis,byteLength:payload.bytes.length};if(size(result)>l.maxReplyBytes)return unavailable();
  return {payload,charge,artifact,result};
}
export function captureSerializedAnnotation(owner,domain,document,c,binding,profile,g){
  const o=owners.get(owner);if(!o||o.artifacts.size>=o.limits.maxArtifacts-pending(o)||o.issued>=o.limits.maxIssued-pending(o)||o.limits.maxArtifactBytes>o.limits.maxHeldBytes-o.held)return unavailable();
  const value=annotationCaptureValue(o,domain,document,c,binding,profile,g);if(!value.payload)return unavailable();const {artifact,payload,charge,result}=value;
  o.artifacts.set(artifact,{annotation:true,payload,charge,stopped:false});o.held+=charge;o.issued++;return result;
}
// Trusted bounded preparation, not an artifact issued to a reader or a write ticket.
export function prepareSerializedAnnotation(owner,domain,document,c,binding,profile,g){
  const o=owners.get(owner);if(!o||o.annotationPreparation||!strictlyAllowed(g,annotationPreparationGates)||invocationUsage(domain)?.faulted!==false||o.artifacts.size>=o.limits.maxArtifacts-pending(o)||o.issued>=o.limits.maxIssued-pending(o)||o.limits.maxArtifactBytes>o.limits.maxHeldBytes-o.held)return unavailable();
  const root=invocationAuthorityRoot(domain),value=annotationCaptureValue(o,domain,document,c,binding,profile,g);if(!root||!value.payload)return unavailable();
  const preparation=Object.freeze({kind:'annotation-capture-preparation/1'}),charge=size({owner,domain:{kind:'invocation-domain/1'},root:{kind:'immutable-authority-root/1'},value,charge:Number.MAX_SAFE_INTEGER,historicalRequired:true,preparation});
  if(!natural(charge)||charge>o.limits.maxArtifactBytes||charge>o.limits.maxHeldBytes-o.held)return unavailable();
  const row={owner,domain,root,value,charge,historicalRequired:binding.sources.some(s=>s.revision!==binding.sourceRevision)};annotationPreparations.set(preparation,row);o.annotationPreparation=preparation;o.held+=charge;return {status:'prepared',preparation};
}
export function commitSerializedAnnotationPreparation(owner,preparation,domain,g){
  const o=owners.get(owner),s=annotationPreparations.get(preparation);if(!o||!s||s.owner!==owner||s.domain!==domain||o.annotationPreparation!==preparation||!strictlyAllowed(g,annotationPreparationGates)||s.historicalRequired&&g.historicalSourceEvidenceReady!==true||invocationUsage(domain)?.faulted!==false||invocationAuthorityRoot(domain)!==s.root||o.artifacts.size>=o.limits.maxArtifacts-(o.sourcePreparation?1:0)||o.issued>=o.limits.maxIssued-(o.sourcePreparation?1:0)||s.value.charge>o.limits.maxHeldBytes-o.held+s.charge)return unavailable();
  const {artifact,payload,charge,result}=s.value;o.artifacts.set(artifact,{annotation:true,payload,charge,stopped:false});o.held+=charge-s.charge;o.issued++;delete o.annotationPreparation;annotationPreparations.delete(preparation);return result;
}
// Trusted atomic destination admission consumes this existing codec preparation.
export function prepareAnnotationDestinationArtifact(owner,domain,document,c,binding,profile,g){const out=prepareSerializedAnnotation(owner,domain,document,c,binding,profile,g);if(out.status!=='prepared')return unavailable();const o=owners.get(owner),s=annotationPreparations.get(out.preparation),token=sealAnnotationDestinationArtifact(owner,o,out.preparation,annotationPreparations);if(!token){discardSerializedAnnotationPreparation(owner,out.preparation,{ownerControlAllowed:true});return unavailable();}return {status:'prepared',preparation:token,basis:copy(s.value.payload.basis),byteLength:s.value.payload.bytes.length};}
// Package-private normalized preparation input, not a recipient acquisition verb.
export function inspectSerializedAnnotationPreparation(owner,preparation,domain,g){
  const o=owners.get(owner),s=annotationPreparations.get(preparation);if(!o||!s||s.owner!==owner||s.domain!==domain||o.annotationPreparation!==preparation||!strictlyAllowed(g,annotationPreparationGates)||s.historicalRequired&&g.historicalSourceEvidenceReady!==true||invocationUsage(domain)?.faulted!==false||invocationAuthorityRoot(domain)!==s.root)return unavailable();
  const out={status:'prepared-annotation',basis:copy(s.value.payload.basis),bytes:copy(s.value.payload.bytes),byteLength:s.value.payload.bytes.length};return size(out)<=o.limits.maxReplyBytes?out:unavailable();
}
export function discardSerializedAnnotationPreparation(owner,preparation,g){
  const o=owners.get(owner),s=annotationPreparations.get(preparation);if(!o||!s||s.owner!==owner||o.annotationPreparation!==preparation||g?.ownerControlAllowed!==true)return unavailable();
  o.held-=s.charge;delete o.annotationPreparation;annotationPreparations.delete(preparation);return {status:'discarded'};
}
export function readSerializedAnnotation(owner,ref,range,basis,g){
  const o=owners.get(owner),entry=o?.artifacts.get(ref),a=entry?.payload;if(!entry?.annotation||entry.stopped||!equal(basis,a.basis)||!annotationRetention.releaseGates.every(key=>g?.[key]===true))return unavailable();
  if(!Array.isArray(range)||range.length!==2||!natural(range[0])||!natural(range[1])||range[0]>range[1]||range[1]>a.bytes.length||range[1]-range[0]>o.limits.maxChunkBytes)return unavailable();
  const result={status:'serialized-annotation-bytes',range:copy(range),bytes:a.bytes.slice(...range)};return size(result)<=o.limits.maxReplyBytes?result:unavailable();
}
export function stopSerializedAnnotation(owner,ref,basis,g){const entry=owners.get(owner)?.artifacts.get(ref);if(!entry?.annotation||!equal(basis,entry.payload.basis)||g?.ownerControlAllowed!==true)return unavailable();entry.stopped=true;return {status:'stopped'};}
export function compareSerializedAnnotations(owner,first,firstBasis,firstGates,second,secondBasis,secondGates){
  const o=owners.get(owner),left=o?.artifacts.get(first),right=o?.artifacts.get(second);
  if(!left?.annotation||!right?.annotation||left.stopped||right.stopped||![firstGates,secondGates].every(g=>annotationRetention.releaseGates.every(k=>g?.[k]===true)&&g?.comparisonAllowed===true))return unavailable();
  const a=left.payload,b=right.payload;if(!equal(firstBasis,a.basis)||!equal(secondBasis,b.basis)||!equal(a.basis.context,b.basis.context)||!equal(a.basis.document,b.basis.document)||a.bytes.length>o.limits.maxCompareBytes-b.bytes.length)return unavailable();
  const result={status:'annotation-output-comparison',comparison:equal(a.bytes,b.bytes)?'equal':'different',first:{sourceRevision:a.basis.sourceRevision,annotationSet:a.basis.annotationSet,byteLength:a.bytes.length},second:{sourceRevision:b.basis.sourceRevision,annotationSet:b.basis.annotationSet,byteLength:b.bytes.length}};
  return size(result)<=o.limits.maxReplyBytes?copy(result):unavailable();
}
export function retireSerializedAnnotation(owner,ref,g){const o=owners.get(owner),entry=o?.artifacts.get(ref);if(!entry?.annotation||g?.ownerControlAllowed!==true||g.retentionReleased!==true||g.deliveryDrained!==true)return unavailable();o.held-=entry.charge;o.artifacts.delete(ref);return {status:'retired'};}
export function sourceCodecUsage(owner){const o=owners.get(owner);return o?{imports:o.imports.size,artifacts:o.artifacts.size,issued:o.issued,heldBytes:o.held}:null;}
export function annotationPreparationUsage(owner){const o=owners.get(owner),s=o&&annotationPreparations.get(o.annotationPreparation);return o?{preparations:pending(o),reservedArtifacts:pending(o),reservedIssuance:pending(o),heldBytes:s?.charge??0}:null;}
export function sourceImportStatus(owner,ref){const s=owners.get(owner)?.imports.get(ref);return s?{phase:s.phase,utf8:s.utf8,bom:s.bom,inputBytes:s.inputBytes,chunks:s.chunks}:null;}
