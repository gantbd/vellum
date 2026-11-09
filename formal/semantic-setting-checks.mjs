import fs from 'node:fs';
import assert from 'node:assert/strict';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import * as original from './semantic-settings.mjs';
const copy=structuredClone;
const context={authority:'a',engine:'engine',recipient:'agent',purpose:'configure',destination:'local',profile:'execution',continuity:'c'};
const limits={maxChanges:8,maxDependencies:8,maxRequestBytes:65536,maxValueBytes:32768,maxIdentifierBytes:128,maxInputNodes:4096,maxInputDepth:64};
const layer={authority:'a',id:'document-profile',incarnation:1};
const change=(key='registered-semantic-key',operation='set',value='literal')=>({layer:copy(layer),expectedGeneration:1,key,operation,value});
const envelope=(route='typed',changes=[change()])=>({route,request:{...(route==='typed'?{family:15}:{name:'activate-context'}),variant:'semantic-settings',version:1,
  arguments:{contract:'semantic-settings-activation/1',operation:'semantic-settings',invocation:'invoke',profile:'execution',changes,dependencies:[]}}});
function oracle(v){
  const children=v!==null&&typeof v==='object'?Object.values(v):[];
  const rows=children.map(oracle);
  return {bytes:Buffer.byteLength(JSON.stringify(v)),nodes:1+rows.reduce((n,r)=>n+r.nodes,0),depth:rows.length?1+Math.max(...rows.map(r=>r.depth)):0};
}
export function checkSemanticSettings(api=original){
  const controls=[],control=(name,run)=>{run();controls.push(name);};let products=0,byteCases=0,unicodeByteCases=0,rejectedSurrogates=0;
  const read=(e=envelope(),l=limits,c=context)=>api.normalizeSemanticSettings(e,c,l);
  const reject=(name,changeRequest)=>control(name,()=>{const e=envelope();changeRequest(e);assert.equal(read(e),null);});
  const values=[null,true,false,0,-0,1.25,'é🙂\n\t\v"\\',[],[1,{text:'literal'}],JSON.parse('{"__proto__":{"literal":true}}')];
  for(const route of ['typed','dynamic'])for(const value of values){
    const e=envelope(route,[change('key','set',value)]),before=copy(e),out=read(e);assert(out);assert(equal(e,before));assert(equal(out.arguments,e.request.arguments));
    assert.equal(out.requestBytes,oracle(e).bytes);assert.equal(out.valueBytes,oracle(value).bytes);products++;
  }
  for(const route of ['typed','dynamic'])for(const operation of ['remove','reset']){const out=read(envelope(route,[change('key',operation,null)]));assert(out);assert.equal(out.arguments.changes[0].operation,operation);products++;}
  control('typed-dynamic-one-semantic-intent',()=>assert(equal(read(envelope('typed')).arguments,read(envelope('dynamic')).arguments)));
  control('owned-frozen-copy-not-caller-state',()=>{const e=envelope(),out=read(e);e.request.arguments.changes[0].value='later';assert.equal(out.arguments.changes[0].value,'literal');assert(Object.isFrozen(out.arguments)&&Object.isFrozen(out.arguments.changes)&&Object.isFrozen(out.arguments.changes[0].layer));});
  control('unknown-key-is-data-not-semantic-acceptance',()=>assert(read(envelope('typed',[change('not-a-registered-engine-key')]))));
  control('change-and-dependency-order-preserved',()=>{const e=envelope('typed',[change('z'),change('a')]);e.request.arguments.dependencies=[{domain:'z',identity:'i',generation:1},{domain:'a',identity:'i',generation:1}];assert(equal(read(e).arguments,e.request.arguments));});
  control('distinct-keys-one-layer-same-generation',()=>assert(read(envelope('typed',[change('a'),change('b')]))));
  control('empty-change-read-set-needs-no-value-budget',()=>assert(read(envelope('typed',[]),{...limits,maxChanges:0,maxDependencies:0,maxValueBytes:0})));
  control('remove-reset-need-no-set-value-budget',()=>assert(read(envelope('typed',[change('a','remove',null),change('b','reset',null)]),{...limits,maxValueBytes:0})));
  control('negative-zero-native-value-preserved',()=>assert(Object.is(read(envelope('typed',[change('key','set',-0)])).arguments.changes[0].value,-0)));
  for(const field of ['keymaps','mode','focus','pendingChord'])reject('ui-field-'+field,e=>{e.request.arguments[field]=[];});
  reject('old-combined-product',e=>{e.request.arguments.contract='settings-activation/1';e.request.arguments.operation='settings-keymap';});
  reject('presentation-route',e=>{e.request.family=14;e.request.variant='scroll';});
  reject('different-engine-family-cannot-reuse-arguments',e=>{e.request.family=10;e.request.variant='exact-batch';});
  reject('unknown-product-field',e=>{e.request.extra=true;});
  reject('unknown-argument-field',e=>{e.request.arguments.extra=true;});
  reject('unknown-operation',e=>{e.request.arguments.changes[0].operation='execute';});
  reject('foreign-authority',e=>{e.request.arguments.changes[0].layer.authority='other';});
  reject('missing-incarnation',e=>{delete e.request.arguments.changes[0].layer.incarnation;});
  reject('zero-incarnation',e=>{e.request.arguments.changes[0].layer.incarnation=0;});
  reject('zero-generation',e=>{e.request.arguments.changes[0].expectedGeneration=0;});
  reject('unsafe-generation',e=>{e.request.arguments.changes[0].expectedGeneration=Number.MAX_SAFE_INTEGER+1;});
  reject('profile-not-bound-context',e=>{e.request.arguments.profile='other';});
  reject('duplicate-change',e=>{e.request.arguments.changes.push(copy(e.request.arguments.changes[0]));});
  reject('one-layer-contradictory-generations',e=>{const c=change('other');c.expectedGeneration=2;e.request.arguments.changes.push(c);});
  reject('non-null-remove',e=>{e.request.arguments.changes[0].operation='remove';});
  reject('non-null-reset',e=>{e.request.arguments.changes[0].operation='reset';});
  reject('duplicate-dependency',e=>{const d={domain:'d',identity:'i',generation:1};e.request.arguments.dependencies=[d,copy(d)];});
  reject('incompatible-dependency',e=>{e.request.arguments.dependencies=[{domain:'d',identity:'i',generation:1},{domain:'d',identity:'i',generation:2}];});
  reject('identifier-byte-not-code-unit-limit',e=>{e.request.arguments.changes[0].key='é'.repeat(65);});
  reject('unpaired-surrogate',e=>{e.request.arguments.changes[0].value='\ud800';});
  control('aggregate-set-value-cap',()=>assert.equal(read(envelope('typed',[change('a','set','123'),change('b','set','456')]),{...limits,maxValueBytes:9}),null));
  control('zero-value-budget-rejects-set',()=>assert.equal(read(envelope(),{...limits,maxValueBytes:0}),null));
  for(const field of ['maxChanges','maxDependencies'])control('count-cap-'+field,()=>{const e=envelope();if(field==='maxDependencies')e.request.arguments.dependencies=[{domain:'d',identity:'i',generation:1}];assert.equal(read(e,{...limits,[field]:0}),null);});
  control('request-byte-boundary',()=>{const e=envelope(),size=oracle(e).bytes;assert(read(e,{...limits,maxRequestBytes:size}));assert.equal(read(e,{...limits,maxRequestBytes:size-1}),null);});
  control('value-node-depth-boundaries',()=>{for(const v of values){const o=oracle(v),l={...limits,maxRequestBytes:o.bytes,maxInputNodes:o.nodes,maxInputDepth:o.depth};const out=api.copySettingsData(v,l);assert(out&&equal(out.value,v));assert.equal(out.bytes,o.bytes);assert.equal(out.nodes,o.nodes);assert.equal(api.copySettingsData(v,{...l,maxRequestBytes:o.bytes-1}),null);if(o.depth)assert.equal(api.copySettingsData(v,{...l,maxInputDepth:o.depth-1}),null);assert.equal(api.copySettingsData(v,{...l,maxInputNodes:o.nodes-1}),null);byteCases++;}});
  control('unicode-string-byte-oracle',()=>{
    const l={...limits,maxRequestBytes:64,maxInputNodes:1,maxInputDepth:0};
    const check=v=>{const size=Buffer.byteLength(JSON.stringify(v)),out=api.copySettingsData(v,l);assert(out);assert.equal(out.value,v);assert.equal(out.bytes,size);assert.equal(api.copySettingsData(v,{...l,maxRequestBytes:size-1}),null);unicodeByteCases++;};
    for(let n=0;n<=0xffff;n++){const v=String.fromCharCode(n);if(n>=0xd800&&n<=0xdfff){assert.equal(api.copySettingsData(v,l),null);rejectedSurrogates++;}else check(v);}
    for(const n of [0x10000,0x1f600,0x40000,0x10ffff])check(String.fromCodePoint(n));
  });
  control('accessors-not-invoked',()=>{let calls=0;const e=envelope();Object.defineProperty(e.request.arguments.changes[0],'value',{enumerable:true,get(){calls++;return 'bad';}});assert.equal(read(e),null);assert.equal(calls,0);});
  control('unsupported-data-prototypes-and-scalars',()=>{for(const v of [new Date(),new Map(),1n,undefined,()=>{},Symbol('s'),NaN,Infinity])assert.equal(api.copySettingsData(v,limits),null);});
  control('symbols-and-nonenumerable-fields-rejected',()=>{for(const enumerable of [true,false]){const v={};Object.defineProperty(v,enumerable?Symbol('s'):'hidden',{value:1,enumerable});assert.equal(api.copySettingsData(v,limits),null);}});
  control('sparse-or-extended-array-rejected',()=>{const a=Array(2),b=[];b.extra=true;assert.equal(api.copySettingsData(a,limits),null);assert.equal(api.copySettingsData(b,limits),null);});
  control('cycles-rejected-aliases-copied',()=>{const cycle={};cycle.self=cycle;assert.equal(api.copySettingsData(cycle,limits),null);const shared={value:1},out=api.copySettingsData({a:shared,b:shared},limits);assert(out&&out.value.a!==out.value.b&&equal(out.value.a,out.value.b));});
  control('deep-profile-needs-no-recursive-runtime-copy',()=>{let v=1;for(let i=0;i<10000;i++)v={x:v};const l={...limits,maxInputDepth:10020,maxInputNodes:20000,maxRequestBytes:100000,maxValueBytes:100000};const out=read(envelope('typed',[change('deep','set',v)]),l);assert(out);assert.equal(out.valueBytes,60001);let at=out.arguments.changes[0].value;for(let i=0;i<10000;i++)at=at.x;assert.equal(at,1);assert(!Object.isFrozen(v));});
  control('limits-are-explicit-finite-profile',()=>{for(const field of Object.keys(limits))for(const value of [-1,Infinity,NaN,1.5])assert.equal(read(envelope(),{...limits,[field]:value}),null);});
  return {status:'bounded-engine-semantic-settings-arguments-only',products,byteCases,unicodeByteCases,rejectedSurrogates,controls,deepLevels:10000,actualRegistryOrPolicyTruthProved:false,nativePublicationProved:false,hostileWireOrProxyDecoderProved:false,physicalAccountingProved:false,implementationRefinement:false};
}
export async function checkSemanticSettingMutations(){
  const source=fs.readFileSync(new URL('./semantic-settings.mjs',import.meta.url),'utf8');
  const load=async(code,name)=>{
    code=code.replace("'./engine-facade.mjs'",JSON.stringify(new URL('./engine-facade.mjs',import.meta.url).href))
      .replace("new URL('./semantic-settings.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./semantic-settings.json',import.meta.url).href)})`);
    return import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
  };
  checkSemanticSettings(await load(source,'baseline'));
  const mutants=[
    ['ui-argument-fields-ignored','!exact(a,C.argumentFields)','false'],
    ['caller-argument-alias','arguments:a,context:c','arguments:envelope.request.arguments,context:c'],
    ['foreign-layer-accepted','change.layer.authority!==c.authority','false'],
    ['duplicate-change-accepted','changes.has(key)','false'],
    ['byte-cap-ignored','n<=maximum-bytes','true'],
    ['depth-cap-ignored','task.depth>maxDepth','false'],
    ['set-value-cap-ignored','maxRequestBytes:limits.maxValueBytes-valueBytes','maxRequestBytes:limits.maxRequestBytes'],
    ['other-engine-family-confused',"if(decoded?.descriptor.id!==C.family||decoded.variant!==C.operation)return null;",'if(!decoded)return null;'],
    ['duplicate-dependency-accepted','if(dependencies.has(key))return null;','if(false)return null;'],
    ['non-null-removal-accepted',"(change.operation!=='set'&&change.value!==null)",'false'],
    ['zero-layer-incarnation-accepted','!version(change.layer.incarnation)','false']
  ],killed=[];
  for(const [name,from,to] of mutants){assert(source.includes(from),name);const module=await load(source.replace(from,to),name);assert.throws(()=>checkSemanticSettings(module),{code:'ERR_ASSERTION'},name);killed.push(name);}
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkSemanticSettings(),mutationBaselineChecked:true,mutations:await checkSemanticSettingMutations()},null,2));
