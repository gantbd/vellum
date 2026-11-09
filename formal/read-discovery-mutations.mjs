import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),descriptor='formal/read-engine-descriptor.mjs',native='formal/native-discovery.mjs',rules=[
 ['read-profile-schema-disabled',native,'profile===catalog.readSchemaProfile&&readInputSchemaValid(input.schema,contract,variant)','false'],
 ['read-schema-enters-old-profile',native,'profile===catalog.readSchemaProfile&&readInputSchemaValid(input.schema,contract,variant)','readInputSchemaValid(input.schema,contract,variant)'],
 ['document-read-handler-omitted',native,'[documentReadEngineDescriptor(),annotationReadEngineDescriptor()]','[annotationReadEngineDescriptor()]'],
 ['annotation-read-handler-omitted',native,'[documentReadEngineDescriptor(),annotationReadEngineDescriptor()]','[documentReadEngineDescriptor()]'],
 ['document-read-classified-as-command',native,"['F02','F03','F05'].includes(source.family)","['F03','F05'].includes(source.family)"],
 ['annotation-read-classified-as-command',native,"['F02','F03','F05'].includes(source.family)","['F02','F05'].includes(source.family)"],
 ['read-profile-added-to-old-source',native,'c.profile===catalog.readSchemaProfile?[documentReadEngineDescriptor(),annotationReadEngineDescriptor()]',"[catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)?[documentReadEngineDescriptor(),annotationReadEngineDescriptor()]"],
 ['metadata-schema-not-exact',descriptor,"export function readInputSchemaValid(value,contract,operation){const expected=contract===documentContract?documentInput(operation):contract===annotationContract?annotationInput(operation):null;return expected!==null&&equal(value,expected);}","export function readInputSchemaValid(value,contract,operation){const expected=contract===documentContract?documentInput(operation):contract===annotationContract?annotationInput(operation):null;return expected!==null;}"],
 ['document-extra-fields-admitted',descriptor,"document=record(['authority','id','incarnation'],{authority:text,id:text,incarnation:positive})","document={...record(['authority','id','incarnation'],{authority:text,id:text,incarnation:positive}),additionalProperties:true}"],
 ['zero-document-incarnation-described',descriptor,'incarnation:positive','incarnation:natural'],
 ['page-limit-zero-described',descriptor,'limit:positive','limit:natural'],
 ['fractional-source-offset-described',descriptor,"natural={type:'integer',minimum:0","natural={type:'number',minimum:0"],
 ['duplicate-classes-described',descriptor,'maxItems:2,uniqueItems:true','maxItems:2,uniqueItems:false'],
 ['empty-class-set-described',descriptor,"items:{enum:['source','annotations']},minItems:1","items:{enum:['source','annotations']},minItems:0"],
 ['empty-participant-set-described',descriptor,"participants:{type:'array',items:participant,minItems:1}","participants:{type:'array',items:participant,minItems:0}"],
 ['retained-selection-reference-optional',descriptor,"record(['kind','reference'],{kind:{const:'retained'},reference:opaque})","record(['kind'],{kind:{const:'retained'},reference:opaque})"],
 ['continuation-no-longer-nullable',descriptor,"limit:positive,continuation:{anyOf:[opaque,{type:'null'}]},annotation:opaque","limit:positive,continuation:opaque,annotation:opaque"],
 ['snapshot-identity-described-as-string',descriptor,'const fieldShapes={document,snapshot:opaque,','const fieldShapes={document,snapshot:text,'],
 ['record-identity-described-as-string',descriptor,'annotation:opaque','annotation:text'],
 ['native-set-status-omitted',descriptor,"['context','version-set','unavailable']","['context','unavailable']"],
 ['invented-success-status-described',descriptor,"[id==='enumerate'?'page':id==='inspect'?'record':'location','unavailable']","[id==='enumerate'?'page':id==='inspect'?'record':'location','unavailable','success']"],
 ['bound-source-input-contract-ignored',descriptor,"k==='contract'||k==='operation'?{const:f[k]}:fieldShapes[k]","k==='operation'?{const:f[k]}:k==='contract'?text:fieldShapes[k]"],
 ['catalog-entry-capacity-overaccepted',native,'rows.length<=l.maxEntries','rows.length<=l.maxEntries+1'],
 ['variant-capacity-overaccepted',native,'r.variants.length<=l.maxVariants','r.variants.length<=l.maxVariants+1'],
 ['row-byte-bound-overaccepted',native,'size(r)<=l.maxRowBytes','size(r)<=l.maxRowBytes+1'],
 ['metadata-byte-bound-overaccepted',native,'size(v)<=l.maxMetadataBytes','size(v)<=l.maxMetadataBytes+1'],
 ['metadata-depth-bound-overaccepted',native,'depth>l.maxMetadataDepth','depth>l.maxMetadataDepth+1'],
 ['metadata-node-bound-overaccepted',native,'--budget.left<0','--budget.left< -1'],
 ['packet-byte-bound-overaccepted',native,'size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes','size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes+1'],
 ['reply-byte-bound-overaccepted',native,'if(queryReplyBytes>m.limits.maxReplyBytes)return no();','if(queryReplyBytes>m.limits.maxReplyBytes+1)return no();'],
 ['read-advice-facts-bypassed',native,'!gates(g,catalog.adviceGates)','false'],
 ['read-advice-target-identity-bypassed',native,'request.target!==s.target.reference','false']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/read-product-discovery-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [name,relative,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1200));assert(!out.stderr.includes('SCHEMA_TOOL_FAILURE')&&!out.stderr.includes('SCHEMA_PROTOCOL_FAILURE'),name+': external tool/protocol fault');assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1200));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-read-product-discovery-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allPublicProductsPhysicalHostOrIndependentProof:false}));
