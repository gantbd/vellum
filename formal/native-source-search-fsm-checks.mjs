import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
import {nativeSourceSearchFixture as fixture,nativeSourceSearchCurrent as current} from './native-source-search-checks.mjs';
const actions=['query','release','lose-query','stop-source','retire-result','retire-pool','deny-cleanup'];
export function checkNativeSourceSearchFSM(api=kernel,horizon=4){
 let traces=0,edges=0;
 function run(trace){const z=fixture(api),r=z.request(),q=current(r),reservation=z.usage().declaredHeldBytes,attachment=parentFundedPoolAttachmentBytes(api,z.x.z.domain,z.handle),m={closed:false,held:false,lost:false,stopped:false,issued:0};let result=null;
  for(const action of trace){
   if(action==='query'){
    // One selected result slot; do not replace an outstanding observer handle.
    if(!m.held){const out=z.query(r,q),accept=!m.closed&&!m.stopped;assert.equal(out.status,accept?'complete':'unavailable');if(accept){result=out;m.held=true;m.lost=false;m.issued++;}}
   }else if(action==='release'||action==='lose-query'){
    const out=z.release(result,action==='lose-query'?{...q,grant:false}:q),accept=m.held&&!m.closed&&!m.stopped&&!m.lost&&action==='release';assert.equal(out.status,accept?'complete':'unavailable');if(m.held&&!m.closed&&action==='lose-query')m.lost=true;
   }else if(action==='stop-source'){
    if(!m.stopped){assert.equal(z.x.stop(z.reference,z.x.c,{ownerControlAllowed:true}).status,'stopped');m.stopped=true;}
   }else if(action==='retire-result'){
    assert.equal(z.retire(result),m.held&&!m.closed);if(m.held&&!m.closed)m.held=false;
   }else if(action==='retire-pool'){
    const accept=!m.closed&&!m.held;assert.equal(api.retireInvocationSourceSearch(z.x.z.domain,z.handle,facts),accept);if(accept)m.closed=true;
   }else{
    assert.equal(z.retire(result,{...facts,retentionReleased:false}),false);
   }
   const s=z.usage(),source=z.x.usage();assert.equal(s.closed,m.closed);assert.equal(s.results,m.held?1:0);assert.equal(s.issued,m.issued);assert.equal(source.holds,m.held?1:0);assert.equal(source.issued,1+m.issued);assert.equal(source.retainedRoots,m.stopped&&!m.held?0:1);assert.equal(s.declaredHeldBytes,m.closed?0:reservation);assert.equal(z.x.z.usage().declaredHeldBytes,z.before.declaredHeldBytes+(m.closed?0:reservation+attachment));edges++;
  }traces++;
 }
 function visit(prefix){run(prefix);if(prefix.length<horizon)for(const a of actions)visit([...prefix,a]);}visit([]);
 return {scope:'isolated-native-search-bounded-lifetime-traces',actions,horizon,traces,edges,unboundedInductionConcurrencyPhysicalF11F12OrAsyncProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeSourceSearchFSM()));
