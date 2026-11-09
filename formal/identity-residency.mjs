// Private finite reference, not an editor backend, host oracle or public schema.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
const copy=x=>structuredClone(x);
const bits=[false,true];

export function resolveOwner({authorized,comparable,relation,continuityRequired,continuityValid,capacity}) {
  if(!['same','distinct','unknown'].includes(relation))throw Error('Unsupported relation');
  if(!authorized)return 'denied';
  if(!comparable||relation==='unknown')return 'unsupportedEvidence';
  if(continuityRequired&&!continuityValid)return 'unsupportedEvidence';
  if(relation==='same')return 'joinExisting';
  return capacity?'newOwner':'capacityRefused';
}

export function activateRegistry(state,key,authorized,limit) {
  // Trusted canonical keys within one declared identity domain; serialized call.
  const next=copy(state);
  if(!authorized)return {kind:'denied',state:next};
  if(next.owners.some(o=>o.key===key))return {kind:'joined',state:next};
  if(next.owners.length>=limit)return {kind:'capacityRefused',state:next};
  next.owners.push({key,body:`original-${key}`});
  return {kind:'created',state:next};
}

export function disposeOwner(state,{authorized,discardSource,discardAnnotations,retainObligations}) {
  const next=copy(state);
  if(!state.live||!authorized)return {kind:'refused',state:next};
  if(state.sourceDirty&&!discardSource)return {kind:'refused',state:next};
  if(state.annotationDirty&&!discardAnnotations)return {kind:'refused',state:next};
  if(state.obligations!==0&&!retainObligations)return {kind:'refused',state:next};
  next.live=false;next.sourceDirty=false;next.annotationDirty=false;
  next.views=0;next.members=0;next.derived=false;
  return {kind:'disposed',state:next};
}

export function dropDerived(state) {
  return {...copy(state),derived:false};
}
export function acceptsMutation(state,capturedInc,authorized) {
  return state.live&&state.inc===capturedInc&&authorized;
}
export function reopenOwner(state,{authorized,capacity}) {
  if(state.live||!authorized||!capacity||state.obligations!==0||state.inc===2)
    return {kind:'refused',state:copy(state)};
  return {kind:'opened',state:{...copy(state),inc:state.inc+1,live:true,derived:false}};
}
export function discharge(state,mask,witness) {
  // One witness authorizes only the selected obligation; cancellation is not one.
  return {...copy(state),obligations:witness?state.obligations&~mask:state.obligations};
}

export function checkIdentityResidency() {
  let identityCases=0,registryCases=0,disposalCases=0,cacheCases=0,mutationCases=0,reopenCases=0,dischargeCases=0;
  const states=[];
  for(const live of bits)for(const inc of [1,2])for(const sourceDirty of bits)
    for(const annotationDirty of bits)for(let obligations=0;obligations<16;obligations++)
      for(const derived of bits)for(const views of [0,1])for(const members of [0,1]) {
        if(!live&&(sourceDirty||annotationDirty||views||members))continue;
        states.push({live,inc,sourceDirty,annotationDirty,obligations,derived,views,members});
      }
  for(const authorized of bits)for(const comparable of bits)for(const relation of ['same','distinct','unknown'])
    for(const continuityRequired of bits)for(const continuityValid of bits)for(const capacity of bits) {
      const input={authorized,comparable,relation,continuityRequired,continuityValid,capacity};
      // Independent complete decision table, over trusted relation/context facts.
      const expected=!authorized?'denied':!comparable?'unsupportedEvidence':
        relation==='unknown'?'unsupportedEvidence':continuityRequired&&!continuityValid?'unsupportedEvidence':
        relation==='same'?'joinExisting':capacity?'newOwner':'capacityRefused';
      const before=copy(input);assert.equal(resolveOwner(input),expected);assert.deepEqual(input,before);identityCases++;
    }
  for(const a of [0,1,2,3])for(const b of [0,1,2,3])for(const reverse of bits)
    for(const grantA of bits)for(const grantB of bits)for(const limit of [1,2,3])
      for(const existing of [null,0,1,2,3]) {
        let current={owners:existing===null?[]:[{key:existing,body:`original-${existing}`}]};
        const expected=new Set(existing===null?[]:[existing]);
        const candidates=reverse?[[b,grantB],[a,grantA]]:[[a,grantA],[b,grantB]];
        for(const [key,authorized] of candidates) {
          const before=copy(current);const r=activateRegistry(current,key,authorized,limit);
          const kind=!authorized?'denied':expected.has(key)?'joined':expected.size<limit?'created':'capacityRefused';
          assert.equal(r.kind,kind);if(kind==='created')expected.add(key);
          assert.deepEqual(current,before);current=r.state;
          assert.equal(current.owners.length,new Set(current.owners.map(o=>o.key)).size);
          assert.deepEqual([...current.owners.map(o=>o.key)].sort(),[...expected].sort());
          for(const owner of before.owners)assert.deepEqual(current.owners.find(o=>o.key===owner.key),owner);
        }
        registryCases++;
      }
  for(const s of states) {
    const before=copy(s);
    for(const authorized of bits)for(const discardSource of bits)for(const discardAnnotations of bits)
      for(const retainObligations of bits) {
        const p={authorized,discardSource,discardAnnotations,retainObligations};
        const allowed=s.live&&authorized&&(!s.sourceDirty||discardSource)&&
          (!s.annotationDirty||discardAnnotations)&&(s.obligations===0||retainObligations);
        const r=disposeOwner(s,p);
        assert.equal(r.kind,allowed?'disposed':'refused');
        if(!allowed)assert.deepEqual(r.state,s);
        else {
          assert(!r.state.live&&!r.state.sourceDirty&&!r.state.annotationDirty);
          assert.equal(r.state.inc,s.inc);assert.equal(r.state.obligations,s.obligations);
          assert.equal(r.state.views,0);assert.equal(r.state.members,0);assert(!r.state.derived);
          assert(!acceptsMutation(r.state,s.inc,true));
        }
        disposalCases++;
      }
    const dropped=dropDerived(s);assert.deepEqual(dropped,{...s,derived:false});cacheCases++;
    for(const capturedInc of [1,2,3])for(const authorized of bits) {
      assert.equal(acceptsMutation(s,capturedInc,authorized),Boolean(s.live&&s.inc===capturedInc&&authorized));mutationCases++;
    }
    for(const authorized of bits)for(const capacity of bits) {
      const r=reopenOwner(s,{authorized,capacity});
      const allowed=!s.live&&authorized&&capacity&&s.obligations===0&&s.inc<2;
      assert.equal(r.kind,allowed?'opened':'refused');
      if(!allowed)assert.deepEqual(r.state,s);
      else {assert(r.state.live);assert.equal(r.state.inc,s.inc+1);assert(!acceptsMutation(r.state,s.inc,true));}
      reopenCases++;
    }
    for(const mask of [1,2,4,8])for(const witness of bits) {
      const r=discharge(s,mask,witness);
      const expected=s.obligations-(witness&&(s.obligations&mask)?mask:0);
      assert.deepEqual(r,{...s,obligations:expected});dischargeCases++;
    }
    assert.deepEqual(s,before);
  }
  const controls=[];
  const control=(name,test)=>{assert(test());controls.push(name);};
  const clean={live:true,inc:1,sourceDirty:false,annotationDirty:false,obligations:0,derived:true,views:0,members:0};
  const p={authorized:true,discardSource:false,discardAnnotations:false,retainObligations:true};
  const base={authorized:true,comparable:true,relation:'distinct',continuityRequired:true,continuityValid:true,capacity:true};
  control('unknown-is-not-distinct',()=>resolveOwner({...base,relation:'unknown'})==='unsupportedEvidence');
  control('cross-domain-is-not-comparable',()=>resolveOwner({...base,comparable:false})==='unsupportedEvidence');
  control('identity-is-not-continuity',()=>resolveOwner({...base,relation:'same',continuityValid:false})==='unsupportedEvidence');
  control('alias-joins-not-duplicates',()=>resolveOwner({...base,relation:'same',capacity:false})==='joinExisting');
  control('alias-is-not-authority',()=>resolveOwner({...base,relation:'same',authorized:false})==='denied');
  control('dirty-source-refuses',()=>disposeOwner({...clean,sourceDirty:true},p).kind==='refused');
  control('dirty-annotation-refuses',()=>disposeOwner({...clean,annotationDirty:true},p).kind==='refused');
  control('last-view-not-discard',()=>dropDerived({...clean,sourceDirty:true}).sourceDirty);
  control('retained-obligations-survive',()=>disposeOwner({...clean,obligations:15},p).state.obligations===15);
  control('no-retention-refuses',()=>disposeOwner({...clean,obligations:1},{...p,retainObligations:false}).kind==='refused');
  control('cancellation-is-not-discharge',()=>discharge({...clean,obligations:15},1,false).obligations===15);
  control('no-incarnation-wrap',()=>reopenOwner({...clean,live:false,inc:2},{authorized:true,capacity:true}).kind==='refused');
  const dead=disposeOwner(clean,p).state;
  const fresh=reopenOwner(dead,{authorized:true,capacity:true}).state;
  control('stale-work-does-not-resurrect',()=>fresh.live&&!acceptsMutation(fresh,1,true));
  control('effect-tombstone-prevents-reuse',()=>reopenOwner({...dead,obligations:8},{authorized:true,capacity:true}).kind==='refused');
  // Same bits can mean held history/snapshot/worker/effect. Real discharge truth is external.
  return {status:'finite-identity-residency-only',states:states.length,identityCases,registryCases,disposalCases,cacheCases,
    mutationCases,reopenCases,dischargeCases,controls};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkIdentityResidency(),null,2));
