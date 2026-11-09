// Normalized discovery projection; actual registration/schema/visibility are premises.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {decodeDynamic,planRoute} from './operation-catalog.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./contract-discovery.json',import.meta.url),'utf8'));
const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
const copy=structuredClone;
const bytes=v=>Buffer.byteLength(JSON.stringify(v),'utf8');
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const unavailable=()=>({status:'unavailable'});
const gates=['granted','retained','live','continuous','transferReady','registryCurrent'];

export function captureDiscovery(registry,binding,limits){
  if(registry.length>limits.maxEntries||registry.some(r=>r.variants.length>limits.maxVariants||bytes(r)>limits.maxInputRowBytes))return null;
  const rows=[];
  for(const entry of registry){
    if(!entry.visible)continue;
    const family=families.find(f=>f.id===entry.family);
    if(!family||entry.kind!==family.kind||entry.version!==1||
        entry.variants.some(v=>!family.variants.includes(v.name)||!exact(v.input,['contract','variant','fields'])||
          v.input.contract!==entry.contract||v.input.variant!==v.name||typeof v.enabled!=='boolean')||
        Object.keys(entry.presentation).some(k=>!catalog.presentation.includes(k)))return null;
    const variants=entry.variants.filter(v=>v.visible).map(v=>({reference:v.reference,
      ...(entry.schemaVisible?{input:copy(v.input),outcomes:copy(v.outcomes),requirements:copy(v.requirements),limits:v.limits,cancellation:v.cancellation}:{}),
      ...(entry.adviceVisible?{enabled:v.enabled}:{} )}));
    if(new Set(variants.map(v=>v.reference)).size!==variants.length)return null;
    rows.push({reference:entry.reference,version:entry.version,kind:entry.kind,variants,
      ...(entry.presentationVisible?{presentation:copy(entry.presentation)}:{})});
  }
  rows.sort((a,b)=>a.reference<b.reference?-1:a.reference>b.reference?1:0);
  if(new Set(rows.map(r=>r.reference)).size!==rows.length||rows.some(r=>bytes(r)>limits.maxRowBytes)||bytes(rows)>limits.maxCaptureBytes)return null;
  return freeze({binding:copy(binding),limits:copy(limits),rows:copy(rows)});
}

export function readDiscovery(capture,request,current,cursors=[]){
  if(!capture||!request||!Object.hasOwn(catalog.variants,request.operation)||
      !exact(request,[...catalog.common,...catalog.variants[request.operation]])||request.contract!==catalog.contract)return unavailable();
  const {binding,rows,limits}=capture;
  if(request.context!==binding.context||request.profile!==binding.profile||!equal(current.binding,binding)||!gates.every(k=>current[k]))return unavailable();
  if(request.operation==='list'){
    if(!Number.isSafeInteger(request.limit)||request.limit<1||request.limit>limits.maxPageRows)return unavailable();
    let start=0;
    if(request.continuation!==null){
      const found=cursors.filter(c=>c.selector===request.continuation);
      if(found.length!==1)return unavailable();const c=found[0];
      if(!equal(c.binding,binding)||c.limit!==request.limit||!Number.isSafeInteger(c.next)||c.next<1||c.next>=rows.length)return unavailable();
      start=c.next;
    }
    const end=Math.min(start+request.limit,rows.length);
    return {status:'page',context:binding.context,entries:rows.slice(start,end).map(r=>({reference:r.reference,version:r.version,kind:r.kind})),next:end<rows.length?end:null};
  }
  const row=rows.find(r=>r.reference===request.entry);if(!row)return unavailable();
  if(request.operation==='inspect'){
    // Enablement has its own target/current-state product, never stale descriptor data.
    const descriptor=copy(row);descriptor.variants=descriptor.variants.map(({enabled,...v})=>v);
    return {status:'descriptor',context:binding.context,descriptor};
  }
  const variant=row.variants.find(v=>v.reference===request.variant);
  if(!variant||!Object.hasOwn(variant,'enabled')||request.target!==binding.target||!current.targetCurrent)return unavailable();
  return {status:'advice',context:binding.context,target:binding.target,entry:row.reference,variant:variant.reference,enabled:variant.enabled};
}

export function checkContractDiscovery(){
  assert.deepEqual(Object.keys(catalog.variants).sort(),families.find(f=>f.id==='F01').variants.toSorted());
  const annotation=JSON.parse(fs.readFileSync(new URL('./annotation-operations.json',import.meta.url),'utf8'));
  const reversal=JSON.parse(fs.readFileSync(new URL('./history-reversal.json',import.meta.url),'utf8'));
  assert.deepEqual(Object.keys(reversal.fieldTypes).sort(),reversal.common.toSorted());
  const entry=(family,reference,product,variants)=>({family,reference,contract:product.contract,version:1,kind:'localCommand',visible:true,schemaVisible:true,presentationVisible:true,adviceVisible:true,
    presentation:{title:reference,description:'Literal descriptive text',category:'Editing',aliases:['execute-body'],bindings:['host-defined']},
    variants:variants.map(name=>({name,reference:`variant-${name}`,visible:true,enabled:true,input:{contract:product.contract,variant:name,
      fields:copy(Array.isArray(product.common)?product.fieldTypes:{...product.common,...product.variants[name]})},
      outcomes:Array.isArray(product.results)?copy(product.results):Object.keys(product.results),requirements:['current-write-authority'],limits:'bounded-profile',cancellation:'profile-bound'}))});
  const registry=[entry('F14','entry-b',reversal,reversal.variants),entry('F13','entry-a',annotation,Object.keys(annotation.variants))];
  const binding={context:'discovery-context',profile:'p',recipient:'agent',purpose:'discover',destination:'client',policy:'epoch1',continuity:'c1',registry:'registry-cut',target:'advisory-target'};
  const limits={maxEntries:8,maxVariants:8,maxInputRowBytes:8192,maxRowBytes:8192,maxCaptureBytes:16384,maxPageRows:2};
  const cap=captureDiscovery(registry,binding,limits),current={binding:copy(binding),...Object.fromEntries(gates.map(k=>[k,true])),targetCurrent:true};
  const common={contract:catalog.contract,context:binding.context,profile:'p'};
  const list={...common,operation:'list',limit:1,continuation:null};
  const inspect={...common,operation:'inspect',entry:'entry-a'};
  const advice={...common,operation:'advisory-enablement',entry:'entry-a',variant:'variant-create',target:binding.target};
  const requests=[list,inspect,advice],controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  const deny=(name,r=list,c=current,capture=cap,cursors=[])=>control(name,equal(readDiscovery(capture,r,c,cursors),unavailable()));
  let releaseCases=0,pairedCases=0,traversalCases=0;
  for(const r of requests)for(let mask=0;mask<64;mask++){
    const c={...copy(current),...Object.fromEntries(gates.map((k,i)=>[k,!!(mask&(1<<i))]))},before=copy({r,c,cap}),out=readDiscovery(cap,r,c);
    assert.equal(out.status,mask===63?({list:'page',inspect:'descriptor','advisory-enablement':'advice'})[r.operation]:'unavailable');
    assert.deepEqual({r,c,cap},before);releaseCases++;
  }
  for(let projection=0;projection<8;projection++)for(let secret=0;secret<8;secret++){
    const visible=copy(registry);visible[1].schemaVisible=!!(projection&1);visible[1].presentationVisible=!!(projection&2);visible[1].adviceVisible=!!(projection&4);
    const hidden={...copy(registry[0]),reference:`private-${secret}`,visible:false};hidden.presentation.description='secret'.repeat(secret);
    const l=captureDiscovery(visible,binding,limits),r=captureDiscovery([hidden,...visible],binding,limits);
    assert.deepEqual(l,r);
    for(const q of requests){assert.deepEqual(readDiscovery(l,q,current),readDiscovery(r,q,current));pairedCases++;}
  }
  for(let n=0;n<=8;n++)for(let limit=1;limit<=2;limit++){
    const entries=Array.from({length:n},(_,i)=>({...copy(registry[0]),reference:`entry-${i}`})),captured=captureDiscovery(entries,binding,limits);
    const seen=[],cursors=[];let continuation=null,previous=0;
    for(let page=0;page<=n;page++){
      const out=readDiscovery(captured,{...list,limit,continuation},current,cursors);assert.equal(out.status,'page');seen.push(...out.entries);
      if(out.next===null)break;assert(out.next>previous&&out.next<n);previous=out.next;
      continuation=`opaque-${page}`;cursors.push({selector:continuation,binding:copy(binding),limit,next:out.next});
    }
    assert.deepEqual(seen,captured.rows.map(r=>({reference:r.reference,version:r.version,kind:r.kind})));traversalCases++;
  }
  const hiddenVariant=copy(registry);hiddenVariant[1].variants.at(-1).visible=false;
  const hiddenCapture=captureDiscovery(hiddenVariant,binding,limits);
  for(let secret=0;secret<8;secret++){
    const changed=copy(hiddenVariant),v=changed[1].variants.at(-1);
    v.reference=`private-variant-${secret}`;v.enabled=!!(secret&1);v.requirements=[`private-requirement-${secret}`];
    const other=captureDiscovery(changed,binding,limits);assert.deepEqual(other,hiddenCapture);
    for(const r of requests){assert.deepEqual(readDiscovery(other,r,current),readDiscovery(hiddenCapture,r,current));pairedCases++;}
  }
  deny('hidden-variant-not-addressable',{...advice,variant:'variant-delete'},current,hiddenCapture);
  const descriptor=readDiscovery(cap,inspect,current).descriptor;
  control('exact-native-contract-shape-reference',descriptor.variants[0].input.contract===annotation.contract&&descriptor.variants[0].input.variant==='create');
  control('selected-native-field-domains-described',equal(descriptor.variants[0].input.fields,{...annotation.common,...annotation.variants.create}));
  control('descriptors-do-not-embed-stale-advice',descriptor.variants.every(v=>!Object.hasOwn(v,'enabled')));
  control('advice-has-bound-target',equal(readDiscovery(cap,advice,current),{status:'advice',context:binding.context,target:binding.target,entry:'entry-a',variant:'variant-create',enabled:true}));
  const decoded=decodeDynamic(families,{name:'author-annotations',variant:'create',version:1});
  const route={schemaValid:true,enabled:true,grant:false,current:true,ordinaryCapacity:true,releaseAllowed:true};
  control('positive-advice-is-not-current-grant',readDiscovery(cap,advice,current).enabled===true&&planRoute(decoded,route).status==='denied');
  control('positive-advice-is-not-current-dependencies',planRoute(decoded,{...route,grant:true,current:false}).status==='stale');
  control('alias-is-not-an-executable-registration',decodeDynamic(families,{name:'execute-body',variant:'create',version:1})===null);
  deny('stale-advisory-target',advice,{...current,targetCurrent:false});
  control('target-change-does-not-stale-registry-list',readDiscovery(cap,list,{...current,targetCurrent:false}).status==='page');
  deny('wrong-advisory-target',{...advice,target:'other'});
  for(const key of Object.keys(binding))deny(`current-${key}`,list,{...current,binding:{...binding,[key]:'other'}});
  deny('unknown-contract',{...list,contract:'contract-discovery/2'});
  deny('unknown-variant',{...list,operation:'execute'});
  deny('unknown-field',{...list,callback:'run'});
  deny('missing-field',Object.fromEntries(Object.entries(list).filter(([k])=>k!=='context')));
  deny('unknown-entry',{...inspect,entry:'hidden'});
  deny('unknown-entry-variant',{...advice,variant:'variant-missing'});
  deny('raw-family-not-entry-reference',{...inspect,entry:'F13'});
  const noAdvice=copy(registry);noAdvice[1].adviceVisible=false;
  deny('withheld-advice-not-false',advice,current,captureDiscovery(noAdvice,binding,limits));
  const disabled=copy(registry);disabled[1].variants[0].enabled=false;
  control('established-disabled-distinct-from-withheld',readDiscovery(captureDiscovery(disabled,binding,limits),advice,current).enabled===false);
  const narrow=copy(registry);narrow[1].schemaVisible=false;narrow[1].presentationVisible=false;
  const limited=readDiscovery(captureDiscovery(narrow,binding,limits),inspect,current).descriptor;
  control('withheld-metadata-omitted',!Object.hasOwn(limited,'presentation')&&limited.variants.every(v=>equal(Object.keys(v),['reference'])));
  const cursor={selector:'next',binding:copy(binding),limit:1,next:1},next={...list,continuation:'next'};
  deny('missing-cursor',next);deny('ambiguous-cursor',next,current,cap,[cursor,copy(cursor)]);
  deny('cursor-bound-query-limit',{...next,limit:2},current,cap,[cursor]);
  deny('cursor-bound-registry',next,current,cap,[{...cursor,binding:{...binding,registry:'new'}}]);
  deny('cursor-cannot-restart',next,current,cap,[{...cursor,next:0}]);
  deny('cursor-cannot-use-raw-offset',{...list,continuation:1});
  deny('zero-page-size',{...list,limit:0});deny('over-page-bound',{...list,limit:3});
  deny('fractional-page-size',{...list,limit:1.5});
  control('complete-visible-end',readDiscovery(cap,next,current,[cursor]).next===null);
  control('established-empty-registry',equal(readDiscovery(captureDiscovery([],binding,limits),list,current),{status:'page',context:binding.context,entries:[],next:null}));
  deny('failed-capture-not-empty',list,current,null);
  const duplicate=[...copy(registry),copy(registry[0])];
  control('duplicate-entry-reference-refused',captureDiscovery(duplicate,binding,limits)===null);
  const duplicateVariant=copy(registry);duplicateVariant[0].variants.push(copy(duplicateVariant[0].variants[0]));
  control('duplicate-variant-reference-refused',captureDiscovery(duplicateVariant,binding,limits)===null);
  const unknown=copy(registry);unknown[0].variants[0].name='execute';
  control('unknown-executable-variant-not-advertised',captureDiscovery(unknown,binding,limits)===null);
  const wrongSchema=copy(registry);wrongSchema[0].variants[0].input.contract='other';
  control('schema-bound-to-registered-contract',captureDiscovery(wrongSchema,binding,limits)===null);
  const wrongSchemaVariant=copy(registry);wrongSchemaVariant[0].variants[0].input.variant='redo';
  control('schema-bound-to-exact-variant',captureDiscovery(wrongSchemaVariant,binding,limits)===null);
  const wrongKind=copy(registry);wrongKind[0].kind='query';
  control('metadata-cannot-reclassify-command',captureDiscovery(wrongKind,binding,limits)===null);
  for(const [name,key,value] of [['entry-bound','maxEntries',1],['variant-bound','maxVariants',1],['input-row-bound','maxInputRowBytes',1],['output-row-bound','maxRowBytes',1],['capture-bound','maxCaptureBytes',1]])
    control(name,captureDiscovery(registry,binding,{...limits,[key]:value})===null);
  const saved=copy(cap);registry[1].presentation.title='mutated';registry[1].variants[0].input.contract='other';
  control('input-cannot-change-owned-capture',equal(cap,saved));
  descriptor.variants[0].input.contract='mutated';control('result-cannot-change-owned-capture',equal(cap,saved));
  control('capture-is-frozen',Object.isFrozen(cap.rows[0].variants[0].input));
  control('diagnostics-not-authority',equal(readDiscovery(cap,advice,current),readDiscovery(cap,advice,{...current,diagnosticFailed:true})));
  return {status:'contract-discovery-products-only',releaseCases,pairedCases,traversalCases,controls,
    contextIssuanceProved:false,registryAndSchemaTruthProved:false,implementationRefinement:false};
}
export async function checkDiscoveryMutations(){
  const source=fs.readFileSync(new URL('./contract-discovery.mjs',import.meta.url),'utf8');
  const mutants=[
    ['expose-hidden-entry','if(!entry.visible)continue;','if(false)continue;'],
    ['ignore-current-projection','!equal(current.binding,binding)','false'],
    ['ignore-current-registry','!gates.every(k=>current[k])',"!gates.filter(k=>k!=='registryCurrent').every(k=>current[k])"],
    ['ignore-page-query-binding','c.limit!==request.limit','false'],
    ['expose-withheld-schema','...(entry.schemaVisible?','...(true?'],
    ['embed-stale-advice-in-descriptor','descriptor.variants=descriptor.variants.map(({enabled,...v})=>v);','descriptor.variants=descriptor.variants;']
  ];
  const killed=[];
  for(const [name,from,to] of mutants){
    assert(source.includes(from),name);
    let code=source.replace(from,to).replace("'./operation-catalog.mjs'",JSON.stringify(new URL('./operation-catalog.mjs',import.meta.url).href));
    for(const file of ['contract-discovery.json','operation-families.json','annotation-operations.json','history-reversal.json'])
      code=code.replace(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(`./${file}`,import.meta.url).href)})`);
    const module=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${name}`);
    assert.throws(()=>module.checkContractDiscovery(),{code:'ERR_ASSERTION'},name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify({...checkContractDiscovery(),mutations:await checkDiscoveryMutations()},null,2));
