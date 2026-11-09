// Closed private F13 receipt policy. Host identity/grant truth and public encodings remain external.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-results.json',import.meta.url))),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort()),natural=n=>Number.isSafeInteger(n)&&n>=0,version=n=>natural(n)&&n>0;
const label=s=>typeof s==='string'&&s.length>0&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Buffer.from(s))===s;
const document=d=>exact(d,['authority','id','incarnation'])&&label(d.authority)&&label(d.id)&&version(d.incarnation),annotation=a=>exact(a,['document','id','incarnation'])&&document(a.document)&&label(a.id)&&version(a.incarnation);
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;},unavailable=()=>({status:'unavailable'}),none=()=>({identity:false,contentVersion:false,targetGeneration:false});
export function validAnnotationResultCapabilities(c){return exact(c,catalog.capabilities)&&catalog.capabilities.every(k=>typeof c[k]==='boolean')&&(c.identity||!c.contentVersion&&!c.targetGeneration);}
export function validAnnotationResultReceipt(r){if(!r||!label(r.invocation)||!catalog.operations.includes(r.operation))return false;if(r.status==='not-published')return exact(r,['status','invocation','operation']);return ['applied','unchanged'].includes(r.status)&&exact(r,['status','invocation','operation','annotation','membership','contentVersion','targetGeneration',...(r.status==='applied'?['publication']:[])])&&(r.status!=='applied'||r.publication===r.invocation)&&(r.status!=='unchanged'||!['create','delete'].includes(r.operation))&&annotation(r.annotation)&&r.membership===(r.operation==='delete'?'retained-absent':'present')&&version(r.contentVersion)&&version(r.targetGeneration);}
export function annotationResultTarget(value){const r=value?.request,a=r?.operation==='create'?value.allocation:r?.annotation;if(!r||!catalog.operations.includes(r.operation)||!label(r.invocation)||!document(r.document)||!a||!label(a.id)||!version(a.incarnation))return null;const target={document:copy(r.document),id:a.id,incarnation:a.incarnation};return annotation(target)?target:null;}
function grantCapabilities(invocation,target,g){if(!Object.hasOwn(g??{},'annotationResultGrant'))return none();const p=g.annotationResultGrant;if(!exact(p,catalog.grantFields)||p.invocation!==invocation||!equal(p.annotation,target))return null;const caps=Object.fromEntries(catalog.capabilities.map(k=>[k,p[k]]));return validAnnotationResultCapabilities(caps)?caps:null;}
export function prepareAnnotationResultAdmission(value,c,g,maximum){
 const target=annotationResultTarget(value),r=value?.request;if(!target||value.recipient!==c.recipient||value.profile!==c.profile||target.document.authority!==c.authority||!natural(maximum))return null;
 const ceiling=grantCapabilities(r.invocation,target,g);if(!ceiling)return null;
 // Reserve the complete private receipt and every permitted public frame before execution.
 const full={status:'applied',invocation:r.invocation,operation:r.operation,publication:r.invocation,annotation:target,membership:'retained-absent',contentVersion:Number.MAX_SAFE_INTEGER,targetGeneration:Number.MAX_SAFE_INTEGER};
 if(size(full)>maximum||size({status:'annotation-result',result:full})>maximum||size(unavailable())>maximum)return null;return freeze(copy(ceiling));
}
export function captureAnnotationResult(value,result){
 const target=annotationResultTarget(value),r=value?.request;if(!target||!['applied','unchanged','not-published'].includes(result?.status))throw Error('invalid native annotation result');
 const base={status:result.status,invocation:r.invocation,operation:r.operation};if(result.status==='not-published')return freeze(base);
 if(result.status==='unchanged'&&['create','delete'].includes(r.operation))throw Error('membership result cannot be unchanged');
 const ds=result.state?.documents?.filter(d=>equal(d.document,target.document));if(ds?.length!==1||ds[0].live===false)throw Error('annotation result owner missing');const d=ds[0],rows=d.records.filter(a=>a.id===target.id&&a.incarnation===target.incarnation);let record,membership;
 if(r.operation==='delete'){const slots=d.annotationLifecycle?.entries?.filter(s=>s.id===target.id&&s.incarnation===target.incarnation);if(rows.length||slots?.length!==1||slots[0].phase!=='retained-absent'||!slots[0].retained)throw Error('deleted annotation result missing');record=slots[0].retained;membership='retained-absent';}
 else{if(rows.length!==1)throw Error('live annotation result missing');record=rows[0];membership='present';}
 if(record.id!==target.id||record.incarnation!==target.incarnation||!version(record.bodyVersion)||!version(record.targetGeneration))throw Error('annotation result versions invalid');
 return freeze({...base,...(result.status==='applied'?{publication:r.invocation}:{}),annotation:target,membership,contentVersion:record.bodyVersion,targetGeneration:record.targetGeneration});
}
export function releaseAnnotationResult(receipt,ceiling,g,maximum){
 if(!validAnnotationResultReceipt(receipt)||!validAnnotationResultCapabilities(ceiling)||!natural(maximum))return unavailable();
 if(receipt.status==='not-published'){const out={status:'annotation-result',result:copy(receipt)};return size(out)<=maximum?out:unavailable();}
 const current=grantCapabilities(receipt.invocation,receipt.annotation,g);if(!current||!ceiling.identity||!current.identity)return unavailable();
 const {contentVersion,targetGeneration,...base}=receipt,result={...copy(base),...(ceiling.contentVersion&&current.contentVersion?{contentVersion}:{}),...(ceiling.targetGeneration&&current.targetGeneration?{targetGeneration}:{})},out={status:'annotation-result',result};return size(out)<=maximum?out:unavailable();
}
