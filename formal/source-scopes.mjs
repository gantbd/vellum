// Caller restrictions, never grants, copied documents or implicit mapped scopes.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {validateCanonicalBatch} from './text-oracles.mjs';
const C=JSON.parse(fs.readFileSync(new URL('./source-scopes.json',import.meta.url)));
const nat=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(v,ks)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
const dense=a=>Array.isArray(a)&&Object.keys(a).length===a.length&&Object.keys(a).every((k,i)=>k===String(i));
const text=(s,n)=>typeof s==='string'&&s.length>0&&Buffer.byteLength(s)<=n&&Buffer.from(s).toString('utf8')===s;
const identity=(d,n)=>exact(d,['authority','id','incarnation'])&&text(d.authority,n)&&text(d.id,n)&&nat(d.incarnation);
const key=d=>JSON.stringify([d.authority,d.id,d.incarnation]);
const limits=l=>exact(l,C.limitFields)&&Object.values(l).every(nat);
export function isSourceScopePlan(plan){return plan?.contract===C.contract||C.compatibleContracts.includes(plan?.contract);}
export function sourceScopePlanFields(){return [...C.planFields];}
function ordered(xs,fields,inclusive){
 if(!dense(xs)||!xs.every(x=>exact(x,fields)&&nat(x[fields[0]])&&nat(x[fields[1]])&&(inclusive?x[fields[0]]<=x[fields[1]]:x[fields[0]]<x[fields[1]])))return null;
 const out=structuredClone(xs).sort((a,b)=>a[fields[0]]-b[fields[0]]||a[fields[1]]-b[fields[1]]);
 if(out.some((r,i)=>i>0&&(inclusive?out[i-1][fields[1]]>=r[fields[0]]:out[i-1][fields[1]]>r[fields[0]])))return null;
 return out;
}
export function normalizeSourceScopes(scopes,l){
 if(!limits(l)||!dense(scopes)||scopes.length>l.maxScopes)return null;
 const out=[],seen=new Set();let ranges=0,sites=0;
 for(const s of scopes){if(!exact(s,C.scopeFields)||s.profile!==C.profile||!identity(s.document,l.maxIdentifierBytes)||!nat(s.expectedSource)||seen.has(key(s.document)))return null;
  const replacementRanges=ordered(s.replacementRanges,C.rangeFields,false),insertionSites=ordered(s.insertionSites,C.siteFields,true);if(!replacementRanges||!insertionSites)return null;
  ranges+=replacementRanges.length;sites+=insertionSites.length;if(!nat(ranges)||!nat(sites)||ranges>l.maxReplacementRanges||sites>l.maxInsertionIntervals)return null;
  seen.add(key(s.document));out.push({...structuredClone(s),replacementRanges,insertionSites});
 }
 // Sort only conjunctive scope/set records. Never merge or widen their extents.
 return out.sort((a,b)=>Buffer.compare(Buffer.from(key(a.document)),Buffer.from(key(b.document))));
}
const boundary=(source,p)=>nat(p)&&p<=source.length&&(p===source.length||(source[p]&0xc0)!==0x80);
export function sourceScopeContainsEdit(scope,edit){
 if(!edit||!nat(edit.lo)||!nat(edit.hi)||edit.lo>edit.hi)return false;
 if(edit.lo===edit.hi)return scope.insertionSites.some(r=>r.first<=edit.lo&&edit.lo<=r.last);
 let covered=edit.lo;
 for(const r of scope.replacementRanges){if(r.hi<=covered)continue;if(r.lo>covered)break;covered=Math.max(covered,r.hi);if(covered>=edit.hi)break;}
 return covered>=edit.hi;
}
export function declaredWriteScopesCurrent(state,plan,l){
 if(!isSourceScopePlan(plan)||!dense(plan.sources)||!dense(plan.scopes))return false;
 if(plan.sources.length===0&&plan.scopes.length===0)return true;
 const scopes=normalizeSourceScopes(plan.scopes,l);if(!scopes||scopes.length!==plan.sources.length)return false;
 const seen=new Set();
 for(const input of plan.sources){if(!input||!identity(input.document,l.maxIdentifierBytes)||seen.has(key(input.document)))return false;seen.add(key(input.document));
  const s=scopes.find(s=>equal(s.document,input.document)),d=state.documents.find(d=>equal(d.document,input.document));
  if(!s||!d||d.live===false||input.document.authority!==state.authority||s.expectedSource!==input.expectedSource||d.revision!==s.expectedSource||!dense(input.edits))return false;
  try{validateCanonicalBatch(d.source,[]);}catch{return false;}
  if(s.replacementRanges.some(r=>!boundary(d.source,r.lo)||!boundary(d.source,r.hi))||s.insertionSites.some(r=>!boundary(d.source,r.first)||!boundary(d.source,r.last)))return false;
  if(!input.edits.every(e=>sourceScopeContainsEdit(s,e)))return false;
 }
 return true;
}
