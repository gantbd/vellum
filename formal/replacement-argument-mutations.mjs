import assert from 'node:assert/strict';
import fs from 'node:fs';
import {checkReplacementArguments,checkReplacementArgumentBoundaries} from './replacement-argument-checks.mjs';
const source=fs.readFileSync(new URL('./replacement-arguments.mjs',import.meta.url),'utf8');
async function graph(m=null){let body=source;if(m)for(const [before,after] of [[m[1],m[2]],...(m[3]?[m[3]]:[])]){assert.equal(body.split(before).length-1,1,m[0]);body=body.replace(before,after);}body=body.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(new URL(p,import.meta.url).href));return import('data:text/javascript;base64,'+Buffer.from(body).toString('base64'));}
const mutations=[
 ['operation-tuple-bypassed',"decoded?.descriptor.id!=='F11'",'false',['decoded.variant!==form.variant','false']],
 ['variant-substitution-accepted','decoded.variant!==form.variant','false'],
 ['extra-request-fields-accepted','!exact(args,form.fields)','false'],
 ['replace-current-index-unbounded','!natural(r.index)','false'],
 ['replace-all-index-accepted','r.index!==null','false'],
 ['copied-result-is-a-retry','a.result===b.result','true'],
 ['changed-request-is-a-retry','equal(a.record,b.record)','true'],
 ['parent-byte-bound-ignored','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['node-bound-ignored','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputNodes:Number.MAX_SAFE_INTEGER,maxRequestBytes:'],
 ['depth-bound-ignored','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputDepth:Number.MAX_SAFE_INTEGER,maxRequestBytes:'],
 ['result-identity-copied','record:capture.value,result,route:e.route','record:capture.value,result:{...result},route:e.route'],
 ['caller-data-retained','record:capture.value','record:{...capture.value,request:raw}']
];
try{const api=await graph(),baseline=checkReplacementArguments(api),boundaries=checkReplacementArgumentBoundaries(api),killed=[];await assert.rejects(()=>graph(['syntax-construction','const forms=','const forms=;']),{name:'SyntaxError'});await assert.rejects(()=>graph(['unknown-declaration-construction','const forms=replacementInputForms(),known=','const forms=[],known=']),/unsupported replacement declaration source/);for(const m of mutations){const candidate=await graph(m);assert.throws(()=>{checkReplacementArguments(candidate);checkReplacementArgumentBoundaries(candidate);},{code:'ERR_ASSERTION'},m[0]);killed.push(m[0]);}console.log(JSON.stringify({scope:baseline.scope,baseline,boundaries,mutations:killed,constructionControls:2,constructionOutsideDetection:true,coherentBaseline:true,retentionReplayOrPublicWireProved:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
