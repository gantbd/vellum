// Coherent copied native graph. Parsing and construction failures are not kills.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const edit=(fn,from,to,file='invocation-publication.mjs')=>({fn,from,to,file});
const rules=[
 ['plan-admission-ordinary-configured',edit('ordinaryPreparation','capture.value.semanticProfile,context,g),true):','capture.value.semanticProfile,context,g),false):')],
 ['plan-admission-prepareInvocationConfiguredLine',edit('prepareInvocationConfiguredLine','entry.configuredLinePrepared),!entry.configuredLinePrepared);','entry.configuredLinePrepared));')],
 ...['prepareInvocationSemanticSettings','prepareInvocationScopedSettingsLifecycle','prepareInvocationBasisSettingsLifecycle','prepareInvocationScopedSettingsDefinition','prepareInvocationScopedSemanticSettings'].map(fn=>['plan-admission-'+fn,edit(fn,'entry.settingsPrepared),!entry.settingsPrepared);','entry.settingsPrepared));')]),
 ...['prepareInvocationSourceSerialization','prepareInvocationScopedSourceSerialization','captureInvocationSerializationBasis'].map(fn=>['plan-admission-'+fn,edit(fn,'if(!admissionSection)return null;','')]),
 ...['reserveInvocationBaseline','reserveInvocationWriteBaseline'].map(fn=>['plan-admission-'+fn,edit(fn,'if(!section)return unavailable();','')]),
 ['plan-admission-write-baseline-lookup-allocates',edit('reserveInvocationWriteBaseline','claim,g,false);','claim,g,true);')],
 ['registration-new-basis-admission-fence-bypassed',edit('captureNativeRegistrationBasis','if(!admissionSection)return no();','','registration-bases.mjs')],
 ['existing-settings-new-product-misclassified-as-retained-service',edit('withAttachedNativeSettingsPool','if(!newProduct)return invoke();','return invoke();')],
 ...[
  ['resource-read.mjs','admitResourceRead','if(admissionDomain&&!admissionSection)return unavailable();'],
  ['analysis-work.mjs','admitAnalysisWork',"if(admissionDomain&&!admissionSection)return {status:'unavailable'};"],
  ['annotation-native-load.mjs','admitInvocationAnnotationRead','if(!admissionSection)return unavailable();'],
  ['history-contexts.mjs','acquireHistoryInspection','if(!admissionSection)return unavailable();'],
  ['native-discovery.mjs','acquireNativeDiscovery','if(!admissionSection)return no();'],
  ['registration-reads.mjs','acquireRegistrationRead','if(!admissionSection)return no();'],
  ['registration-events.mjs','establishRegistrationObservation','if(!admissionSection)return no();'],
  ['registration-events.mjs','prepareRegistrationEventSnapshot','if(stream===null&&!admissionSection)return null;'],
  ...['prepareInvocationVersionSetCapture','prepareInvocationAnnotationCapture','holdInvocationSourceContext'].map(fn=>['invocation-publication.mjs',fn,'if(!admissionSection)return null;']),
  ...['installInvocationVersionSetContext','acquireInvocationRetainedVersionSetContext','installInvocationAnnotationContext','acquireInvocationRetainedAnnotationContext','queryInvocationSourceSearch','queryInvocationDecodedSearch','acquireInvocationSourceContext','acquireInvocationDecodedReadContext','acquireInvocationSettingsRead'].map(fn=>['invocation-publication.mjs',fn,'if(!admissionSection)return unavailable();'])
 ].map(([file,fn,guard])=>['existing-owner-admission-'+fn,edit(fn,guard,'',file)]),
 ['native-publication-observer-misclassified',edit('openInvocationPublicationObserver',"beginInvocationOwnerAttachment(domain,'publication-observer',","beginInvocationOwnerAttachment(domain,'history',",'native-publication-observation.mjs')],
 ['native-publication-observer-local-ceiling-unfunded',edit('openInvocationPublicationObserver','units=captured.maxHeldBytes+recordBytes','units=recordBytes','native-publication-observation.mjs')],
 ['native-publication-observer-private-record-unfunded',edit('openInvocationPublicationObserver','units=captured.maxHeldBytes+recordBytes','units=captured.maxHeldBytes','native-publication-observation.mjs')],
 ['native-publication-observer-commit-omitted',edit('openInvocationPublicationObserver',"if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native publication observer attachment invariant');",'','native-publication-observation.mjs')],
 ['native-publication-resync-caller-binding-omitted',edit('resyncInvocationPublicationObserver','o&&publicationObserverClaimMatches(o.rawOwner,claimed)','o','native-publication-observation.mjs')],
 ['native-publication-read-caller-binding-omitted',edit('captureInvocationPublicationRead','o&&publicationObserverClaimMatches(o.rawOwner,claimed)','o','native-publication-observation.mjs')],
 ['native-publication-outcome-origin-renamed',edit('captureInvocationPublicationOutcome','state.prepared.facts,state.reference,',"state.prepared.facts,state.reference+'-renewed',",'native-publication-observation.mjs')],
 ['native-publication-private-producer-facets-accepted',edit('captureNativePublicationFacts',"if(!facts||!['applied','unchanged','notApplied'].includes(facts.publication)||facts.effect!=='none'||facts.basis!=='none'||!Array.isArray(facts.causes)||facts.causes.length||!Array.isArray(facts.participants)||facts.participants.length)return null;",'','publication-observation.mjs')],
 ['native-publication-stop-drops-retained-owner',edit('stopInvocationPublicationObserver','const o=owners.get(owner);','const o=owners.get(owner);owners.delete(owner);','native-publication-observation.mjs')],
 ['native-publication-truthy-protected-outcome-discharge',edit('retireInvocationPublicationOutcome','g?.retentionReleased===true','Boolean(g?.retentionReleased)','native-publication-observation.mjs')],
 ['shared-writer-first-node-reservation-omitted',edit('bindInvocationSourceWriter','funding.edge+(o.nativeWriterSponsor?0:funding.node)','funding.edge','source-save.mjs')],
 ['shared-writer-peer-duplicates-whole-node-charge',edit('bindInvocationSourceWriter','funding.edge+(o.nativeWriterSponsor?0:funding.node)','funding.edge+funding.node','source-save.mjs')],
 ['shared-writer-actual-edge-commit-omitted',edit('bindInvocationSourceWriter',"if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('shared source writer attachment invariant');",'','source-save.mjs')],
 ['shared-writer-reentrant-sponsor-selection',edit('bindInvocationSourceWriter','o.nativeWriterBindingPending||','','source-save.mjs')],
 ['shared-writer-existing-edge-bypasses-fence',edit('bindInvocationSourceWriter','if(o.nativeWriterDomains.has(domain))return invocationOrdinaryAdmissionOpen(domain);','if(o.nativeWriterDomains.has(domain))return true;','source-save.mjs')],
 ['ordinary-annotation-unfunded-writer-accepted',edit('admitOrdinaryAnnotationSave','if(!owners.get(owner)?.nativeWriterDomains.has(domain))return unavailable();','','source-save.mjs')],
 ['shared-writer-raw-source-fence-bypassed',edit('nativeWriterRootAdmission','return !o.nativeWriterDomains.size||','return true||!o.nativeWriterDomains.size||','source-save.mjs')],
 ['observation-pool-misclassified',edit('openObservationAdmission',"beginInvocationOwnerAttachment(domain,'observation-admission',","beginInvocationOwnerAttachment(domain,'history',",'observation-admission.mjs')],
 ['observation-pool-bookkeeping-parent-debit-omitted',edit('openObservationAdmission',"'observation-admission',limits?.maxBookkeepingBytes,","'observation-admission',0,",'observation-admission.mjs')],
 ['observation-pool-commit-omitted',edit('openObservationAdmission',"if(!commitInvocationOwnerAttachment(domain,attachment,pool))throw Error('native observation admission attachment invariant');",'','observation-admission.mjs')],
 ['observation-reservation-workspace-parent-debit-omitted',edit('admitPooledObservation',"'observation-admission',reservedWorkspace,reservation","'observation-admission',0,reservation",'observation-admission.mjs')],
 ['observation-reservation-commit-omitted',edit('admitPooledObservation',"if(!commitInvocationOwnerAttachment(o.domain,attachment,reservation))throw Error('native observation reservation attachment invariant');",'','observation-admission.mjs')],
 ['observation-reservation-known-refusal-refund-omitted',edit('admitPooledObservation',"if(!context&&!releaseInvocationOwnerAttachment(o.domain,record.parentAttachment,{ownerControlAllowed:true,retentionReleased:true,deliveryDrained:true}))throw Error('native refused observation reservation discharge invariant');",'','observation-admission.mjs')],
 ['observation-reservation-retirement-refund-omitted',edit('retireObservationReservation',"if(!releaseInvocationOwnerAttachment(o.domain,r.parentAttachment,g))throw Error('native observation reservation discharge invariant');",'','observation-admission.mjs')],
 ['observation-reservation-recursive-retirement-identity-recheck-omitted',edit('retireObservationReservation','o.leases.get(reservation)!==r||','','observation-admission.mjs')],
 ['lazy-settings-pool-misclassified',edit('withAttachedNativeSettingsPool',"beginInvocationOwnerAttachment(domain,'settings',","beginInvocationOwnerAttachment(domain,'history',")],
 ['lazy-settings-pool-commit-omitted',edit('withAttachedNativeSettingsPool',"if(handle&&!commitInvocationOwnerAttachment(domain,attachment,handle))throw Error('native lazy settings attachment invariant');",'')],
 ['lazy-settings-pool-pending-refund-omitted',edit('withAttachedNativeSettingsPool','abortInvocationOwnerAttachment(domain,attachment);','')],
 ['lazy-settings-retained-pool-lost-after-exception',edit('withAttachedNativeSettingsPool','try{return invoke();}finally{const handle=nativeEngineSettingsPoolHandle(o,domain);','let completed=false;try{const value=invoke();completed=true;return value;}finally{const handle=completed?nativeEngineSettingsPoolHandle(o,domain):null;')],
 ['lazy-settings-pool-edge-retirement-bypassed',edit('retireInvocationSemanticSettingsPool',"return retireAttachedNativePool(domain,o?nativeEngineSettingsPoolHandle(o,domain):null,'settings',g,(owner,facts)=>retireNativeEngineSettingsPool(owner,domain,facts));",'return o?retireNativeEngineSettingsPool(o,domain,g):false;')],
 ['serialization-pool-misclassified',edit('openInvocationSerializationBasisPool',"openAttachedNativePool(domain,'serialization',","openAttachedNativePool(domain,'history',")],
 ['serialization-pool-edge-retirement-bypassed',edit('retireInvocationSerializationBasisPool',"return retireAttachedNativePool(domain,handle,'serialization',g,(owner,facts)=>retireSerializationBasisPool(owner,domain,facts));","return o?retireSerializationBasisPool(o,domain,g):false;")],
 ['catalog-reader-misclassified',edit('openRegistrationReadRegistry',"beginInvocationOwnerAttachment(domain,'registration-reads',","beginInvocationOwnerAttachment(domain,'history',",'registration-reads.mjs')],
 ['catalog-reader-edge-commit-omitted',edit('openRegistrationReadRegistry',"if(!commitInvocationOwnerAttachment(domain,attachment,owner))throw Error('native registration read attachment invariant');",'','registration-reads.mjs')],
 ['catalog-reader-edge-refund-omitted',edit('retireRegistrationReadRegistry',"if(!releaseInvocationOwnerAttachment(m.domain,m.parentAttachment,g))throw Error('native registration read discharge invariant');",'','registration-reads.mjs')],
 ['catalog-reader-truthy-discharge',edit('retireRegistrationReadRegistry','g?.[k]===true','Boolean(g?.[k])','registration-reads.mjs')],
 ['catalog-reader-discharge-retained-child-recheck-omitted',edit('retireRegistrationReadRegistry','||owners.get(owner)!==m||m.decoding===true||m.contexts.size||m.packets.size','','registration-reads.mjs')],
 ['baseline-reentrant-capture-singleton-recheck-omitted',edit('openInvocationBaselineReservations','try{if(o.baselineReservations)return null;','try{')],
 ['baseline-pool-misclassified',edit('openInvocationBaselineReservations',"beginInvocationOwnerAttachment(domain,'baseline-reservations',","beginInvocationOwnerAttachment(domain,'history',")],
 ['baseline-local-ceiling-parent-debit-omitted',edit('openInvocationBaselineReservations',"'baseline-reservations',captured?.maxHeldBytes,","'baseline-reservations',0,")],
 ['baseline-pool-attachment-commit-omitted',edit('openInvocationBaselineReservations',"if(!commitInvocationOwnerAttachment(domain,attachment,pool))throw Error('native baseline reservation attachment invariant');",'')],
 ['owner-factory-fence-bypassed',edit('beginInvocationOwnerAttachment','beginInvocationAdmissionMutation(domain)','beginInvocationAdmissionMutation(domain,true)')],
 ['child-envelope-parent-debit-omitted',edit('beginInvocationOwnerAttachment','charge=units+frame','charge=frame')],
 ['edge-frame-parent-debit-omitted',edit('beginInvocationOwnerAttachment','charge=units+frame','charge=units')],
 ['parent-headroom-ignored',edit('beginInvocationOwnerAttachment','charge>o.limits.maxHeldBytes-o.charge','false')],
 ['unknown-owner-kind-accepted',edit('invocationOwnerAttachmentFrame','!nativeOwnerKinds.includes(kind)||','')],
 ['undersized-metadata-envelope',edit('invocationOwnerAttachmentFrame','return size({pending:','return 1;return size({pending:')],
 ['undeclared-owner-handle-substitution',edit('commitInvocationOwnerAttachment','||!equal(owner,p.ownerTemplate)','')],
 ['duplicate-actual-owner-edge-overwritten',edit('commitInvocationOwnerAttachment','||o.ownerAttachments?.has(owner)','')],
 ['commit-section-not-released',edit('commitInvocationOwnerAttachment','endInvocationAdmissionMutation(domain,p.section);return true;','return true;')],
 ['aborted-reservation-not-refunded',edit('abortInvocationOwnerAttachment','o.charge-=p.charge;','')],
 ['copied-abort-lease-accepted',edit('abortInvocationOwnerAttachment','p.lease!==lease||','')],
 ['release-context-unbound',edit('invocationOwnerAttachmentInventory','!equal(o.context,c)','false')],
 ['inventory-truthy-supervision',edit('invocationOwnerAttachmentInventory','g?.ownerControlAllowed!==true','!g?.ownerControlAllowed')],
 ['truthy-discharge-facts',edit('releaseInvocationOwnerAttachment',"gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])","['ownerControlAllowed','retentionReleased','deliveryDrained'].every(k=>g?.[k])")],
 ['retired-edge-child-ceiling-not-refunded',edit('releaseInvocationOwnerAttachment','o.charge-=row[1].charge','o.charge-=row[1].frame')],
 ['source-reader-misclassified',edit('newInvocationSourceReadRegistry',"beginInvocationOwnerAttachment(domain,'source-read',","beginInvocationOwnerAttachment(domain,'history',",'resource-read.mjs')],
 ['history-owner-misclassified',edit('openHistoryInspection',"beginInvocationOwnerAttachment(domain,'history',","beginInvocationOwnerAttachment(domain,'source-read',",'history-contexts.mjs')],
 ['observation-context-misclassified',edit('openInvocationObservation',"beginInvocationOwnerAttachment(domain,'observation',","beginInvocationOwnerAttachment(domain,'history',",'invocation-observation.mjs')],
 ['native-observation-retirement-parent-refund-omitted',edit('retireInvocationObservation','if(!releaseInvocationOwnerAttachment(e.domain,e.parentAttachment,g))return unavailable();','','invocation-observation.mjs')],
 ['annotation-reader-misclassified',edit('newInvocationAnnotationReadRegistry',"beginInvocationOwnerAttachment(domain,'annotation-read',","beginInvocationOwnerAttachment(domain,'history',",'annotation-native-load.mjs')],
 ['analysis-owner-misclassified',edit('newInvocationAnalysisRegistry',"beginInvocationOwnerAttachment(domain,'analysis',","beginInvocationOwnerAttachment(domain,'history',",'analysis-work.mjs')],
 ['registration-basis-owner-misclassified',edit('openNativeRegistrationBasisRegistry',"beginInvocationOwnerAttachment(domain,'registration-bases',","beginInvocationOwnerAttachment(domain,'history',",'registration-bases.mjs')],
 ['registration-event-owner-misclassified',edit('openRegistrationEventRegistry',"beginInvocationOwnerAttachment(domain,'registration-events',","beginInvocationOwnerAttachment(domain,'history',",'registration-events.mjs')],
 ['discovery-owner-misclassified',edit('openNativeDiscoveryRegistry',"beginInvocationOwnerAttachment(domain,'discovery',","beginInvocationOwnerAttachment(domain,'history',",'native-discovery.mjs')],
 ['diagnostic-capture-owner-misclassified',edit('openDiagnosticCapture',"beginInvocationOwnerAttachment(domain,'diagnostic-capture',","beginInvocationOwnerAttachment(domain,'history',",'diagnostic-capture.mjs')],
 ['diagnostic-drain-owner-misclassified',edit('openDiagnosticDrain',"beginInvocationOwnerAttachment(domain,'diagnostic-drain',","beginInvocationOwnerAttachment(domain,'history',",'diagnostic-drain.mjs')],
 ...[['registration-basis','retireNativeRegistrationBasisRegistry','registration-bases.mjs'],['registration-event','retireRegistrationEventRegistry','registration-events.mjs'],['discovery','retireNativeDiscoveryRegistry','native-discovery.mjs'],['diagnostic-capture','retireDiagnosticCapture','diagnostic-capture.mjs'],['diagnostic-drain','retireDiagnosticDrain','diagnostic-drain.mjs']].map(([name,fn,file])=>[name+'-retirement-parent-refund-omitted',edit(fn,`if(!releaseInvocationOwnerAttachment(m.domain,m.parentAttachment,${name==='registration-event'?'f':'g'}))return no();`,'',file)]),
 ['parent-funded-pool-charged-as-additional-child',edit('openAttachedNativePool','beginInvocationOwnerAttachment(domain,kind,0,','beginInvocationOwnerAttachment(domain,kind,1,')],
 ['native-pool-attachment-commit-omitted',edit('openAttachedNativePool',"if(!commitInvocationOwnerAttachment(domain,attachment,handle))throw Error('parent-funded native pool attachment invariant');",'')],
 ['native-pool-retirement-skipped',edit('retireAttachedNativePool','!retire(o,facts)','false')],
 ['native-pool-attachment-refund-skipped',edit('retireAttachedNativePool',"if(!releaseInvocationOwnerAttachment(domain,edge.lease,facts))throw Error('parent-funded native pool discharge invariant');",'')],
 ['native-pool-truthy-discharge',edit('retireAttachedNativePool','g?.[k]===true','Boolean(g?.[k])')],
 ...[['source-contexts','openInvocationSourceContexts'],['source-search','openInvocationSourceSearch'],['annotation-capture','openInvocationAnnotationCapture'],['annotation-contexts','openInvocationAnnotationContexts'],['version-set-capture','openInvocationVersionSetCapture'],['version-set-contexts','openInvocationVersionSetContexts'],['settings-reads','openInvocationSettingsReads']].map(([kind,fn])=>[kind+'-pool-misclassified',edit(fn,"openAttachedNativePool(domain,'"+kind+"',","openAttachedNativePool(domain,'history',")])
];
export const ownerAttachmentMutationNames=Object.freeze(rules.map(([name])=>name));
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/owner-attachment-native-checks.mjs')],{encoding:'utf8',timeout:60000,maxBuffer:1048576,env:process.env});
export function checkOwnerAttachmentMutations(selectedNames=null){
 if(selectedNames!==null){assert(Array.isArray(selectedNames)&&selectedNames.length>0);assert.equal(new Set(selectedNames).size,selectedNames.length);assert(selectedNames.every(n=>rules.some(([name])=>name===n)));}
 const selected=selectedNames===null?rules:rules.filter(([name])=>selectedNames.includes(name));
 const first=run(root);assert.equal(first.status,0,first.stderr);const baseline=JSON.parse(first.stdout),mutations=[],scope=createCaptureScope();
 const names=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],previous=names.map(k=>process.env[k]);process.env[names[0]]='1';process.env[names[1]]=scope.directory;
 try{for(const [name,{fn,from,to,file}] of selected){const c=captureInputs(root),target=path.join(c.directory,'formal',file),raw=fs.readFileSync(target,'utf8');let a=raw.indexOf('export function '+fn+'(');if(a<0)a=raw.indexOf('function '+fn+'(');assert(a>=0,name+': missing function');const ends=['\nexport function ','\nfunction '].map(s=>raw.indexOf(s,a+1)).filter(n=>n>=0),b=ends.length?Math.min(...ends):raw.length,region=raw.slice(a,b);assert.equal(region.split(from).length-1,1,name+': unique construction anchor');fs.writeFileSync(target,raw.slice(0,a)+region.replace(from,to)+raw.slice(b));const r=run(c.directory);assert.equal(r.status,1,name+': '+r.stderr);assert(r.stderr.startsWith('ERR_ASSERTION:'),name+': not semantic assertion');assert(!r.stderr.includes('SyntaxError'),name+': construction failure');mutations.push(name);}assert.equal(releaseCaptureScope(scope,true),true);}
 finally{names.forEach((k,i)=>{if(previous[i]===undefined)delete process.env[k];else process.env[k]=previous[i];});}
 return {scope:'selected-actual-owner-attachment-mutations-only',baseline,mutations,selectedMutationRun:selectedNames!==null,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allNativeFactoryCoverageOrCompleteShutdownProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)try{const arg=process.argv.find(a=>a.startsWith('--mutation-bank='));let selected=null;if(arg){const bank=Number(arg.slice('--mutation-bank='.length));assert(Number.isInteger(bank)&&bank>=0&&bank<Math.ceil(ownerAttachmentMutationNames.length/10));selected=ownerAttachmentMutationNames.slice(bank*10,(bank+1)*10);}console.log(JSON.stringify(checkOwnerAttachmentMutations(selected)));}catch(e){console.error(e.code+': '+String(e.message).slice(0,1200));process.exitCode=1;}
