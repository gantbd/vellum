// Already bounded F08 reply frames only; not raw ingress or proxy isolation.
const natural=n=>Number.isSafeInteger(n)&&n>=0;
function fields(value){
 if(!value||typeof value!=='object'||Array.isArray(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return null;
 const descriptors=Object.getOwnPropertyDescriptors(value),keys=Reflect.ownKeys(descriptors);
 if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(descriptors[k],'value')||descriptors[k].enumerable!==true))return null;
 return Object.fromEntries(keys.map(k=>[k,descriptors[k].value]));
}
const exact=(v,ks)=>v&&Object.keys(v).length===ks.length&&ks.every(k=>Object.hasOwn(v,k));
function sequence(value){
 if(!Array.isArray(value))return null;
 const descriptors=Object.getOwnPropertyDescriptors(value),length=descriptors.length?.value;
 if(!natural(length)||Reflect.ownKeys(descriptors).length!==length+1)return null;
 for(let i=0;i<length;i++){const d=descriptors[String(i)];if(!d||!Object.hasOwn(d,'value')||d.enumerable!==true)return null;}
 return {descriptors,length};
}
const item=(sequence,index)=>sequence.descriptors[String(index)].value;
const identity=value=>{const v=fields(value);return Boolean(exact(v,['authority','id','incarnation'])&&['authority','id'].every(k=>typeof v[k]==='string'&&v[k].length>0)&&natural(v.incarnation));};
const generations=['grammarGeneration','highlightQueryGeneration','discoveryQueryGeneration','registryGeneration','configurationGeneration'];
function context(value){const v=fields(value);return Boolean(exact(v,['document','sourceRevision','analysis',...generations,'languageProfile'])&&identity(v.document)&&identity(v.analysis)&&natural(v.sourceRevision)&&generations.every(k=>natural(v[k]))&&v.languageProfile==='pinned-markdown-javascript-reference/1');}
export function analysisQueryValueValid(value,operation){
 if(!['supported-analysis','supplied-evidence-inspection'].includes(operation))return false;
 const v=fields(value);if(!v)return false;
 if(v.status==='unavailable')return Boolean(exact(v,['status']));
 const capture=operation==='supported-analysis',status=capture?'captures':'analysis-summary';
 if(v.status!==status||!exact(v,['status','context','parseState','coverage',...(capture?['captures']:[])])||!context(v.context)||!['parsed','syntax-errors'].includes(v.parseState)||!['complete-within-profile','partial'].includes(v.coverage))return false;
 if(!capture)return true;
 const rows=sequence(v.captures);if(!rows)return false;let prior=0;
 for(let i=0;i<rows.length;i++){
  const row=sequence(item(rows,i));if(!row||row.length!==4)return false;
  const layer=item(row,0),name=item(row,1),lo=item(row,2),hi=item(row,3);
  if(!['host-block','host-inline','child'].includes(layer)||typeof name!=='string'||Buffer.from(name,'utf8').toString('utf8')!==name||!natural(lo)||!natural(hi)||lo>=hi||lo<prior)return false;
  prior=hi;
 }
 return true;
}
