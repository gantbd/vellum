// Existing F04 value structure. A matching shape is not history/alias authority.
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const forms=freeze({frontier:['status','context','group'],availability:['status','context','value'],'blocking-detail':['status','context','blockers'],unavailable:['status']});
const values=Object.freeze(['empty','eligible','blocked']),natural=n=>Number.isSafeInteger(n)&&n>=0,label=s=>typeof s==='string'&&s.length>0&&new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Buffer.from(s))===s;
function exact(v,keys){if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),own=Reflect.ownKeys(d);return own.length===keys.length&&keys.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable===true);}
function dense(a,max){if(!Array.isArray(a)||Object.getPrototypeOf(a)!==Array.prototype||a.length>max)return false;const d=Object.getOwnPropertyDescriptors(a),own=Reflect.ownKeys(d);if(own.length!==a.length+1)return false;for(let i=0;i<a.length;i++)if(!Object.hasOwn(d,i)||!Object.hasOwn(d[i],'value')||d[i].enumerable!==true)return false;return true;}
export function historyQueryResultForms(){return forms;}
export function historyQueryResultAvailability(){return values;}
export function historyQueryResultValid(out,request,maxDetails){
 if(!natural(maxDetails)||!out||typeof out!=='object')return false;const status=Object.getOwnPropertyDescriptor(out,'status')?.value;if(typeof status!=='string'||!Object.hasOwn(forms,status)||!exact(out,forms[status]))return false;
 if(status==='unavailable')return true;if(!request||out.context!==request.context)return false;
 if(status==='frontier')return request.operation==='frontier'&&(out.group===null||label(out.group));
 if(status==='availability')return request.operation==='availability'&&values.includes(out.value);
 if(request.operation!=='permitted-blocking-detail'||!dense(out.blockers,maxDetails))return false;
 for(let i=0;i<out.blockers.length;i++){const row=out.blockers[i];if(!exact(row,['scope','group'])||!label(row.scope)||!label(row.group)||(i>0&&out.blockers[i-1].scope>=row.scope))return false;}return true;
}
// The actual owner supplies its independently captured projection, never reply data.
export function historyQueryResultAliasesValid(out,binding){
 if(out.status==='frontier')return out.group===null||binding.groups.some(g=>g.reference===out.group);
 if(out.status==='blocking-detail')return binding.details===true&&out.blockers.every(r=>binding.scopes.some(s=>s.reference===r.scope)&&binding.groups.some(g=>g.reference===r.group));
 return true;
}
