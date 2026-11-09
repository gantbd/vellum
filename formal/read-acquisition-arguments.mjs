// F02 handoff argument meaning only. No root capture, cut validation or read grant.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {singleReadAcquisitionInputForm,singleReadAcquisitionResultValid} from './read-contexts.mjs';
import {versionSetAcquisitionInputForm,versionSetAcquisitionResultValid} from './version-set-contexts.mjs';
const single=singleReadAcquisitionInputForm(),multiple=versionSetAcquisitionInputForm(),forms=[single,multiple],contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
if(!equal(single.fields.toSorted(),['contract','operation','document','selection','data','profile'].sort())||!equal(multiple.fields.toSorted(),['contract','operation','authority','participants','selection','profile'].sort())||single.contract!=='read-context/1'||multiple.contract!=='read-version-set/1'||!equal(single.data.toSorted(),['annotations','source'])||!equal(multiple.participantFields.toSorted(),['data','document'])||forms.some(f=>f.family!=='F02'||f.operation!=='snapshot-handoff'||f.variant!=='snapshot-handoff'||!equal(f.selections,{'capture-current':[],retained:['reference']})))throw Error('unsupported native read acquisition declaration');
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const d=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
export const readAcquisitionClassificationBytes=Buffer.byteLength(JSON.stringify(Object.fromEntries(limitFields.map(k=>[k,Number.MAX_SAFE_INTEGER]))));
export function readAcquisitionArgumentInputForms(){return Object.freeze([...forms]);}
export function captureReadAcquisitionLimits(input){const p=fields(input);return exact(p,limitFields)&&Object.values(p).every(natural)&&p.maxInputNodes>=1?Object.freeze(p):null;}
export function captureReadPreparationBinding(input){if(input===null)return Object.freeze({kind:'none'});const p=fields(input);return exact(p,['pool','cut'])&&object(p.pool)&&object(p.cut)?Object.freeze({kind:'prepared',pool:p.pool,cut:p.cut}):null;}
function document(v,authority){return exact(v,['authority','id','incarnation'])&&v.authority===authority&&text(v.id)&&natural(v.incarnation)&&v.incarnation>=1;}
function data(v){return Array.isArray(v)&&v.length>=1&&v.length<=single.data.length&&new Set(v).size===v.length&&v.every(k=>single.data.includes(k))?[...v].sort():null;}
export function normalizeReadAcquisitionArguments(envelope,authorityContext,inputLimits,maximum){
 const limits=fields(inputLimits);if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1)return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F02'||decoded.variant!=='snapshot-handoff')return null;
 const args=fields(raw),form=forms.find(f=>f.contract===args?.contract);if(!form||!exact(args,form.fields)||args.operation!==form.operation||!text(args.profile))return null;
 const selection=fields(args.selection);if(!selection||typeof selection.kind!=='string'||!Object.hasOwn(form.selections,selection.kind)||!exact(selection,['kind',...form.selections[selection.kind]]))return null;
 const retained=selection.kind==='retained',opaque=retained?selection.reference:null;if(retained&&!object(opaque))return null;
 const shadow={...args,selection:retained?{kind:'retained',reference:null}:{kind:'capture-current'}},whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(authorityContext,cap);if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const r=whole.value.request.arguments,c=bound.value;let request;
 if(form.contract===single.contract){const classes=data(r.data);if(!document(r.document,c.authority)||!classes)return null;request={...r,data:classes};}
 else{if(r.authority!==c.authority||!Array.isArray(r.participants)||r.participants.length<1)return null;const keys=new Set(),participants=[];for(const p of r.participants){if(!exact(p,form.participantFields)||!document(p.document,c.authority))return null;const classes=data(p.data),key=JSON.stringify([p.document.authority,p.document.id]);if(!classes||keys.has(key))return null;keys.add(key);participants.push({...p,data:classes});}request={...r,participants};}
 const capture=copySettingsData({request,context:c,limits,opaqueBindings:retained?['retained-reference']:[]},cap);if(!capture)return null;
 return Object.freeze({kind:'read-acquisition-arguments',record:capture.value,opaque,route:e.route});
}
export function sameReadAcquisitionArguments(a,b){return Boolean(a&&b&&a.kind==='read-acquisition-arguments'&&b.kind==='read-acquisition-arguments'&&a.opaque===b.opaque&&equal(a.record,b.record));}
export function readAcquisitionProducerRequest(n){return n?{...n.record.request,selection:n.opaque?{kind:'retained',reference:n.opaque}:{kind:'capture-current'}}:null;}
export function readAcquisitionResultValid(out,request){return request.contract==='read-context/1'?singleReadAcquisitionResultValid(out,request):request.contract==='read-version-set/1'?versionSetAcquisitionResultValid(out,request):false;}
