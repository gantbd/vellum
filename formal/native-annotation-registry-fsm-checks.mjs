import assert from 'node:assert/strict';
import {parentFundedPoolAttachmentBytes} from './owner-attachment-test-helpers.mjs';
import {pathToFileURL} from 'node:url';
import * as api from './invocation-publication.mjs';
import {nativeAnnotationQueryFixture as fixture} from './native-annotation-query-checks.mjs';
import {nativeAnnotationInstallationFacts as facts} from './native-annotation-installation-checks.mjs';
const actions=['page','next','inspect','source','lose','stop','retire','retire-pool'];
export function checkNativeAnnotationRegistryFSM(horizon=4){let traces=0,edges=0;
 function run(trace){const z=fixture({},undefined,true),reservation=z.usage().declaredHeldBytes,attachment=parentFundedPoolAttachmentBytes(api,z.x.z.domain,z.handle),parent=api.invocationUsage(z.x.z.domain).declaredHeldBytes,root=z.x.z.current(),m={pool:true,exists:true,live:true,first:false,second:false,selectors:1,issued:1};let first=null;
  for(const action of trace){const available=m.pool&&m.exists&&m.live;
   if(action==='page'){const out=z.query();assert.equal(out.status,available?'page':'unavailable');if(available){if(!m.first){m.first=true;m.selectors+=3;m.issued+=3;first=out;}else assert.deepEqual(out,first);}}
   else if(action==='next'){const out=z.query(z.request('enumerate',{continuation:first?.continuation??{kind:'unknown'}}));assert.equal(out.status,available&&m.first?'page':'unavailable');if(available&&m.first){assert.equal(out.records.length,1);assert.equal(out.continuation,null);if(!m.second){m.second=true;m.selectors++;m.issued++;}}}
   else if(action==='inspect'){const out=z.query(z.request('inspect',{annotation:first?.records[0].reference??{kind:'unknown'}}));assert.equal(out.status,available&&m.first?'record':'unavailable');}
   else if(action==='source'){const out=api.queryInvocationCombinedSource(z.x.z.domain,z.handle,{contract:'source-query/1',operation:'bounded-range',document:z.x.document,snapshot:z.context,profile:z.x.scope.profile,lo:0,hi:1},z.x.scope,facts);assert.equal(out.status,available?'content':'unavailable');}
   else if(action==='lose'){assert.equal(z.query(z.request(),z.x.scope,{...facts,projectionCurrent:false}).status,'unavailable');if(available)m.live=false;}
   else if(action==='stop'){const out=api.stopInvocationAnnotationContext(z.x.z.domain,z.handle,{contract:'read-context/1',operation:'stop-observation',context:z.context},z.x.scope,{ownerControlAllowed:true});assert.equal(out.status,m.pool&&m.exists?'stopped':'unavailable');if(m.pool&&m.exists)m.live=false;}
   else if(action==='retire'){const accept=m.pool&&m.exists&&!m.live;assert.equal(api.retireInvocationAnnotationContext(z.x.z.domain,z.handle,z.context,facts),accept);if(accept){m.exists=false;m.selectors=0;}}
   else{const accept=m.pool&&!m.exists;assert.equal(api.retireInvocationAnnotationContexts(z.x.z.domain,z.handle,facts),accept);if(accept)m.pool=false;}
   const actual=z.usage();assert.equal(actual.closed,!m.pool);assert.equal(actual.contexts,m.exists?1:0);assert.equal(actual.selectors,m.selectors);assert.equal(actual.stopped,m.exists&&!m.live?1:0);assert.equal(actual.issued,m.issued);assert.equal(actual.declaredHeldBytes,m.pool?reservation:0);assert.equal(api.invocationUsage(z.x.z.domain).declaredHeldBytes,parent-(m.pool?0:reservation+attachment));assert.equal(z.x.z.current(),root);edges++;
  }traces++;
 }
 function visit(prefix){run(prefix);if(prefix.length<horizon)for(const action of actions)visit([...prefix,action]);}visit([]);
 return {scope:'isolated-native-annotation-registry-bounded-traces',horizon,actions:actions.length,traces,edges,actualRegistryAndParentCompared:true,unboundedConcurrentPhysicalOrAllProfilesProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeAnnotationRegistryFSM()));
