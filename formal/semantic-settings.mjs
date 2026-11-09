// Bounded engine argument normalization, not a settings store or publication authority.
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
const C=JSON.parse(fs.readFileSync(new URL('./semantic-settings.json',import.meta.url)));
// Metadata from the same catalog consumed by the argument guard below.
export function semanticSettingsInputForm(){return Object.freeze({family:C.family,contract:C.contract,operation:C.operation,variant:C.operation,fields:Object.freeze([...C.argumentFields])});}
const natural=n=>Number.isSafeInteger(n)&&n>=0;
const exact=(v,keys)=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const put=(o,k,v)=>Object.defineProperty(o,k,{value:v,enumerable:true,writable:true,configurable:true});
// Exact compact JSON UTF-8 string size, without allocating an escaped string.
function quotedBytes(s,maximum){
  let size=2;
  for(let i=0;i<s.length;i++){
    const n=s.charCodeAt(i);
    if(n>=0xd800&&n<=0xdbff){const next=s.charCodeAt(++i);if(!(next>=0xdc00&&next<=0xdfff))return null;size+=4;}
    else if(n>=0xdc00&&n<=0xdfff)return null;
    else size+=n===34||n===92||[8,9,10,12,13].includes(n)?2:n<32?6:n<128?1:n<2048?2:3;
    if(size>maximum)return null;
  }
  return size<=maximum?size:null;
}
// Explicit enter/exit stack: deep finite trees need no recursive copy/stringify/freeze.
// Plain normalized data only; proxy/native-code containment and pre-allocation are not proved.
export function copySettingsData(value,{maxRequestBytes:maximum,maxInputNodes:maxNodes,maxInputDepth:maxDepth}){
  if(![maximum,maxNodes,maxDepth].every(natural)||maxNodes<1)return null;
  const result={},active=new WeakSet(),stack=[{value,parent:result,key:'value',depth:0}];
  let bytes=0,nodes=0,reserved=1;
  const charge=n=>natural(n)&&n<=maximum-bytes?(bytes+=n,true):false;
  while(stack.length){
    const task=stack.pop();
    if(task.exit){active.delete(task.value);Object.freeze(task.output);continue;}
    const v=task.value;if(task.depth>maxDepth||++nodes>maxNodes)return null;
    if(v===null||typeof v==='boolean'||typeof v==='number'||typeof v==='string'){
      let size;
      if(v===null)size=4;
      else if(typeof v==='boolean')size=v?4:5;
      else if(typeof v==='number'){if(!Number.isFinite(v))return null;size=JSON.stringify(v).length;}
      else {size=quotedBytes(v,maximum-bytes);if(size===null)return null;}
      if(!charge(size))return null;put(task.parent,task.key,v);continue;
    }
    if(!v||typeof v!=='object'||active.has(v))return null;
    const array=Array.isArray(v),prototype=Object.getPrototypeOf(v);
    if(array?prototype!==Array.prototype:prototype!==Object.prototype&&prototype!==null)return null;
    const descriptors=Object.getOwnPropertyDescriptors(v),all=Reflect.ownKeys(descriptors);
    if(all.some(k=>typeof k!=='string'))return null;
    let keys=all;
    if(array){
      const length=descriptors.length?.value;
      if(!natural(length)||all.length!==length+1)return null;
      keys=all.filter(k=>k!=='length');if(keys.some((k,i)=>k!==String(i)))return null;
    }
    if(keys.length>maxNodes-reserved)return null;reserved+=keys.length;
    if(keys.some(k=>!Object.hasOwn(descriptors[k],'value')||descriptors[k].enumerable!==true))return null;
    if(!charge(2+Math.max(0,keys.length-1)))return null;
    if(!array)for(const key of keys){const size=quotedBytes(key,maximum-bytes);if(size===null||!charge(size+1))return null;}
    const output=array?[]:{};put(task.parent,task.key,output);active.add(v);
    stack.push({exit:true,value:v,output});
    for(let i=keys.length-1;i>=0;i--)stack.push({value:descriptors[keys[i]].value,parent:output,key:keys[i],depth:task.depth+1});
  }
  return {value:result.value,bytes,nodes};
}
export function normalizeSemanticSettings(envelope,context,limits){
  if(!exact(limits,C.limits)||!Object.values(limits).every(natural))return null;
  const captured=copySettingsData(envelope,limits),bound=copySettingsData(context,limits);
  if(!captured||!bound)return null;
  const e=captured.value,c=bound.value;
  const label=s=>typeof s==='string'&&s.length>0&&Buffer.byteLength(s,'utf8')<=limits.maxIdentifierBytes;
  const version=n=>Number.isSafeInteger(n)&&n>0;
  if(!exact(c,C.contextFields)||!Object.values(c).every(label)||!exact(e,C.envelopeFields)||!['typed','dynamic'].includes(e.route))return null;
  const identity=e.route==='typed'?'family':'name',r=e.request;
  if(!exact(r,[identity,'variant','version','arguments']))return null;
  const tuple=Object.fromEntries([identity,'variant','version'].map(k=>[k,r[k]]));
  const decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
  if(decoded?.descriptor.id!==C.family||decoded.variant!==C.operation)return null;
  const a=r.arguments;
  if(!exact(a,C.argumentFields)||a.contract!==C.contract||a.operation!==C.operation||!label(a.invocation)||a.profile!==c.profile||
    !Array.isArray(a.changes)||a.changes.length>limits.maxChanges||!Array.isArray(a.dependencies)||a.dependencies.length>limits.maxDependencies)return null;
  const changes=new Set(),layers=new Map(),dependencies=new Set();let valueBytes=0;
  for(const change of a.changes){
    if(!exact(change,C.changeFields)||!exact(change.layer,C.layerFields)||!label(change.layer.authority)||!label(change.layer.id)||
      !version(change.layer.incarnation)||change.layer.authority!==c.authority||!version(change.expectedGeneration)||!label(change.key)||!C.operations.includes(change.operation)||
      (change.operation!=='set'&&change.value!==null))return null;
    const layer=JSON.stringify([change.layer.authority,change.layer.id,change.layer.incarnation]),key=JSON.stringify([layer,change.key]);
    if(changes.has(key)||layers.has(layer)&&layers.get(layer)!==change.expectedGeneration)return null;
    changes.add(key);layers.set(layer,change.expectedGeneration);
    if(change.operation==='set'){
      const measured=copySettingsData(change.value,{...limits,maxRequestBytes:limits.maxValueBytes-valueBytes});if(!measured)return null;valueBytes+=measured.bytes;
    }
  }
  for(const d of a.dependencies){
    if(!exact(d,C.dependencyFields)||!label(d.domain)||!label(d.identity)||!version(d.generation))return null;
    const key=JSON.stringify([d.domain,d.identity]);if(dependencies.has(key))return null;dependencies.add(key);
  }
  // Data ownership, not an issued selector/grant or an accepted semantic resolution.
  return Object.freeze({arguments:a,context:c,requestBytes:captured.bytes,contextBytes:bound.bytes,valueBytes});
}
