// Normalized F20 source-load request facade. Not a hostile decoder or wire format.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {decodeTyped,decodeDynamic,planRoute} from './operation-catalog.mjs';
import {requestInvocationSourceRead} from './resource-read.mjs';
import {requestInvocationAnnotationRead} from './annotation-native-load.mjs';
const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
const exact=(v,keys)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
// Catalog routing grants no authority. The native owner privately captures the
// semantic input before consulting strict current permission facts.
export function invokeSourceLoadRequest(owner,envelope,route,context,g){
 const identity=route==='typed'?'family':route==='dynamic'?'name':null;
 if(!identity||!exact(envelope,[identity,'variant','version','arguments']))return {status:'unavailable'};
 const decoded=(route==='typed'?decodeTyped:decodeDynamic)(families,Object.fromEntries([identity,'variant','version'].map(k=>[k,envelope[k]])));
 if(decoded?.descriptor.id!=='F20'||decoded.variant!=='source-load-candidate'||
    planRoute(decoded,{schemaValid:true,enabled:true,grant:true,current:true,ordinaryCapacity:true,releaseAllowed:false}).sink!=='hostRequest')return {status:'unavailable'};
 return requestInvocationSourceRead(owner,envelope.arguments,context,g);
}
export function invokeAnnotationLoadRequest(owner,envelope,route,context,g){
 const identity=route==='typed'?'family':route==='dynamic'?'name':null;
 if(!identity||!exact(envelope,[identity,'variant','version','arguments']))return {status:'unavailable'};
 const decoded=(route==='typed'?decodeTyped:decodeDynamic)(families,Object.fromEntries([identity,'variant','version'].map(k=>[k,envelope[k]])));
 if(decoded?.descriptor.id!=='F20'||decoded.variant!=='annotation-load-candidate'||
    planRoute(decoded,{schemaValid:true,enabled:true,grant:true,current:true,ordinaryCapacity:true,releaseAllowed:false}).sink!=='hostRequest')return {status:'unavailable'};
 return requestInvocationAnnotationRead(owner,envelope.arguments,context,g);
}
