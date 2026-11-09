// Selected native acquisition shape. Only native staging authenticates a new alias.
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;},forms=freeze({context:['status','context','coverage'],unavailable:['status']});
export function registrationAcquisitionResultForms(){return forms;}
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
function exact(v,keys){if(!object(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),all=Reflect.ownKeys(d);return all.length===keys.length&&keys.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable===true);}
export function registrationAcquisitionResultValid(value,expectedContext){
 if(!object(value))return false;const s=Object.getOwnPropertyDescriptor(value,'status');if(!s||!Object.hasOwn(s,'value')||typeof s.value!=='string'||!Object.hasOwn(forms,s.value)||!exact(value,forms[s.value]))return false;
 return s.value==='unavailable'||value.coverage==='captured-permitted'&&object(value.context)&&value.context===expectedContext;
}
