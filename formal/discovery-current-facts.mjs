// Copy only the native query's data facts, never unrelated host bookkeeping.
import {copySettingsData} from './semantic-settings.mjs';
import {nativeDiscoveryQueryCurrentFactFields} from './native-discovery.mjs';
export function captureNativeDiscoveryQueryCurrentFacts(input,limits,maximum){
 if(!input||typeof input!=='object'||Array.isArray(input)||![Object.prototype,null].includes(Object.getPrototypeOf(input)))return null;
 const facts={};
 for(const key of nativeDiscoveryQueryCurrentFactFields()){
  const field=Object.getOwnPropertyDescriptor(input,key);if(!field)continue;
  if(!Object.hasOwn(field,'value')||field.enumerable!==true)return null;
  if(field.value!==undefined)facts[key]=field.value;
 }
 return copySettingsData({facts},{...limits,maxRequestBytes:maximum});
}
