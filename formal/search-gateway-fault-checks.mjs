// Fixed pre-query fault and reentry probes in coherent copied graphs only.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),seam='  return queryNativeSourceSearch(o,domain,handle,searchProducerRequest(n),captured.sourceContext,captured.current,captured.facts);';
const code=`
import assert from 'node:assert/strict';
import * as api from './formal/invocation-publication.mjs';
import {nativeSourceSearchFixture as fixture,nativeSourceSearchCurrent as current} from './formal/native-source-search-checks.mjs';
import {nativeSourceContextFacts as facts} from './formal/native-source-context-checks.mjs';
const z=fixture(),domain=z.x.z.domain,r=z.request(),context=api.invocationAuthorityContext(domain),envelope={route:'typed',request:{family:4,variant:r.operation,version:1,arguments:r}},p={maxRequestBytes:4096,maxInputNodes:4096,maxInputDepth:128},before=z.x.z.usage(),source=z.x.usage();
const out=api.queryInvocationDecodedSearch(domain,z.handle,envelope,p,context,z.x.c,current(r),facts);
assert.equal(out.status,process.env.PROBE==='fault'?'unavailable':'complete');
assert.equal(z.x.z.usage().declaredHeldBytes,before.declaredHeldBytes);
assert.equal(z.x.z.usage().faulted,process.env.PROBE==='fault');
assert.deepEqual(api.invocationSearchDecodeUsage(domain),{busy:false,declaredWorkspaceBytes:0});
if(process.env.PROBE==='fault'){assert.deepEqual(z.x.usage(),source);assert.equal(z.usage().issued,0);assert.equal(z.usage().results,0);assert.equal(z.query().status,'unavailable');}else{assert.equal(z.usage().issued,1);assert.equal(z.retire(out),true);}
`;
const probes=[
 ['baseline',''],
 ['fault',`  throw Error('fixed pre-query fault');\n`],
 ['reentry',`  if(o.searchDecodeCharge!==charged||o.charge>o.limits.maxHeldBytes||!o.searchDecoding)throw Error('workspace not held');
  const nested=queryInvocationDecodedSearch(domain,handle,envelope,inputLimits,authorityContext,sourceContext,currentQuery,g);
  if(nested.status!=='unavailable')throw Error('reentry admitted');\n`]
],results=[];
for(const [name,injection]of probes){const graph=captureInputs(root),file=path.join(graph.directory,'formal/invocation-publication.mjs'),original=fs.readFileSync(file,'utf8');assert.equal(original.split(seam).length-1,1);fs.writeFileSync(file,original.replace(seam,injection+seam));const out=spawnSync(process.execPath,['--input-type=module','-e',code],{cwd:graph.directory,encoding:'utf8',env:{...process.env,PROBE:name},maxBuffer:1048576});assert.equal(out.status,0,out.stderr);results.push(name);}
console.log(JSON.stringify({scope:'isolated-pre-query-F05-workspace-protection',results,coherentCopiedGraphs:true,postQueryCrashOrPhysicalAccountingProved:false}));
