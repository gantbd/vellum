// Existing released F13/F14 values only; no public retrieval route or codec.
import fs from 'node:fs';
const annotation=JSON.parse(fs.readFileSync(new URL('./annotation-results.json',import.meta.url),'utf8'));
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const declarations=freeze({
 annotation:{envelope:'annotation-result',operations:[...annotation.operations],notPublished:['status','invocation','operation'],published:['status','invocation','operation','annotation','membership'],optionalVersions:[...annotation.capabilities.filter(k=>k!=='identity')],annotationFields:['document','id','incarnation'],documentFields:['authority','id','incarnation']},
 reversal:{envelope:'reversal-result',operations:['undo','redo'],notPublished:['status','invocation','operation'],published:['status','invocation','operation','publication'],groupTarget:['kind','group'],frontierTarget:['kind','scope','expectedGroup']}
});
function fields(value){
 if(!value||typeof value!=='object'||Array.isArray(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;
 const ds=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(ds);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(ds[k],'value')||ds[k].enumerable!==true))return null;
 return Object.fromEntries(keys.map(k=>[k,ds[k].value]));
}
const exact=(v,keys)=>v&&Object.keys(v).length===keys.length&&keys.every(k=>Object.hasOwn(v,k));
const label=v=>typeof v==='string'&&v.length>0&&Buffer.from(v,'utf8').toString('utf8')===v,version=v=>Number.isSafeInteger(v)&&v>0;
const document=v=>{const d=fields(v);return exact(d,declarations.annotation.documentFields)&&label(d.authority)&&label(d.id)&&version(d.incarnation);};
const identity=v=>{const a=fields(v);return exact(a,declarations.annotation.annotationFields)&&document(a.document)&&label(a.id)&&version(a.incarnation);};
function annotationProjection(v){
 const r=fields(v),d=declarations.annotation;if(!r||!label(r.invocation)||!d.operations.includes(r.operation))return false;
 if(r.status==='not-published')return exact(r,d.notPublished);
 if(!['applied','unchanged'].includes(r.status)||r.status==='unchanged'&&['create','delete'].includes(r.operation))return false;
 const optional=d.optionalVersions.filter(k=>Object.hasOwn(r,k));
 return exact(r,[...d.published,...(r.status==='applied'?['publication']:[]),...optional])&&identity(r.annotation)&&r.membership===(r.operation==='delete'?'retained-absent':'present')&&(r.status!=='applied'||r.publication===r.invocation)&&optional.every(k=>version(r[k]));
}
function target(value){const t=fields(value),d=declarations.reversal;if(!t)return false;return t.kind==='group'?exact(t,d.groupTarget)&&label(t.group):t.kind==='scope-frontier'&&exact(t,d.frontierTarget)&&label(t.scope)&&label(t.expectedGroup);}
function reversalProjection(v){
 const r=fields(v),d=declarations.reversal;if(!r||!label(r.invocation)||!d.operations.includes(r.operation))return false;
 if(r.status==='not-published')return exact(r,d.notPublished);
 return r.status==='applied'&&exact(r,[...d.published,...(Object.hasOwn(r,'target')?['target']:[])])&&r.publication===r.invocation&&(!Object.hasOwn(r,'target')||target(r.target));
}
export function projectedEditResultDeclarations(){return declarations;}
export function projectedEditResultValid(value,family){
 if(!['F13','F14'].includes(family))return false;const v=fields(value);if(!v)return false;
 if(v.status==='unavailable')return exact(v,['status']);
 return exact(v,['status','result'])&&(family==='F13'?v.status===declarations.annotation.envelope&&annotationProjection(v.result):v.status===declarations.reversal.envelope&&reversalProjection(v.result));
}
