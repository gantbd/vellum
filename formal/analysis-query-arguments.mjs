// F08 arguments only. Native snapshot ownership and current release stay native.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {analysisQueryInputForms} from './analysis-work.mjs';
const operations=['supported-analysis','supplied-evidence-inspection'];
const contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'];
const limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0;
const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
const exact=(v,keys)=>v&&Object.keys(v).length===keys.length&&keys.every(k=>Object.hasOwn(v,k));
function fields(value){
 if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;
 const descriptors=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(descriptors);
 if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(descriptors[k],'value')||descriptors[k].enumerable!==true))return null;
 return Object.fromEntries(keys.map(k=>[k,descriptors[k].value]));
}
export const analysisQueryClassificationBytes=Buffer.byteLength(JSON.stringify(Object.fromEntries(limitFields.map(k=>[k,Number.MAX_SAFE_INTEGER]))));
export function captureAnalysisQueryLimits(input){
 const p=fields(input);return exact(p,limitFields)&&Object.values(p).every(natural)&&p.maxInputNodes>=1?Object.freeze(p):null;
}
export function analysisQueryArgumentForms(){
 const forms=analysisQueryInputForms();
 return forms.length===2&&equal(forms.map(f=>f.operation),operations)&&forms.every(f=>f.family==='F08'&&f.variant===f.operation&&f.contract==='analysis-query/1'&&equal(f.fields,['contract','operation','snapshot','view',...(f.operation==='supported-analysis'?['range']:[])]))?forms:null;
}
export function normalizeAnalysisQueryArguments(envelope,authorityContext,inputLimits,maximum){
 const limits=captureAnalysisQueryLimits(inputLimits);if(!limits||!natural(maximum))return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);
 if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);
 if(decoded?.descriptor.id!=='F08')return null;
 const args=fields(raw),form=analysisQueryArgumentForms()?.find(f=>f.operation===args?.operation);
 if(!form||!exact(args,form.fields)||args.contract!==form.contract||decoded.variant!==form.operation||!object(args.snapshot))return null;
 const snapshot=args.snapshot,shadow={...args,snapshot:null};
 const whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(authorityContext,cap);
 if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const request=whole.value.request.arguments,context=bound.value;
 if(!exact(request.view,['authority','id','incarnation'])||request.view.authority!==context.authority||!text(request.view.id)||!natural(request.view.incarnation))return null;
 if(request.operation==='supported-analysis'&&(!Array.isArray(request.range)||request.range.length!==2||!request.range.every(natural)||request.range[0]>request.range[1]))return null;
 const captured=copySettingsData({request,context,limits,opaqueBindings:['analysis-snapshot']},cap);if(!captured)return null;
 return Object.freeze({kind:'analysis-query-arguments',record:captured.value,snapshot,route:e.route});
}
export function sameAnalysisQueryArguments(a,b){return Boolean(a&&b&&a.kind==='analysis-query-arguments'&&b.kind==='analysis-query-arguments'&&a.snapshot===b.snapshot&&equal(a.record,b.record));}
export function analysisQueryProducerRequest(n){return n?{...n.record.request,snapshot:n.snapshot}:null;}
