// Existing released native F16 values. This is not a public retrieval contract.
// Frames must already be bounded. Reflection is not a proxy isolation boundary.
function fields(value){
 if(!value||typeof value!=='object'||Array.isArray(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;
 const descriptors=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(descriptors);
 if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(descriptors[k],'value')||descriptors[k].enumerable!==true))return null;
 return Object.fromEntries(keys.map(k=>[k,descriptors[k].value]));
}
const exact=(value,keys)=>value&&Object.keys(value).length===keys.length&&keys.every(k=>Object.hasOwn(value,k));
const label=v=>typeof v==='string'&&v.length>0&&Buffer.from(v,'utf8').toString('utf8')===v;
const natural=v=>Number.isSafeInteger(v)&&v>=0;
function rows(value){
 if(!Array.isArray(value))return null;
 const descriptors=Object.getOwnPropertyDescriptors(value),length=descriptors.length?.value;
 if(!natural(length)||Reflect.ownKeys(descriptors).length!==length+1)return null;
 const result=[];for(let i=0;i<length;i++){const d=descriptors[String(i)];if(!d||!Object.hasOwn(d,'value')||d.enumerable!==true)return null;result.push(d.value);}return result;
}
function identity(value){const v=fields(value);return exact(v,['authority','id','incarnation'])&&label(v.authority)&&label(v.id)&&natural(v.incarnation)?[v.authority,v.id,v.incarnation]:null;}
export function registrationResultValueValid(value,kind){
 if(!['host-registration','scoped-registration','created-settings'].includes(kind))return false;
 const envelope=fields(value);if(!envelope)return false;
 if(envelope.status==='unavailable')return Boolean(exact(envelope,['status']));
 if(!exact(envelope,['status','result'])||envelope.status!=='result')return false;
 const result=fields(envelope.result),key=kind==='created-settings'?'created':'registrations';
 if(!exact(result,['invocation','status',key])||!label(result.invocation)||!['applied','unchanged'].includes(result.status))return false;
 const list=rows(result[key]);if(!list)return false;const seen=new Set();
 for(let i=0;i<list.length;i++){
  const row=fields(list[i]);if(!row)return false;
  if(kind==='host-registration'){
   if(!exact(row,['table','provider','resource','generation'])||!['read','write'].includes(row.table)||!(row.generation===null||natural(row.generation)))return false;
   const provider=identity(row.provider),resource=identity(row.resource);if(!provider||!resource)return false;
   const slot=JSON.stringify([row.table,provider,resource]);if(seen.has(slot))return false;seen.add(slot);
  }else if(kind==='scoped-registration'){
   if(!exact(row,['basis','state'])||!natural(row.basis)||row.basis>=list.length||!['absent','present'].includes(row.state)||seen.has(row.basis))return false;seen.add(row.basis);
  }else{
   // Opaque reference identity is preserved, never reflected into or serialized.
   if(!exact(row,['index','reference'])||row.index!==i||!row.reference||typeof row.reference!=='object'||seen.has(row.reference))return false;seen.add(row.reference);
  }
 }
 return true;
}
