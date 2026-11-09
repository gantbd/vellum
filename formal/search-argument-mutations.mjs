import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),relative='formal/search-arguments.mjs',rules=[
 ['family-routing-gate-ignored',"if(decoded?.descriptor.id!=='F05')return null;",''],
 ['variant-agreement-ignored','decoded.variant!==args.operation','false'],
 ['unknown-argument-fields-stripped','!exact(args,form.fields)','!args'],
 ['maximum-capture-ceiling-ignored','const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)','const cap={...limits,maxRequestBytes:limits.maxRequestBytes'],
 ['byte-capture-off-by-one','const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)','const cap={...limits,maxRequestBytes:Math.min(maximum,limits.maxRequestBytes)+1'],
 ['node-capture-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputNodes:limits.maxInputNodes+1,maxRequestBytes:'],
 ['depth-capture-off-by-one','const cap={...limits,maxRequestBytes:','const cap={...limits,maxInputDepth:limits.maxInputDepth+1,maxRequestBytes:'],
 ['opaque-identity-copied','const opaque=args.context,','const opaque=structuredClone(args.context),'],
 ['route-enters-canonical-meaning','{request:r,context:c,limits,opaqueBindings:','{route:e.route,request:r,context:c,limits,opaqueBindings:'],
 ['literal-query-normalized',"const capture=copySettingsData({request:r,", "const capture=copySettingsData({request:{...r,query:r.query.normalize('NFC')},"],
 ['raw-request-alias-retained','record:capture.value','record:{...capture.value,request:args}'],
 ['non-string-query-accepted',"typeof r.query!=='string'",'false'],
 ['unsupported-profile-rejected-before-evaluation','const capture=copySettingsData(',"if(r.profile!==form.profile)return null;const capture=copySettingsData("],
 ['authority-frame-extended-fields-accepted','!exact(bound.value,contextFields)','false'],
 ['authority-domain-ignored','r.ref.authority!==c.authority','false'],
 ['navigation-wrap-not-boolean',"typeof r.navigation.wrap!=='boolean'",'false'],
 ['scope-order-not-validated','s.lo>s.hi','false'],
 ['scope-offset-not-numeric','!natural(s.lo)','false']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/search-argument-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name);assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-F05-native-argument-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,queryGatewayOwnershipWireAsyncPhysicalOrIndependentProof:false}));
