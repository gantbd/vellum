import assert from 'node:assert/strict';import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';import {fileURLToPath} from 'node:url';import {captureInputs,createCaptureScope,releaseCaptureScope} from './evidence-harness.mjs';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),descriptor='formal/read-engine-descriptor.mjs',native='formal/native-discovery.mjs',rules=[
 ['result-fields-open',descriptor,'record(fields,Object.fromEntries(fields.map(k=>[k,properties[k]])))','({...record(fields,Object.fromEntries(fields.map(k=>[k,properties[k]]))),additionalProperties:true})'],
 ['result-numeric-fractions-described',descriptor,"natural={type:'integer',minimum:0","natural={type:'number',minimum:0"],
 ['negative-result-naturals-described',descriptor,"natural={type:'integer',minimum:0","natural={type:'integer',minimum:-1"],
 ['fractional-source-bytes-described',descriptor,"bytes={type:'array',items:{type:'integer',minimum:0,maximum:255}}","bytes={type:'array',items:{type:'number',minimum:0,maximum:255}}"],
 ['source-byte-ceiling-widened',descriptor,"bytes={type:'array',items:{type:'integer',minimum:0,maximum:255}}","bytes={type:'array',items:{type:'integer',minimum:0,maximum:256}}"],
 ['negative-source-bytes-described',descriptor,"bytes={type:'array',items:{type:'integer',minimum:0,maximum:255}}","bytes={type:'array',items:{type:'integer',minimum:-1,maximum:255}}"],
 ['range-tuple-too-wide',descriptor,'range:{type:\'array\',items:natural,minItems:2,maxItems:2}','range:{type:\'array\',items:natural,minItems:2,maxItems:3}'],
 ['range-tuple-too-short',descriptor,'range:{type:\'array\',items:natural,minItems:2,maxItems:2}','range:{type:\'array\',items:natural,minItems:1,maxItems:2}'],
 ['bare-CR-delimiter-omitted',descriptor,'[[],[10],[13],[13,10]]','[[],[10],[13,10]]'],
 ['CRLF-delimiter-reversed',descriptor,'[[],[10],[13],[13,10]]','[[],[10],[13],[10,13]]'],
 ['source-unit-reclassified',descriptor,'unit:{const:source.unit}',"unit:{const:'utf16-code-unit'}"],
 ['source-reference-authority-is-numeric',descriptor,'ref=map(source.refFields,{authority:text,document:text,incarnation:natural,revision:natural})','ref=map(source.refFields,{authority:natural,document:text,incarnation:natural,revision:natural})'],
 ['source-unavailable-product-omitted',descriptor,"branches=[map(source.fields[status],properties),map(source.fields.unavailable,{status:{const:'unavailable'}})]","branches=[map(source.fields[status],properties)]"],
 ['withheld-body-made-required',descriptor,'record(values.recordRequired,{reference:opaque,content,location})',"record([...values.recordRequired,'content'],{reference:opaque,content,location})"],
 ['withheld-location-made-required',descriptor,'record(values.recordRequired,{reference:opaque,content,location})',"record([...values.recordRequired,'location'],{reference:opaque,content,location})"],
 ['annotation-reference-made-optional',descriptor,'record(values.recordRequired,{reference:opaque,content,location})','record([],{reference:opaque,content,location})'],
 ['annotation-reference-described-as-text',descriptor,'properties:{reference:opaque,content,location}','properties:{reference:text,content,location}'],
 ['null-body-facet-described',descriptor,"content=map(values.content,{body:{type:'string'},version:natural})","content={anyOf:[map(values.content,{body:{type:'string'},version:natural}),{type:'null'}]}"],
 ['null-location-facet-described',descriptor,'location=map(values.location,{placement,targetGeneration:natural})',"location={anyOf:[map(values.location,{placement,targetGeneration:natural}),{type:'null'}]}"],
 ['detached-placement-requires-target',descriptor,"map(values.detachedPlacement,{state:{const:'detached'}})","record(['state','target'],{state:{const:'detached'},target})"],
 ['affected-placement-state-omitted',descriptor,'[...values.attachedStates]',"values.attachedStates.filter(s=>s!=='affected')"],
 ['content-point-profile-omitted',descriptor,'[...targets.pointProfiles]',"targets.pointProfiles.filter(p=>p!=='content')"],
 ['after-point-affinity-omitted',descriptor,'[...targets.pointAffinities]',"targets.pointAffinities.filter(a=>a!=='after')"],
 ['serialized-row-equality-substitutes-for-alias-identity',descriptor,"records:{type:'array',items:annotationRecord}","records:{type:'array',items:annotationRecord,uniqueItems:true}"],
 ['empty-annotation-body-refused',descriptor,"content=map(values.content,{body:{type:'string'},version:natural})","content=map(values.content,{body:{type:'string',minLength:1},version:natural})"],
 ['duplicate-acquired-classes-described',descriptor,'maxItems:2,uniqueItems:true','maxItems:2,uniqueItems:false'],
 ['empty-acquired-class-set-described',descriptor,"items:{enum:['source','annotations']},minItems:1","items:{enum:['source','annotations']},minItems:0"],
 ['empty-coherent-result-described',descriptor,"participants:{type:'array',minItems:1,items:record(","participants:{type:'array',minItems:0,items:record("],
 ['zero-document-incarnation-described',descriptor,'incarnation:positive','incarnation:natural'],
 ['coherent-member-alias-described-as-text',descriptor,"record(['document','snapshot','data'],{document,snapshot:opaque,data:classes})","record(['document','snapshot','data'],{document,snapshot:text,data:classes})"],
 ['single-context-described-as-text',descriptor,"map(single.context,{status:{const:'context'},context:opaque,data:classes})","map(single.context,{status:{const:'context'},context:text,data:classes})"],
 ['result-recognition-not-exact',descriptor,'export function readResultMetadataValid(value,contract,operation){const expected=readResultMetadata(contract,operation);return expected!==null&&equal(value,expected);}','export function readResultMetadataValid(value,contract,operation){const expected=readResultMetadata(contract,operation);return expected!==null;}'],
 ['schema-claims-token-authority',descriptor,"coverage:'native-value-structure-only'","coverage:'token-authentication-proved'"],
 ['native-owner-binding-contract-omitted',descriptor,"'actual-native-owner-and-current-authority',",''],
 ['wire-codec-limit-omitted',descriptor,"'public-wire-codec',",''],
 ['result-metadata-enters-older-builtins',descriptor,"outcomes:{statusVocabulary:statuses,effect:","outcomes:{statusVocabulary:statuses,result:null,effect:"],
 ['profile-six-input-meaning-disabled',native,'if(profile===catalog.readResultSchemaProfile)return inputValid(input,contract,variant,catalog.readSchemaProfile);','if(profile===catalog.readResultSchemaProfile)return false;'],
 ['profile-six-enrichment-disabled',native,'if(c.profile===catalog.readResultSchemaProfile){','if(false){'],
 ['document-result-handler-missing',native,'[documentReadResultEngineDescriptor(),annotationReadResultEngineDescriptor()]','[null,annotationReadResultEngineDescriptor()]'],
 ['annotation-result-handler-missing',native,'[documentReadResultEngineDescriptor(),annotationReadResultEngineDescriptor()]','[documentReadResultEngineDescriptor(),null]'],
 ['native-result-metadata-recognition-omitted',native,'readResultMetadataValid(v.outcomes.result,r.contract,v.id)','true'],
 ['foreign-authority-scope-admitted',native,'&&c.authority===m.authority',''],
 ['foreign-engine-scope-admitted',native,'&&c.engine===m.engine',''],
 ['result-entry-capacity-overaccepted',native,'rows.length<=l.maxEntries','rows.length<=l.maxEntries+1'],
 ['result-variant-capacity-overaccepted',native,'r.variants.length<=l.maxVariants','r.variants.length<=l.maxVariants+1'],
 ['result-row-byte-bound-overaccepted',native,'size(r)<=l.maxRowBytes','size(r)<=l.maxRowBytes+1'],
 ['result-metadata-byte-bound-overaccepted',native,'size(v)<=l.maxMetadataBytes','size(v)<=l.maxMetadataBytes+1'],
 ['result-metadata-depth-overaccepted',native,'depth>l.maxMetadataDepth','depth>l.maxMetadataDepth+1'],
 ['result-metadata-node-bound-overaccepted',native,'--budget.left<0','--budget.left< -1'],
 ['result-packet-byte-bound-overaccepted',native,'size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes','size({root,scope:c,registry,visibility,target,advice})>m.limits.maxPacketBytes+1'],
 ['result-reply-byte-bound-overaccepted',native,'if(queryReplyBytes>m.limits.maxReplyBytes)return no();','if(queryReplyBytes>m.limits.maxReplyBytes+1)return no();'],
 ['result-advice-facts-bypassed',native,'!gates(g,catalog.adviceGates)','false'],
 ['result-advice-target-copy-admitted',native,'request.target!==s.target.reference','false'],
 ['result-schema-ceiling-expanded',native,'selected.schema&&f.schema?','f.schema?'],
 ['result-metadata-origin-borrowed',native,'if((p.builtin===true)!==(s.builtin===true))return no();','if(false)return no();']
];
const bankCount=6,bank=Number(process.argv.find(a=>a.startsWith('--bank='))?.slice(7)??0);if(!Number.isInteger(bank)||bank<0||bank>=bankCount)throw Error('unsupported read-result mutation bank');
for(const[name,relative,from]of rules){const raw=fs.readFileSync(path.join(root,relative),'utf8');assert.equal(raw.split(from).length-1,1,name+': construction target');}
if(process.argv.includes('--list'))console.log(JSON.stringify({rules:rules.length,bankCount,banks:Array.from({length:bankCount},(_,n)=>rules.filter((_,i)=>i%bankCount===n).map(r=>r[0]))}));
else{const scope=createCaptureScope(process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE||undefined),keys=['VELLUM_TRANSIENT_PROOF_CAPTURES','VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE'],saved=Object.fromEntries(keys.map(k=>[k,process.env[k]]));let passed=false;
 try{process.env.VELLUM_TRANSIENT_PROOF_CAPTURES='1';process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE=scope.directory;const run=d=>spawnSync(process.execPath,[path.join(d,'formal/read-result-metadata-checks.mjs')],{encoding:'utf8',maxBuffer:2097152,env:process.env}),positive=captureInputs(root),baseline=run(positive.directory);assert.equal(baseline.status,0,baseline.stderr);const mutations=[];
  for(const[name,relative,from,to]of rules.filter((_,i)=>i%bankCount===bank)){const graph=captureInputs(root),file=path.join(graph.directory,relative),raw=fs.readFileSync(file,'utf8');fs.writeFileSync(file,raw.replace(from,to));const out=run(graph.directory);assert.equal(out.status,1,name+': '+out.stderr.slice(0,1300));assert(!out.stderr.includes('SCHEMA_TOOL_FAILURE')&&!out.stderr.includes('SCHEMA_PROTOCOL_FAILURE'),name+': verifier tool/protocol failure');assert(out.stderr.includes('ERR_ASSERTION'),name+': '+out.stderr.slice(0,1300));assert(!out.stderr.includes('SyntaxError'),name);mutations.push(name);}
  passed=true;console.log(JSON.stringify({scope:'isolated-read-result-metadata-sensitivity',bank,bankCount,totalMutations:rules.length,baseline:JSON.parse(baseline.stdout),mutations,coherentCopiedGraphBaseline:true,constructionFailuresCounted:false,allPublicProductsPhysicalHostOrIndependentProof:false}));
 }finally{releaseCaptureScope(scope,passed);for(const k of keys)if(saved[k]===undefined)delete process.env[k];else process.env[k]=saved[k];}
}
