import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import {fixture,envelope,checkSelectionArguments} from './selection-arguments-checks.mjs';
import {editingInvocationContext as context} from './editing-invocation-checks.mjs';
const file=new URL('./selection-arguments.mjs',import.meta.url),source=fs.readFileSync(file,'utf8'),limits={maxRequestBytes:65536,maxInputNodes:4096,maxInputDepth:128};
async function graph(m=null){let body=source;if(m){for(const [before,after] of [[m[1],m[2]],...(m[3]?[m[3]]:[])]){assert.equal(body.split(before).length-1,1,m[0]);body=body.replace(before,after);}}body=body.replace(/from '((?:\.\/|\/)[^']+)'/g,(_,p)=>'from '+JSON.stringify(p.startsWith('/')?pathToFileURL(p).href:new URL(p,import.meta.url).href));return import('data:text/javascript;base64,'+Buffer.from(body).toString('base64'));}
function boundaries(api){const size=v=>Buffer.byteLength(JSON.stringify(v)),nodes=v=>1+(v&&typeof v==='object'?Object.values(v).reduce((s,v)=>s+nodes(v),0):0),depth=v=>v&&typeof v==='object'&&Object.keys(v).length?1+Math.max(...Object.values(v).map(depth)):0;let cases=0;for(const kind of ['set','logical','boundary','match']){const x=fixture(kind),n=api.normalizeSelectionArguments(envelope(x.request),context,limits,65536);assert(n);const request={...x.request,...(n.opaqueField?{[n.opaqueField]:null}:{})},vs=[n.record,context,envelope(request),envelope(request,'dynamic')],maxima=[Math.max(...vs.map(size)),Math.max(...vs.map(nodes)),Math.max(...vs.map(depth))];for(const route of ['typed','dynamic'])for(const offset of [-1,0,1])for(const [i,key] of ['maxRequestBytes','maxInputNodes','maxInputDepth'].entries()){const l=i?{...limits,[key]:maxima[i]+offset}:limits,maximum=i?65536:maxima[0]+offset;assert.equal(Boolean(api.normalizeSelectionArguments(envelope(x.request,route),context,l,maximum)),offset>=0,kind+'/'+key+'/'+offset);cases++;}}return cases;}
const mutations=[
 ['operation-tuple-validation-bypassed',"decoded?.descriptor.id!=='F12'",'false',['decoded.variant!==form.variant','false']],
 ['wrong-variant-admitted','decoded.variant!==form.variant','false'],
 ['extra-request-fields-admitted','!exact(args,form.fields)','false'],
 ['profile-constant-ignored','args.profile!==form.profileConstant','false'],
 ['copied-evidence-is-a-retry','a.opaque===b.opaque','true'],
 ['changed-data-is-a-retry','equal(a.record,b.record)','true'],
 ['parent-byte-ceiling-ignored','Math.min(maximum,limits.maxRequestBytes)','limits.maxRequestBytes'],
 ['node-bound-ignored','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputNodes:Number.MAX_SAFE_INTEGER,maxRequestBytes:'],
 ['depth-bound-ignored','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputDepth:Number.MAX_SAFE_INTEGER,maxRequestBytes:'],
 ['producer-opaque-binding-lost','{[n.opaqueField]:n.opaque}','{[n.opaqueField]:null}'],
 ['caller-data-retained','record:capture.value','record:{...capture.value,request:raw}']
];
try{const api=await graph(),baseline=checkSelectionArguments(api),boundaryCases=boundaries(api),killed=[];await assert.rejects(()=>graph(['syntax-construction','const forms=','const forms=;']),{name:'SyntaxError'});await assert.rejects(()=>graph(['unknown-declaration-construction','...selectionInputForms(),','']),/unsupported selection declaration source/);for(const m of mutations){const candidate=await graph(m);assert.throws(()=>{checkSelectionArguments(candidate);boundaries(candidate);},{code:'ERR_ASSERTION'},m[0]);killed.push(m[0]);}console.log(JSON.stringify({scope:baseline.scope,baseline,boundaryCases,mutations:killed,constructionOutsideDetection:true,constructionControls:2,invocationReplayOrPublicCodecProved:false}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
