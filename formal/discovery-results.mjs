// Closed existing F01 native values. Structural validity is not selector authority.
import fs from 'node:fs';
const catalog=JSON.parse(fs.readFileSync(new URL('./contract-discovery.json',import.meta.url),'utf8')),native=JSON.parse(fs.readFileSync(new URL('./native-discovery.json',import.meta.url),'utf8'));
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const forms=freeze({page:['status','context','coverage','entries','continuation'],descriptor:['status','context','descriptor'],advice:['status','context','target','entry','variant','enabled'],unavailable:['status']}),entryFields=Object.freeze(['reference','version','kind']),descriptorFields=Object.freeze(['reference','version','kind','variants']),facets=Object.freeze(['input','outcomes','requirements','limits','cancellation']),declarations=freeze({forms,entryFields,descriptorFields,facets,coverage:'captured-permitted',kinds:[...native.kinds],presentation:[...catalog.presentation]});
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v),natural=n=>Number.isSafeInteger(n)&&n>=0,opaque=object;
function fields(v){if(!object(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return null;const ds=Object.getOwnPropertyDescriptors(v),keys=Reflect.ownKeys(ds);return keys.every(k=>typeof k==='string'&&Object.hasOwn(ds[k],'value')&&ds[k].enumerable===true)?Object.fromEntries(keys.map(k=>[k,ds[k].value])):null;}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
function dense(a,max){if(!Array.isArray(a)||Object.getPrototypeOf(a)!==Array.prototype||a.length>max)return false;const ds=Object.getOwnPropertyDescriptors(a),keys=Reflect.ownKeys(ds);if(keys.length!==a.length+1)return false;for(let i=0;i<a.length;i++)if(!Object.hasOwn(ds,i)||!Object.hasOwn(ds[i],'value')||ds[i].enumerable!==true)return false;return true;}
const utf8=s=>typeof s==='string'&&Buffer.from(s,'utf8').toString('utf8')===s;
// Iterative JSON cost validation; opaque references never enter this traversal.
export function discoveryResultMetadataValid(value,l){
 if(!['maxMetadataBytes','maxMetadataNodes','maxMetadataDepth'].every(k=>natural(l?.[k])))return false;
 const stack=[{value,depth:0}];let nodes=0,bytes=0;
 while(stack.length){const {value:v,depth}=stack.pop();if(++nodes>l.maxMetadataNodes||depth>l.maxMetadataDepth)return false;
  if(v===null||typeof v==='boolean'||typeof v==='number'&&Number.isFinite(v)){bytes+=Buffer.byteLength(JSON.stringify(v));}
  else if(typeof v==='string'){if(!utf8(v))return false;bytes+=Buffer.byteLength(JSON.stringify(v));}
  else if(Array.isArray(v)){if(!dense(v,l.maxMetadataNodes-nodes))return false;bytes+=2+Math.max(0,v.length-1);for(let i=v.length-1;i>=0;i--)stack.push({value:v[i],depth:depth+1});}
  else {const data=fields(v);if(!data)return false;const keys=Object.keys(data);if(keys.length>l.maxMetadataNodes-nodes)return false;bytes+=2+Math.max(0,keys.length-1);for(const k of keys){if(!utf8(k))return false;bytes+=Buffer.byteLength(JSON.stringify(k))+1;stack.push({value:data[k],depth:depth+1});}}
  if(bytes>l.maxMetadataBytes)return false;
 }
 return true;
}
export function nativeDiscoveryResultDeclarations(){return declarations;}
export function nativeDiscoveryResultValid(value,request,l){
 const r=fields(request),v=fields(value);if(!r||!Object.hasOwn(catalog.variants,r.operation)||!exact(r,[...catalog.common,...catalog.variants[r.operation]])||r.contract!==catalog.contract||!utf8(r.profile)||!r.profile.length||!opaque(r.context)||!v||typeof v.status!=='string'||!Object.hasOwn(forms,v.status)||!exact(v,forms[v.status])||!['maxPageRows','maxVariants','maxMetadataBytes','maxMetadataNodes','maxMetadataDepth'].every(k=>natural(l?.[k])))return false;
 if(v.status==='unavailable')return true;if(v.context!==r.context)return false;
 const versionKind=x=>natural(x.version)&&x.version>0&&declarations.kinds.includes(x.kind);
 if(v.status==='page'){
  if(r.operation!=='list'||!natural(r.limit)||r.limit<1||v.coverage!==declarations.coverage||!dense(v.entries,Math.min(r.limit,l.maxPageRows))||!(v.continuation===null||opaque(v.continuation)))return false;
  const refs=new Set();for(const entry of v.entries){const e=fields(entry);if(!exact(e,entryFields)||!opaque(e.reference)||!versionKind(e)||refs.has(e.reference))return false;refs.add(e.reference);}return true;
 }
 if(v.status==='advice')return r.operation==='advisory-enablement'&&opaque(r.entry)&&opaque(r.variant)&&opaque(r.target)&&v.entry===r.entry&&v.variant===r.variant&&v.target===r.target&&typeof v.enabled==='boolean';
 if(r.operation!=='inspect'||!opaque(r.entry))return false;
 const d=fields(v.descriptor);if(!d||!exact(d,[...descriptorFields,...(Object.hasOwn(d,'presentation')?['presentation']:[])])||d.reference!==r.entry||!versionKind(d)||!dense(d.variants,l.maxVariants))return false;
 const refs=new Set();for(const item of d.variants){const f=fields(item);if(!f)return false;const visible=Object.hasOwn(f,'input');if(!exact(f,['reference',...(visible?facets:[])])||!opaque(f.reference)||refs.has(f.reference)||visible&&!facets.every(k=>discoveryResultMetadataValid(f[k],l)))return false;refs.add(f.reference);}
 if(Object.hasOwn(d,'presentation')){const p=fields(d.presentation);if(!p||Object.keys(p).some(k=>!declarations.presentation.includes(k))||!discoveryResultMetadataValid(d.presentation,l))return false;}
 return true;
}
