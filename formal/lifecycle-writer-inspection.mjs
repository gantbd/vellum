// First-party read-only companion handle. Never a source-write capability.
const byOwner=new WeakMap(),inspections=new WeakMap();
// Only the actual source-save registry factory supplies its private writer record.
export function sealLifecycleWriterInspection(owner,writer){if(byOwner.has(owner))return byOwner.get(owner);const handle=Object.freeze({kind:'lifecycle-writer-inspection/1'});byOwner.set(owner,handle);inspections.set(handle,{owner,writer});return handle;}
// Trusted lifecycle composition only; never return this record northbound.
export function readLifecycleWriterInspection(handle){return inspections.get(handle)?.writer??null;}
// Trusted read-producer pairing only; this record is never a northbound product.
export function readLifecycleWriterInspectionBinding(handle){return inspections.get(handle)??null;}
