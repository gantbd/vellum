// Selected native F07 query structure and binding, not registration or policy truth.
import fs from 'node:fs';
import {nativeResourceMetadataVocabulary} from './resource-registration-state.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./registration-reads.json',import.meta.url))),freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const declarations=freeze({forms:{page:['status','context','coverage','entries','continuation'],registration:['status','context','coverage','entry'],unavailable:['status']},entryFields:['reference','table'],readFields:catalog.readFields,writeFields:catalog.writeFields,tables:['read','write'],insertionLineBreaks:nativeResourceMetadataVocabulary().insertionLineBreaks});
export function registrationQueryResultDeclarations(){return declarations;}
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v),text=v=>typeof v==='string'&&v.length>0&&Buffer.from(v).toString('utf8')===v;
function exact(v,keys){if(!object(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),all=Reflect.ownKeys(d);return all.length===keys.length&&keys.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable===true);}
const dense=v=>Array.isArray(v)&&Object.getPrototypeOf(v)===Array.prototype&&Reflect.ownKeys(v).length===v.length+1&&Array.from({length:v.length},(_,i)=>Object.hasOwn(v,i)&&Object.getOwnPropertyDescriptor(v,String(i))?.enumerable===true&&Object.hasOwn(Object.getOwnPropertyDescriptor(v,String(i)),'value')).every(Boolean);
function entryValid(v,described){
 if(!object(v)||!exact(v,[...declarations.entryFields,...(described?['descriptor']:[])])||!object(v.reference)||!declarations.tables.includes(v.table))return false;
 if(!described)return true;const d=v.descriptor;if(!object(d))return false;
 if(v.table==='write')return exact(d,declarations.writeFields)&&text(d.procedure)&&declarations.writeFields.filter(k=>k!=='procedure').every(k=>typeof d[k]==='boolean');
 const profile=Object.getOwnPropertyDescriptor(d,'profile');if(!profile||!Object.hasOwn(profile,'value')||typeof profile.value!=='string'||!Object.hasOwn(declarations.readFields,profile.value)||!exact(d,declarations.readFields[profile.value])||typeof d.atomicRead!=='boolean')return false;
 return profile.value!=='strict-utf8-one-bom/1'||declarations.insertionLineBreaks.includes(d.insertionLineBreak);
}
export function registrationQueryResultValid(value,operation,context,request){
 if(!['list','inspect'].includes(operation)||!object(value))return false;const status=Object.getOwnPropertyDescriptor(value,'status');if(!status||!Object.hasOwn(status,'value')||typeof status.value!=='string'||!Object.hasOwn(declarations.forms,status.value)||!exact(value,declarations.forms[status.value]))return false;
 if(status.value==='unavailable')return true;if(value.context!==context||value.coverage!=='captured-permitted')return false;
 if(operation==='inspect')return status.value==='registration'&&entryValid(value.entry,Object.hasOwn(value.entry??{},'descriptor'))&&value.entry.reference===request?.entry;
 return status.value==='page'&&Number.isSafeInteger(request?.limit)&&request.limit>0&&dense(value.entries)&&value.entries.length<=request.limit&&value.entries.every(v=>entryValid(v,false))&&new Set(value.entries.map(v=>v.reference)).size===value.entries.length&&(value.continuation===null||object(value.continuation));
}
// Trusted native captured rows and staged maps; callers cannot mint authority here.
export function registrationQueryPageBindingsValid(value,context,rows,start,end,entries,continuation,selectors,limit){
 if(!registrationQueryResultValid(value,'list',context,{limit})||value.status!=='page'||!Array.isArray(rows)||!Number.isSafeInteger(start)||!Number.isSafeInteger(end)||start<0||end<start||end>rows.length||value.entries.length!==end-start||value.continuation!==continuation)return false;
 if(!value.entries.every((row,i)=>{const index=start+i,reference=entries.get(index),s=selectors.get(reference);return row.reference===reference&&row.table===rows[index]?.value.table&&s?.kind==='entry'&&s.context===context&&s.index===index;}))return false;
 if(continuation===null)return end===rows.length;
 const s=selectors.get(continuation);return end<rows.length&&s?.kind==='continuation'&&s.context===context&&s.start===end&&s.limit===limit;
}
