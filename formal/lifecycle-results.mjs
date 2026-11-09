// Existing native receipt shapes only: neither a new query nor a wire profile.
import fs from 'node:fs';
const lifecycle=JSON.parse(fs.readFileSync(new URL('./lifecycle-invocations.json',import.meta.url),'utf8'));
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const declarations=freeze({
 identityFields:['authority','id','incarnation'],
 document:{fields:['invocation','status','operation','document'],operations:[...lifecycle.nativeOperations,'source-save-as'],statuses:['applied','unchanged']},
 load:{fields:['invocation','status','operation','activation','document'],operation:'source-load',activation:{created:'applied',joined:'unchanged','already-activated':'unchanged'}},
 annotations:{fields:['invocation','status','operation','phase','disposition','document'],operation:'annotation-load',phases:{restore:{restored:'applied',unchanged:'unchanged'},'adopt-baseline':{installed:'applied',current:'unchanged'}}},
 relation:{fields:['invocation','status','kind','target'],statuses:['created','retired'],kinds:['view','workspace']}
});
function exact(v,fields){
 if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;
 const descriptors=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(descriptors);
 return keys.length===fields.length&&fields.every(k=>Object.hasOwn(descriptors,k)&&Object.hasOwn(descriptors[k],'value')&&descriptors[k].enumerable===true);
}
const label=v=>typeof v==='string'&&v.length>0&&Buffer.from(v,'utf8').toString('utf8')===v;
const identity=v=>exact(v,declarations.identityFields)&&label(v.authority)&&label(v.id)&&Number.isSafeInteger(v.incarnation)&&v.incarnation>=0;
export function lifecycleReceiptDeclarations(){return declarations;}
export function nativeLifecycleReceiptValid(v){
 const operation=Object.getOwnPropertyDescriptor(v??{},'operation')?.value;
 if(operation===declarations.load.operation){const d=declarations.load;return exact(v,d.fields)&&label(v.invocation)&&identity(v.document)&&typeof v.activation==='string'&&Object.hasOwn(d.activation,v.activation)&&v.status===d.activation[v.activation];}
 if(operation===declarations.annotations.operation){const d=declarations.annotations;if(!exact(v,d.fields)||!label(v.invocation)||!identity(v.document)||typeof v.phase!=='string'||!Object.hasOwn(d.phases,v.phase)||typeof v.disposition!=='string')return false;const phase=d.phases[v.phase];return Object.hasOwn(phase,v.disposition)&&v.status===phase[v.disposition];}
 const d=declarations.document;return exact(v,d.fields)&&label(v.invocation)&&identity(v.document)&&d.operations.includes(v.operation)&&d.statuses.includes(v.status);
}
export function nativeRelationReceiptValid(v){
 const d=declarations.relation;return exact(v,d.fields)&&label(v.invocation)&&identity(v.target)&&d.statuses.includes(v.status)&&d.kinds.includes(v.kind);
}
export function nativeLifecycleResultValid(v,kind='lifecycle'){
 if(!['lifecycle','relation'].includes(kind))return false;
 if(exact(v,['status'])&&v.status==='unavailable')return true;
 return exact(v,['status','result'])&&v.status==='result'&&(kind==='lifecycle'?nativeLifecycleReceiptValid(v.result):nativeRelationReceiptValid(v.result));
}
