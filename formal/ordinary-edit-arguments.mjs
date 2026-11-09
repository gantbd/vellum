// Isolated normalized native port; no byte/wire decoder or publication authority.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {selectionInputForms} from './selection-products.mjs';
import {lineInputForm} from './line-edits.mjs';
import {boundaryInputForms} from './boundary-commands.mjs';
import {configuredLineInputForm} from './configured-line-state.mjs';
const forms=[...selectionInputForms().filter(f=>f.family==='F11'),lineInputForm(),...boundaryInputForms().filter(f=>f.family==='F11'),configuredLineInputForm()];
const contracts=['selection-edit/1','line-edit/1','boundary-command/1','configured-line-edit/1'];
if(forms.length!==contracts.length||Object.keys(forms).length!==forms.length||new Set(forms.map(f=>f.contract)).size!==contracts.length||forms.some(f=>f.family!=='F11'||!contracts.includes(f.contract)||f.operation!=='ordinary-edit'||f.variant!=='ordinary-edit'))throw Error('unsupported ordinary edit declaration source');
const contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const descriptors=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(descriptors);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(descriptors[k],'value')||descriptors[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,descriptors[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
const identity=(v,c)=>object(v)&&exact(v,['authority','id','incarnation'])&&v.authority===c.authority&&text(v.id)&&text(v.authority)&&natural(v.incarnation);
export function normalizeOrdinaryEditArguments(envelope,context,inputLimits,maximum){
 const limits=fields(inputLimits);if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1)return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const header=fields(e.request);if(!exact(header,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=header,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F11'||decoded.variant!=='ordinary-edit')return null;
 const args=fields(raw),form=args&&forms.find(f=>f.contract===args.contract);if(!form||!exact(args,form.fields)||args.operation!==form.operation||args.profile!==form.profileConstant)return null;
 const unicode=form.contract==='boundary-command/1',boundary=unicode?args.boundary:null;if(unicode&&!object(boundary))return null;
 // Opaque binding is retained by identity, never traversed, frozen or serialized.
 const shadow={...args,...(unicode?{boundary:null}:{})},whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(context,cap);
 if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const r=whole.value.request.arguments,c=bound.value;
 if(!text(r.invocation)||!identity(r.document,c)||!identity(r.view,c)||!natural(r.sourceRevision)||!natural(r.expectedSelection))return null;
 if(form.contract==='selection-edit/1'&&(typeof r.text!=='string'||!text(r.selectionAfter)))return null;
 if(form.contract==='line-edit/1'&&(!text(r.action)||!object(r.settings)))return null;
 if(unicode&&(!text(r.unit)||!text(r.direction)||!natural(r.count)||r.count<1||typeof r.extend!=='boolean'))return null;
 if(form.contract==='configured-line-edit/1'&&(!text(r.action)||!object(r.configuration)))return null;
 const configured=form.contract==='configured-line-edit/1',kind=unicode?'boundary':form.contract==='selection-edit/1'?'selection':'line';
 const captured=copySettingsData({request:r,context:c,limits,opaqueBindings:unicode?['unicode-boundary']:[]},cap);if(!captured)return null;
 return Object.freeze({kind,configured,record:captured.value,boundary,route:e.route});
}
export function sameOrdinaryEditArguments(left,right){return Boolean(left&&right&&left.kind===right.kind&&left.configured===right.configured&&left.boundary===right.boundary&&equal(left.record,right.record));}
export function ordinaryEditProducerRequest(normalized){return normalized?{...normalized.record.request,...(normalized.kind==='boundary'?{boundary:normalized.boundary}:{})}:null;}
