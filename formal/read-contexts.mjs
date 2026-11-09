// Serialized normalized local registry reference; no actual policy or heap oracle.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {captureAnnotations,readAnnotations} from './annotation-queries.mjs';
import {captureSourceProfile,readSourceProduct} from './source-products.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./read-contexts.json',import.meta.url),'utf8'));
const clone=structuredClone;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const unavailable=()=>({status:'unavailable'});
const scopeKeys=['document','recipient','purpose','destination','policy','continuity','profile'];
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const scoped=(a,b)=>scopeKeys.every(k=>equal(a[k],b[k]));
const lookup=(m,token,kind)=>m.selectors.find(s=>s.token===token&&s.kind===kind);
const contextFor=(m,s)=>s&&m.contexts.find(c=>c.token===s.context);
const room=(m,count)=>m.selectors.length+count<=m.limits.selectors&&m.issued+count<=m.limits.issuance;
const mint=m=>`${m.issuer}:private-${++m.issued}`; // Private test encoding, never a selected public token format.
const active=(c,current)=>c?.state==='active'&&(!c.parent||current.versionSetPermit===c.parent)&&scoped(c.scope,current)&&current.granted&&current.live&&current.continuous&&current.transferReady;
const charged=m=>m.contexts.reduce((n,c)=>n+c.units,0);

export function newReadRegistry(issuer,limits){return {issuer,limits:clone(limits),issued:0,contexts:[],selectors:[],holds:[]};}
export function singleReadAcquisitionInputForm(){return freeze({family:catalog.acquisition.family,contract:catalog.contract,operation:catalog.acquisition.variant,variant:catalog.acquisition.variant,fields:clone(catalog.acquisition.fields),selections:clone(catalog.acquisition.selections),data:clone(catalog.acquisition.data)});}

// Existing native acquisition values, not a token decoder or authority oracle.
const singleResultFields=freeze({context:['status','context','data'],unavailable:['status']});
export function nativeReadProductExact(v,keys){if(!v||typeof v!=='object'||Array.isArray(v)||![Object.prototype,null].includes(Object.getPrototypeOf(v)))return false;const d=Object.getOwnPropertyDescriptors(v),ks=Reflect.ownKeys(d);return ks.length===keys.length&&keys.every(k=>Object.hasOwn(d,k)&&Object.hasOwn(d[k],'value')&&d[k].enumerable);}
export function readProjectionClassesMatch(v,expected){if(!Array.isArray(v)||v.length<1||v.length>2||v.length!==expected.length)return false;for(let i=0;i<v.length;i++)if(!Object.hasOwn(v,i)||!catalog.acquisition.data.includes(v[i])||!expected.includes(v[i])||i>0&&v[i]===v[0])return false;return true;}
export function singleReadAcquisitionResultForms(){return singleResultFields;}
export function singleReadAcquisitionResultValid(out,request,expectedContext=null){
 if(!out||typeof out.status!=='string'||!Object.hasOwn(singleResultFields,out.status)||!nativeReadProductExact(out,singleResultFields[out.status]))return false;if(out.status==='unavailable')return true;
 return request.contract==='read-context/1'&&out.context!==null&&typeof out.context==='object'&&!Array.isArray(out.context)&&(expectedContext===null||out.context===expectedContext)&&(request.selection?.kind!=='retained'||out.context!==request.selection.reference)&&readProjectionClassesMatch(out.data,request.data);
}

export function prepareReadContext(root,request,captured,limits){
  if(!exact(request,catalog.acquisition.fields)||request.contract!==catalog.contract||request.operation!=='snapshot-handoff'||
      !equal(request.document,captured.document)||request.profile!==captured.profile||
      !Array.isArray(request.data)||request.data.length<1||request.data.length>2||new Set(request.data).size!==request.data.length||
      request.data.some(x=>!catalog.acquisition.data.includes(x))||!request.selection||
      !Object.hasOwn(catalog.acquisition.selections,request.selection.kind)||
      !exact(request.selection,['kind',...catalog.acquisition.selections[request.selection.kind]]))return null;
  if(!captured.granted||!captured.live||!captured.continuous||!captured.captureComplete||!captured.preparationReserved||
      !equal(root.document,request.document)||
      (request.selection.kind==='retained'&&request.selection.reference!==root.retainedReference))return null;
  const scope=Object.fromEntries(scopeKeys.map(k=>[k,clone(captured[k])]));
  const annotations=request.data.includes('annotations')?captureAnnotations(root.records,{...scope,snapshot:'unissued'},limits.annotation):null;
  if(request.data.includes('annotations')&&!annotations)return null;
  if(request.data.includes('source')&&root.source.length>limits.sourceBytes)return null;
  const sourceProfile=request.data.includes('source')?captureSourceProfile(limits.source):null;
  if(request.data.includes('source')&&(!sourceProfile||root.source.length>sourceProfile.maxRootBytes||root.source.length>sourceProfile.maxInputBytes))return null;
  // Declared abstract units, not actual allocation bytes. The retained root is shared by all selectors/holds.
  const units=(sourceProfile?root.source.length:0)+(annotations?annotations.rows.length:0);
  const retainedRoot={document:clone(root.document),...(sourceProfile?{sourceRevision:root.sourceRevision,source:clone(root.source)}:{})};
  return freeze({scope,data:[...request.data].sort(),root:retainedRoot,annotations,sourceProfile,units});
}

export function acquireReadContext(m,candidate,current){
  if(!candidate||!scoped(candidate.scope,current)||!current.granted||!current.live||!current.continuous||!current.transferReady||
      !current.registryReady||!current.retentionReady||!current.notStopped||
      m.contexts.length>=m.limits.contexts||charged(m)+candidate.units>m.limits.units||!room(m,1))return {state:m,result:unavailable()};
  const n=clone(m),token=mint(n),scope={...clone(candidate.scope),snapshot:token};
  const annotations=candidate.annotations?freeze({...clone(candidate.annotations),scope:clone(scope)}):null;
  n.contexts.push({token,state:'active',scope,root:clone(candidate.root),data:clone(candidate.data),annotations,sourceProfile:clone(candidate.sourceProfile),units:candidate.units});
  n.selectors.push({token,kind:'context',context:token});
  return {state:n,result:{status:'context',context:token,data:clone(candidate.data)}};
}

export function queryReadContext(m,request,current){
  const selected=lookup(m,request.snapshot,'context'),c=contextFor(m,selected);
  const fail=()=>({state:m,result:unavailable()});
  if(!active(c,current)||!equal(request.document,c.scope.document)||request.profile!==c.scope.profile)return fail();
  if(['bounded-range','line'].includes(request.operation)){
    if(!c.data.includes('source'))return fail();
    const ref={authority:c.root.document.authority,document:c.root.document.id,incarnation:c.root.document.incarnation,revision:c.root.sourceRevision};
    const result=readSourceProduct({ref,source:c.root.source,selector:c.token,profile:c.scope.profile},request,
      {grant:true,scope:true,continuity:true,capacity:current.replyReady},c.sourceProfile);
    return {state:m,result};
  }
  if(!c.data.includes('annotations')||!current.replyReady)return fail();
  const r=clone(request),cursors=[];
  if(request.operation!=='enumerate'){
    const record=lookup(m,request.annotation,'record');
    if(!record||record.context!==c.token)return fail();
    r.annotation=record.reference;
  }else if(request.continuation!==null){
    const continuation=lookup(m,request.continuation,'continuation');
    if(!continuation||continuation.context!==c.token)return fail();
    cursors.push({selector:continuation.token,scope:c.scope,limit:continuation.limit,next:continuation.next});
  }
  const result=readAnnotations(c.annotations,r,{scope:c.scope,granted:true,retained:true,live:true,continuous:true,transferReady:true},cursors);
  if(result.status==='unavailable')return fail();
  const pending=[];
  const recordToken=reference=>{
    const existing=m.selectors.find(s=>s.kind==='record'&&s.context===c.token&&s.reference===reference);
    if(existing)return existing.token;
    const already=pending.find(s=>s.kind==='record'&&s.reference===reference);if(already)return already;
    const s={kind:'record',context:c.token,reference};pending.push(s);return s;
  };
  let pageNext=null;
  if(result.status==='page'){
    result.records.forEach(row=>recordToken(row.reference));
    if(result.next!==null){
      pageNext=m.selectors.find(s=>s.kind==='continuation'&&s.context===c.token&&s.limit===request.limit&&s.next===result.next);
      if(!pageNext){pageNext={kind:'continuation',context:c.token,limit:request.limit,next:result.next};pending.push(pageNext);}
    }
  }
  if(!room(m,pending.length))return fail();
  const n=clone(m);
  for(const s of pending){s.token=mint(n);n.selectors.push(clone(s));}
  const tokenFor=ref=>n.selectors.find(s=>s.kind==='record'&&s.context===c.token&&s.reference===ref).token;
  if(result.status==='page'){
    result.records.forEach(row=>{row.reference=tokenFor(row.reference);});
    result.continuation=pageNext?.token??null;delete result.next;
  }else if(result.status==='record')result.record.reference=request.annotation;
  else result.reference=request.annotation;
  return {state:n,result};
}

// Trusted internal F05 input only, not a new recipient snapshot/read product.
export function captureReadContextSearchSource(m,token,current){
 const c=contextFor(m,lookup(m,token,'context'));
 if(!active(c,current)||!c.data.includes('source')||!['granted','live','continuous','transferReady','replyReady','notStopped'].every(k=>current?.[k]===true))return null;
 const ref={authority:c.root.document.authority,document:c.root.document.id,incarnation:c.root.document.incarnation,revision:c.root.sourceRevision},snapshot={ref,source:c.root.source,selector:c.token,profile:c.scope.profile};
 const probe=readSourceProduct(snapshot,{contract:'source-query/1',operation:'bounded-range',document:clone(c.root.document),snapshot:c.token,profile:c.scope.profile,lo:0,hi:0},{grant:true,scope:true,continuity:true,capacity:true},c.sourceProfile);
 if(probe.status!=='content')return null;
 return freeze({snapshot:{ref:clone(ref),source:clone(c.root.source)},sourceProfile:clone(c.sourceProfile)});
}

function retire(m,token){
  const n=clone(m),c=n.contexts.find(c=>c.token===token);if(!c)return n;
  c.state='invalidated';n.selectors=n.selectors.filter(s=>s.context!==token);
  if(!n.holds.some(h=>h.context===token))n.contexts=n.contexts.filter(x=>x.token!==token);
  return n;
}
export function closeReadContext(m,request,recipient){
  const s=lookup(m,request.context,'context'),c=contextFor(m,s);
  if(!exact(request,catalog.close.fields)||request.contract!==catalog.contract||request.operation!=='stop-observation'||
      !c||c.parent||recipient!==c.scope.recipient)return {state:m,result:unavailable()};
  // Owner can stop future delivery even after losing a content grant. No ordinary-capacity gate.
  return {state:retire(m,c.token),result:{status:'stopped'}};
}
export function invalidateReadContext(m,token){return retire(m,token);}
export function holdReadContext(m,token){
  if(!m.contexts.some(c=>c.token===token&&c.state==='active')||m.holds.length>=m.limits.holds||m.issued>=m.limits.issuance)return {state:m,ticket:null};
  const n=clone(m),ticket=mint(n);n.holds.push({ticket,context:token});return {state:n,ticket};
}
export function releaseReadHold(m,ticket){
  const hold=m.holds.find(h=>h.ticket===ticket);if(!hold)return m;
  const n=clone(m);n.holds=n.holds.filter(h=>h.ticket!==ticket);
  const c=n.contexts.find(c=>c.token===hold.context);
  if(c?.state==='invalidated'&&!n.holds.some(h=>h.context===c.token))n.contexts=n.contexts.filter(x=>x.token!==c.token);
  return n;
}

function invariants(m){
  assert(m.contexts.length<=m.limits.contexts&&charged(m)<=m.limits.units&&m.selectors.length<=m.limits.selectors&&m.holds.length<=m.limits.holds);
  assert(m.issued<=m.limits.issuance);
  assert.equal(new Set([...m.selectors.map(s=>s.token),...m.holds.map(h=>h.ticket)]).size,m.selectors.length+m.holds.length);
  for(const s of m.selectors)assert(m.contexts.some(c=>c.token===s.context&&c.state==='active'));
  for(const h of m.holds)assert(m.contexts.some(c=>c.token===h.context));
  for(const c of m.contexts)if(c.state==='invalidated'){assert(!m.selectors.some(s=>s.context===c.token));assert(m.holds.some(h=>h.context===c.token));}
}

export function checkReadContexts(){
  assert.deepEqual(scopeKeys,catalog.scopeFields);
  const families=JSON.parse(fs.readFileSync(new URL('./operation-families.json',import.meta.url),'utf8'));
  for(const part of [catalog.acquisition,catalog.close])assert(families.find(f=>f.id===part.family).variants.includes(part.variant));
  const limits={contexts:2,units:64,selectors:8,holds:2,issuance:32,sourceBytes:32,
    source:{maxRootBytes:32,maxInputBytes:32,maxContentBytes:32,maxReplyBytes:2048},
    annotation:{maxInputRows:8,maxCaptureBytes:4096,maxRowBytes:512,maxPageRows:2}};
  const document={authority:'a',id:'d',incarnation:1};
  const root={document,retainedReference:'retained-root',sourceRevision:1,source:[65,66],records:
    ['r1','r2','r3'].map(reference=>({reference,visible:true,bodyVisible:true,targetVisible:true,body:'body',contentVersion:1,
      location:{state:'attached',source:'source1',targetGeneration:1,target:{kind:'point',offset:0,affinity:'before'}}}))};
  const originalRoot=clone(root);
  const current={document,recipient:'reader',purpose:'edit',destination:'view',policy:'p1',continuity:'k1',profile:'base',
    granted:true,live:true,continuous:true,transferReady:true,captureComplete:true,preparationReserved:true,
    registryReady:true,retentionReady:true,notStopped:true,replyReady:true};
  const request={contract:catalog.contract,operation:'snapshot-handoff',document,selection:{kind:'capture-current'},data:['annotations'],profile:'base'};
  const candidate=prepareReadContext(root,request,current,limits),initial=newReadRegistry('issuer1',limits);
  const acquired=acquireReadContext(initial,candidate,current),token=acquired.result.context;
  const page={contract:'annotation-query/1',operation:'enumerate',document,snapshot:token,profile:'base',limit:2,continuation:null};
  const stop={contract:catalog.contract,operation:'stop-observation',context:token};
  const controls=[];const control=(name,ok)=>{assert(ok,name);controls.push(name);};
  const denied=(name,m,r=page,host=current)=>{const before=clone({m,r,host}),out=queryReadContext(m,r,host);assert.deepEqual(out.result,unavailable());assert.deepEqual(out.state,m);assert.deepEqual({m,r,host},before);controls.push(name);};
  let admissionCases=0;
  const gates=['granted','live','continuous','transferReady','registryReady','retentionReady','notStopped'];
  for(let mask=0;mask<128;mask++){
    const host={...clone(current),...Object.fromEntries(gates.map((k,i)=>[k,!!(mask&(1<<i))]))};
    const before=clone(initial),out=acquireReadContext(initial,candidate,host);
    assert.equal(out.result.status,mask===127?'context':'unavailable');
    if(mask!==127)assert.deepEqual(out.state,initial);else{assert.equal(out.state.contexts.length,1);assert.equal(charged(out.state),3);}
    assert.deepEqual(initial,before);invariants(out.state);admissionCases++;
  }
  const first=queryReadContext(acquired.state,page,current),again=queryReadContext(first.state,page,current);
  control('page-retry-reuses-selectors',equal(first,again));
  control('no-raw-offset-in-public-page',!Object.hasOwn(first.result,'next')&&typeof first.result.continuation==='string');
  control('record-selectors-not-source-identifiers',first.result.records.every(r=>!['r1','r2','r3'].includes(r.reference)));
  const record=first.result.records[0].reference;
  const inspected=queryReadContext(first.state,{contract:'annotation-query/1',operation:'inspect',document,snapshot:token,profile:'base',annotation:record},current);
  control('issued-record-resolves-exact-context',inspected.result.record.content.body==='body'&&inspected.result.record.reference===record);
  const second=queryReadContext(first.state,{...page,continuation:first.result.continuation},current);
  control('issued-continuation-finishes',second.result.records.length===1&&second.result.continuation===null);
  const held=holdReadContext(first.state,token),closed=closeReadContext(held.state,stop,'reader');
  control('close-invalidates-all-aliases',closed.result.status==='stopped'&&closed.state.selectors.length===0);
  control('held-root-stays-charged',charged(closed.state)===3&&closed.state.contexts[0].state==='invalidated');
  denied('closed-context-no-read',closed.state);
  control('last-hold-reclaims',releaseReadHold(closed.state,held.ticket).contexts.length===0);
  control('duplicate-release-does-not-refund-twice',equal(releaseReadHold(releaseReadHold(closed.state,held.ticket),held.ticket),releaseReadHold(closed.state,held.ticket)));
  control('owner-stop-needs-no-content-grant',closeReadContext(first.state,stop,'reader').result.status==='stopped');
  control('foreign-stop-does-not-close',equal(closeReadContext(first.state,stop,'other'),{state:first.state,result:unavailable()}));
  const stopped=closeReadContext(first.state,stop,'reader').state;
  const reissued=acquireReadContext(stopped,candidate,current);
  control('retirement-does-not-reuse-identity',reissued.result.context!==token&&reissued.state.issued>stopped.issued);
  denied('old-context-not-new-context',reissued.state);
  denied('record-selector-not-context-selector',first.state,{...page,snapshot:record});
  denied('continuation-not-record-selector',first.state,{contract:'annotation-query/1',operation:'inspect',document,snapshot:token,profile:'base',annotation:first.result.continuation});
  const other=acquireReadContext(first.state,candidate,current);
  denied('record-cannot-cross-context',other.state,{contract:'annotation-query/1',operation:'inspect',document,snapshot:other.result.context,profile:'base',annotation:record});
  denied('continuation-cannot-cross-context',other.state,{...page,snapshot:other.result.context,continuation:first.result.continuation});
  for(const key of scopeKeys)denied(`current-${key}`,first.state,page,{...current,[key]:key==='document'?{...document,incarnation:2}:'other'});
  denied('accepted-policy-change-refuses',first.state,page,{...current,policy:'p2'});
  const invalid=invalidateReadContext(first.state,token);
  denied('regrant-cannot-revive-invalidated',invalid);
  denied('annotation-context-does-not-grant-source',first.state,{contract:'source-query/1',operation:'bounded-range',document,snapshot:token,profile:'base',lo:0,hi:2});
  const sourceCandidate=prepareReadContext(root,{...request,data:['source']},current,limits);
  const source=acquireReadContext(initial,sourceCandidate,current);
  denied('source-context-does-not-grant-annotations',source.state,{...page,snapshot:source.result.context});
  const sourceRead=queryReadContext(source.state,{contract:'source-query/1',operation:'bounded-range',document,snapshot:source.result.context,profile:'base',lo:0,hi:2},current);
  control('source-context-exact-bytes',equal(sourceRead.result.source,[65,66])&&sourceRead.result.ref.revision===1);
  const both=acquireReadContext(initial,prepareReadContext(originalRoot,{...request,data:['source','annotations']},current,limits),current);
  const bothSource=queryReadContext(both.state,{contract:'source-query/1',operation:'bounded-range',document,snapshot:both.result.context,profile:'base',lo:0,hi:2},current);
  const bothAnnotations=queryReadContext(both.state,{...page,snapshot:both.result.context},current);
  control('one-context-binds-source-and-annotations',equal(bothSource.result.source,[65,66])&&bothAnnotations.result.records[0].content.body==='body');
  const changedRoot=clone(root);changedRoot.source=[90];changedRoot.sourceRevision=2;changedRoot.records[0].body='new';
  control('later-root-does-not-relabel-capture',queryReadContext(source.state,{contract:'source-query/1',operation:'bounded-range',document,snapshot:source.result.context,profile:'base',lo:0,hi:2},current).result.ref.revision===1);
  const capBefore=clone(candidate);root.records[0].body='mutated';root.source[0]=90;
  control('prepared-candidate-is-owned',equal(candidate,capBefore)&&Object.isFrozen(candidate.annotations.rows));
  control('exact-historical-selection',prepareReadContext(changedRoot,{...request,selection:{kind:'retained',reference:'retained-root'}},current,limits)!==null);
  control('missing-history-not-current',prepareReadContext(changedRoot,{...request,selection:{kind:'retained',reference:'missing'}},current,limits)===null);
  for(const bad of [{...request,data:[]},{...request,data:['execute']},{...request,data:['source','source']},
    {...request,selection:{kind:'latest-fallback'}},{...request,extra:true},{...request,contract:'read-context/2'}])
    control(`closed-acquisition-product-${controls.length}`,prepareReadContext(root,bad,current,limits)===null);
  for(const key of ['captureComplete','preparationReserved','granted','live','continuous'])
    control(`prepare-${key}`,prepareReadContext(root,request,{...current,[key]:false},limits)===null);
  for(const [name,patch] of [['context-capacity',{contexts:0}],['unit-capacity',{units:2}],['selector-capacity',{selectors:0}],['issuance-exhausted',{issuance:0}]])
    control(name,acquireReadContext(newReadRegistry('i',{...limits,...patch}),candidate,current).result.status==='unavailable');
  denied('all-selectors-reserved-before-page',{...acquired.state,limits:{...limits,selectors:3}});
  denied('issuance-exhausted-before-page',{...acquired.state,limits:{...limits,issuance:3}});
  denied('reply-bound-no-delivery',first.state,page,{...current,replyReady:false});
  const full={...first.state,limits:{...limits,selectors:first.state.selectors.length,units:3,contexts:1}};
  control('protected-stop-survives-full-capacity',closeReadContext(full,stop,'reader').result.status==='stopped');
  control('hold-bound',holdReadContext({...first.state,limits:{...limits,holds:0}},token).ticket===null);
  denied('foreign-issuer-cannot-resolve',acquireReadContext(newReadRegistry('issuer2',limits),candidate,current).state);
  control('diagnostics-not-authority',equal(queryReadContext(first.state,page,current),queryReadContext(first.state,page,{...current,diagnosticFailed:true})));
  let pairedCases=0;
  for(let mask=0;mask<16;mask++)for(let hidden=0;hidden<4;hidden++){
    const leftRoot=clone(originalRoot);leftRoot.records=leftRoot.records.slice(0,2);
    for(let i=0;i<2;i++){leftRoot.records[i].bodyVisible=!!(mask&(1<<(2*i)));leftRoot.records[i].targetVisible=!!(mask&(1<<(2*i+1)));}
    const rightRoot=clone(leftRoot);rightRoot.records.push({reference:`hidden-${hidden}`,visible:false,body:'secret'.repeat(hidden),bodyVisible:true,targetVisible:true});
    const left=acquireReadContext(initial,prepareReadContext(leftRoot,request,current,limits),current);
    const right=acquireReadContext(initial,prepareReadContext(rightRoot,request,current,limits),current);
    assert.deepEqual(left,right);
    assert.deepEqual(queryReadContext(left.state,{...page,snapshot:left.result.context},current),queryReadContext(right.state,{...page,snapshot:right.result.context},current));
    pairedCases++;
  }
  // Explore actual registry actions for a bounded trace horizon, including held retirement and re-acquisition.
  let transitionCases=0;const states=new Map(),queue=[acquired.state];states.set(JSON.stringify(queue[0]),0);
  for(let i=0;i<queue.length;i++){
    const m=queue[i],depth=states.get(JSON.stringify(m));invariants(m);if(depth===5)continue;
    const activeContext=m.contexts.find(c=>c.state==='active'),t=activeContext?.token??token;
    const actions=[acquireReadContext(m,candidate,current).state,queryReadContext(m,{...page,snapshot:t},current).state,
      invalidateReadContext(m,t),closeReadContext(m,{...stop,context:t},'reader').state,holdReadContext(m,t).state,
      releaseReadHold(m,m.holds[0]?.ticket??'absent')];
    for(const n of actions){invariants(n);assert(n.issued>=m.issued);transitionCases++;
      for(const old of m.contexts.filter(c=>c.state==='invalidated'))assert(!n.contexts.some(c=>c.token===old.token&&c.state==='active'));
      const key=JSON.stringify(n);if(!states.has(key)){states.set(key,depth+1);queue.push(n);}}
  }
  return {status:'read-context-registry-only',admissionCases,pairedCases,transitionCases,states:states.size,controls,
    physicalAccountingProved:false,hostPolicyProved:false,implementationRefinement:false};
}
export async function checkReadContextMutations(){
  const source=fs.readFileSync(new URL('./read-contexts.mjs',import.meta.url),'utf8');
  const mutations=[
    ['omit-policy-from-scope',"'destination','policy','continuity'","'destination','continuity'"],
    ['ignore-current-policy','scopeKeys.every(k=>equal(a[k],b[k]))',"scopeKeys.filter(k=>k!=='policy').every(k=>equal(a[k],b[k]))"],
    ['refund-held-root',"if(!n.holds.some(h=>h.context===token))n.contexts=",'n.contexts='],
    ['reuse-retired-identities','const n=clone(m),c=n.contexts.find(c=>c.token===token);','const n=clone(m);n.issued=0;const c=n.contexts.find(c=>c.token===token);'],
    ['cross-context-record','if(!record||record.context!==c.token)return fail();','if(!record)return fail();'],
    ['expose-raw-page-offset','delete result.next;','/* leak next */'],
    ['omit-selector-reservation','if(!room(m,pending.length))return fail();','/* omit capacity */'],
    ['omit-registry-readiness','!current.registryReady||!current.retentionReady','!current.retentionReady']
  ];
  const rejected=[];
  for(const [name,from,to] of mutations){
    assert(source.includes(from),`missing mutation site ${name}`);
    let changed=source.replace(from,to);
    for(const file of ['annotation-queries.mjs','source-products.mjs'])changed=changed.replace(`from './${file}'`,`from '${new URL(file,import.meta.url).href}'`);
    for(const file of ['read-contexts.json','operation-families.json'])changed=changed.replaceAll(`new URL('./${file}',import.meta.url)`,`new URL('${new URL(file,import.meta.url).href}')`);
    const module=await import(`data:text/javascript;base64,${Buffer.from(changed).toString('base64')}`);
    let failed=false;try{module.checkReadContexts();}catch(e){assert.equal(e.code,'ERR_ASSERTION',`non-assertion mutation failure: ${name}`);failed=true;}
    assert(failed,`surviving mutation ${name}`);rejected.push(name);
  }
  return rejected;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkReadContexts(),mutations:await checkReadContextMutations()},null,2));
