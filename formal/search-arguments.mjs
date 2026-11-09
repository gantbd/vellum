// Selected native F05 argument normalization only, not a wire codec or query grant.
import {isDeepStrictEqual as equal} from 'node:util';
import {copySettingsData} from './semantic-settings.mjs';
import {decodeEngineTyped,decodeEngineDynamic} from './engine-facade.mjs';
import {literalSearchInputForm} from './literal-search.mjs';
import {nativeSourceSearchCurrentForm} from './native-source-search.mjs';
const form=literalSearchInputForm(),operations=['literal-matches','next-match','previous-match'],requestFields=['contract','operation','context','ref','query','profile','scopes','generation','navigation'];
if(form.family!=='F05'||form.contract!=='literal-search/1'||!equal([...form.operations].sort(),operations.toSorted())||!equal([...form.fields].sort(),requestFields.toSorted())||form.operations.length!==3||form.fields.length!==9)throw Error('unsupported literal search declaration source');
const contextFields=['authority','engine','recipient','purpose','destination','profile','continuity'],limitFields=['maxRequestBytes','maxInputNodes','maxInputDepth'];
const natural=n=>Number.isSafeInteger(n)&&n>=0,text=v=>typeof v==='string'&&v.length>0,object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
function fields(value){if(!object(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;const d=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(d);if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(d[k],'value')||d[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,d[k].value]));}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
const valueFields=Object.freeze({ref:Object.freeze(['authority','document','incarnation','revision']),scope:Object.freeze(['lo','hi']),navigation:Object.freeze(['start','current','wrap'])});
export function searchArgumentValueFields(){return valueFields;}
export const searchArgumentClassificationBytes=Buffer.byteLength(JSON.stringify(Object.fromEntries(limitFields.map(k=>[k,Number.MAX_SAFE_INTEGER]))));
export function captureSearchArgumentLimits(input){const p=fields(input);return exact(p,limitFields)&&Object.values(p).every(natural)&&p.maxInputNodes>=1?Object.freeze(p):null;}
// Current request and current opaque identity are independently captured, never
// reconstructed from the decoded command. Semantic agreement stays native.
export function captureSearchCurrentInputs(sourceContext,currentQuery,facts,limits,maximum){
 const f=nativeSourceSearchCurrentForm(),q=fields(currentQuery);if(!exact(q,f.fields))return null;
 const r=fields(q.request);if(!exact(r,f.requestFields)||!object(r.context))return null;
 const opaque=r.context,packet=copySettingsData({sourceContext,current:{...q,request:{...r,context:null}},facts},{...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)});if(!packet)return null;
 return {sourceContext:packet.value.sourceContext,current:{...packet.value.current,request:{...packet.value.current.request,context:opaque}},facts:packet.value.facts};
}
export function normalizeSearchArguments(envelope,authorityContext,inputLimits,maximum){
 const limits=fields(inputLimits);if(!natural(maximum)||!exact(limits,limitFields)||!Object.values(limits).every(natural)||limits.maxInputNodes<1)return null;
 const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)},e=fields(envelope);if(!exact(e,['route','request'])||!['typed','dynamic'].includes(e.route))return null;
 const h=fields(e.request);if(!exact(h,e.route==='typed'?['family','variant','version','arguments']:['name','variant','version','arguments']))return null;
 const {arguments:raw,...tuple}=h,decoded=e.route==='typed'?decodeEngineTyped(tuple):decodeEngineDynamic(tuple);if(decoded?.descriptor.id!=='F05')return null;
 const args=fields(raw);if(!exact(args,form.fields)||args.contract!==form.contract||!operations.includes(args.operation)||decoded.variant!==args.operation||!object(args.context))return null;
 // Opaque identity is bound separately and never traversed as caller data.
 const opaque=args.context,shadow=Object.fromEntries(form.fields.map(k=>[k,k==='context'?null:args[k]])),whole=copySettingsData({route:e.route,request:{...tuple,arguments:shadow}},cap),bound=copySettingsData(authorityContext,cap);if(!whole||!bound||!exact(bound.value,contextFields)||!Object.values(bound.value).every(text))return null;
 const r=whole.value.request.arguments,c=bound.value;if(!exact(r.ref,valueFields.ref)||r.ref.authority!==c.authority||!text(r.ref.document)||!natural(r.ref.incarnation)||!natural(r.ref.revision)||!natural(r.generation)||typeof r.query!=='string'||!text(r.profile)||!Array.isArray(r.scopes)||r.scopes.some(s=>!exact(s,valueFields.scope)||!natural(s.lo)||!natural(s.hi)||s.lo>s.hi))return null;
 if(r.operation==='literal-matches'?r.navigation!==null:!exact(r.navigation,valueFields.navigation)||!natural(r.navigation.start)||(r.navigation.current!==null&&!natural(r.navigation.current))||typeof r.navigation.wrap!=='boolean')return null;
 const capture=copySettingsData({request:r,context:c,limits,opaqueBindings:['read-context']},cap);if(!capture)return null;
 return Object.freeze({kind:'literal-search-arguments',record:capture.value,opaque,route:e.route});
}
export function sameSearchArguments(a,b){return Boolean(a&&b&&a.kind==='literal-search-arguments'&&b.kind==='literal-search-arguments'&&a.opaque===b.opaque&&equal(a.record,b.record));}
export function searchProducerRequest(n){return n?{...n.record.request,context:n.opaque}:null;}
