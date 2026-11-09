// Selected F03 argument meaning only; native lookup owns selector authentication.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {annotationQueryInputForms} from './annotation-queries.mjs';
const forms=annotationQueryInputForms(),operations=['enumerate','inspect','target-location'],common=['contract','operation','snapshot','document','profile'];
if(forms.length!==3||!equal(forms.map(f=>f.operation).toSorted(),operations.toSorted())||forms.some(f=>f.family!=='F03'||f.contract!=='annotation-query/1'||f.variant!==f.operation||!equal([...f.fields].sort(),[...common,...(f.operation==='enumerate'?['limit','continuation']:['annotation'])].sort())))throw Error('unsupported native annotation query declaration');
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v),contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const d=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
export const annotationQueryClassificationBytes=Buffer.byteLength(JSON.stringify(Object.fromEntries(limitFields.map(k=>[k,Number.MAX_SAFE_INTEGER]))));
export function annotationQueryArgumentInputForms(){return Object.freeze([...forms]);}
export function captureAnnotationQueryLimits(input){const p=fields(input);return exact(p,limitFields)&&Object.values(p).every(natural)&&p.maxInputNodes>=1?Object.freeze(p):null;}
export function normalizeAnnotationQueryArguments(envelope,authorityContext,inputLimits,maximum){
 const limits=fields(inputLimits);if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1)return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F03')return null;
 const args=fields(raw),form=forms.find(f=>f.operation===args?.operation);if(!form||!exact(args,form.fields)||args.contract!==form.contract||decoded.variant!==args.operation||!object(args.snapshot))return null;
 const role=args.operation==='enumerate'?'continuation':'annotation',selector=args[role];if(role==='continuation'?selector!==null&&!object(selector):!object(selector))return null;
 const opaque=Object.freeze({snapshot:args.snapshot,role,selector}),shadow=Object.fromEntries(form.fields.map(k=>[k,k==='snapshot'||k===role?null:args[k]])),whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(authorityContext,cap);if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const r=whole.value.request.arguments,c=bound.value;if(!exact(r.document,['authority','id','incarnation'])||r.document.authority!==c.authority||!text(r.document.id)||!natural(r.document.incarnation)||r.document.incarnation<1||!text(r.profile))return null;
 if(r.operation==='enumerate'&&(!natural(r.limit)||r.limit<1))return null;
 const capture=copySettingsData({request:r,context:c,limits,opaqueBindings:['annotation-snapshot',role]},cap);if(!capture)return null;
 return Object.freeze({kind:'annotation-query-arguments',record:capture.value,opaque,route:e.route});
}
export function sameAnnotationQueryArguments(a,b){return Boolean(a&&b&&a.kind==='annotation-query-arguments'&&b.kind==='annotation-query-arguments'&&a.opaque.snapshot===b.opaque.snapshot&&a.opaque.role===b.opaque.role&&a.opaque.selector===b.opaque.selector&&equal(a.record,b.record));}
export function annotationQueryProducerRequest(n){return n?{...n.record.request,snapshot:n.opaque.snapshot,[n.opaque.role]:n.opaque.selector}:null;}
