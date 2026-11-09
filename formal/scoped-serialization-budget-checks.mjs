// Private owner access exists only in generated test graphs.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import {scopedFormatFixture} from './scoped-serialization-checks.mjs';
const size=v=>Buffer.byteLength(JSON.stringify(v));
export function checkScopedFormatBudgets(api){
 let parentCases=0,planCases=0,rootCases=0;
 const seed=scopedFormatFixture(api),plan=seed.prepare();assert(plan);const owner=api.formatTestOwner(seed.domain),value=api.formatTestIntent(plan,owner.limits.maxIntentBytes,owner,seed.domain,seed.ticket).value,cap=size({kind:'settings',plan:value});
 for(const offset of [-1,0,1]){const x=scopedFormatFixture(api),o=api.formatTestOwner(x.domain),old=o.limits,before=x.root(),held=x.held(),usage=x.usage(),slot=o.slots.get(x.ticket).settingsPrepared;try{o.limits={...old,maxIntentBytes:cap+offset};const p=x.prepare();assert.equal(Boolean(p),offset>=0,'parent/'+offset);assert.equal(x.root(),before);assert.deepEqual(x.held(),held);assert.deepEqual(x.usage(),usage);assert.equal(o.slots.get(x.ticket).settingsPrepared,p??slot);if(offset<0){o.limits=old;assert(x.prepare());}}finally{o.limits=old;}parentCases++;}
 let planCap=size(value);for(let i=0;i<8;i++){const next=size({...value,profile:{...value.profile,maxPlanBytes:planCap}});if(next===planCap)break;planCap=next;}assert.equal(size({...value,profile:{...value.profile,maxPlanBytes:planCap}}),planCap);
 for(const offset of [-1,0,1]){const x=scopedFormatFixture(api,{nativeProfile:{maxPlanBytes:planCap+offset}}),before=x.root(),usage=x.usage(),held=x.held();assert.equal(Boolean(x.prepare()),offset>=0,'plan/'+offset);assert.equal(x.root(),before);assert.deepEqual(x.usage(),usage);assert.deepEqual(x.held(),held);assert(x.resolve(x.basis));planCases++;}
 for(const offset of [-1,0,1]){const x=scopedFormatFixture(api,{initialBom:true,bom:false}),o=api.formatTestOwner(x.domain),old=o.limits,before=x.root(),next=structuredClone(before),usage=x.usage(),held=x.held();next.publication++;next.documents[0].serialization={generation:2,profile:'strict-utf8-one-bom/1',bom:false};const rootCap=size(next);assert.equal(rootCap,size(before)+1);try{o.limits={...old,maxRootBytes:rootCap+offset};const p=x.prepare();assert.equal(Boolean(p),offset>=0,'root/'+offset);assert.equal(x.root(),before);assert.deepEqual(x.held(),held);assert.deepEqual(x.usage(),usage);assert(x.resolve(x.basis));if(offset<0){o.limits=old;assert(x.prepare());}}finally{o.limits=old;}rootCases++;}
 return {scope:'isolated-scoped-format-logical-budget-boundaries',parentCases,planCases,rootCases,testAccessorOnlyInGeneratedGraph:true,physicalPeakAllocationOrMeasuredCostProved:false};
}
export async function scopedFormatBudgetGraph(change=null){
 const base=new URL('./',import.meta.url),read=name=>fs.readFileSync(new URL(name,base),'utf8');
 const url=(raw,overrides={})=>'data:text/javascript;base64,'+Buffer.from(raw.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,base).href)+',import.meta.url)')).toString('base64');
 let state=read('source-serialization-state.mjs');if(change){assert.equal(state.split(change[0]).length-1,1);state=state.replace(change[0],change[1]);}
 const stateURL=url(state),editing=url(read('editing-intents.mjs'),{'./source-serialization-state.mjs':stateURL});
 const kernel=url(read('invocation-publication.mjs')+"\nexport function formatTestOwner(domain){return domains.get(domain);}\nexport {sourceSerializationIntent as formatTestIntent} from './source-serialization-state.mjs';",{'./source-serialization-state.mjs':stateURL,'./editing-intents.mjs':editing});return import(kernel);
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{
 const result=checkScopedFormatBudgets(await scopedFormatBudgetGraph()),killed=[];
 for(const from of ['not-a-source-preimage','return'])await assert.rejects(()=>scopedFormatBudgetGraph([from,'']),{code:'ERR_ASSERTION'});
 for(const [name,from,to] of [['whole-parent-bound','||size({kind:\'settings\',plan:data.value})>owner.limits.maxIntentBytes',''],['candidate-root-bound','||size(d.state)>owner.limits.maxRootBytes',''],['captured-plan-bound','Math.min(p.maxPlanBytes,owner.limits.maxIntentBytes)','owner.limits.maxIntentBytes']]){const api=await scopedFormatBudgetGraph([from,to]);let code=null;try{checkScopedFormatBudgets(api);}catch(e){code=e.code;assert.equal(code,'ERR_ASSERTION',name+': '+String(e.message).slice(0,160));}assert.equal(code,'ERR_ASSERTION',name+'/survived');killed.push(name);}
 console.log(JSON.stringify({...result,coherentBaselinePassed:true,constructionOutsideDetection:true,preimageControls:2,assertionOnlyKills:killed}));
}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
