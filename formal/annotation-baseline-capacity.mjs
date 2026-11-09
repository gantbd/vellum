// Finite normalized sizing contract, not authority or an installed reservation.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
const fields=JSON.parse(fs.readFileSync(new URL('./annotation-baselines.json',import.meta.url))).baselineFields;
const contextFields=JSON.parse(fs.readFileSync(new URL('./annotation-artifact.json',import.meta.url))).contextFields;
const copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const natural=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const scalar=s=>typeof s==='string'&&Buffer.from(s).toString('utf8')===s;
const version=v=>exact(v,['epoch','token'])&&scalar(v.epoch)&&v.epoch.length>0&&scalar(v.token);
const identity=v=>exact(v,['authority','id','incarnation'])&&scalar(v.authority)&&v.authority.length>0&&scalar(v.id)&&v.id.length>0&&natural(v.incarnation);
const limits=l=>exact(l,['maxBaselineBytes','maxRootBytes','maxTagBytes'])&&Object.values(l).every(natural);
const octets=xs=>Array.isArray(xs)&&Object.keys(xs).length===xs.length&&Object.keys(xs).every((k,i)=>k===String(i))&&xs.every(n=>natural(n)&&n<=255);
function valid(b){return exact(b,fields)&&b.kind==='acknowledged-annotation-write'&&[b.annotationBindingGeneration,b.receiptBindingGeneration,b.sourceRevision,b.annotationSet,b.dispatchSerial].every(natural)&&b.annotationBindingGeneration>0&&b.profile==='vellum.annotations/1'&&exact(b.context,contextFields)&&Object.values(b.context).every(s=>scalar(s)&&s.length>0)&&exact(b.origin,['operation','effect','attempt'])&&Object.values(b.origin).every(s=>scalar(s)&&s.length>0)&&identity(b.provider)&&scalar(b.procedure)&&b.procedure.length>0&&['written','synchronized'].includes(b.acknowledgment)&&['ordinary','write-only','synchronized'].includes(b.requiredAcknowledgment)&&(b.resourceVersion===null||version(b.resourceVersion))&&octets(b.bytes);}
const template=b=>({...copy(b),resourceVersion:null,dispatchSerial:Number.MAX_SAFE_INTEGER,acknowledgment:'synchronized'});
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
// Only these three future evidence fields vary. Binding, context, origin, bytes,
// source/set and captured requested strength remain immutable.
export function prepareAnnotationBaselineEnvelope(draft,l){
 if(!limits(l)||!valid(draft))return null;
 const value=template(draft),versionGrowth=Math.max(0,l.maxTagBytes-size(null)),baselineBytes=size(value)+versionGrowth;
 if(!natural(baselineBytes)||baselineBytes>l.maxBaselineBytes)return null;
 return freeze({kind:'annotation-baseline-envelope/1',template:value,maxTagBytes:l.maxTagBytes,baselineBytes});
}
function validEnvelope(e,l){if(!limits(l)||!exact(e,['kind','template','maxTagBytes','baselineBytes'])||e.kind!=='annotation-baseline-envelope/1'||e.maxTagBytes!==l.maxTagBytes)return false;const actual=prepareAnnotationBaselineEnvelope(e.template,l);return actual!==null&&equal(actual,e);}
export function annotationBaselineWithinEnvelope(envelope,b,l){
 if(!validEnvelope(envelope,l)||!valid(b)||b.resourceVersion!==null&&size(b.resourceVersion)>l.maxTagBytes||!equal(template(b),envelope.template))return false;
 return size(b)<=envelope.baselineBytes;
}
// Re-evaluate against the CURRENT root, not the root that admitted a write.
// A host must enforce this floor on every competing publication before a held
// envelope can become an ordinary-save reservation promise.
export function annotationBaselineRootCapacity(state,document,envelope,l){
 if(!validEnvelope(envelope,l)||!identity(document)||!state||state.authority!==document.authority||!natural(state.publication)||state.publication===Number.MAX_SAFE_INTEGER||size(state)>l.maxRootBytes||!Array.isArray(state.documents))return null;
 const rows=state.documents.filter(d=>equal(d.document,document));if(rows.length!==1||rows[0].live!==true)return null;
 const next=copy(state);next.documents.find(d=>equal(d.document,document)).annotationBaseline=null;next.publication=Number.MAX_SAFE_INTEGER;
 const rootBytes=size(next)-size(null)+envelope.baselineBytes;
 if(!natural(rootBytes)||rootBytes>l.maxRootBytes)return null;
 return {baselineBytes:envelope.baselineBytes,rootBytes,additionalRootBytes:Math.max(0,rootBytes-size(state)),publicationUnits:1};
}
