// One local published-cut reference composed with existing source/annotation registries.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {isDeepStrictEqual as equal} from 'node:util';
import {pathToFileURL} from 'node:url';
import {newReadRegistry,prepareReadContext,acquireReadContext,queryReadContext,closeReadContext,
  invalidateReadContext,holdReadContext,releaseReadHold,nativeReadProductExact,readProjectionClassesMatch} from './read-contexts.mjs';
import {decideExact} from './source-edit-contracts.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./version-set-contexts.json',import.meta.url),'utf8'));
const copy=structuredClone;
const exact=(v,keys)=>v&&typeof v==='object'&&!Array.isArray(v)&&equal(Object.keys(v).sort(),[...keys].sort());
const unavailable=()=>({status:'unavailable'});
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
const sameScope=(a,b)=>catalog.scopeFields.every(k=>equal(a[k],b[k]));
const memberHost=(host,document)=>host.members.filter(m=>equal(m.document,document));
const groupFor=(m,token)=>m.groups.find(g=>g.token===token||g.members.some(p=>p.snapshot===token));
const groupReady=(g,host)=>g?.state==='active'&&host.granted&&host.live&&host.continuous&&sameScope(g.scope,host)&&g.members.every(p=>{
  const found=memberHost(host,p.document);return found.length===1&&found[0].granted&&found[0].live&&found[0].continuous&&
    sameScope(found[0],g.scope)&&equal(found[0].document,p.document)&&equal(found[0].stamp,p.stamp);
});
const internal=(host,document)=>memberHost(host,document)[0];

export function newVersionRegistry(issuer,limits){return {registry:newReadRegistry(issuer,limits),groups:[],groupLimit:limits.groups};}
export function versionSetAcquisitionInputForm(){return freeze({family:catalog.family,contract:catalog.contract,operation:catalog.variant,variant:catalog.variant,fields:copy(catalog.fields),participantFields:copy(catalog.participantFields),selections:copy(catalog.selections)});}

const nativeSetResultFields=freeze({'version-set':['status','context','participants'],unavailable:['status']});
export function versionSetAcquisitionResultForms(){return nativeSetResultFields;}
export function versionSetAcquisitionResultValid(out,request,expectedContext=null){
 if(!out||typeof out.status!=='string'||!Object.hasOwn(nativeSetResultFields,out.status)||!nativeReadProductExact(out,nativeSetResultFields[out.status]))return false;if(out.status==='unavailable')return true;
 if(request.contract!=='read-version-set/1'||out.context===null||typeof out.context!=='object'||Array.isArray(out.context)||expectedContext!==null&&out.context!==expectedContext||request.selection?.kind==='retained'&&out.context===request.selection.reference||!Array.isArray(out.participants)||out.participants.length<1||out.participants.length!==request.participants.length)return false;
 for(let i=0;i<out.participants.length;i++){if(!Object.hasOwn(out.participants,i))return false;const p=out.participants[i],r=request.participants[i];if(!nativeReadProductExact(p,['document','snapshot','data'])||!equal(p.document,r.document)||!readProjectionClassesMatch(p.data,r.data)||p.snapshot===null||typeof p.snapshot!=='object'||Array.isArray(p.snapshot)||p.snapshot===out.context)return false;for(let j=0;j<i;j++)if(out.participants[j].snapshot===p.snapshot)return false;}
 return true;
}

export function prepareVersionSet(authority,request,host,limits){
  if(!exact(request,catalog.fields)||request.contract!==catalog.contract||request.operation!=='snapshot-handoff'||
      request.authority!==authority.id||request.profile!==host.profile||
      !Array.isArray(request.participants)||!request.participants.length||request.participants.length>limits.participants||
      !request.selection||!Object.hasOwn(catalog.selections,request.selection.kind)||
      !exact(request.selection,['kind',...catalog.selections[request.selection.kind]])||
      !host.captureComplete||!host.preparationReserved||authority.retained.length>limits.retainedCuts)return null;
  // One immutable owner root lookup. No per-document current-root callbacks.
  const matches=request.selection.kind==='capture-current'?[authority.current]:authority.retained.filter(c=>c.reference===request.selection.reference);
  if(matches.length!==1)return null;
  const cut=matches[0];if(cut.roots.length>limits.inputRoots)return null;
  const keys=new Set(),members=[];
  for(const p of request.participants){
    if(!exact(p,catalog.participantFields)||!exact(p.document,['authority','id','incarnation'])||p.document.authority!==authority.id)return null;
    const key=JSON.stringify([p.document.authority,p.document.id]);if(keys.has(key))return null;keys.add(key);
    const roots=cut.roots.filter(r=>equal(r.document,p.document)),hosts=memberHost(host,p.document);
    if(roots.length!==1||hosts.length!==1||!sameScope(hosts[0],host))return null;
    const single={contract:'read-context/1',operation:'snapshot-handoff',document:p.document,selection:{kind:'capture-current'},data:p.data,profile:request.profile};
    const candidate=prepareReadContext(roots[0],single,hosts[0],limits);
    if(!candidate)return null;
    members.push({document:copy(p.document),candidate,stamp:copy(hosts[0].stamp)});
  }
  return freeze({scope:Object.fromEntries(catalog.scopeFields.map(k=>[k,copy(host[k])])),cut:cut.identity,members});
}

export function acquireVersionSet(m,candidate,host){
  const fail=()=>({state:m,result:unavailable()});
  if(!candidate||!sameScope(candidate.scope,host)||!host.registryReady||!host.retentionReady||!host.notStopped||!host.transferReady||!host.replyReady||
      m.groups.length>=m.groupLimit||m.registry.selectors.length+m.groups.length+1>m.registry.limits.selectors||
      m.registry.issued>=m.registry.limits.issuance)return fail();
  const n=copy(m),parent=`${n.registry.issuer}:private-${++n.registry.issued}`,members=[];
  // Provisional construction is private. No member entry/selector is published on partial failure.
  const selectorLimit=n.registry.limits.selectors;n.registry.limits.selectors-=n.groups.length+1;
  for(const p of candidate.members){
    const hosts=memberHost(host,p.document);if(hosts.length!==1||!equal(hosts[0].stamp,p.stamp))return fail();
    const installed=acquireReadContext(n.registry,p.candidate,hosts[0]);if(installed.result.status!=='context')return fail();
    n.registry=installed.state;
    n.registry.contexts.find(c=>c.token===installed.result.context).parent=parent;
    members.push({document:copy(p.document),snapshot:installed.result.context,data:installed.result.data,stamp:copy(p.stamp)});
  }
  n.registry.limits.selectors=selectorLimit;
  const group={token:parent,state:'active',scope:copy(candidate.scope),cut:candidate.cut,members};
  if(!groupReady(group,host))return fail();n.groups.push(group);
  return {state:n,result:{status:'version-set',context:parent,participants:members.map(({document,snapshot,data})=>({document:copy(document),snapshot,data:copy(data)}))}};
}

export function queryVersionMember(m,request,host){
  const g=groupFor(m,request.snapshot),p=g?.members.find(p=>p.snapshot===request.snapshot&&equal(p.document,request.document));
  if(!p||!groupReady(g,host)||!g.members.every(p=>m.registry.contexts.some(c=>c.token===p.snapshot&&c.state==='active'&&c.parent===g.token))||
      !host.transferReady||!host.replyReady)return {state:m,result:unavailable()};
  const registry=copy(m.registry),limit=registry.limits.selectors;registry.limits.selectors-=m.groups.length;
  const out=queryReadContext(registry,request,{...internal(host,p.document),versionSetPermit:g.token,replyReady:host.replyReady});
  if(out.result.status==='unavailable')return {state:m,result:out.result};
  out.state.limits.selectors=limit;
  return {state:{...copy(m),registry:out.state},result:out.result};
}

export function invalidateVersionSet(m,token){
  const existing=groupFor(m,token);if(!existing)return m;
  const n=copy(m),g=n.groups.find(g=>g.token===existing.token);g.state='invalidated';
  for(const p of g.members)n.registry=invalidateReadContext(n.registry,p.snapshot);
  if(!g.members.some(p=>n.registry.contexts.some(c=>c.token===p.snapshot)))n.groups=n.groups.filter(x=>x.token!==g.token);
  return n;
}
export function stopVersionSet(m,request,recipient){
  const g=groupFor(m,request.context);
  if(!exact(request,['contract','operation','context'])||request.contract!=='read-context/1'||request.operation!=='stop-observation'||
      !g||g.state!=='active'||g.scope.recipient!==recipient)return {state:m,result:unavailable()};
  return {state:invalidateVersionSet(m,g.token),result:{status:'stopped'}};
}
export function holdVersionMember(m,token){
  const g=groupFor(m,token);if(!g||g.state!=='active')return {state:m,ticket:null};
  const out=holdReadContext(m.registry,token);return {state:{...copy(m),registry:out.state},ticket:out.ticket};
}
export function releaseVersionHold(m,ticket){
  const n={...copy(m),registry:releaseReadHold(m.registry,ticket)};
  n.groups=n.groups.filter(g=>g.state==='active'||g.members.some(p=>n.registry.contexts.some(c=>c.token===p.snapshot)));
  return n;
}

// Finite authority fixture: atomic publications update both selected roots in one action.
export function publishFixture(authority,value,ids=authority.current.roots.map(r=>r.document.id)){
  const n=copy(authority);
  const selected=n.current.roots.filter(r=>ids.includes(r.document.id));
  if(selected.every(r=>equal(r.source,[48+value])&&r.records[0].body===`body-${value}`))return freeze(n);
  n.current.publication=(n.current.publication??0)+1;n.current.identity=`cut-${n.current.publication}`;n.current.reference=`history-${n.current.publication}`;
  for(const r of selected){
    if(!equal(r.source,[48+value])){r.source=[48+value];r.sourceRevision++;}
    if(r.records[0].body!==`body-${value}`){r.records[0].body=`body-${value}`;r.records[0].contentVersion++;}
  }
  return freeze(n);
}

function invariants(m){
  assert(m.groups.length<=m.groupLimit);
  assert(m.registry.selectors.length+m.groups.length<=m.registry.limits.selectors);
  assert(m.registry.contexts.length<=m.registry.limits.contexts&&m.registry.holds.length<=m.registry.limits.holds);
  assert(m.registry.contexts.reduce((n,c)=>n+c.units,0)<=m.registry.limits.units);
  assert(m.registry.issued<=m.registry.limits.issuance);
  const tokens=[...m.groups.map(g=>g.token),...m.registry.selectors.map(s=>s.token),...m.registry.holds.map(h=>h.ticket)];
  assert.equal(new Set(tokens).size,tokens.length);
  for(const g of m.groups)for(const p of g.members){const c=m.registry.contexts.find(c=>c.token===p.snapshot);
    if(g.state==='active')assert(c?.state==='active'&&c.parent===g.token);
    else assert(!c||c.state==='invalidated');}
  for(const c of m.registry.contexts)assert(m.groups.some(g=>g.token===c.parent&&g.members.some(p=>p.snapshot===c.token)));
  for(const s of m.registry.selectors)assert(m.registry.contexts.some(c=>c.token===s.context&&c.state==='active'));
  for(const h of m.registry.holds)assert(m.registry.contexts.some(c=>c.token===h.context));
}

export function checkVersionSets(){
  const limits={groups:2,participants:3,inputRoots:8,retainedCuts:3,contexts:6,units:64,selectors:24,holds:4,issuance:128,sourceBytes:32,
    source:{maxRootBytes:32,maxInputBytes:32,maxContentBytes:32,maxReplyBytes:2048},
    annotation:{maxInputRows:8,maxCaptureBytes:4096,maxRowBytes:512,maxPageRows:2}};
  const doc=id=>({authority:'a',id,incarnation:1});
  const root=id=>({document:doc(id),source:[48],sourceRevision:1,records:[{reference:'same-local-id',visible:true,bodyVisible:true,targetVisible:false,body:'body-0',contentVersion:1}]});
  const authority=freeze({id:'a',current:{identity:'cut-0',reference:'history-0',roots:[root('A'),root('B')]},retained:[]});
  const common={recipient:'reader',purpose:'edit',destination:'view',policy:'p1',continuity:'k1',profile:'base',
    granted:true,live:true,continuous:true,transferReady:true,replyReady:true,registryReady:true,retentionReady:true,notStopped:true,captureComplete:true,preparationReserved:true};
  const host={...common,members:['A','B'].map(id=>({...common,document:doc(id),stamp:{membership:1}}))};
  const request={contract:catalog.contract,operation:'snapshot-handoff',authority:'a',participants:['A','B'].map(id=>({document:doc(id),data:['source','annotations']})),selection:{kind:'capture-current'},profile:'base'};
  const candidate=prepareVersionSet(authority,request,host,limits),initial=newVersionRegistry('issuer',limits);
  const acquired=acquireVersionSet(initial,candidate,host),members=acquired.result.participants;
  const range=p=>({contract:'source-query/1',operation:'bounded-range',document:p.document,snapshot:p.snapshot,profile:'base',lo:0,hi:1});
  const enumeration=p=>({contract:'annotation-query/1',operation:'enumerate',document:p.document,snapshot:p.snapshot,profile:'base',limit:2,continuation:null});
  const stop={contract:'read-context/1',operation:'stop-observation',context:acquired.result.context};
  const controls=[],control=(name,ok)=>{assert(ok,name);controls.push(name);};
  const refuse=(name,h=host,m=initial,c=candidate)=>{const before=copy({m,c,h}),out=acquireVersionSet(m,c,h);assert.deepEqual(out,{state:m,result:unavailable()});assert.deepEqual({m,c,h},before);controls.push(name);};
  const deny=(name,r=range(members[0]),h=host,m=acquired.state)=>{const before=copy({m,r,h}),out=queryVersionMember(m,r,h);assert.deepEqual(out.result,unavailable());assert.deepEqual(out.state,m);assert.deepEqual({m,r,h},before);controls.push(name);};
  let admissionCases=0;
  for(let mask=0;mask<256;mask++){
    const h=copy(host);h.registryReady=!!(mask&1);h.retentionReady=!!(mask&2);h.notStopped=!!(mask&4);h.transferReady=!!(mask&8);
    for(let i=0;i<2;i++){h.members[i].granted=!!(mask&(1<<(4+2*i)));h.members[i].live=!!(mask&(1<<(5+2*i)));}
    const before=copy(initial),out=acquireVersionSet(initial,candidate,h);
    assert.equal(out.result.status,mask===255?'version-set':'unavailable');if(mask!==255)assert.deepEqual(out.state,initial);
    assert.deepEqual(initial,before);invariants(out.state);admissionCases++;
  }
  let coherentCases=0;
  for(let capture=0;capture<4;capture++)for(let later=0;later<4;later++)for(const reverse of [false,true]){
    const a=publishFixture(authority,capture),c=prepareVersionSet(a,{...request,participants:reverse?[...request.participants].reverse():request.participants},host,limits);
    const installed=acquireVersionSet(initial,c,host),after=publishFixture(a,later);
    const expectedRevision=capture===0?1:2,newer=prepareVersionSet(after,request,host,limits);
    assert(newer.members.every(p=>p.candidate.root.sourceRevision===expectedRevision+(later===capture?0:1)));
    for(const p of installed.result.participants){const out=queryVersionMember(installed.state,range(p),host);assert.deepEqual(out.result.source,[48+capture]);assert.equal(out.result.ref.revision,expectedRevision);
      const annotation=queryVersionMember(installed.state,enumeration(p),host);assert.equal(annotation.result.records[0].content.body,`body-${capture}`);assert.equal(annotation.result.records[0].content.version,expectedRevision);}
    assert.equal(installed.state.groups[0].cut,capture===0?'cut-0':'cut-1');coherentCases++;
  }
  control('members-one-parent',members.length===2&&members.every(p=>acquired.state.registry.contexts.find(c=>c.token===p.snapshot).parent===acquired.result.context));
  control('no-public-cut-ordinal',!JSON.stringify(acquired.result).includes('cut-0')&&!Object.hasOwn(acquired.result,'generation'));
  const oldA=prepareReadContext(authority.current.roots[0],{contract:'read-context/1',operation:'snapshot-handoff',document:doc('A'),selection:{kind:'capture-current'},data:['source'],profile:'base'},host.members[0],limits);
  const advanced=publishFixture(authority,1);
  const newB=prepareReadContext(advanced.current.roots[1],{contract:'read-context/1',operation:'snapshot-handoff',document:doc('B'),selection:{kind:'capture-current'},data:['source'],profile:'base'},host.members[1],limits);
  control('independent-current-reads-can-form-unpublished-vector',oldA.root.sourceRevision===1&&newB.root.sourceRevision===2&&
    [authority.current,advanced.current].every(c=>!equal(c.roots.map(r=>r.sourceRevision),[oldA.root.sourceRevision,newB.root.sourceRevision])));
  const distinct=publishFixture(authority,1,['B']),distinctSet=acquireVersionSet(initial,prepareVersionSet(distinct,request,host,limits),host);
  control('coherent-cut-does-not-require-equal-document-revisions',equal(distinctSet.result.participants.map(p=>queryVersionMember(distinctSet.state,range(p),host).result.ref.revision),[1,2]));
  const aPage=queryVersionMember(acquired.state,enumeration(members[0]),host),bPage=queryVersionMember(aPage.state,enumeration(members[1]),host);
  control('same-local-record-id-distinct-scoped-selectors',aPage.result.records[0].reference!==bPage.result.records[0].reference);
  deny('record-selector-cannot-cross-member',{contract:'annotation-query/1',operation:'inspect',document:doc('B'),snapshot:members[1].snapshot,profile:'base',annotation:aPage.result.records[0].reference},host,bPage.state);
  const pagedAuthority=copy(authority);
  for(const r of pagedAuthority.current.roots)r.records.push({...copy(r.records[0]),reference:'second-record'});
  const paged=acquireVersionSet(initial,prepareVersionSet(pagedAuthority,request,host,limits),host),pm=paged.result.participants;
  const firstPage=queryVersionMember(paged.state,{...enumeration(pm[0]),limit:1},host);
  deny('continuation-cannot-cross-member',{...enumeration(pm[1]),limit:1,continuation:firstPage.result.continuation},host,firstPage.state);
  const finalPage=queryVersionMember(firstPage.state,{...enumeration(pm[0]),limit:1,continuation:firstPage.result.continuation},host);
  control('within-set-continuation-completes',finalPage.result.status==='page'&&finalPage.result.records.length===1&&finalPage.result.continuation===null);
  deny('parent-is-not-member-source-context',{...range(members[0]),snapshot:acquired.result.context});
  deny('wrong-document-for-member',{...range(members[0]),document:doc('B')});
  control('base-query-cannot-bypass-parent-check',queryReadContext(acquired.state.registry,range(members[0]),host.members[0]).result.status==='unavailable');
  control('base-close-cannot-partially-close-member',closeReadContext(acquired.state.registry,{...stop,context:members[0].snapshot},'reader').result.status==='unavailable');
  const evicted=copy(acquired.state);evicted.registry=invalidateReadContext(evicted.registry,members[1].snapshot);
  deny('other-member-retention-loss-refuses-whole-read',range(members[0]),host,evicted);
  for(const field of ['granted','live','continuous'])deny(`authority-context-${field}`,range(members[0]),{...host,[field]:false});
  for(const field of ['granted','live','continuous']){const h=copy(host);h.members[1][field]=false;deny(`other-member-${field}`,range(members[0]),h);}
  for(const field of catalog.scopeFields){const h=copy(host);h.members[1][field]='other';deny(`other-member-scope-${field}`,range(members[0]),h);}
  const changedStamp=copy(host);changedStamp.members[1].stamp.membership=2;deny('actual-membership-dependency',range(members[0]),changedStamp);refuse('membership-changed-before-install',changedStamp);
  control('different-member-dependency-stamps-supported',acquireVersionSet(initial,prepareVersionSet(authority,request,changedStamp,limits),changedStamp).result.status==='version-set');
  refuse('complete-envelope-reservation-required',{...host,replyReady:false});
  const absent=copy(host);absent.members.pop();deny('missing-required-current-member',range(members[0]),absent);refuse('missing-member-at-final-install',absent);
  const doubled=copy(host);doubled.members.push(copy(host.members[1]));deny('ambiguous-current-member',range(members[0]),doubled);
  refuse('late-second-member-refusal', {...host,members:[host.members[0],{...host.members[1],granted:false}]});
  for(const [name,patch] of [['parent-and-members-selector-capacity',{selectors:2}],['member-row-capacity',{contexts:1}],['aggregate-unit-capacity',{units:3}],['parent-issuance-capacity',{issuance:2}],['group-capacity',{groups:0}]])
    refuse(name,host,newVersionRegistry('issuer',{...limits,...patch}));
  for(const [name,bad] of [['empty-set',{...request,participants:[]}],['duplicate-owner',{...request,participants:[request.participants[0],request.participants[0]]}],
    ['foreign-authority',{...request,authority:'other'}],['foreign-member',{...request,participants:[{document:{...doc('B'),authority:'other'},data:['source']}]}],
    ['unknown-member',{...request,participants:[{document:doc('C'),data:['source']}]}],['old-incarnation',{...request,participants:[{document:{...doc('A'),incarnation:2},data:['source']}]}],
    ['unknown-field',{...request,execute:true}],['unknown-selection',{...request,selection:{kind:'join-contexts',references:[]}}]])
    control(name,prepareVersionSet(authority,bad,host,limits)===null);
  const history=publishFixture(authority,1),withHistory={...history,retained:[authority.current]};
  const historical=prepareVersionSet(withHistory,{...request,selection:{kind:'retained',reference:'history-0'}},host,limits);
  control('exact-retained-cut',historical.cut==='cut-0'&&historical.members.every(p=>p.candidate.root.sourceRevision===1));
  control('history-missing-not-latest',prepareVersionSet(withHistory,{...request,selection:{kind:'retained',reference:'missing'}},host,limits)===null);
  control('history-ambiguous-refuses',prepareVersionSet({...withHistory,retained:[authority.current,authority.current]},{...request,selection:{kind:'retained',reference:'history-0'}},host,limits)===null);
  const held=holdVersionMember(bPage.state,members[1].snapshot),closed=stopVersionSet(held.state,{...stop,context:members[0].snapshot},'reader');
  control('member-stop-closes-entire-set',closed.result.status==='stopped'&&closed.state.groups[0]?.state==='invalidated'&&closed.state.registry.selectors.length===0);
  control('held-member-retained-unheld-member-reclaimed',closed.state.registry.contexts.length===1&&closed.state.registry.contexts[0].token===members[1].snapshot);
  deny('closed-parent-no-read',range(members[1]),host,closed.state);
  const released=releaseVersionHold(closed.state,held.ticket);control('last-hold-retires-parent',released.groups.length===0&&released.registry.contexts.length===0);
  control('hold-discharge-idempotent',equal(releaseVersionHold(released,held.ticket),released));
  control('foreign-owner-cannot-stop-set',equal(stopVersionSet(acquired.state,stop,'other'),{state:acquired.state,result:unavailable()}));
  control('stop-after-content-revocation',stopVersionSet(acquired.state,stop,'reader').result.status==='stopped');
  const invalid=invalidateVersionSet(acquired.state,members[1].snapshot);deny('regrant-does-not-revive-set',range(members[0]),host,invalid);
  const fresh=acquireVersionSet(invalid,candidate,host);control('new-set-new-identity',fresh.result.context!==acquired.result.context);
  const subsetRequest={...request,participants:[request.participants[0]]},subset=acquireVersionSet(invalid,prepareVersionSet(authority,subsetRequest,host,limits),host);
  control('explicit-subset-is-new-context',subset.result.context!==acquired.result.context&&subset.result.participants.length===1);
  const returned=members.map(p=>queryVersionMember(acquired.state,range(p),host).result);
  const editState={authority:'a',generation:0,documents:returned.map(r=>({ref:copy(r.ref),source:copy(r.source)}))};
  const proposal={reads:returned.map(r=>({ref:copy(r.ref)})),writes:[{ref:copy(returned[0].ref),edits:[{context:1,lo:0,hi:1,expected:[48],ins:[88]}]}],fixtureEditLimit:4};
  const gates={grant:true,scope:true,continuity:true,capacity:true,companions:true,notStopped:true};
  const applied=decideExact(editState,proposal,gates);assert.equal(applied.status,'applied');assert.deepEqual(applied.state.documents.map(d=>d.source),[[88],[48]]);
  const stale=copy(editState);stale.documents[1].source=[49];stale.documents[1].ref.revision=2;
  assert.deepEqual(decideExact(stale,proposal,gates),{status:'rejected',state:stale});
  const editCases=2;
  let scopeCases=0;const dataScopes=[['source'],['annotations'],['source','annotations']];
  for(const first of dataScopes)for(const second of dataScopes){
    const r={...request,participants:[{document:doc('A'),data:first},{document:doc('B'),data:second}]};
    const out=acquireVersionSet(initial,prepareVersionSet(authority,r,host,limits),host);
    assert.equal(out.result.status,'version-set');
    for(let i=0;i<2;i++){
      const p=out.result.participants[i],source=queryVersionMember(out.state,range(p),host),annotation=queryVersionMember(out.state,enumeration(p),host);
      assert.equal(source.result.status,[first,second][i].includes('source')?'content':'unavailable');
      assert.equal(annotation.result.status,[first,second][i].includes('annotations')?'page':'unavailable');
    }scopeCases++;
  }
  let pairedCases=0;
  for(let hidden=0;hidden<8;hidden++){
    const a=copy(authority);a.current.identity=`hidden-publication-${hidden}`;
    a.current.roots.push({...root('hidden'),source:Array(hidden).fill(88)});
    a.current.roots[0].records.push({reference:'private',visible:false,body:'secret'.repeat(hidden)});
    const out=acquireVersionSet(initial,prepareVersionSet(a,request,host,limits),host);
    assert.deepEqual(out.result,acquired.result);
    for(const p of out.result.participants)assert.deepEqual(queryVersionMember(out.state,enumeration(p),host).result,queryVersionMember(acquired.state,enumeration(p),host).result);
    pairedCases++;
  }
  let transitionCases=0;const states=new Map(),queue=[acquired.state];states.set(JSON.stringify(queue[0]),0);
  for(let i=0;i<queue.length;i++){
    const m=queue[i],depth=states.get(JSON.stringify(m));invariants(m);if(depth===4)continue;
    const g=m.groups.find(g=>g.state==='active'),p=g?.members[0]??members[0],t=g?.token??stop.context;
    const next=[acquireVersionSet(m,candidate,host).state,queryVersionMember(m,enumeration(p),host).state,
      stopVersionSet(m,{...stop,context:t},'reader').state,invalidateVersionSet(m,t),holdVersionMember(m,p.snapshot).state,releaseVersionHold(m,m.registry.holds[0]?.ticket??'absent')];
    for(const n of next){invariants(n);assert(n.registry.issued>=m.registry.issued);transitionCases++;
      const key=JSON.stringify(n);if(!states.has(key)){states.set(key,depth+1);queue.push(n);}}
  }
  return {status:'version-set-contexts-only',admissionCases,coherentCases,editCases,scopeCases,pairedCases,transitionCases,states:states.size,controls,
    hostCoherenceProved:false,physicalAccountingProved:false,implementationRefinement:false};
}
export async function checkVersionSetMutations(){
  const source=fs.readFileSync(new URL('./version-set-contexts.mjs',import.meta.url),'utf8');
  const mutations=[
    ['mix-member-snapshots','const candidate=prepareReadContext(roots[0],single,hosts[0],limits);',
      'const candidate=prepareReadContext(members.length?{...roots[0],source:[roots[0].source[0]+1],sourceRevision:roots[0].sourceRevision+1}:roots[0],single,hosts[0],limits);'],
    ['check-only-requested-member','g.members.every(p=>{','g.members.slice(0,1).every(p=>{'],
    ['ignore-required-retention',"!g.members.every(p=>m.registry.contexts.some(c=>c.token===p.snapshot&&c.state==='active'&&c.parent===g.token))",'false'],
    ['omit-parent-selector-charge','n.registry.limits.selectors-=n.groups.length+1;','n.registry.limits.selectors-=0;'],
    ['partial-member-install',"if(installed.result.status!=='context')return fail();","if(installed.result.status!=='context')return {state:{...n,registry:n.registry},result:unavailable()};"],
    ['current-fallback-for-missing-history',"if(matches.length!==1)return null;","if(matches.length!==1)matches.splice(0,matches.length,authority.current);"],
    ['member-stop-not-parent-stop','for(const p of g.members)n.registry=invalidateReadContext(n.registry,p.snapshot);','for(const p of g.members.slice(0,1))n.registry=invalidateReadContext(n.registry,p.snapshot);'],
    ['reclaim-held-members','n.registry=invalidateReadContext(n.registry,p.snapshot);','n.registry=invalidateReadContext(n.registry,p.snapshot);n.registry.contexts=[];'],
    ['ignore-query-stamp-change','equal(found[0].stamp,p.stamp)','true']
  ];
  const rejected=[];
  for(const [name,from,to] of mutations){
    assert(source.includes(from),`missing mutation site ${name}`);let changed=source.replace(from,to);
    for(const file of ['read-contexts.mjs','source-edit-contracts.mjs'])changed=changed.replace(`from './${file}'`,`from '${new URL(file,import.meta.url).href}'`);
    changed=changed.replaceAll("new URL('./version-set-contexts.json',import.meta.url)",`new URL('${new URL('version-set-contexts.json',import.meta.url).href}')`);
    const module=await import(`data:text/javascript;base64,${Buffer.from(changed).toString('base64')}`);
    let failed=false;try{module.checkVersionSets();}catch(e){assert.equal(e.code,'ERR_ASSERTION',`non-assertion failure ${name}`);failed=true;}
    assert(failed,`surviving mutation ${name}`);rejected.push(name);
  }
  return rejected;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkVersionSets(),mutations:await checkVersionSetMutations()},null,2));
