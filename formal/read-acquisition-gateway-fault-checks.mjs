// Fixed faults precede native handoff. No post-install rollback is inferred.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),classification='  const p=captureReadAcquisitionLimits(inputLimits);',handoff="  if(source){if(request.contract!=='read-context/1'||prepared.kind!=='none')return unavailable();";
const code=`
import assert from 'node:assert/strict';
import * as api from './formal/invocation-publication.mjs';
import {readAcquisitionGatewayFixture as setup} from './formal/read-acquisition-gateway-checks.mjs';
let cases=0;
for(const kind of ['source','annotations','combined','version-set','narrow-source'])for(const retained of kind==='narrow-source'?[true]:[false,true]){
 const z=setup(kind,retained),root=z.root(),parent=api.invocationUsage(z.domain),native=z.usage(),prepared=z.preparedUsage(),fault=process.env.VELLUM_ACQUISITION_PROBE.endsWith('fault'),out=z.acquire();
 assert.equal(out.status,fault?'unavailable':kind==='version-set'?'version-set':'context');assert.equal(z.root(),root);
 assert.deepEqual(api.invocationUsage(z.domain),{...parent,faulted:fault});assert.deepEqual(z.decodeUsage(),{busy:false,declaredWorkspaceBytes:0});
 if(fault){assert.deepEqual(z.usage(),native);assert.deepEqual(z.preparedUsage(),prepared);assert.equal(z.acquire().status,'unavailable');}
 else{assert.equal(z.usage().issued,native.issued+(kind==='version-set'?3:1));if(z.preparation)assert.equal(z.preparedUsage().prepared,prepared.prepared-1);else assert.deepEqual(z.preparedUsage(),prepared);}
 cases++;
}
console.log(JSON.stringify({cases}));
`;
const probes=[['baseline',handoff,''],['classification-fault',classification,"  throw Error('fixed classification fault');\n"],['workspace-fault',handoff,"  throw Error('fixed pre-handoff fault');\n"],['reentry',handoff,`  if(o.readAcquisitionDecodeCharge!==acquisitionCharge||o.charge>o.limits.maxHeldBytes||!o.readAcquisitionDecoding)throw Error('workspace not held');
  const nested=acquireInvocationDecodedReadContext(domain,handle,envelope,inputLimits,authorityContext,nativeCurrent,g,preparation);
  if(nested.status!=='unavailable')throw Error('reentry admitted');\n`]],results=[];
for(const [name,seam,injection]of probes){const graph=captureInputs(root),file=path.join(graph.directory,'formal/invocation-publication.mjs'),original=fs.readFileSync(file,'utf8');assert.equal(original.split(seam).length-1,1);fs.writeFileSync(file,original.replace(seam,injection+seam));const out=spawnSync(process.execPath,['--input-type=module','-e',code],{cwd:graph.directory,encoding:'utf8',env:{...process.env,VELLUM_ACQUISITION_PROBE:name},maxBuffer:1048576});assert.equal(out.status,0,name+': '+out.stderr);results.push({name,...JSON.parse(out.stdout)});}
console.log(JSON.stringify({scope:'isolated-decoded-F02-acquisition-protected-workspace-faults',results,coherentCopiedGraphs:true,postHandoffCrashPhysicalOrIndependentProof:false}));
