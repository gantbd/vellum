import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {captureSearchCurrentInputs as capture} from './search-arguments.mjs';
import {nativeSourceSearchFixture as fixture,nativeSourceSearchCurrent as current} from './native-source-search-checks.mjs';
import {nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
const limits={maxRequestBytes:8192,maxInputNodes:4096,maxInputDepth:128},nodes=v=>1+(v&&typeof v==='object'?Object.values(v).reduce((n,x)=>n+nodes(x),0):0),depth=v=>v&&typeof v==='object'&&Object.keys(v).length?1+Math.max(...Object.values(v).map(depth)):0;
export function checkSearchCurrentInputs(){const z=fixture(),r=z.request(),q=current(r),shadow={sourceContext:z.x.c,current:{...q,request:{...q.request,context:null}},facts};let boundaries=0,denials=0;
 const baseline=capture(z.x.c,q,facts,limits,limits.maxRequestBytes);assert(baseline);assert.equal(baseline.current.request.context,r.context);assert.deepEqual(baseline.current,q);
 for(const [field,min]of [['maxRequestBytes',Buffer.byteLength(JSON.stringify(shadow))],['maxInputNodes',nodes(shadow)],['maxInputDepth',depth(shadow)]])for(const delta of [-1,0,1]){assert.equal(capture(z.x.c,q,facts,{...limits,[field]:min+delta},8192)!==null,delta>=0,field);boundaries++;}
 const bytes=Buffer.byteLength(JSON.stringify(shadow));for(const delta of [-1,0,1]){assert.equal(capture(z.x.c,q,facts,limits,bytes+delta)!==null,delta>=0);boundaries++;}
 for(const bad of [null,{},[],{...q,extra:true},{...q,request:{...q.request,extra:true}},{...q,request:{...q.request,context:null}}]){assert.equal(capture(z.x.c,bad,facts,limits,8192),null);denials++;}
 const cyclic={...facts};cyclic.cycle=cyclic;assert.equal(capture(z.x.c,q,cyclic,limits,8192),null);denials++;
 const changed={...q,capacity:false,request:{...q.request,query:'other',context:{}}},preserved=capture(z.x.c,changed,{...facts,resourcesReady:false},limits,8192);assert(preserved);assert.equal(preserved.current.capacity,false);assert.equal(preserved.current.request.query,'other');assert.equal(preserved.current.request.context,changed.request.context);assert.equal(preserved.facts.resourcesReady,false);assert.notEqual(preserved.current.request.context,r.context);
 const s={...z.x.c},g={...facts},cq=current(r),out=capture(s,cq,g,limits,8192);assert(out);s.recipient='other';g.sourceReadable=false;cq.request.query='other';assert.equal(out.sourceContext.recipient,'reader');assert.equal(out.facts.sourceReadable,true);assert.equal(out.current.request.query,'a');assert(Object.isFrozen(out.facts));
 return {scope:'isolated-search-independent-current-packet-capture',boundaries,denials,opaqueIdentityAndCurrentFactsPreserved:true,nativeAuthorityOrPhysicalAccountingProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkSearchCurrentInputs()));
