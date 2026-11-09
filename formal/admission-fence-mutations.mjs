// Coherent copied-module sensitivity. Parsing/import errors are not kills.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const edit=(fn,from,to,file='invocation-publication.mjs')=>({fn,from,to,file});
const rules=[
 ['original-control-context-unbound',[edit('sealInvocationOrdinaryAdmission','!equal(o.context,c)','false')]],
 ['truthy-control-authority',[edit('sealInvocationOrdinaryAdmission',"gates(g,['ownerControlAllowed','controlContextCurrent','controlScopeValid','protectedControlReady'])","['ownerControlAllowed','controlContextCurrent','controlScopeValid','protectedControlReady'].every(k=>g?.[k])")]],
 ['equal-root-substitution',[edit('sealInvocationOrdinaryAdmission','o.root!==before','false')]],
 ['changed-fence-retry-accepted',[edit('sealInvocationOrdinaryAdmission','o.admissionFence.before===before?o.admissionFence.token:null','o.admissionFence.token')]],
 ['fence-parent-debit-omitted',[edit('sealInvocationOrdinaryAdmission','o.charge+=charge;o.admissionFence=','o.charge+=0;o.admissionFence=')]],
 ['fence-parent-headroom-ignored',[edit('sealInvocationOrdinaryAdmission','charge>o.limits.maxHeldBytes-o.charge','false')]],
 ['invocation-issuance-fence-omitted',[edit('issueInvocation','beginInvocationAdmissionMutation(domain)','beginInvocationAdmissionMutation(domain,true)'),edit('issueInvocation','o.faulted||o.admissionFence||','o.faulted||')]],
 ['annotation-admission-fence-omitted',[edit('admitOrdinaryAnnotationSave','beginInvocationAdmissionMutation(domain)','beginInvocationAdmissionMutation(domain,true)','source-save.mjs'),edit('admitOrdinaryAnnotationSave','if(!invocationOrdinaryAdmissionOpen(domain))return unavailable();','','source-save.mjs'),edit('prepareInvocationOrdinaryBaseline','||o.admissionFence||','||'),edit('validateInvocationOrdinaryBaseline','&&!o.admissionFence&&','&&'),edit('commitInvocationOrdinaryBaseline','||o.admissionFence||','||')]],
 ['source-preparation-fence-omitted',[edit('prepareNativeSourceDestination',"beginInvocationAdmissionMutation(domain,['baseline','release','release-history'].includes(kind))",'beginInvocationAdmissionMutation(domain,true)','source-save.mjs'),edit('prepareNativeSourceDestination','beginInvocationAdmissionMutation(domain,protectedKind)','beginInvocationAdmissionMutation(domain,true)','source-save.mjs'),edit('prepareNativeSourceDestination',"if(!['baseline','release','release-history'].includes(kind)&&!invocationOrdinaryAdmissionOpen(domain))return null;",'','source-save.mjs'),edit('bindInvocationSourceWriter','if(o.nativeWriterDomains.has(domain))return invocationOrdinaryAdmissionOpen(domain);','if(o.nativeWriterDomains.has(domain))return true;','source-save.mjs'),edit('nativeWriterRootAdmission','&&invocationOrdinaryAdmissionOpen(domain)','&&true','source-save.mjs')]],
 ['prepared-source-commit-fence-omitted',[edit('installInvocationSourceDestination',"if(o.admissionFence&&!['baseline','release','release-history'].includes(p.kind))return unavailable();",'')]],
 ['prepared-publication-fence-omitted',[edit('runInvocation','if(o.admissionFence&&!entry.stop)return unavailable();','')]],
 ['active-admission-exclusion-omitted',[edit('sealInvocationOrdinaryAdmission','o.admissionMutation||','')]],
 ['active-publication-exclusion-omitted',[edit('sealInvocationOrdinaryAdmission',"[...o.slots.values()].some(e=>e.phase==='running'||e.ordinaryPreparing)",'false')]],
 ['fault-substituted-for-shutdown-control',[edit('sealInvocationOrdinaryAdmission','o.faulted||','')]]
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/admission-fence-checks.mjs')],{encoding:'utf8',timeout:60000,maxBuffer:1048576,env:process.env});
export function checkAdmissionFenceMutations(){
 const first=run(root);assert.equal(first.status,0,first.stderr);const baseline=JSON.parse(first.stdout),mutations=[],scope=createCaptureScope();
 const names=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],previous=names.map(k=>process.env[k]);
 process.env[names[0]]='1';process.env[names[1]]=scope.directory;
 try{
  for(const [name,edits] of rules){const c=captureInputs(root);
   for(const {fn,from,to,file} of edits){const target=path.join(c.directory,'formal',file),raw=fs.readFileSync(target,'utf8');let a=raw.indexOf('export function '+fn+'(');if(a<0)a=raw.indexOf('function '+fn+'(');assert(a>=0,name+': missing function');const end=raw.indexOf('\nexport function ',a+1),b=end<0?raw.length:end,region=raw.slice(a,b);assert.equal(region.split(from).length-1,1,name+': unique construction anchor');fs.writeFileSync(target,raw.slice(0,a)+region.replace(from,to)+raw.slice(b));}
   const r=run(c.directory);assert.equal(r.status,1,name+': '+r.stderr);assert(r.stderr.startsWith('ERR_ASSERTION:'),name+': not semantic assertion');assert(!r.stderr.includes('SyntaxError'),name+': construction failure');mutations.push(name);
  }
  assert.equal(releaseCaptureScope(scope,true),true);
 }finally{names.forEach((k,i)=>{if(previous[i]===undefined)delete process.env[k];else process.env[k]=previous[i];});}
 return {scope:'actual-domain-admission-fence-mutations-only',baseline,mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,fullShutdownOrImplementationOrIndependentProof:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{console.log(JSON.stringify(checkAdmissionFenceMutations()));}catch(e){console.error(e.code+': '+String(e.message).slice(0,1200));process.exitCode=1;}
