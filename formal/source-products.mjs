// Fixed normalized source products; caller/issuer/policy truth is not established here.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {readRange} from './source-edit-contracts.mjs';
import {logicalLines} from './reference-profile.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./source-products.json',import.meta.url),'utf8'));
const copy=structuredClone;
export function sourceReadInputForms(){return Object.freeze(Object.entries(catalog.variants).map(([operation,fields])=>Object.freeze({family:catalog.family,contract:catalog.contract,operation,variant:operation,fields:Object.freeze([...catalog.common,...fields])})));}
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const resultFields=Object.freeze({content:Object.freeze(['status','ref','range','unit','source','context']),line:Object.freeze(['status','context','ref','unit','line','body','delimiter']),unavailable:Object.freeze(['status'])}),refFields=Object.freeze(['authority','document','incarnation','revision']),spanFields=Object.freeze(['lo','hi','source']);
const natural=n=>Number.isSafeInteger(n)&&n>=0,span=v=>exact(v,spanFields)&&natural(v.lo)&&natural(v.hi)&&v.lo<=v.hi&&byteArray(v.source)&&v.source.length===v.hi-v.lo;
function byteArray(v){if(!Array.isArray(v))return false;for(let i=0;i<v.length;i++)if(!Object.hasOwn(v,i)||!Number.isInteger(v[i])||v[i]<0||v[i]>255)return false;return true;}
export function sourceReadResultForms(){return Object.freeze({fields:resultFields,refFields,spanFields,unit:'utf8-byte',context:'original-snapshot-identity; not structural authentication'});}
// Shape and original-request binding only; the producer owns source/line semantics.
export function sourceReadResultValid(out,request,expectedRef=null){
 if(!out||typeof out.status!=='string'||!Object.hasOwn(resultFields,out.status)||!exact(out,resultFields[out.status]))return false;if(out.status==='unavailable')return true;
 if(out.context!==request.snapshot||out.unit!=='utf8-byte'||!exact(out.ref,refFields)||!['authority','document'].every(k=>typeof out.ref[k]==='string'&&out.ref[k].length>0)||!natural(out.ref.incarnation)||!natural(out.ref.revision)||expectedRef!==null&&!equal(out.ref,expectedRef)||out.ref.authority!==request.document.authority||out.ref.document!==request.document.id||out.ref.incarnation!==request.document.incarnation)return false;
 if(out.status==='content')return request.operation==='bounded-range'&&Array.isArray(out.range)&&out.range.length===2&&out.range.every(natural)&&out.range[0]===request.lo&&out.range[1]===request.hi&&out.range[0]<=out.range[1]&&byteArray(out.source)&&out.source.length===out.range[1]-out.range[0];
 return request.operation==='line'&&natural(out.line)&&out.line===request.line&&span(out.body)&&span(out.delimiter)&&out.body.hi===out.delimiter.lo&&out.delimiter.source.length<=2&&(out.delimiter.source.length===0||out.delimiter.source.length===1&&[10,13].includes(out.delimiter.source[0])||out.delimiter.source[0]===13&&out.delimiter.source[1]===10);
}
function ensureSourceReadResult(out,request,expectedRef){if(!sourceReadResultValid(out,request,expectedRef)){const error=Error('native source result violates its closed shape');error.code='NATIVE_SOURCE_RESULT_INVARIANT';throw error;}return out;}
const unavailable=()=>ensureSourceReadResult({status:'unavailable'},null,null);
export function captureSourceProfile(profile){
  if(!exact(profile,catalog.profileFields)||!catalog.profileFields.every(k=>Number.isSafeInteger(profile[k])&&profile[k]>=0))return null;
  return Object.freeze(copy(profile));
}
export function readSourceProduct(snapshot,request,release,limits){
  const fail=unavailable;
  if(!request||!Object.hasOwn(catalog.variants,request.operation)||!exact(request,[...catalog.common,...catalog.variants[request.operation]])||
      request.contract!==catalog.contract||!captureSourceProfile(limits)||!snapshot||
      request.snapshot!==snapshot.selector||request.profile!==snapshot.profile||
      !equal(request.document,{authority:snapshot.ref.authority,id:snapshot.ref.document,incarnation:snapshot.ref.incarnation})||
      snapshot.source.length>limits.maxRootBytes||snapshot.source.length>limits.maxInputBytes)return fail();
  // Validate canonical source and current release before deriving even line metadata.
  if(readRange(snapshot,{ref:snapshot.ref,lo:0,hi:0},release).status!=='content')return fail();
  let out;
  if(request.operation==='bounded-range'){
    if(request.hi-request.lo>limits.maxContentBytes)return fail();
    const read=readRange(snapshot,{ref:snapshot.ref,lo:request.lo,hi:request.hi},release);
    if(read.status!=='content')return fail();out={...read,context:request.snapshot};
  }else{
    const rows=logicalLines(snapshot.source);
    if(!rows||!Number.isSafeInteger(request.line)||request.line<0||request.line>=rows.length)return fail();
    const row=rows[request.line];
    if(row.end-row.start>limits.maxContentBytes)return fail();
    const read=readRange(snapshot,{ref:snapshot.ref,lo:row.start,hi:row.end},release);if(read.status!=='content')return fail();
    out={status:'line',context:request.snapshot,ref:copy(read.ref),unit:'utf8-byte',line:request.line,
      body:{lo:row.start,hi:row.bodyEnd,source:read.source.slice(0,row.bodyEnd-row.start)},
      delimiter:{lo:row.bodyEnd,hi:row.end,source:read.source.slice(row.bodyEnd-row.start)}};
  }
  ensureSourceReadResult(out,request,snapshot.ref);
  return Buffer.byteLength(JSON.stringify(out),'utf8')<=limits.maxReplyBytes?out:fail();
}
