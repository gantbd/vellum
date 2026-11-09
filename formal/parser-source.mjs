// Trusted source-publication bridge; no native parser, callbacks, I/O or diff repair.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {validateCanonicalBatch,emitByOrigin} from './text-oracles.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./parser-source.json',import.meta.url),'utf8'));
const roots=new WeakMap(),receipts=new WeakMap(),prepared=new WeakMap(),copy=structuredClone;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const size=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const natural=n=>Number.isSafeInteger(n)&&n>=0;
const validLimits=(l,keys)=>exact(l,keys)&&Object.values(l).every(natural);
const boundary=(s,p)=>natural(p)&&p<=s.length&&(p===s.length||(s[p]&0xc0)!==0x80);
const identity=d=>exact(d,['authority','id','incarnation'])&&typeof d.authority==='string'&&d.authority.length>0&&typeof d.id==='string'&&d.id.length>0&&natural(d.incarnation);
const snapshot=s=>exact(s,['document','revision','source'])&&identity(s.document)&&natural(s.revision)&&Array.isArray(s.source);
const documentAt=(state,d)=>{const rows=state?.documents?.filter(x=>equal(x.document,d));return rows?.length===1?rows[0]:null;};
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
function canonical(s){try{validateCanonicalBatch(s,[]);return true;}catch{return false;}}
export function parserPoint(source,offset,maxCoordinate=catalog.coordinateMaximum){
  if(!natural(maxCoordinate)||maxCoordinate>catalog.coordinateMaximum||!boundary(source,offset)||offset>maxCoordinate)return null;
  let row=0,start=0;for(let i=0;i<offset;i++)if(source[i]===10){row++;start=i+1;}
  const column=offset-start;return row<=maxCoordinate&&column<=maxCoordinate?{row,column}:null;
}
export function parserByte(source,point,maxCoordinate=catalog.coordinateMaximum){
  if(!exact(point,['row','column'])||!natural(point.row)||!natural(point.column)||!natural(maxCoordinate)||maxCoordinate>catalog.coordinateMaximum||point.row>maxCoordinate||point.column>maxCoordinate)return null;
  let row=0,start=0;for(let i=0;i<source.length&&row<point.row;i++)if(source[i]===10){row++;start=i+1;}
  if(row!==point.row)return null;let end=start;while(end<source.length&&source[end]!==10)end++;
  const offset=start+point.column;return offset<=end&&offset<=maxCoordinate&&boundary(source,offset)?offset:null;
}
// Called before publication; absence/refusal is optional-derived-work unavailability.
export function prepareSourceJournal(before,after,entries,limits,reserved){
  if(!reserved||!validLimits(limits,catalog.journalLimits)||!Array.isArray(entries)||entries.length>limits.maxDocuments)return null;
  const rows=[],seen=new Set();let batches=0,edits=0,charged=0;
  for(const entry of entries){
    if(!exact(entry,['document','batches'])||!identity(entry.document)||!Array.isArray(entry.batches))return null;
    const key=JSON.stringify([entry.document.authority,entry.document.id,entry.document.incarnation]);if(seen.has(key))return null;seen.add(key);
    const a=documentAt(before,entry.document),b=documentAt(after,entry.document);
    if(!a||!b||!natural(a.revision)||!natural(b.revision)||b.revision!==a.revision+1||a.source.length>limits.maxRootBytes||b.source.length>limits.maxRootBytes||!canonical(a.source)||!canonical(b.source))return null;
    let source=a.source;const normalized=[];
    for(const batch of entry.batches){
      if(!Array.isArray(batch)||++batches>limits.maxBatches||(edits+=batch.length)>limits.maxEdits)return null;
      if(batch.some(e=>!Array.isArray(e?.ins)||e.ins.length>limits.maxRootBytes))return null;
      try{validateCanonicalBatch(source,batch);}catch{return null;}
      const length=source.length+batch.reduce((n,e)=>n+e.ins.length-(e.hi-e.lo),0);if(!natural(length)||length>limits.maxRootBytes)return null;
      const clean=batch.map(e=>({lo:e.lo,hi:e.hi,ins:copy(e.ins)}));
      source=emitByOrigin(source,clean);if(source.length>limits.maxRootBytes)return null;normalized.push(clean);
    }
    if(!equal(source,b.source))return null;
    const row={document:copy(entry.document),from:a.revision,to:b.revision,before:copy(a.source),after:copy(b.source),batches:normalized};
    charged+=size(row);if(!natural(charged)||charged>limits.maxJournalBytes)return null;rows.push(row);
  }
  const token=Object.freeze({kind:'prepared-source-journal/1'});prepared.set(token,{after,rows});return token;
}
export function commitSourceJournal(root,token){
  const p=prepared.get(token);if(!p||p.after!==root||p.rows.some(r=>{const d=documentAt(root,r.document);return !d||d.revision!==r.to||!equal(d.source,r.after);}))return false;
  const issued=p.rows.map(row=>{const ref=Object.freeze({kind:'committed-source-transform/1'});receipts.set(ref,freeze(copy(row)));return ref;});
  roots.set(root,Object.freeze(issued));prepared.delete(token);return true;
}
export function carrySourceJournal(from,to){
  const refs=roots.get(from);if(!refs||from.publication!==to.publication)return false;
  if(refs.some(ref=>{const r=receipts.get(ref),d=documentAt(to,r.document);return !d||d.revision!==r.to||!equal(d.source,r.after);}))return false;
  roots.set(to,refs);return true;
}
// A baseline-only publication is not a new source transform. Preserve existing
// owned receipts only through the exact baseline/publication frame below.
export function carryBaselineSourceJournal(from,to,document){
 const refs=roots.get(from),a=documentAt(from,document),b=documentAt(to,document);if(!refs||!a||!b||from.authority!==to.authority||!natural(from.publication)||!natural(to.publication)||to.publication!==from.publication+1)return false;
 const frame=copy(to),d=documentAt(frame,document);frame.publication=from.publication;if(Object.hasOwn(a,'annotationBaseline'))d.annotationBaseline=copy(a.annotationBaseline);else delete d.annotationBaseline;if(!equal(frame,from))return false;
 if(refs.some(ref=>{const r=receipts.get(ref),target=documentAt(to,r.document);return !target||target.revision!==r.to||!equal(target.source,r.after);}))return false;roots.set(to,refs);return true;
}
// Trusted bounded composition facet, not northbound reflection/disclosure permission.
export function carryRestoredAnnotationSourceJournal(from,to,document){
 const refs=roots.get(from),a=documentAt(from,document),b=documentAt(to,document);if(!refs||!a||!b||from.authority!==to.authority||!natural(from.publication)||!natural(to.publication)||to.publication!==from.publication+1||a.records?.length!==0||a.annotationLifecycle?.entries?.length!==0||b.annotationSet!==a.annotationSet+1)return false;
 const frame=copy(to),d=documentAt(frame,document);frame.publication=from.publication;d.records=copy(a.records);d.annotationLifecycle=copy(a.annotationLifecycle);d.annotationSet=a.annotationSet;if(!equal(frame,from))return false;
 if(refs.some(ref=>{const r=receipts.get(ref),target=documentAt(to,r.document);return !target||target.revision!==r.to||!equal(target.source,r.after);}))return false;roots.set(to,refs);return true;
}
export function committedSourceTransforms(root){return [...(roots.get(root)??[])];}
// A checked retained-lifecycle publication changes metadata, never source identity.
export function carryRetainedLifecycleSourceJournal(from,to){
 const refs=roots.get(from);if(!refs||from.authority!==to.authority||!natural(from.publication)||!natural(to.publication)||to.publication!==from.publication+1||!Array.isArray(from.documents)||!Array.isArray(to.documents)||from.documents.length!==to.documents.length||from.documents.some((d,i)=>!equal(d.document,to.documents[i].document)||d.revision!==to.documents[i].revision||!equal(d.source,to.documents[i].source)))return false;
 if(refs.some(ref=>{const r=receipts.get(ref),d=documentAt(to,r.document);return !d||d.revision!==r.to||!equal(d.source,r.after);}))return false;roots.set(to,refs);return true;
}
export function carryScratchSourceJournal(from,to,document){
 const refs=roots.get(from);if(!refs||from.authority!==to.authority||!natural(from.publication)||!natural(to.publication)||to.publication!==from.publication+1||!Array.isArray(from.documents)||!Array.isArray(to.documents)||from.documents.length+1!==to.documents.length||from.documents.some((d,i)=>!equal(d.document,to.documents[i].document)||d.revision!==to.documents[i].revision||!equal(d.source,to.documents[i].source))||!equal(to.documents.at(-1).document,document)||to.documents.at(-1).revision!==1||from.documents.some(d=>equal(d.document,document)))return false;
 if(refs.some(ref=>{const r=receipts.get(ref),d=documentAt(to,r.document);return !d||d.revision!==r.to||!equal(d.source,r.after);}))return false;roots.set(to,refs);return true;
}
function compile(before,batches,l,steps){
  let source=copy(before);
  for(const batch of batches){
    try{validateCanonicalBatch(source,batch);}catch{return null;}
    for(const e of [...batch].reverse()){
      if(steps.length>=l.maxEdits)return null;
      const start_point=parserPoint(source,e.lo,l.maxCoordinate),old_end_point=parserPoint(source,e.hi,l.maxCoordinate);
      const next=emitByOrigin(source,[e]),new_end_byte=e.lo+e.ins.length;
      if(next.length>l.maxRootBytes||next.length>l.maxCoordinate)return null;
      const new_end_point=parserPoint(next,new_end_byte,l.maxCoordinate);
      if(!start_point||!old_end_point||!new_end_point)return null;
      steps.push({start_byte:e.lo,old_end_byte:e.hi,new_end_byte,start_point,old_end_point,new_end_point});source=next;
    }
  }
  return source;
}
export function prepareParserSource(base,target,chain,g,limits){
  if(!g?.authorized||!g.sourceReadable||!g.continuous||!g.live||!g.notStopped||!g.captureReserved||!validLimits(limits,catalog.inputLimits)||limits.maxCoordinate>catalog.coordinateMaximum||
      !snapshot(target)||target.source.length>limits.maxRootBytes||target.source.length>limits.maxCoordinate||!canonical(target.source)||!Array.isArray(chain))return null;
  const fresh=reason=>{const out={profile:catalog.profile,mode:'fresh',reason,document:copy(target.document),fromRevision:null,toRevision:target.revision,source:copy(target.source),edits:[]};return size(out)<=limits.maxPlanBytes?out:null;};
  if(base===null)return fresh('no-base');
  if(!snapshot(base)||base.source.length>limits.maxRootBytes||!canonical(base.source))return null;
  if(!equal(base.document,target.document)||base.revision>target.revision)return fresh('incompatible-base');
  if(base.revision===target.revision){
    if(!equal(base.source,target.source))return null;
    const out={profile:catalog.profile,mode:'unchanged',document:copy(target.document),fromRevision:base.revision,toRevision:target.revision,source:copy(target.source),edits:[]};
    return size(out)<=limits.maxPlanBytes?out:null;
  }
  if(chain.length>limits.maxReceipts)return fresh('journal-budget');
  let source=copy(base.source),revision=base.revision,batches=0;const steps=[];
  for(const reference of chain){
    const r=receipts.get(reference);if(!r||!equal(r.document,target.document)||r.from!==revision||!equal(r.before,source))return fresh('missing-journal');
    batches+=r.batches.length;if(batches>limits.maxBatches)return fresh('journal-budget');
    const next=compile(source,r.batches,limits,steps);if(!next)return fresh('journal-budget');source=next;revision=r.to;
  }
  if(revision!==target.revision||!equal(source,target.source))return fresh('missing-journal');
  const out={profile:catalog.profile,mode:'incremental',document:copy(target.document),fromRevision:base.revision,toRevision:target.revision,source:copy(target.source),edits:steps};
  return size(out)<=limits.maxPlanBytes?out:fresh('journal-budget');
}
