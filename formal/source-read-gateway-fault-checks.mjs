// Fixed pre-query classification/workspace faults and reentry in copied graphs.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),classification='  const p=captureSourceReadArgumentLimits(inputLimits);',query='  const readRequest=sourceReadProducerRequest(n),readResult=port(o,domain,handle,readRequest,packet.value.current,packet.value.facts);';
const code=`
import assert from 'node:assert/strict';
import * as api from './formal/invocation-publication.mjs';
import {sourceReadGatewayFixture as setup} from './formal/source-read-gateway-checks.mjs';
const z=setup(),root=z.root(),parent=api.invocationUsage(z.domain),native=z.usage(),out=z.query(),fault=process.env.VELLUM_READ_PROBE.endsWith('fault');
assert.equal(out.status,fault?'unavailable':'content');assert.equal(z.root(),root);
assert.deepEqual(api.invocationUsage(z.domain),{...parent,faulted:fault});assert.deepEqual(z.usage(),native);assert.deepEqual(z.decodeUsage(),{busy:false,declaredWorkspaceBytes:0});
if(fault)assert.equal(z.query().status,'unavailable');
`;
const probes=[['baseline',query,''],['classification-fault',classification,"  throw Error('fixed classification fault');\n"],['workspace-fault',query,"  throw Error('fixed pre-query fault');\n"],['reentry',query,`  if(o.sourceReadDecodeCharge!==charged||o.charge>o.limits.maxHeldBytes||!o.sourceReadDecoding)throw Error('workspace not held');
  const nested=queryInvocationDecodedSourceRead(domain,handle,envelope,inputLimits,authorityContext,nativeCurrent,g);
  if(nested.status!=='unavailable')throw Error('reentry admitted');\n`]],results=[];
for(const [name,seam,injection]of probes){const graph=captureInputs(root),file=path.join(graph.directory,'formal/invocation-publication.mjs'),original=fs.readFileSync(file,'utf8');assert.equal(original.split(seam).length-1,1);fs.writeFileSync(file,original.replace(seam,injection+seam));const out=spawnSync(process.execPath,['--input-type=module','-e',code],{cwd:graph.directory,encoding:'utf8',env:{...process.env,VELLUM_READ_PROBE:name},maxBuffer:1048576});assert.equal(out.status,0,name+': '+out.stderr);results.push(name);}
console.log(JSON.stringify({scope:'isolated-decoded-F02-protected-workspace-faults',results,coherentCopiedGraphs:true,postQueryCrashPhysicalOrIndependentProof:false}));
