// Selected F07 value structure and original binding, not policy or schema truth.
import {copySettingsData} from './semantic-settings.mjs';
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const forms=freeze({value:['status','context','target','key','value'],origins:['status','context','target','key','coverage','origins'],capabilities:['status','context','target','coverage','capabilities'],unavailable:['status']}),declarationFields=Object.freeze(['key','type','domain','scopes','composition','sensitivity']);
export function settingsQueryResultDeclarations(){return freeze({forms,declarationFields});}
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v),text=v=>typeof v==='string'&&v.length>0;
function exact(v,keys){if(!object(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),all=Reflect.ownKeys(d);return all.length===keys.length&&keys.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable===true);}
const dense=v=>Array.isArray(v)&&Object.getPrototypeOf(v)===Array.prototype&&Reflect.ownKeys(v).length===v.length+1&&Array.from({length:v.length},(_,i)=>Object.hasOwn(v,i)&&Object.getOwnPropertyDescriptor(v,String(i))?.enumerable===true&&Object.hasOwn(Object.getOwnPropertyDescriptor(v,String(i)),'value')).every(Boolean);
function originArray(v){return dense(v)&&v.every(object);}
export function settingsQueryResultFrameValid(value,operation,context,target,key){
 if(!['effective-configuration','permitted-origin','capabilities'].includes(operation)||!object(value))return false;
 const status=Object.getOwnPropertyDescriptor(value,'status');if(!status||!Object.hasOwn(status,'value')||typeof status.value!=='string'||!Object.hasOwn(forms,status.value)||!exact(value,forms[status.value]))return false;
 if(status.value==='unavailable')return true;
 if(value.context!==context||value.target!==target)return false;
 if(operation==='effective-configuration')return status.value==='value'&&value.key===key&&text(value.key);
 if(operation==='permitted-origin'){
  if(status.value!=='origins'||value.key!==key||!text(value.key)||value.coverage!=='permitted-only')return false;
  if(Array.isArray(value.origins))return originArray(value.origins);
  if(!object(value.origins)||![Object.prototype,null].includes(Object.getPrototypeOf(value.origins)))return false;
  const d=Object.getOwnPropertyDescriptors(value.origins);return Reflect.ownKeys(d).every(k=>typeof k==='string'&&Object.hasOwn(d[k],'value')&&d[k].enumerable===true&&originArray(d[k].value));
 }
 if(status.value!=='capabilities'||value.coverage!=='permitted-only'||!dense(value.capabilities))return false;
 return value.capabilities.every(d=>exact(d,declarationFields)&&text(d.key)&&object(d.type)&&text(d.domain)&&text(d.composition)&&typeof d.sensitivity==='boolean'&&dense(d.scopes)&&d.scopes.every(text))&&new Set(value.capabilities.map(d=>d.key)).size===value.capabilities.length;
}
export function settingsQueryResultValid(value,operation,context,target,key,limits){
 if(!settingsQueryResultFrameValid(value,operation,context,target,key))return false;
 if(value.status==='value')return copySettingsData(value.value,limits)!==null;
 if(value.status==='capabilities')return copySettingsData(value.capabilities,limits)!==null;
 return true;
}
// Original context-local reference identities; structural equality cannot replace them.
export function settingsQueryOriginBindingsValid(value,originData,references){
 const array=(actual,ids)=>dense(actual)&&dense(ids)&&actual.length===ids.length&&ids.every((id,i)=>references.has(id)&&actual[i]===references.get(id));
 if(Array.isArray(originData))return array(value,originData);
 if(!object(originData)||!object(value)||Array.isArray(value))return false;
 const keys=Object.keys(originData);return exact(value,keys)&&keys.every(k=>array(value[k],originData[k]));
}
