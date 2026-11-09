// Fixed pre-query faults do not establish post-selector-publication rollback.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),classification='  const p=captureAnnotationQueryLimits(inputLimits);',query='  const annotationRequest=annotationQueryProducerRequest(annotationQuery),annotationResult=annotationPort(o,domain,handle,annotationRequest,annotationPacket.value.current,annotationPacket.value.facts);';
const code=`
import assert from 'node:assert/strict';
import * as api from './formal/invocation-publication.mjs';
import {annotationQueryGatewayFixture as setup} from './formal/annotation-query-gateway-checks.mjs';
let cases=0;for(const kind of ['annotations','combined','version-set'])for(const retained of [false,true]){
 const z=setup(kind,retained),root=z.root(),parent=api.invocationUsage(z.domain),native=z.usage(),fault=process.env.VELLUM_ANNOTATION_QUERY_PROBE.endsWith('fault'),out=z.query();
 assert.equal(out.status,fault?'unavailable':'page');assert.equal(z.root(),root);assert.deepEqual(z.usage(),native);assert.deepEqual(api.invocationUsage(z.domain),{...parent,faulted:fault});assert.deepEqual(z.decodeUsage(),{busy:false,declaredWorkspaceBytes:0});if(fault)assert.equal(z.query().status,'unavailable');cases++;
}console.log(JSON.stringify({cases}));
`;
const probes=[['baseline',query,''],['classification-fault',classification,"  throw Error('fixed annotation classification fault');\n"],['workspace-fault',query,"  throw Error('fixed annotation pre-query fault');\n"],['reentry',query,`  if(o.annotationQueryDecodeCharge!==annotationCharge||o.charge>o.limits.maxHeldBytes||!o.annotationQueryDecoding)throw Error('workspace not held');
  const nested=queryInvocationDecodedAnnotation(domain,handle,envelope,inputLimits,authorityContext,nativeCurrent,g);
  if(nested.status!=='unavailable')throw Error('reentry admitted');\n`]],results=[];
for(const [name,seam,injection]of probes){const graph=captureInputs(root),file=path.join(graph.directory,'formal/invocation-publication.mjs'),original=fs.readFileSync(file,'utf8');assert.equal(original.split(seam).length-1,1);fs.writeFileSync(file,original.replace(seam,injection+seam));const out=spawnSync(process.execPath,['--input-type=module','-e',code],{cwd:graph.directory,encoding:'utf8',env:{...process.env,VELLUM_ANNOTATION_QUERY_PROBE:name},maxBuffer:1048576});assert.equal(out.status,0,name+': '+out.stderr);results.push({name,...JSON.parse(out.stdout)});}
console.log(JSON.stringify({scope:'isolated-decoded-F03-protected-query-workspace-faults',results,coherentCopiedGraphs:true,postQueryCrashPhysicalOrIndependentProof:false}));
