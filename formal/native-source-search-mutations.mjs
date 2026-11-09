import assert from 'node:assert/strict';
import fs from 'node:fs';
import {checkNativeSourceSearch} from './native-source-search-checks.mjs';
import {checkNativeSourceSearchFSM} from './native-source-search-fsm-checks.mjs';
import {checkNativeSourceSearchBoundaries} from './native-source-search-boundaries.mjs';
const base=new URL('./',import.meta.url);
function url(file,rules=[],overrides={}){
 let s=fs.readFileSync(new URL(file,base),'utf8');for(const [from,to]of rules){assert.equal(s.split(from).length-1,1,from);s=s.replace(from,to);}
 s=s.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,base).href)+',import.meta.url)');return 'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
}
async function graph(m=null){
 const source=url('native-source-contexts.mjs',m?.source??[]),search=url('native-source-search.mjs',m?.search??[],{'./native-source-contexts.mjs':source});
 return import(url('invocation-publication.mjs',[],{'./native-source-contexts.mjs':source,'./native-source-search.mjs':search}));
}
const mutations=[
 ['parent-charge-omitted','owner.charge+=charge;','void charge;'],
 ['actual-parent-headroom-ignored','charge>owner.limits.maxHeldBytes-owner.charge','false'],
 ['truthy-query-facts-accepted','ks.every(k=>g?.[k]===true)','ks.every(k=>g?.[k])'],
 ['query-authority-not-checked-before-retention','!yes(v.current,flags)','false'],
 ['query-loss-not-absorbed','if(loss.some(k=>v.current[k]===false))held.stopped=true;','void held;'],
 ['unknown-query-facts-invalidate','loss.some(k=>v.current[k]===false)','loss.some(k=>v.current[k]!==true)'],
 ['current-context-compared-by-shape','q.request.context!==r.context','!equal(q.request.context,r.context)'],
 ['changed-current-request-accepted','!equal(a,v.current.request)','false'],
 ['foreign-scope-can-poison-result','!equal(s,held.scope)','false'],
 ['query-capacity-masks-source-loss','const source=inspectLease','const source=yes(v.current,flags)&&inspectLease'],
 ['unpublished-source-hold-leaked','if(!published)issuers[issuer.kind].discard','if(false)issuers[issuer.kind].discard'],
 ['pool-retires-with-live-result','m.results.size||!yes(g,discharge)','false||!yes(g,discharge)'],
 ['retirement-refunds-query-issuance','m.results.delete(result);return true;','m.results.delete(result);m.issued=0;return true;'],
 ['retired-pool-can-reopen','pools.has(owner)','pools.get(owner)&&!pools.get(owner).closed'],
 ['pool-charge-not-released','owner.charge-=m.charge;','void m.charge;'],
 ['retirement-keeps-result-row','m.results.delete(result);return true;','void result;return true;'],
 ['current-root-substitutes-retained-source','searchLiteral(captured.snapshot,v.request,v.current,limits)','searchLiteral({...captured.snapshot,source:owner.root.documents[0].source},v.request,v.current,limits)'],
 ['result-looked-up-by-public-shape','held=m?.results.get(result);if(!held||held.stopped','held=[...(m?.results??[])].find(([key])=>equal(key,result))?.[1];if(!held||held.stopped']
].map(([name,from,to])=>({name,search:[[from,to]]}));
mutations.push(
 {name:'stopped-source-can-release-child',source:[["return bound&&bound.ctx===held.ctx&&bound.m.holds.get(lease)===held.reference?{snapshot:held.ctx.snapshot,sourceProfile:bound.m.profile.sourceProfile}:null;","return held.ctx.snapshot?{snapshot:held.ctx.snapshot,sourceProfile:pool(owner,domain,handle).profile.sourceProfile}:null;"]]},
 {name:'published-lease-not-marked',source:[['h.published=true;return true;','return true;']]},
 {name:'all-protected-discharge-gates-bypassed',search:[['if(!yes(facts,discharge)||m.results.get(result)!==held||!issuers','if(false||m.results.get(result)!==held||!issuers'],['discardLiteralSearchResult(result,facts)','discardLiteralSearchResult(result,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true})']],source:[['published&&!yes(g,discharge)','false']]}
);
try{
 const positive=await graph(),baseline=checkNativeSourceSearch(positive),boundaries=checkNativeSourceSearchBoundaries(positive),fsm=checkNativeSourceSearchFSM(positive),killed=[],fsmKilled=[];
 await assert.rejects(()=>graph({search:[['const catalog=','const catalog=;']]}),{name:'SyntaxError'});
 for(const m of mutations){const api=await graph(m);assert.throws(()=>checkNativeSourceSearch(api),{code:'ERR_ASSERTION'},m.name);killed.push(m.name);}
 for(const name of ['query-loss-not-absorbed','stopped-source-can-release-child','pool-charge-not-released']){const m=mutations.find(m=>m.name===name),api=await graph(m);assert.throws(()=>checkNativeSourceSearchFSM(api),{code:'ERR_ASSERTION'},name);fsmKilled.push(name);}
 console.log(JSON.stringify({baseline,boundaries,fsm,mutations:killed,fsmMutations:fsmKilled,coherentSourceLeaseKernelBaseline:true,constructionOutsideDetection:true,constructionControls:1,asynchronousDispatchF11F12OrIndependentProof:false}));
}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
