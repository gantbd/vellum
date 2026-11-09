// Isolated F12 normalization prototype, not an adopted request/execution port.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {selectionInputForms} from './selection-products.mjs';
import {boundaryInputForms} from './boundary-commands.mjs';
const forms=[...selectionInputForms(),...boundaryInputForms()].filter(f=>f.family==='F12');
const known=['selection-edit/1:set-selection','selection-edit/1:logical-movement','selection-edit/1:select-search-match','boundary-command/1:logical-movement'];
if(forms.length!==4||Object.keys(forms).length!==4||new Set(forms.map(f=>f.contract+':'+f.operation)).size!==4||forms.some(f=>!known.includes(f.contract+':'+f.operation)||f.variant!==f.operation))throw Error('unsupported selection declaration source');
const contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const d=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
const identity=(v,c)=>object(v)&&exact(v,['authority','id','incarnation'])&&v.authority===c.authority&&text(v.id)&&text(v.authority)&&natural(v.incarnation);
export function normalizeSelectionArguments(envelope,context,inputLimits,maximum){
 const limits=fields(inputLimits);if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1)return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F12')return null;
 const args=fields(raw),form=args&&forms.find(f=>f.contract===args.contract&&f.operation===args.operation);if(!form||!exact(args,form.fields)||args.profile!==form.profileConstant||decoded.variant!==form.variant)return null;
 const kind=form.contract==='boundary-command/1'?'boundary':'selection',opaqueField=kind==='boundary'?'boundary':form.operation==='select-search-match'?'match':null,opaque=opaqueField?args[opaqueField]:null;if(opaqueField&&!object(opaque))return null;
 const shadow={...args,...(opaqueField?{[opaqueField]:null}:{})},whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(context,cap);if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const r=whole.value.request.arguments,c=bound.value;if(!text(r.invocation)||!identity(r.document,c)||!identity(r.view,c)||!natural(r.sourceRevision)||!natural(r.expectedSelection))return null;
 if(r.operation==='set-selection'&&(!Array.isArray(r.selections)||!natural(r.primary)))return null;
 if(r.operation==='logical-movement'&&(!text(r.unit)||typeof r.extend!=='boolean'))return null;
 if(kind==='boundary'&&(!text(r.direction)||!natural(r.count)||r.count<1))return null;
 if(r.operation==='select-search-match'&&(!text(r.direction)||!text(r.affinity)||!text(r.selectionMode)))return null;
 const capture=copySettingsData({request:r,context:c,limits,opaqueBindings:opaqueField?[opaqueField]:[]},cap);if(!capture)return null;
 return Object.freeze({kind,record:capture.value,opaqueField,opaque,route:e.route});
}
export function sameSelectionArguments(a,b){return Boolean(a&&b&&a.kind===b.kind&&a.opaqueField===b.opaqueField&&a.opaque===b.opaque&&equal(a.record,b.record));}
export function selectionProducerRequest(n){return n?{...n.record.request,...(n.opaqueField?{[n.opaqueField]:n.opaque}:{})}:null;}
