// Private task-bound local activation authority, never a writer/dispatch capability.
const handles=new WeakMap(),byTask=new WeakMap();
const valid=p=>p.writer.tasks.get(p.task)===p.record&&p.record.nativeSourceDomain===p.domain&&!p.record.annotation&&p.record.capture.request.purpose==='save-as';
// Only the actual source-save owner supplies its private registry record.
export function sealSourceSaveAsActivationAuthority(owner,writer,task,domain){
 const p={owner,writer,task,domain,record:writer.tasks.get(task)};
 if(!p.record||!valid(p))return null;
 const old=byTask.get(task);if(old){const q=handles.get(old);return q&&q.owner===owner&&q.writer===writer&&q.domain===domain&&valid(q)?old:null;}
 const handle=Object.freeze({kind:'source-saveas-activation-authority/1'});handles.set(handle,p);byTask.set(task,handle);return handle;
}
// Trusted local composition only. Do not release this record or handle remotely.
export function readSourceSaveAsActivationAuthority(handle,domain){const p=handles.get(handle);return p&&p.domain===domain&&valid(p)?p:null;}
