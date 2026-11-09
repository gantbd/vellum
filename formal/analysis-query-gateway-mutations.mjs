import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),owner='analysis-work.mjs',argumentsFile='analysis-query-arguments.mjs';
const rules=[
 ['caller-scope-unbound',owner,'queryDecodedInvocationAnalysis','!equal(normalized.record.context,invocationAuthorityContext(o.nativeDomain))','false'],
 ['opaque-owner-unchecked',owner,'queryDecodedInvocationAnalysis','snapshots.get(request.snapshot)?.owner!==o','false'],
 ['classification-headroom-ignored',owner,'queryDecodedInvocationAnalysis','analysisQueryClassificationBytes>o.limits.maxHeldBytes-usage(o).heldBytes','false'],
 ['classification-charge-omitted',owner,'queryDecodedInvocationAnalysis','o.gatewayBytes=analysisQueryClassificationBytes','o.gatewayBytes=0'],
 ['complete-frames-underfunded',owner,'queryDecodedInvocationAnalysis','frames=7*bytes','frames=bytes'],
 ['complete-frame-headroom-ignored',owner,'queryDecodedInvocationAnalysis','frames>o.limits.maxHeldBytes-usage(o).heldBytes','false'],
 ['faulted-owner-can-decode',owner,'queryDecodedInvocationAnalysis','o.queryFaulted||',''],
 ['faulted-domain-can-decode',owner,'queryDecodedInvocationAnalysis',"invocationUsage(o.nativeDomain)?.faulted!==false||",''],
 ['current-facts-not-captured',owner,'queryDecodedInvocationAnalysis','const facts=copySettingsData({facts:g},{...limits,maxRequestBytes:bytes});','const facts={value:{facts:g}};'],
 ['recursive-gateway-entry-allowed',owner,'queryDecodedInvocationAnalysis','o.queryBusy||',''],
 ['classification-not-locked',owner,'queryDecodedInvocationAnalysis','o.queryBusy=true;o.gatewayBytes=','o.queryBusy=false;o.gatewayBytes='],
 ['native-query-handoff-left-locked',owner,'queryDecodedInvocationAnalysis','o.queryBusy=false;\n  try{return queryInvocationAnalysis','o.queryBusy=true;\n  try{return queryInvocationAnalysis'],
 ['gateway-refunded-before-native-query',owner,'queryDecodedInvocationAnalysis','o.queryBusy=false;\n  try{return queryInvocationAnalysis','o.gatewayBytes=0;o.queryBusy=false;\n  try{return queryInvocationAnalysis'],
 ['temporary-charge-not-refunded',owner,'queryDecodedInvocationAnalysis','finally{o.gatewayBytes=0;o.queryBusy=false;}','finally{o.queryBusy=false;}'],
 ['busy-marker-not-released',owner,'queryDecodedInvocationAnalysis','finally{o.gatewayBytes=0;o.queryBusy=false;}','finally{o.gatewayBytes=0;o.queryBusy=true;}'],
 ['capture-fault-not-latched',owner,'queryDecodedInvocationAnalysis','catch{o.queryFaulted=true;return fail();}','catch{return fail();}'],
 ['gateway-charge-hidden-from-native-budget',owner,null,'+o.queryBytes+(o.gatewayBytes??0)','+o.queryBytes'],
 ['closed-native-argument-fields-ignored',argumentsFile,'normalizeAnalysisQueryArguments','!exact(args,form.fields)','false'],
 ['family-discriminator-corrupted',argumentsFile,'normalizeAnalysisQueryArguments',"decoded?.descriptor.id!=='F08'","decoded?.descriptor.id!=='F07'"],
 ['variant-operation-correlation-ignored',argumentsFile,'normalizeAnalysisQueryArguments','decoded.variant!==form.operation','false'],
 ['producer-snapshot-cloned',argumentsFile,'analysisQueryProducerRequest','snapshot:n.snapshot','snapshot:structuredClone(n.snapshot)'],
 ['canonical-original-role-ignored',argumentsFile,'sameAnalysisQueryArguments','a.snapshot===b.snapshot','true'],
 ['opaque-reference-copied-as-data',argumentsFile,'normalizeAnalysisQueryArguments','const snapshot=args.snapshot,shadow={...args,snapshot:null};','const snapshot=args.snapshot,shadow={...args};'],
 ['caller-context-not-captured',argumentsFile,'normalizeAnalysisQueryArguments','bound=copySettingsData(authorityContext,cap)','bound={value:authorityContext}'],
 ['backward-range-admitted',argumentsFile,'normalizeAnalysisQueryArguments','request.range[0]>request.range[1]','false'],
 ['foreign-view-authority-admitted',argumentsFile,'normalizeAnalysisQueryArguments','request.view.authority!==context.authority','false'],
 ['extra-caller-context-fields-admitted',argumentsFile,'normalizeAnalysisQueryArguments','!exact(bound.value,contextFields)','false']
];
const descriptorFile='analysis-query-descriptor.mjs';
rules.push(...[
 ['discovery-family-corrupted',descriptorFile,'analysisQueryEngineDescriptor',"family:'F08'","family:'F09'",'discovery'],
 ['discovery-kind-corrupted',descriptorFile,'analysisQueryEngineDescriptor',"kind:'query'","kind:'effectRequest'",'discovery'],
 ['discovery-version-corrupted',descriptorFile,'analysisQueryEngineDescriptor','version:1','version:2','discovery'],
 ['discovery-variant-omitted',descriptorFile,'analysisQueryEngineDescriptor','variants:forms.map(','variants:forms.slice(0,1).map(','discovery'],
 ['discovery-contract-constant-corrupted',descriptorFile,'analysisQueryInputSchema','contract:{const:form.contract}',"contract:{const:'foreign'}",'discovery'],
 ['discovery-operation-constant-corrupted',descriptorFile,'analysisQueryInputSchema','operation:{const:operation}',"operation:{const:'unknown'}",'discovery'],
 ['discovery-opaque-object-shape-omitted',descriptorFile,'analysisQueryInputSchema',"snapshot:{type:'object'}",'snapshot:{}','discovery'],
 ['discovery-negative-incarnation-admitted',descriptorFile,null,'minimum:0','minimum:-1','discovery'],
 ['discovery-view-field-set-opened',descriptorFile,'analysisQueryInputSchema',"required:['authority','id','incarnation'],additionalProperties:false","required:['authority','id','incarnation'],additionalProperties:true",'discovery'],
 ['discovery-request-field-set-opened',descriptorFile,'analysisQueryInputSchema','required:[...form.fields],additionalProperties:false','required:[...form.fields],additionalProperties:true','discovery'],
 ['discovery-status-vocabulary-corrupted',descriptorFile,'analysisQueryEngineDescriptor',"?'captures':'analysis-summary'","?'complete':'analysis-summary'",'discovery'],
 ['discovery-effect-misrepresented',descriptorFile,'analysisQueryEngineDescriptor',"effect:'no-authored-or-provider-effect'","effect:'source-publication'",'discovery'],
 ['discovery-query-receipt-misrepresented',descriptorFile,'analysisQueryEngineDescriptor',"receipt:'immediate-query-value-not-a-retained-command-receipt'","receipt:'retained-command-receipt'",'discovery'],
 ['analysis-arguments-import-ui',argumentsFile,null,"import {isDeepStrictEqual as equal} from 'node:util';","import './frontend-keymaps.mjs';\nimport {isDeepStrictEqual as equal} from 'node:util';",'boundary'],
 ['analysis-descriptor-import-ui',descriptorFile,null,"import {analysisQueryArgumentForms} from './analysis-query-arguments.mjs';","import './frontend-keymaps.mjs';\nimport {analysisQueryArgumentForms} from './analysis-query-arguments.mjs';",'boundary']
]);
export const analysisQueryGatewayMutationNames=Object.freeze(rules.map(([name])=>name));
const checks={gateway:'analysis-query-gateway-checks.mjs',discovery:'analysis-query-discovery-checks.mjs',boundary:'engine-ui-checks.mjs'};
const run=(directory,mode='gateway')=>spawnSync(process.execPath,[path.join(directory,'formal',checks[mode])],{env:process.env,encoding:'utf8',timeout:60000,maxBuffer:2097152});
function region(raw,fn){
 if(fn===null)return [0,raw.length];const start=raw.indexOf('export function '+fn+'(');assert(start>=0,'MUTATION_CONSTRUCTION missing function '+fn);
 const next=raw.indexOf('\nexport function ',start+1);return [start,next<0?raw.length:next];
}
export function checkAnalysisQueryGatewayMutations(){
 const baselineScope=createCaptureScope(),savedScope=process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE,savedTransient=process.env.VELLUM_TRANSIENT_PROOF_CAPTURES,mutations=[];let baselineGraph,baseline,baselinePassed=false;
 try{
  process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=baselineScope.directory;process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';
  baselineGraph=captureInputs(root);baseline={};for(const mode of Object.keys(checks)){const positive=run(baselineGraph.directory,mode);assert.equal(positive.status,0,positive.stderr);baseline[mode]=JSON.parse(positive.stdout);}baselinePassed=true;
 }finally{if(savedScope===undefined)delete process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE;else process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=savedScope;if(savedTransient===undefined)delete process.env.VELLUM_TRANSIENT_PROOF_CAPTURES;else process.env.VELLUM_TRANSIENT_PROOF_CAPTURES=savedTransient;releaseCaptureScope(baselineScope,baselinePassed);}
 for(const [name,file,fn,from,to,mode='gateway']of rules){
  const scope=createCaptureScope(),previous=process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE,transient=process.env.VELLUM_TRANSIENT_PROOF_CAPTURES;let passed=false;
  try{
   process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';
   const graph=captureInputs(root);assert.deepEqual(graph.hashes,baselineGraph.hashes,'coherent unchanged input graph');
   const target=path.join(graph.directory,'formal',file),raw=fs.readFileSync(target,'utf8'),[a,b]=region(raw,fn),body=raw.slice(a,b);
   assert.equal(body.split(from).length-1,1,'MUTATION_CONSTRUCTION '+name);fs.writeFileSync(target,raw.slice(0,a)+body.replace(from,to)+raw.slice(b));
   const result=run(graph.directory,mode);assert(!result.error&&!result.signal,name+': tool failure');assert.equal(result.status,1,name+': '+result.stderr.slice(-1200));
   assert(result.stderr.includes('ERR_ASSERTION'),name+': nonsemantic failure '+result.stderr.slice(-1200));assert(!result.stderr.includes('SyntaxError'),name+': construction failure');
   mutations.push(name);passed=true;
  }finally{if(previous===undefined)delete process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE;else process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=previous;if(transient===undefined)delete process.env.VELLUM_TRANSIENT_PROOF_CAPTURES;else process.env.VELLUM_TRANSIENT_PROOF_CAPTURES=transient;releaseCaptureScope(scope,passed);}
 }
 return {scope:'actual-native-analysis-query-gateway-sensitivity',baseline,mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,completeConditionalFacadeOrPhysicalOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkAnalysisQueryGatewayMutations()));}catch(e){console.error(String(e.stack??e).replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g,'<private-module>'));process.exitCode=1;}
