// Closed private F14 result values. Host policy/alias truth and wire encodings are external.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
const catalog=JSON.parse(fs.readFileSync(new URL('./reversal-results.json',import.meta.url))),copy=structuredClone,size=v=>Buffer.byteLength(JSON.stringify(v));
const exact=(v,ks)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort()),natural=n=>Number.isSafeInteger(n)&&n>=0;
const label=s=>typeof s==='string'&&s.length>0&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Buffer.from(s))===s;
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;},unavailable=()=>({status:'unavailable'});
const target=t=>t?.kind==='group'?exact(t,['kind','group'])&&label(t.group):t?.kind==='scope-frontier'&&exact(t,['kind','scope','expectedGroup'])&&label(t.scope)&&label(t.expectedGroup);
function grant(invocation,authority,t,g){if(!Object.hasOwn(g??{},'reversalResultGrant'))return {target:false};const p=g.reversalResultGrant;return exact(p,catalog.grantFields)&&p.invocation===invocation&&p.authority===authority&&equal(p.target,t)&&typeof p.discloseTarget==='boolean'?{target:p.discloseTarget}:null;}
export function validReversalResultReceipt(r){if(!r||!label(r.invocation)||!['undo','redo'].includes(r.operation))return false;return r.status==='not-published'?exact(r,['status','invocation','operation']):r.status==='applied'&&exact(r,['status','invocation','operation','authority','publication','target'])&&label(r.authority)&&r.publication===r.invocation&&target(r.target);}
export function prepareReversalResultAdmission(value,c,g,maximum){
 const r=value?.request;if(!r||!['undo','redo'].includes(r.operation)||!label(r.invocation)||!label(r.authority)||!target(r.target)||r.authority!==c.authority||value.recipient!==c.recipient||value.profile!==c.profile||!natural(maximum))return null;
 const ceiling=grant(r.invocation,r.authority,r.target,g);if(!ceiling)return null;
 const full={status:'applied',invocation:r.invocation,operation:r.operation,authority:r.authority,publication:r.invocation,target:copy(r.target)},frame={status:'reversal-result',result:full};
 return size(full)<=maximum&&size(frame)<=maximum&&size(unavailable())<=maximum?freeze(copy(ceiling)):null;
}
export function captureReversalResult(value,result){
 const r=value?.request;if(!r||!['undo','redo'].includes(r.operation)||!label(r.invocation)||!label(r.authority)||!target(r.target)||!['applied','not-published'].includes(result?.status))throw Error('invalid reversal decision');
 const base={status:result.status,invocation:r.invocation,operation:r.operation};if(result.status==='not-published')return freeze(base);
 const group=value.group,root=result.state,groups=root?.groups?.filter(g=>g.id===group?.id),opposite=r.operation==='undo'?'redo':'undo';
 if(root?.authority!==r.authority||groups?.length!==1||!equal(groups[0],group)||!group.valid||!group.scopes.length||!group.scopes.every(s=>root.frontiers[s]?.[opposite]?.at(-1)===group.id&&root.frontiers[s]?.[r.operation]?.at(-1)!==group.id))throw Error('reversal decision does not match complete frontier movement');
 return freeze({...base,authority:r.authority,publication:r.invocation,target:copy(r.target)});
}
export function releaseReversalResult(receipt,ceiling,g,maximum){
 if(!validReversalResultReceipt(receipt)||!exact(ceiling,['target'])||typeof ceiling.target!=='boolean'||!natural(maximum))return unavailable();
 if(receipt.status==='not-published'){const out={status:'reversal-result',result:copy(receipt)};return size(out)<=maximum?out:unavailable();}
 const current=grant(receipt.invocation,receipt.authority,receipt.target,g);if(!current)return unavailable();
 const {authority,target:t,...base}=receipt,result={...copy(base),...(ceiling.target&&current.target?{target:copy(t)}:{})},out={status:'reversal-result',result};return size(out)<=maximum?out:unavailable();
}
