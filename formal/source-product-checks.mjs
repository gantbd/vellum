// Composes actual source helpers, selector registry and all-required version-set gate.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import * as sourceApi from './source-products.mjs';
import * as registryApi from './read-contexts.mjs';
import * as versionApi from './version-set-contexts.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./source-products.json',import.meta.url),'utf8'));
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8'));
const denied={status:'unavailable'};

export function checkSourceProducts(source=sourceApi,registry=registryApi,version=versionApi){
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  const readCatalog=JSON.parse(fs.readFileSync(new URL('./read-contexts.json',import.meta.url),'utf8'));
  const versionCatalog=JSON.parse(fs.readFileSync(new URL('./version-set-contexts.json',import.meta.url),'utf8'));
  assert.deepEqual([...Object.keys(catalog.variants),readCatalog.acquisition.variant].sort(),families.find(f=>f.id===catalog.family).variants.toSorted());
  assert.equal(readCatalog.sourceQueries.contract,catalog.contract);assert.deepEqual(readCatalog.sourceQueries.variants,Object.keys(catalog.variants));
  const profile={maxRootBytes:128,maxInputBytes:128,maxContentBytes:128,maxReplyBytes:2048};
  const document={authority:'a',id:'A',incarnation:1};
  const snapshot={ref:{authority:'a',document:'A',incarnation:1,revision:1},source:bytes('A🙂\r\né\r'),selector:'selector',profile:'base'};
  const common={contract:catalog.contract,snapshot:'selector',document,profile:'base'};
  const range={...common,operation:'bounded-range',lo:1,hi:5},line={...common,operation:'line',line:1};
  const release={grant:true,scope:true,continuity:true,capacity:true};
  const read=(r=line,s=snapshot,l=profile,g=release)=>source.readSourceProduct(s,r,g,l);
  const controls=[],control=(name,condition)=>{assert(condition,name);controls.push(name);};
  let lineCases=0,rangeCases=0,releaseCases=0,registryCases=0,parentCases=0,pairedCases=0;
  const alphabet=['a','é','\r','\n','\u2028'];let texts=[''],layer=[''];
  for(let length=1;length<=4;length++){layer=layer.flatMap(s=>alphabet.map(c=>s+c));texts.push(...layer);}
  for(const text of [...texts,'🙂\r\n\uFEFFe\u0301']){
    const parts=text.split(/(\r\n|\r|\n)/u),expected=[];let offset=0;
    for(let i=0;i<parts.length;i+=2){
      const body=bytes(parts[i]),delimiter=bytes(parts[i+1]??''),start=offset;offset+=body.length+delimiter.length;
      expected.push({status:'line',context:'selector',ref:copy(snapshot.ref),unit:'utf8-byte',line:i/2,
        body:{lo:start,hi:start+body.length,source:body},delimiter:{lo:start+body.length,hi:offset,source:delimiter}});
    }
    const s={...snapshot,source:bytes(text)},before=copy(s),positions=[0];
    for(const scalar of text)positions.push(positions.at(-1)+bytes(scalar).length);
    for(let index=0;index<expected.length;index++){
      const out=read({...line,line:index},s);assert.deepEqual(out,expected[index]);
      assert(!Object.hasOwn(out,'total')&&!Object.hasOwn(out,'sourceLength'));lineCases++;
    }
    for(let lo=0;lo<positions.length;lo++)for(let hi=lo;hi<positions.length;hi++){
      const out=read({...range,lo:positions[lo],hi:positions[hi]},s);
      assert.equal(out.status,'content');assert.deepEqual(out.source,bytes([...text].slice(lo,hi).join('')));
      assert.deepEqual(out.range,[positions[lo],positions[hi]]);rangeCases++;
    }
    assert.deepEqual(s,before);
  }
  for(const r of [range,line])for(let mask=0;mask<16;mask++){
    const g=Object.fromEntries(Object.keys(release).map((k,i)=>[k,!!(mask&(1<<i))]));
    assert.equal(read(r,snapshot,profile,g).status,mask===15?(r.operation==='line'?'line':'content'):'unavailable');releaseCases++;
  }
  for(const r of [{...line,contract:'source-query/2'},{...line,operation:'eval'},{...line,extra:true},{...line,line:-1},{...line,line:1.5},
    {...line,line:3},{...line,snapshot:'other'},{...line,profile:'other'},{...line,document:{...document,incarnation:2}},
    {...range,lo:2},{...range,hi:129},{...range,lo:5,hi:1}])control(`invalid-product-${controls.length}`,equal(read(r),denied));
  control('missing-required-field',equal(read(Object.fromEntries(Object.entries(line).filter(([k])=>k!=='contract'))),denied));
  control('invalid-utf8-no-metadata',equal(read(line,{...snapshot,source:[255]}),denied));
  for(const key of catalog.profileFields)control(`limit-${key}`,equal(read(line,snapshot,{...profile,[key]:0}),denied));
  control('unbounded-profile-refused',source.captureSourceProfile({...profile,maxReplyBytes:Infinity})===null);
  control('unknown-profile-field-refused',source.captureSourceProfile({...profile,extra:true})===null);
  control('incomplete-profile-refused',source.captureSourceProfile({maxRootBytes:128})===null);
  const captured=source.captureSourceProfile(profile);profile.maxContentBytes=127;
  control('profile-is-owned',captured.maxContentBytes===128&&Object.isFrozen(captured));profile.maxContentBytes=128;
  const eof=read({...line,line:2});
  control('trailing-delimiter-final-empty-line',eof.status==='line'&&eof.body.lo===10&&eof.body.hi===10&&eof.delimiter.source.length===0);
  const crlf=read({...line,line:0});
  control('crlf-exact-delimiter',equal(crlf.delimiter.source,[13,10])&&equal(crlf.body.source,bytes('A🙂')));
  control('crlf-interior-remains-byte-range',equal(read({...range,lo:6,hi:7}).source,[10]));
  control('empty-authorized-range-is-content',read({...range,lo:0,hi:0}).status==='content');
  const changed=read();changed.body.source[0]=90;
  control('result-bytes-are-owned',read().body.source[0]===195&&snapshot.source[7]===195);

  const limits={contexts:8,units:512,selectors:32,holds:4,issuance:128,sourceBytes:128,source:copy(profile),
    annotation:{maxInputRows:8,maxCaptureBytes:4096,maxRowBytes:512,maxPageRows:2},groups:2,participants:3,inputRoots:8,retainedCuts:3};
  const host={document,recipient:'reader',purpose:'read',destination:'agent',policy:'p1',continuity:'c1',profile:'base',
    granted:true,live:true,continuous:true,transferReady:true,replyReady:true,captureComplete:true,preparationReserved:true,
    registryReady:true,retentionReady:true,notStopped:true};
  const root={document,retainedReference:'retained-root',sourceRevision:1,source:copy(snapshot.source),records:[]};
  const acquire={contract:'read-context/1',operation:'snapshot-handoff',document,selection:{kind:'capture-current'},data:['source'],profile:'base'};
  const install=(r=root,a=acquire,h=host,l=limits)=>registry.acquireReadContext(registry.newReadRegistry('issuer',l),registry.prepareReadContext(r,a,h,l),h);
  const acquired=install(),token=acquired.result.context,ranges={...range,snapshot:token},lines={...line,snapshot:token};
  const query=(r=lines,m=acquired.state,h=host)=>registry.queryReadContext(m,r,h);
  for(const r of [ranges,lines])for(let mask=0;mask<32;mask++){
    const h={...host,...Object.fromEntries(['granted','live','continuous','transferReady','replyReady'].map((k,i)=>[k,!!(mask&(1<<i))]))};
    const before=copy(acquired.state),out=query(r,acquired.state,h);
    assert.equal(out.result.status,mask===31?(r.operation==='line'?'line':'content'):'unavailable');assert.deepEqual(acquired.state,before);registryCases++;
  }
  control('source-context-not-annotation-count',!JSON.stringify(query().result).includes('annotation'));
  const annotation=install(root,{...acquire,data:['annotations']});
  control('annotation-only-not-line-authority',equal(query({...lines,snapshot:annotation.result.context},annotation.state).result,denied));
  for(const key of ['recipient','purpose','destination','policy','continuity','profile'])
    control(`current-${key}`,equal(query(lines,acquired.state,{...host,[key]:'other'}).result,denied));
  const later=copy(root);later.source=bytes('NEW');later.sourceRevision=2;
  const fresh=registry.acquireReadContext(acquired.state,registry.prepareReadContext(later,acquire,host,limits),host);
  control('old-context-not-latest',query(lines,fresh.state).result.ref.revision===1&&equal(query(lines,fresh.state).result.body.source,bytes('é'))&&
    fresh.result.context!==token&&registry.queryReadContext(fresh.state,{...lines,snapshot:fresh.result.context,line:0},host).result.ref.revision===2);
  const held=registry.holdReadContext(acquired.state,token),closed=registry.closeReadContext(held.state,{contract:'read-context/1',operation:'stop-observation',context:token},'reader');
  control('held-closed-context-no-line-release',closed.state.contexts.length===1&&equal(query(lines,closed.state).result,denied));
  const changedLimits=copy(acquired.state);changedLimits.limits.source.maxReplyBytes=0;
  control('captured-query-profile-not-current-mutable-limits',query(lines,changedLimits).result.status==='line');
  const mutable=copy(limits),candidate=registry.prepareReadContext(root,acquire,host,mutable);mutable.source.maxReplyBytes=0;
  control('captured-profile-cannot-be-mutated',candidate.sourceProfile.maxReplyBytes===2048&&Object.isFrozen(candidate.sourceProfile));
  const absent=copy(limits);delete absent.source;
  control('source-acquisition-needs-finite-product-profile',registry.prepareReadContext(root,acquire,host,absent)===null);
  control('annotation-only-no-source-profile-prerequisite',registry.prepareReadContext(root,{...acquire,data:['annotations']},host,absent)!==null);
  control('source-root-must-fit-captured-profile',registry.prepareReadContext(root,acquire,host,{...limits,source:{...profile,maxRootBytes:0}})===null);
  for(let secret=0;secret<12;secret++)for(const data of [['source'],['source','annotations']]){
    const hidden={...copy(root),records:[{reference:`secret-${secret}`,visible:false,bodyVisible:true,targetVisible:false,body:'private'.repeat(secret),contentVersion:1}]};
    const left=install(root,{...acquire,data}),right=install(hidden,{...acquire,data});
    for(const r of [ranges,lines]){assert.deepEqual(query(r,left.state).result,query(r,right.state).result);pairedCases++;}
  }
  const other={...copy(root),document:{...document,id:'B'},source:bytes('B\n')};
  const authority={id:'a',current:{identity:'cut',reference:'retained-cut',roots:[root,other]},retained:[]};
  const parentHost={...host,members:[root,other].map(r=>({...copy(host),document:r.document,stamp:{membership:1}}))};
  const parentRequest={contract:versionCatalog.contract,operation:'snapshot-handoff',authority:'a',participants:[root,other].map(r=>({document:r.document,data:['source']})),selection:{kind:'capture-current'},profile:'base'};
  const parentCandidate=version.prepareVersionSet(authority,parentRequest,parentHost,limits);
  const parent=version.acquireVersionSet(version.newVersionRegistry('set-issuer',limits),parentCandidate,parentHost);
  assert.equal(parent.result.status,'version-set');
  const member=parent.result.participants[0],memberRequests=[{...ranges,snapshot:member.snapshot},{...lines,snapshot:member.snapshot}];
  for(const r of memberRequests)for(let mask=0;mask<64;mask++){
    const h=copy(parentHost);
    for(const [i,key] of ['granted','live','continuous'].entries()){h[key]=!!(mask&(1<<i));h.members[1][key]=!!(mask&(1<<(i+3)));}
    const out=version.queryVersionMember(parent.state,r,h);
    assert.equal(out.result.status,mask===63?(r.operation==='line'?'line':'content'):'unavailable');parentCases++;
  }
  control('member-query-cannot-bypass-parent',equal(query({...lines,snapshot:member.snapshot},parent.state.registry,parentHost.members[0]).result,denied));
  const stopped=version.stopVersionSet(parent.state,{contract:'read-context/1',operation:'stop-observation',context:member.snapshot},'reader');
  control('member-stop-closes-line-products',equal(version.queryVersionMember(stopped.state,memberRequests[1],parentHost).result,denied));
  control('source-read-does-not-publish',!Object.hasOwn(query().result,'publication')&&equal(query().state,acquired.state));
  return {status:'source-query-products-only',lineCases,rangeCases,releaseCases,registryCases,parentCases,pairedCases,controls,...catalog.scope};
}

export async function checkSourceMutationEvidence(){
  const core=fs.readFileSync(new URL('./source-products.mjs',import.meta.url),'utf8');
  const registry=fs.readFileSync(new URL('./read-contexts.mjs',import.meta.url),'utf8');
  const version=fs.readFileSync(new URL('./version-set-contexts.mjs',import.meta.url),'utf8');
  const mutants=[
    ['wrong-logical-row','core','const row=rows[request.line]','const row=rows[0]'],
    ['body-includes-delimiter','core','source:read.source.slice(0,row.bodyEnd-row.start)','source:read.source'],
    ['normalize-delimiter','core','source:read.source.slice(row.bodyEnd-row.start)','source:[10]'],
    ['omit-reply-envelope-limit','core',"Buffer.byteLength(JSON.stringify(out),'utf8')<=limits.maxReplyBytes",'true'],
    ['ignore-captured-profile','core','request.profile!==snapshot.profile','false'],
    ['reinterpret-held-profile','registry','c.sourceProfile);','m.limits.source);'],
    ['ignore-other-required-member','version','g.members.every(p=>{','g.members.slice(0,1).every(p=>{']
  ];
  const killed=[],mutationDispositions=[];
  for(const [name,target,from,to] of mutants){
    const sources={core,registry,version};assert(sources[target].includes(from),name);sources[target]=sources[target].replace(from,to);
    for(const file of ['source-edit-contracts.mjs','reference-profile.mjs'])sources.core=sources.core.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    sources.core=sources.core.replace("new URL('./source-products.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./source-products.json',import.meta.url).href)})`);
    const coreURL=`data:text/javascript;base64,${Buffer.from(sources.core).toString('base64')}#${name}`;
    sources.registry=sources.registry.replace("'./source-products.mjs'",JSON.stringify(coreURL));
    for(const file of ['annotation-queries.mjs','source-edit-contracts.mjs'])sources.registry=sources.registry.replace(`'./${file}'`,JSON.stringify(new URL(file,import.meta.url).href));
    for(const file of ['read-contexts.json','operation-families.json'])sources.registry=sources.registry.replaceAll(`new URL('./${file}',import.meta.url)`,`new URL(${JSON.stringify(new URL(file,import.meta.url).href)})`);
    const registryURL=`data:text/javascript;base64,${Buffer.from(sources.registry).toString('base64')}#${name}`;
    sources.version=sources.version.replace("'./read-contexts.mjs'",JSON.stringify(registryURL)).replace("'./source-edit-contracts.mjs'",JSON.stringify(new URL('./source-edit-contracts.mjs',import.meta.url).href));
    sources.version=sources.version.replace("new URL('./version-set-contexts.json',import.meta.url)",`new URL(${JSON.stringify(new URL('./version-set-contexts.json',import.meta.url).href)})`);
    const c=await import(coreURL),r=await import(registryURL),v=await import(`data:text/javascript;base64,${Buffer.from(sources.version).toString('base64')}#${name}`);
    let failed=false;try{checkSourceProducts(c,r,v);}catch(error){assert(['ERR_ASSERTION','NATIVE_SOURCE_RESULT_INVARIANT'].includes(error.code),name);mutationDispositions.push({name,detectedBy:error.code});failed=true;}
    assert(failed,name);killed.push(name);
  }
  return {mutations:killed,mutationDispositions};
}
export async function checkSourceMutations(){return (await checkSourceMutationEvidence()).mutations;}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkSourceProducts(),...await checkSourceMutationEvidence()},null,2));
