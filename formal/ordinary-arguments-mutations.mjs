import assert from 'node:assert/strict';
import fs from 'node:fs';
import {checkOrdinaryArguments,checkOrdinaryArgumentBudgets} from './ordinary-arguments-checks.mjs';
const base=new URL('./',import.meta.url),source=fs.readFileSync(new URL('ordinary-edit-arguments.mjs',base),'utf8');
const mutants=[
 ['wrong-operation-tuple-admitted',"decoded?.descriptor.id!=='F11'||decoded.variant!=='ordinary-edit'",'false'],
 ['extra-outer-field-admitted','!exact(args,form.fields)','false'],
 ['profile-constant-ignored','args.profile!==form.profileConstant','false'],
 ['foreign-namespace-admitted','v.authority===c.authority','true'],
 ['copied-boundary-is-a-retry','left.boundary===right.boundary','true'],
 ['request-data-substitution-is-a-retry','equal(left.record,right.record)','true'],
 ['parent-byte-bound-ignored','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['node-bound-ignored','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputNodes:Number.MAX_SAFE_INTEGER,maxRequestBytes:'],
 ['depth-bound-ignored','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputDepth:Number.MAX_SAFE_INTEGER,maxRequestBytes:'],
 ['data-accessor-invoked',"if(keys.some(k=>typeof k!=='string'||!Object.hasOwn(descriptors[k],'value')||descriptors[k].enumerable!==true))return null;return Object.fromEntries(keys.map(k=>[k,descriptors[k].value]));","return Object.fromEntries(keys.map(k=>[k,value[k]]));"],
 ['boundary-traversed-as-json','unicode?{boundary:null}','unicode?{boundary}'],
 ['producer-boundary-lost',"normalized.kind==='boundary'?{boundary:normalized.boundary}","normalized.kind==='boundary'?{boundary:null}"],
 ['caller-data-retained','record:captured.value','record:{...captured.value,request:raw}'],
];
async function graph(m){let raw=source;if(m){assert.equal(raw.split(m[1]).length-1,1,m[0]);raw=raw.replace(m[1],m[2]);}const code=raw.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,base).href));return import('data:text/javascript;base64,'+Buffer.from(code).toString('base64'));}
try{const api=await graph(),baseline=checkOrdinaryArguments(api),budgets=checkOrdinaryArgumentBudgets(api),killed=[];await assert.rejects(()=>graph(['unknown-declaration-construction',"...selectionInputForms().filter(f=>f.family==='F11')",'']),/unsupported ordinary edit declaration source/);await assert.rejects(()=>graph(['syntax-construction','const forms=','const forms=;']),{name:'SyntaxError'});for(const m of mutants){const candidate=await graph(m);let failed=false;try{checkOrdinaryArguments(candidate);checkOrdinaryArgumentBudgets(candidate);}catch(e){assert.equal(e.code,'ERR_ASSERTION',m[0]+': '+String(e.message).slice(0,160));failed=true;}assert(failed,m[0]+'/survived');killed.push(m[0]);}console.log(JSON.stringify({scope:'isolated-ordinary-edit-argument-mutations',baseline,budgets,mutations:killed,constructionControls:2,constructionOutsideDetection:true,coherentBaselinePassed:true,publicationReplayPhysicalOrPublicCodecProved:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
