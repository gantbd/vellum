import fs from 'node:fs';
import assert from 'node:assert/strict';
import {checkReadSearch} from './read-search-checks.mjs';
import {checkReadSearchPaths} from './read-search-path-checks.mjs';
const base=new URL('./',import.meta.url);
function url(file,m=null,overrides={}){let body=fs.readFileSync(new URL(file,base),'utf8');if(m){assert.equal(body.split(m[1]).length-1,1,m[0]);body=body.replace(m[1],m[2]);}body=body.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,base).href)+',import.meta.url)');return 'data:text/javascript;base64,'+Buffer.from(body).toString('base64');}
async function graph(m=null){const child=m?.[3]==='read-contexts.mjs'?url('read-contexts.mjs',m):new URL('read-contexts.mjs',base).href;return import(url('read-search-composition.mjs',m&&m[3]!=='read-contexts.mjs'?m:null,{'./read-contexts.mjs':child}));}
const mutations=[
 ['scope-row-normalization-bypassed',"if(!Array.isArray(r.scopes)||r.scopes.some(s=>!s||typeof s!=='object'||Array.isArray(s)||Object.keys(s).length!==2||!Object.hasOwn(s,'lo')||!Object.hasOwn(s,'hi')||!Number.isSafeInteger(s.lo)||s.lo<0||!Number.isSafeInteger(s.hi)||s.hi<0))return no();","if(false)return no();"],
 ['query-grants-accepted-as-truthy','queryCurrent[k]===true','queryCurrent[k]'],
 ['captured-reply-ceiling-widened','Math.min(queryLimits.maxReplyBytes,captured.sourceProfile.maxReplyBytes)','queryLimits.maxReplyBytes'],
 ['query-current-request-refreshed','searchLiteral(captured.snapshot,r,queryCurrent,effectiveLimits)','searchLiteral(captured.snapshot,r,{...queryCurrent,request:r},effectiveLimits)'],
 ['read-scope-binding-bypassed',"if(!active(c,current)||!c.data.includes('source')","if(!c||!c.data.includes('source')",'read-contexts.mjs'],
 ['unknown-source-selector-retargeted',"const c=contextFor(m,lookup(m,token,'context'));","const c=contextFor(m,lookup(m,m.selectors.find(s=>s.kind==='context')?.token,'context'));",'read-contexts.mjs'],
 ['readonly-query-refunds-issuance','return {state:registry,result};','return {state:{...registry,issued:0},result};'],
 ['caller-input-byte-bound-ignored','copySettingsData({request,current,searchCurrent,searchLimits},bounds)','copySettingsData({request,current,searchCurrent,searchLimits},{...bounds,maxRequestBytes:Number.MAX_SAFE_INTEGER})'],
 ...['maxRequestBytes','maxInputNodes','maxInputDepth'].map(k=>['packet-'+k+'-off-by-one','copySettingsData({request,current,searchCurrent,searchLimits},bounds)',`copySettingsData({request,current,searchCurrent,searchLimits},{...bounds,${k}:bounds.${k}+1})`])
];
try{const api=await graph(),baseline=checkReadSearch(api.searchCapturedReadContext),pathBaseline=checkReadSearchPaths(api.searchCapturedReadContext),killed=[];await assert.rejects(()=>graph(['syntax-construction','const form=','const form=;']),{name:'SyntaxError'});for(const m of mutations){const mutant=await graph(m);assert.throws(()=>{checkReadSearch(mutant.searchCapturedReadContext);checkReadSearchPaths(mutant.searchCapturedReadContext);},{code:'ERR_ASSERTION'},m[0]);killed.push(m[0]);}console.log(JSON.stringify({scope:baseline.scope,baseline,pathBaseline,mutations:killed,coherentBaseline:true,constructionOutsideDetection:true,constructionControls:1,nativeIssuerLifetimePhysicalOrIndependentProof:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
