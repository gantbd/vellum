import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import * as kernel from './invocation-publication.mjs';
import {nativeSourceContextFixture as fixture,nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
const events=['hold','stop','loss','unknown','release','retire','capture','close-pool'];
export function checkSourceContextFSM(api=kernel,horizon=4){
 let traces=0,edges=0;
 function run(trace){
  const x=fixture(api,{maxContexts:1,maxHolds:1,maxIssued:3}),root=x.z.root(),fixed=x.usage().declaredHeldBytes,attachment=parentFundedPoolAttachmentBytes(api,x.z.domain,x.handle);
  let context=x.acquire().context,exists=true,active=true,invalidated=false,held=null,issued=1,closed=false;
  assert(context);const oldContexts=[],oldHolds=[];
  for(const event of trace){
   if(event==='hold'){const out=api.holdInvocationSourceContext(x.z.domain,x.handle,context,facts),ok=exists&&active&&!closed&&held===null&&issued<3;assert.equal(out!==null,ok);if(ok){held=out;issued++;}}
   if(event==='stop'){const out=x.stop(context,x.c,{ownerControlAllowed:true});assert.equal(out.status,exists&&!closed?'stopped':'unavailable');if(exists&&!closed)active=false;}
   if(event==='loss'){assert.deepEqual(x.query(context,{},x.c,{...facts,scope:false}),{status:'unavailable'});if(exists&&active&&!closed){active=false;invalidated=true;}}
   if(event==='unknown')assert.deepEqual(x.query(context,{},x.c,{...facts,scope:undefined}),{status:'unavailable'});
   if(event==='release'){const out=api.releaseInvocationSourceContextHold(x.z.domain,x.handle,held,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true});assert.equal(out,held!==null);if(held!==null){oldHolds.push(held);held=null;}}
   if(event==='retire'){const out=api.retireInvocationSourceContext(x.z.domain,x.handle,context,facts),ok=exists&&!active&&held===null&&!closed;assert.equal(out,ok);if(ok){exists=false;invalidated=false;oldContexts.push(context);}}
   if(event==='capture'){const out=x.acquire(),ok=!exists&&!closed&&issued<3;assert.equal(out.status,ok?'context':'unavailable');if(ok){assert(!oldContexts.includes(out.context));context=out.context;exists=true;active=true;invalidated=false;issued++;}}
   if(event==='close-pool'){const out=api.retireInvocationSourceContexts(x.z.domain,x.handle,facts),ok=!exists&&held===null&&!closed;assert.equal(out,ok);if(ok)closed=true;}
   const u=x.usage();assert.equal(u.contexts,exists?1:0);assert.equal(u.holds,held!==null?1:0);assert.equal(u.retainedRoots,exists&&(active||held!==null)?1:0);assert.equal(u.invalidated,invalidated?1:0);assert.equal(u.issued,issued);assert.equal(u.closed,closed);assert.equal(u.declaredHeldBytes,closed?0:fixed);assert.equal(x.z.usage().declaredHeldBytes,x.prior.declaredHeldBytes+(closed?0:fixed+attachment));assert.equal(x.z.root(),root);
   for(const old of oldContexts)assert.deepEqual(x.query(old),{status:'unavailable'});
   for(const old of oldHolds)assert.equal(api.releaseInvocationSourceContextHold(x.z.domain,x.handle,old,facts),false);
   if(closed)assert.equal(api.openInvocationSourceContexts(x.z.domain,x.p,facts),null);
   edges++;
  }
  traces++;if(trace.length<horizon)for(const event of events)run([...trace,event]);
 }
 run([]);return {scope:'isolated-native-source-context-finite-lifetime-traces',horizon,events:events.length,traces,edges,oracleUsesActualNativeRootAndCharges:true,unboundedOrConcurrentCompletenessProved:false};
}
const base=new URL('./',import.meta.url);
function url(file,mutation=null,overrides={}){let body=fs.readFileSync(new URL(file,base),'utf8');if(mutation){assert.equal(body.split(mutation[1]).length-1,1);body=body.replace(mutation[1],mutation[2]);}body=body.replace(/from '(\.\/[^']+)'/g,(_,p)=>'from '+JSON.stringify(overrides[p]??new URL(p,base).href)).replace(/new URL\('(\.\/[^']+)',import\.meta\.url\)/g,(_,p)=>'new URL('+JSON.stringify(new URL(p,base).href)+',import.meta.url)');return 'data:text/javascript;base64,'+Buffer.from(body).toString('base64');}
async function graph(m=null){return import(url('invocation-publication.mjs',null,{'./native-source-contexts.mjs':url('native-source-contexts.mjs',m)}));}
async function mutations(){
 const positive=checkSourceContextFSM(await graph(),2),rules=[['active-row-retired','!ctx?.stopped||ctx.holds!==0','false||ctx.holds!==0'],['release-refunds-pool-while-live','m.holds.delete(ticket);','owner.charge-=m.charge;m.holds.delete(ticket);'],['stop-drops-held-root','if(ctx.holds===0)ctx.snapshot=null;','ctx.snapshot=null;']],killed=[];
 assert.equal(positive.traces,73);
 for(const m of rules){const api=await graph(m);assert.throws(()=>checkSourceContextFSM(api,3),{code:'ERR_ASSERTION'},m[0]);killed.push(m[0]);}
 return {coherentKernelBaseline:true,mutations:killed,constructionErrorsCounted:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify({...checkSourceContextFSM(),...await mutations()}));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
