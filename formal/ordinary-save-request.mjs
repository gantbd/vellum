// Normalized F21 bindings. Technical profiles/pool/issuer are actual host bindings,
// not payload-selected authority. No provider dispatch or wire decoder is added.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {decodeTyped,decodeDynamic,planRoute} from './operation-catalog.mjs';
import {requestInvocationOrdinarySourceSave} from './native-source-destinations.mjs';
import {requestInvocationOrdinaryAnnotationSave} from './source-save.mjs';
const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url)));
const exact=(v,ks)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...ks].sort());
function normalized(e,route,variant,g){
 const identity=route==='typed'?'family':route==='dynamic'?'name':null;
 if(!identity||!exact(e,[identity,'variant','version','arguments']))return null;
 const d=(route==='typed'?decodeTyped:decodeDynamic)(families,Object.fromEntries([identity,'variant','version'].map(k=>[k,e[k]])));
 // Routing metadata grants no authority. Current write/disclosure facts are
 // checked strictly by the owning operation path AFTER private intent capture.
 return d?.descriptor.id==='F21'&&d.variant===variant&&planRoute(d,{schemaValid:true,enabled:true,grant:true,current:true,ordinaryCapacity:true,releaseAllowed:false}).sink==='hostRequest'?e.arguments:null;
}
export function invokeOrdinarySourceSaveRequest(writer,domain,profile,envelope,route,c,g){
 const r=normalized(envelope,route,'captured-source-save',g);
 return r?requestInvocationOrdinarySourceSave(domain,writer,profile,r,c,g):{status:'unavailable'};
}
export function invokeOrdinaryAnnotationSaveRequest(writer,domain,pool,profile,envelope,route,c,g){
 const r=normalized(envelope,route,'captured-annotation-save',g);
 return r&&exact(r,['request','context','binding'])?requestInvocationOrdinaryAnnotationSave(writer,domain,pool,r.request,r.context,r.binding,profile,c,g):{status:'unavailable'};
}
