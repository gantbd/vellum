import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {captureInputs} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),rules=[
 ['native-discovery.mjs','search-profile-unrecognized','[catalog.profile,catalog.schemaProfile,catalog.editingSchemaProfile,catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)','[catalog.profile,catalog.schemaProfile,catalog.editingSchemaProfile,catalog.readSchemaProfile].includes(c.profile)'],
 ['native-discovery.mjs','legacy-catalog-expanded','[catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)?[searchEngineDescriptor()]:[]','true?[searchEngineDescriptor()]:[]'],
 ['native-discovery.mjs','search-handler-source-omitted','[catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(c.profile)?[searchEngineDescriptor()]:[]','c.profile===catalog.readSchemaProfile?[searchEngineDescriptor()]:[]'],
 ['native-discovery.mjs','arbitrary-search-schema-recognized','searchInputSchemaValid(input.schema,variant)','true'],
 ['native-discovery.mjs','search-schema-smuggled-into-older-profile',"[catalog.searchSchemaProfile,catalog.readSchemaProfile].includes(profile)&&contract==='literal-search/1'","true&&contract==='literal-search/1'"],
 ['native-discovery.mjs','captured-search-schema-ceiling-widened','selected.schema&&f.schema?{input','f.schema?{input'],
 ['native-discovery.mjs','current-search-schema-withdrawal-ignored','current===null||!equal(current,s.rows)','current===null'],
 ['native-discovery.mjs','lookalike-origin-accepted','(p.builtin===true)!==(s.builtin===true)','false'],
 ['native-discovery.mjs','unknown-loss-stops-context',"[...catalog.authorityGates,'discoveryInterpretationCurrent','retained'].some(k=>g?.[k]===false)","[...catalog.authorityGates,'discoveryInterpretationCurrent','retained'].some(k=>g?.[k]!==true)"],
 ['native-discovery.mjs','publication-guard-ignored',"g?.outsidePublicationGuard!==true||!request||!Object.hasOwn(queries.variants,request.operation)","!request||!Object.hasOwn(queries.variants,request.operation)"],
 ['search-engine-descriptor.mjs','search-described-as-local-command',"kind:'query'","kind:'localCommand'"],
 ['search-engine-descriptor.mjs','required-search-field-omitted','required:[...f.fields]','required:[...f.fields].slice(0,-1)'],
 ['search-engine-descriptor.mjs','extra-search-fields-admitted','required:[...f.fields],additionalProperties:false','required:[...f.fields],additionalProperties:true'],
 ['search-engine-descriptor.mjs','negative-generation-described',"minimum:0","minimum:-1"],
 ['search-engine-descriptor.mjs','empty-query-rejected-by-schema',"query:{type:'string'}","query:{type:'string',minLength:1}"],
 ['search-engine-descriptor.mjs','literal-navigation-described-as-object',"k==='navigation'&&operation==='literal-matches'","false"],
 ['literal-search.mjs','partial-total-fabricated','total:complete?matches.length:null','total:complete?matches.length:0'],
 ['literal-search.mjs','partial-coverage-inflated',"coverage:complete?'complete-within-scope':'known-subset'","coverage:'complete-within-scope'"],
 ['literal-search.mjs','incomplete-navigation-invented','out.navigation=complete?navigate','out.navigation=true?navigate'],
 ['literal-search.mjs','unsupported-disposition-not-declared',"statusOnly:['superseded','unavailable','exhausted','unsupported']","statusOnly:['superseded','unavailable','exhausted']"],
 ['literal-search.mjs','result-total-not-declared',"'coverage','matches','total'","'coverage','matches'"],
 ['literal-search.mjs','native-generation-omitted','generation:request.generation,',''],
 ['search-engine-descriptor.mjs','discovery-claims-search-events',"events:'no-asynchronous-events-in-this-synchronous-profile'","events:'asynchronous-progress-stream'"],
 ['native-discovery.mjs','protected-discovery-retirement-bypassed',"!s?.stopped||!gates(g,['ownerControlAllowed','retentionReleased','deliveryDrained'])",'!s?.stopped'],
 ['native-discovery.mjs','catalog-count-off-by-one','rows.length<=l.maxEntries','rows.length<=l.maxEntries+1'],
 ['native-discovery.mjs','variant-count-off-by-one','r.variants.length<=l.maxVariants','r.variants.length<=l.maxVariants+1'],
 ['native-discovery.mjs','registry-row-byte-off-by-one','size(r)<=l.maxRowBytes','size(r)<=l.maxRowBytes+1'],
 ['native-discovery.mjs','metadata-byte-off-by-one','size(v)<=l.maxMetadataBytes','size(v)<=l.maxMetadataBytes+1'],
 ['native-discovery.mjs','metadata-node-off-by-one','--budget.left<0','--budget.left< -1'],
 ['native-discovery.mjs','metadata-depth-off-by-one','depth>l.maxMetadataDepth','depth>l.maxMetadataDepth+1'],
 ['native-discovery.mjs','prepared-packet-off-by-one','>m.limits.maxPacketBytes','>m.limits.maxPacketBytes+1'],
 ['native-discovery.mjs','reply-frame-off-by-one','if(queryReplyBytes>m.limits.maxReplyBytes)return no();','if(queryReplyBytes>m.limits.maxReplyBytes+1)return no();'],
 ['search-engine-descriptor.mjs','nested-extra-fields-admitted','required:[...required],additionalProperties:false','required:[...required],additionalProperties:true'],
 ['search-engine-descriptor.mjs','source-ref-revision-not-required','record(nested.ref,','record(nested.ref.filter(k=>k!=="revision"),'],
 ['search-engine-descriptor.mjs','scope-hi-not-required','record(nested.scope,','record(nested.scope.filter(k=>k!=="hi"),'],
 ['search-engine-descriptor.mjs','navigation-current-not-required','record(nested.navigation,','record(nested.navigation.filter(k=>k!=="current"),'],
 ['search-engine-descriptor.mjs','numeric-current-index-not-described',"current:{anyOf:[natural,{type:'null'}]}","current:{type:'null'}"],
 ['search-engine-descriptor.mjs','navigation-wrap-not-boolean',"wrap:{type:'boolean'}","wrap:{type:'integer'}"],
 ['search-engine-descriptor.mjs','safe-integer-ceiling-omitted',',maximum:Number.MAX_SAFE_INTEGER',''],
 ['search-engine-descriptor.mjs','empty-identity-text-admitted','minLength:1','minLength:0']
];
const run=directory=>spawnSync(process.execPath,[path.join(directory,'formal/search-product-discovery-checks.mjs')],{encoding:'utf8',env:process.env,maxBuffer:1048576}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
for(const [relative,name,from,to]of rules){const graph=captureInputs(root),file=path.join(graph.directory,'formal',relative),original=fs.readFileSync(file,'utf8');assert.equal(original.split(from).length-1,1,name);fs.writeFileSync(file,original.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name);assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1000));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
console.log(JSON.stringify({scope:'isolated-search-product-and-versioned-discovery-sensitivity',baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,completeFamilyPublicWireAsyncOrIndependentProof:false}));
