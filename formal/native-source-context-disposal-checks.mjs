import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeRetainedLifecycleFixture} from './native-retained-lifecycle-checks.mjs';
import {nativeSourceContextProfile as profile,nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
import {captureInputs} from './evidence-harness.mjs';
export function checkSourceContextDisposal(){
 const z=nativeRetainedLifecycleFixture(),domain=z.a.x.z.domain,document=z.root().documents[0].document,scope={document,recipient:'reader',purpose:'query',destination:'client',policy:'policy-1',continuity:'c1',profile:'source-read'};
 const before=api.invocationUsage(domain).declaredHeldBytes,handle=api.openInvocationSourceContexts(domain,profile,facts);assert(handle);
 const acquired=api.acquireInvocationSourceContext(domain,handle,{contract:'read-context/1',operation:'snapshot-handoff',document,selection:{kind:'capture-current'},data:['source'],profile:scope.profile},scope,facts);assert.equal(acquired.status,'context');
 const hold=api.holdInvocationSourceContext(domain,handle,acquired.context,facts);assert(hold);
 const request={contract:'source-query/1',operation:'bounded-range',snapshot:acquired.context,document,profile:scope.profile,lo:0,hi:z.root().documents[0].source.length};assert.equal(api.queryInvocationSourceContext(domain,handle,request,scope,facts).status,'content');
 assert.equal(z.run('dispose').status,'applied');assert.equal(z.root().documents[0].live,false);const disposed=z.root(),charged=api.invocationSourceContextUsage(domain);
 // Disposal fences new retention before any query-induced private invalidation.
 assert.equal(charged.invalidated,0);assert.equal(api.holdInvocationSourceContext(domain,handle,acquired.context,facts),null);
 assert.deepEqual(api.queryInvocationSourceContext(domain,handle,request,scope,facts),{status:'unavailable'});
 const invalid=api.invocationSourceContextUsage(domain);assert.equal(invalid.invalidated,1);assert.equal(invalid.retainedRoots,1);assert.equal(invalid.holds,1);assert.equal(invalid.declaredHeldBytes,charged.declaredHeldBytes);assert.equal(invalid.issued,charged.issued);
 assert.equal(api.retireInvocationSourceContext(domain,handle,acquired.context,facts),false);assert.equal(api.releaseInvocationSourceContextHold(domain,handle,hold,facts),true);assert.equal(api.invocationSourceContextUsage(domain).retainedRoots,0);assert.equal(api.retireInvocationSourceContext(domain,handle,acquired.context,facts),true);assert.equal(api.retireInvocationSourceContexts(domain,handle,facts),true);assert.equal(api.invocationUsage(domain).declaredHeldBytes,before);assert.equal(z.root(),disposed);
 return {scope:'isolated-native-source-context-disposal-composition',nativeCases:1,actualLifecycleAndBoundSourceChecks:true,eagerBulkInvalidationOrPhysicalReclamationProved:false};
}
async function mutations(){
 const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/native-source-contexts.mjs',original=fs.readFileSync(path.join(root,relative),'utf8');
 const rules=[['disposed-owner-can-deliver','||!rootLive(owner,s.document)',''],['disposed-owner-can-acquire-new-hold','||!rootLive(owner,ctx.scope.document)','']],killed=[];
 const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/native-source-context-disposal-checks.mjs'),'--baseline-only'],{encoding:'utf8',env:process.env,maxBuffer:1048576});
 const positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr.slice(0,400));
 for(const [name,from,to]of rules){const a=name==='disposed-owner-can-deliver'?original.indexOf('export function queryNativeSourceContext('):0,b=name==='disposed-owner-can-deliver'?original.indexOf('export function stopNativeSourceContext(',a):original.length;assert(a>=0&&b>a);const region=original.slice(a,b);assert.equal(region.split(from).length-1,1);const graph=captureInputs(root);fs.writeFileSync(path.join(graph.directory,relative),original.slice(0,a)+region.replace(from,to)+original.slice(b));const r=run(graph.directory);assert.equal(r.status,1,name);assert(r.stderr.includes('ERR_ASSERTION'),name);assert(!r.stderr.includes('SyntaxError'),name);killed.push(name);}
 return {coherentCopiedGraphBaseline:true,mutations:killed,constructionErrorsCounted:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkSourceContextDisposal(),...(process.argv.includes('--baseline-only')?{}:await mutations())}));
