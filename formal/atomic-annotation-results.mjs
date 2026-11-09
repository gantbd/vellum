// Internal compound receipt ownership. Not public ingress, issuer truth or physical accounting.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {prepareAnnotationResultAdmission,captureAnnotationResult,releaseAnnotationResult}
  from './annotation-results.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./annotation-compounds.json',import.meta.url))),holds=new WeakMap();
const copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const natural=n=>Number.isSafeInteger(n)&&n>=0,version=n=>natural(n)&&n>0;
const label=s=>typeof s==='string'&&s.length>0&&new TextDecoder('utf-8',{fatal:true}).decode(Buffer.from(s))===s;
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const dense=a=>Array.isArray(a)&&Object.keys(a).length===a.length&&Object.keys(a).every((k,i)=>k===String(i));
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const unavailable=()=>({status:'unavailable'});
function grantAt(g,count,index){
 if(!Object.hasOwn(g??{},'atomicAnnotationResultGrants'))return {};
 const gs=g.atomicAnnotationResultGrants;if(!dense(gs)||gs.length!==count)return null;
 return {annotationResultGrant:gs[index]};
}
function bound(hold,parent,domain,ticket){const p=holds.get(hold);return p&&p.parent===parent&&p.domain===domain&&p.ticket===ticket?p:null;}
export function prepareAtomicAnnotationResultHold(parent,domain,ticket,plan,c,g,maximumIntent,maximumResult,available){
 if(!natural(maximumIntent)||!natural(maximumResult)||!natural(available)||!natural(g?.maxParticipants)||!dense(plan?.annotations)||plan.annotations.length<1||plan.annotations.length>g.maxParticipants||!label(plan.invocation)||size(plan)>maximumIntent)return null;
 const values=[],ceilings=[];
 for(let index=0;index<plan.annotations.length;index++){
  const a=plan.annotations[index],fields=catalog.annotationFields[a?.kind];
  if(!fields||!exact(a,fields)||!exact(a.document,['authority','id','incarnation'])||a.document.authority!==c.authority||!label(a.document.id)||!version(a.document.incarnation)||!label(a.id)||!version(a.incarnation))return null;
  const operation=a.kind==='body'?'update-body':a.kind,annotation={document:copy(a.document),id:a.id,incarnation:a.incarnation};
  const value={request:{operation,invocation:plan.invocation,document:copy(a.document),annotation},recipient:c.recipient,profile:c.profile,...(a.kind==='create'?{allocation:{invocation:plan.invocation,...annotation}}:{})};
  const grant=grantAt(g,plan.annotations.length,index);if(!grant)return null;
  const ceiling=prepareAnnotationResultAdmission(value,c,grant,maximumResult);if(!ceiling)return null;
  values.push(value);ceilings.push(ceiling);
 }
 const retained={plan:copy(plan),values,ceilings},bytes=size(retained);
 // Two retained/capture frames plus three complete per-component result frames:
 // staged, retained and release workspace. Units are logical serialized bytes.
 const charge=2*bytes+3*values.length*maximumResult;
 if(bytes>maximumIntent||!natural(charge)||charge>available)return null;
 const hold=Object.freeze({kind:'owned-atomic-annotation-results/1'});
 holds.set(hold,{parent,domain,ticket,value:freeze(retained),maximumResult,charge});return {hold,charge};
}
export function atomicAnnotationResultPlanMatches(hold,parent,domain,ticket,plan){const p=bound(hold,parent,domain,ticket);return Boolean(p&&equal(p.value.plan,plan));}
export function atomicAnnotationResultHoldUsage(hold,parent,domain,ticket){const p=bound(hold,parent,domain,ticket);return p?{components:p.value.values.length,chargedBytes:p.charge}:null;}
export function stageAtomicAnnotationResults(hold,parent,domain,ticket,result){
 const p=bound(hold,parent,domain,ticket);if(!p)throw Error('atomic annotation receipt owner missing');
 const receipts=p.value.values.map(value=>{const r=captureAnnotationResult(value,result);if(size(r)>p.maximumResult)throw Error('atomic annotation receipt exceeded reservation');return r;});
 if(receipts.length!==p.value.values.length||size(receipts)>receipts.length*p.maximumResult)throw Error('atomic annotation receipt vector exceeded reservation');
 return freeze(receipts);
}
export function releaseAtomicAnnotationResult(hold,parent,domain,ticket,receipts,index,g){
 const p=bound(hold,parent,domain,ticket);if(!p||!natural(index)||index>=p.value.values.length||!dense(receipts)||receipts.length!==p.value.values.length)return unavailable();
 const grant=grantAt(g,receipts.length,index);if(!grant)return unavailable();
 return releaseAnnotationResult(receipts[index],p.value.ceilings[index],grant,p.maximumResult);
}
export function retireAtomicAnnotationResultHold(hold,parent,domain,ticket,g){
 const p=bound(hold,parent,domain,ticket);if(!p||!['ownerControlAllowed','retentionReleased','deliveryDrained'].every(k=>g?.[k]===true))return false;
 holds.delete(hold);return true;
}
