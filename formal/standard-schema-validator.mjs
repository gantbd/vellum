// Verification plumbing only. No production loader or generic engine schema API.
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';
export function validateSchemaReply(out,cases){
 if(out.error||out.signal||out.status!==0)throw Error('SCHEMA_TOOL_FAILURE');
 let values;try{values=JSON.parse(out.stdout);}catch{throw Error('SCHEMA_PROTOCOL_FAILURE');}
 if(!Array.isArray(values)||values.length!==cases.length||values.some(v=>typeof v!=='boolean'))throw Error('SCHEMA_PROTOCOL_FAILURE');
 for(let i=0;i<values.length;i++)assert.equal(values[i],cases[i].valid,'standard schema value case '+i);
 return values.length;
}
export function validateStandardSchemas(cases){
 const python=process.env.VELLUM_JSONSCHEMA_PYTHON;if(!python)throw Error('SCHEMA_TOOL_FAILURE: configure the standard validator; no fallback');
 if(!Array.isArray(cases)||cases.some(c=>!c||typeof c.valid!=='boolean'||!Object.hasOwn(c,'schema')||!Object.hasOwn(c,'value')))throw Error('SCHEMA_PROTOCOL_FAILURE: invalid fixture cases');
 // One explicit UTF-8 frame: completion does not depend on pipe EOF delivery.
 const payload=Buffer.from(JSON.stringify(cases),'utf8'),out=spawnSync(python,['-c','import json,sys,jsonschema\nsize=int(sys.argv[1])\ndata=sys.stdin.buffer.read(size)\nassert len(data)==size\ncases=json.loads(data)\nvalues=[]\nfor c in cases:\n jsonschema.Draft202012Validator.check_schema(c["schema"])\n values.append(jsonschema.Draft202012Validator(c["schema"]).is_valid(c["value"]))\nprint(json.dumps(values))',String(payload.length)],{input:payload,encoding:'utf8',maxBuffer:1048576,env:{...process.env,PYTHONDONTWRITEBYTECODE:'1'}});
 return validateSchemaReply(out,cases);
}
export function checkSchemaValidatorProtocol(){const one=[{valid:true}];for(const out of [{status:1,stdout:'[true]'},{status:null,signal:'SIGTERM',stdout:''}])assert.throws(()=>validateSchemaReply(out,one),e=>e.code!=='ERR_ASSERTION'&&e.message==='SCHEMA_TOOL_FAILURE');for(const stdout of ['invalid','[]','["true"]'])assert.throws(()=>validateSchemaReply({status:0,stdout},one),e=>e.code!=='ERR_ASSERTION'&&e.message==='SCHEMA_PROTOCOL_FAILURE');assert.throws(()=>validateSchemaReply({status:0,stdout:'[false]'},one),{code:'ERR_ASSERTION'});assert.equal(validateSchemaReply({status:0,stdout:'[true]'},one),1);return 7;}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){const controls=checkSchemaValidatorProtocol(),standardCases=validateStandardSchemas([{schema:{type:'integer'},value:1,valid:true},{schema:{type:'integer'},value:'1',valid:false},{schema:{type:'string'},value:'é🙂',valid:true}]);console.log(JSON.stringify({scope:'standard-schema-tool-protocol-boundary',controls,standardCases,toolAndProtocolFailuresAreNotSemanticAssertions:true,byteCountedUtf8Frame:true,physicalHostOrIndependentProof:false}));}
