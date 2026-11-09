// Private read-task plus paired-writer authority. No read/write dispatch capability.
import {isDeepStrictEqual as equal} from 'node:util';
const handles=new WeakMap(),byTask=new WeakMap();
const valid=p=>p.reader.tasks.get(p.task)===p.record&&p.reader.nativeDomain===p.domain&&p.writer.readOwner===p.readerOwner&&p.writer.authority===p.reader.authority&&(!p.reader.nativeWriter||p.reader.nativeWriter===p.writer)&&p.record.phase==='submitted'&&!p.record.stopped&&!p.record.contested&&!p.record.codecRefused&&p.reader.codec===p.witness.codecOwner&&p.record.candidate===p.witness.candidate&&p.witness.stream.candidate===p.witness.candidate&&p.witness.stream.phase==='complete'&&[...p.witness.codec.imports.values()].includes(p.witness.stream)&&equal(p.record.context,p.witness.stream.context);
// Only the actual resource-read factory supplies its reader, task and codec witness.
export function sealSourceLoadActivationAuthority(readerOwner,reader,task,domain,pair,witness){
 const p={readerOwner,reader,task,domain,writerOwner:pair?.owner,writer:pair?.writer,record:reader.tasks.get(task),witness};if(!p.record||!p.writer||!witness||!valid(p))return null;
 const old=byTask.get(task);if(old){const q=handles.get(old);return q&&q.readerOwner===readerOwner&&q.reader===reader&&q.domain===domain&&q.writer===p.writer&&valid(q)?old:null;}
 const handle=Object.freeze({kind:'source-load-activation-authority/1'});handles.set(handle,p);byTask.set(task,handle);return handle;
}
export function readSourceLoadActivationAuthority(handle,domain){const p=handles.get(handle);return p&&p.domain===domain&&valid(p)?p:null;}
// Actual read retirement revokes the handle before its accounted records retire.
export function revokeSourceLoadActivationAuthority(task){const handle=byTask.get(task);if(!handle)return false;byTask.delete(task);return handles.delete(handle);}
// Decoded content stays private and is reconstructed from the actual live codec.
export function sourceLoadActivationDecoded(p){const s=p.witness.stream;return {status:'source-candidate',context:structuredClone(s.context),source:structuredClone(s.source),bom:s.bom,rawByteLength:s.inputBytes};}
