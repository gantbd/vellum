import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),descriptor='formal/history-query-descriptor.mjs',native='formal/native-discovery.mjs',oldRead='formal/read-engine-descriptor.mjs',rules=[
 ['empty-labels-described',descriptor,"text={type:'string',minLength:1}","text={type:'string',minLength:0}"],
 ['native-context-described-as-text',descriptor,"opaque={type:'object'}","opaque={type:'string'}"],
 ['original-context-made-optional',descriptor,'required:[...fields]',"required:fields.filter(k=>k!=='context')"],
 ['extra-native-fields-described',descriptor,'additionalProperties:false','additionalProperties:true'],
 ['input-scope-field-omitted',descriptor,'record(f.fields,{contract:',"record(f.fields.filter(k=>k!=='scope'),{contract:"],
 ['redo-input-omitted',descriptor,'direction:{enum:[...f.directions]}',"direction:{enum:f.directions.filter(d=>d!=='redo')}"],
 ['null-input-group-refused',descriptor,"profile:text,group:{anyOf:[text,{type:'null'}]}","profile:text,group:text"],
 ['empty-frontier-group-refused',descriptor,"context:opaque,group:{anyOf:[text,{type:'null'}]}","context:opaque,group:text"],
 ['unavailable-result-branch-omitted',descriptor,'oneOf:[success,unavailable]','oneOf:[success]'],
 ['blocked-availability-omitted',descriptor,'value:{enum:[...historyQueryResultAvailability()]}',"value:{enum:historyQueryResultAvailability().filter(v=>v!=='blocked')}"],
 ['blocker-group-described-as-integer',descriptor,"record(['scope','group'],{scope:text,group:text})","record(['scope','group'],{scope:text,group:{type:'integer'}})"],
 ['blocker-group-field-omitted',descriptor,"record(['scope','group'],{scope:text,group:text})","record(['scope'],{scope:text,group:text})"],
 ['input-recognition-not-exact',descriptor,'export function historyQueryInputSchemaValid(value,c,operation){const expected=c===contract?inputSchema(operation):null;return expected!==null&&equal(value,expected);}','export function historyQueryInputSchemaValid(value,c,operation){const expected=c===contract?inputSchema(operation):null;return expected!==null;}'],
 ['result-recognition-not-exact',descriptor,'export function historyQueryResultMetadataValid(value,c,operation){const expected=c===contract?resultMetadata(operation):null;return expected!==null&&equal(value,expected);}','export function historyQueryResultMetadataValid(value,c,operation){const expected=c===contract?resultMetadata(operation):null;return expected!==null;}'],
 ['schema-claims-history-authority',descriptor,"coverage:'native-value-structure-only'","coverage:'history-authority-proved'"],
 ['original-owner-binding-omitted',descriptor,"'actual-history-owner-and-original-context',",''],
 ['projection-binding-omitted',descriptor,"'captured-alias-projection-and-detail-ceiling',",''],
 ['wire-event-limit-omitted',descriptor,"'public-wire-or-asynchronous-events','physical-global-or-independent-refinement'","'physical-global-or-independent-refinement'"],
 ['asynchronous-event-completeness-claimed',descriptor,"events:'no-asynchronous-events-in-this-synchronous-profile'","events:'history-events-complete'"],
 ['history-query-reclassified-as-command',descriptor,"kind:'query'","kind:'localCommand'"],
 ['history-family-substituted',descriptor,"freeze({family:'F04',contract","freeze({family:'F99',contract"],
 ['history-query-described-as-mutating',descriptor,"effect:'none'","effect:'authored-write'"],
 ['profile-seven-not-recognized',native,'||c.profile===catalog.historySchemaProfile',''],
 ['history-input-schema-disabled',native,'historyQueryInputSchemaValid(input.schema,contract,variant)','false'],
 ['arbitrary-history-input-schema-admitted',native,'historyQueryInputSchemaValid(input.schema,contract,variant)','true'],
 ['arbitrary-history-result-schema-admitted',native,'historyQueryResultMetadataValid(v.outcomes.result,r.contract,v.id)','true'],
 ['older-result-recognition-lost-in-seven',native,'![catalog.readResultSchemaProfile,catalog.historySchemaProfile,catalog.observationSchemaProfile,catalog.settingsSchemaProfile,catalog.lifecycleSchemaProfile,catalog.selectionSchemaProfile,catalog.advisorySchemaProfile,catalog.selfSchemaProfile].includes(profile)','profile!==catalog.readResultSchemaProfile'],
 ['fixed-history-source-omitted',native,'source=historyQueryEngineDescriptor()','source=null'],
 ['previous-read-profile-downgraded',native,'previous=builtinRows(m,{...c,profile:catalog.readResultSchemaProfile}),source=historyQueryEngineDescriptor()','previous=builtinRows(m,{...c,profile:catalog.readSchemaProfile}),source=historyQueryEngineDescriptor()'],
 ['unknown-twelve-profile-admitted',native,'||c.profile===catalog.historySchemaProfile',"||c.profile===catalog.historySchemaProfile||c.profile==='owned-contract-discovery/14'"],
 ['foreign-authority-scope-admitted',native,'&&c.authority===m.authority',''],
 ['foreign-engine-scope-admitted',native,'&&c.engine===m.engine',''],
 ['entry-capacity-overaccepted',native,'rows.length<=l.maxEntries','rows.length<=l.maxEntries+1'],
 ['variant-capacity-overaccepted',native,'r.variants.length<=l.maxVariants','r.variants.length<=l.maxVariants+1'],
 ['row-byte-bound-overaccepted',native,'size(r)<=l.maxRowBytes','size(r)<=l.maxRowBytes+1'],
 ['metadata-byte-bound-overaccepted',native,'size(v)<=l.maxMetadataBytes','size(v)<=l.maxMetadataBytes+1'],
 ['metadata-node-bound-overaccepted',native,'--budget.left<0','--budget.left< -1'],
 ['metadata-depth-overaccepted',native,'depth>l.maxMetadataDepth','depth>l.maxMetadataDepth+1'],
 ['packet-byte-bound-overaccepted',native,'size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes','size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes+1'],
 ['reply-byte-bound-overaccepted',native,'if(queryReplyBytes>m.limits.maxReplyBytes)return no();','if(queryReplyBytes>m.limits.maxReplyBytes+1)return no();'],
 ['captured-schema-ceiling-expanded',native,'selected.schema&&f.schema?','f.schema?'],
 ['generic-origin-borrows-builtin',native,'if((p.builtin===true)!==(s.builtin===true))return no();','if(false)return no();'],
 ['advice-current-facts-bypassed',native,'!gates(g,catalog.adviceGates)','false'],
 ['advice-target-copy-admitted',native,'request.target!==s.target.reference','false'],
 ['known-loss-does-not-stop',native,'stop(m,s);return null;','return null;'],
 ['metadata-withdrawal-ignored',native,'if(current===null||!equal(current,s.rows)){stop(m,s);return no();}','if(false){stop(m,s);return no();}'],
 ['query-publication-guard-bypassed',native,"const m=owners.get(owner);if(g?.outsidePublicationGuard!==true||!request","const m=owners.get(owner);if(!request"],
 ['retirement-skips-drain-and-retention',native,"if(!s?.stopped||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained']))","if(!s?.stopped||g?.ownerControlAllowed!==true)"],
 ['retirement-refunds-lifetime-issuance',native,"m.contexts.delete(context);return {status:'retired'};","m.contexts.delete(context);m.issued=0;return {status:'retired'};"],
 ['history-source-enters-older-read-profile',native,'[documentReadEngineDescriptor(),annotationReadEngineDescriptor()]','[documentReadEngineDescriptor(),annotationReadEngineDescriptor(),historyQueryEngineDescriptor()]'],
 ['older-read-result-product-drift',oldRead,'outcomes:{statusVocabulary:statuses,effect:','outcomes:{statusVocabulary:statuses,result:null,effect:']
];
const bankCount=3,bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);if(!Number.isInteger(bank)||bank<0||bank>=bankCount)throw Error('unsupported history-discovery mutation bank');
for(const[name,file,from]of rules)assert.equal(fs.readFileSync(path.join(root,file),'utf8').split(from).length-1,1,name+': construction target');
if(process.argv.includes('--list'))console.log(JSON.stringify({rules:rules.length,bankCount,banks:Array.from({length:bankCount},(_,n)=>rules.filter((_,i)=>i%bankCount===n).map(r=>r[0]))}));
else{const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
 try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/history-discovery-metadata-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:2097152,timeout:50000}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
  for(const[name,file,from,to]of rules.filter((_,i)=>i%bankCount===bank)){const graph=captureInputs(root),target=path.join(graph.directory,file),raw=fs.readFileSync(target,'utf8');fs.writeFileSync(target,raw.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1300));assert(!out.error&&!out.signal,name+': process/tool failure');assert(!out.stderr.includes('SCHEMA_TOOL_FAILURE')&&!out.stderr.includes('SCHEMA_PROTOCOL_FAILURE'),name+': verifier tool/protocol failure');assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1300));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
  passed=true;console.log(JSON.stringify({scope:'isolated-F04-history-discovery-sensitivity',bank,bankCount,totalMutations:rules.length,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,completePublicPhysicalGlobalOrIndependentProof:false}));
 }finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
}
