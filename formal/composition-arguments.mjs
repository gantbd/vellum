// Closed normalized native facade binding; not an IME or hostile byte decoder.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {compositionInputForm} from './selection-products.mjs';
const form=compositionInputForm(),limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'],contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0;
function fields(v){if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return null;const ds=Object.getOwnPropertyDescriptors(v),ks=Reflect.ownKeys(ds);if(ks.some(k=>typeof k!=='string'||!Object.hasOwn(ds[k],'value')||ds[k].enumerable!==true))return null;return Object.fromEntries(ks.map(k=>[k,ds[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
const identity=(v,c)=>exact(v,['authority','id','incarnation'])&&v.authority===c.authority&&text(v.authority)&&text(v.id)&&natural(v.incarnation);
export function normalizeCompositionArguments(envelope,context,inputLimits,maximum){
  const limits=fields(inputLimits),e=fields(envelope);
  if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1||!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
  const header=fields(e.request);if(!exact(header,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
  const {arguments:raw,...tuple}=header,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
  if(decoded?.descriptor.id!=='F11'||decoded.variant!==form.variant)return null;
  const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},whole=copySettingsData({request:raw,context,limits},cap);
  if(!whole)return null;
  const {request:r,context:c}=whole.value;
  if(!exact(r,form.fields)||r.contract!==form.contract||r.operation!==form.operation||r.profile!==form.profileConstant||!exact(c,contextFields)||!Object.values(c).every(text)||!text(r.invocation)||!identity(r.document,c)||!identity(r.view,c)||!natural(r.sourceRevision)||!natural(r.expectedSelection)||typeof r.text!=='string'||!exact(r.replacement,['lo','hi','expected'])||!natural(r.replacement.lo)||!natural(r.replacement.hi)||r.replacement.lo>r.replacement.hi||!Array.isArray(r.replacement.expected)||r.replacement.expected.some(b=>!Number.isInteger(b)||b<0||b>255))return null;
  return Object.freeze({kind:'selection',route:e.route,record:whole.value});
}
export function sameCompositionArguments(a,b){return Boolean(a&&b&&equal(a.record,b.record));}
