// D04 reference profiles, not a public schema or production position index.
import assert from 'node:assert/strict';
import {validateCanonicalBatch,emitByOrigin} from './text-oracles.mjs';
const bytes=s=>Array.from(new TextEncoder().encode(s));
const validBoundary=(source,p)=>Number.isSafeInteger(p)&&p>=0&&p<=source.length&&
  (p===source.length||(source[p]&0xc0)!==0x80);

// One representative positional replacement rule. Affinity chooses an interior
// collapse boundary; pure insertion uses affinity. Replacement outer edges persist.
export function pointStep(p,e,affinity) {
  if(p<e.lo)return p;
  if(p>e.hi)return p+e.ins.length-(e.hi-e.lo);
  if(e.lo<e.hi&&p===e.lo)return e.lo;
  if(e.lo<e.hi&&p===e.hi)return e.lo+e.ins.length;
  return affinity==='right'?e.lo+e.ins.length:e.lo;
}
export function mapPointReverse(p,edits,affinity) {
  for(const e of [...edits].reverse())p=pointStep(p,e,affinity);
  return p;
}
function mapPointForward(p,edits,affinity) {
  let delta=0;
  for(const e of edits) {
    p=pointStep(p,{...e,lo:e.lo+delta,hi:e.hi+delta},affinity);
    delta+=e.ins.length-(e.hi-e.lo);
  }
  return p;
}
function coveredByIntervals(lo,hi,edits) {
  let cursor=lo;
  for(const e of edits) {
    if(e.hi<=cursor)continue;
    if(e.lo>cursor)break;
    cursor=Math.max(cursor,e.hi);
    if(cursor>=hi)return true;
  }
  return false;
}
function touched(lo,hi,edits) {
  return edits.some(e=>e.lo===e.hi?e.ins.length>0&&lo<e.lo&&e.lo<hi:e.lo<hi&&e.hi>lo);
}
export function mapAttachment(source,edits,target) {
  validateCanonicalBatch(source,edits);
  assert(['position','content'].includes(target.profile),'unsupported private profile');
  assert(['left','right'].includes(target.affinity),'missing private point affinity');
  assert(validBoundary(source,target.lo)&&validBoundary(source,target.hi)&&target.lo<=target.hi,'invalid private target');
  if(target.evidence==='detached'||target.evidence==='unavailable')return structuredClone(target);
  const collapsed=target.lo===target.hi;
  const lost=target.profile==='content'&&(collapsed?
    edits.some(e=>e.lo<target.lo&&target.lo<e.hi):coveredByIntervals(target.lo,target.hi,edits));
  const result={...target};
  if(lost) {result.evidence='detached';result.location=null;return result;}
  const lo=mapPointReverse(target.lo,edits,collapsed?target.affinity:target.profile==='content'?'right':'left');
  const hi=collapsed?lo:mapPointReverse(target.hi,edits,target.profile==='content'?'left':'right');
  result.location={lo,hi};
  result.evidence=target.evidence==='affected'||touched(target.lo,target.hi,edits)?'affected':'mapped';
  return result;
}
export function proposalEligible(proposal,currentRevision,granted) {
  return granted&&proposal.expectedRevision===currentRevision;
}

// Original-byte provenance oracle: replacements remove identities even for equal
// inserted values. It shares source admission/encoding but not coverage traversal.
function provenance(source,edits) {
  const output=[];let cursor=0;
  for(const e of edits) {
    for(;cursor<e.lo;cursor++)output.push({byte:source[cursor],origin:cursor});
    for(const byte of e.ins)output.push({byte,origin:null});
    cursor=e.hi;
  }
  for(;cursor<source.length;cursor++)output.push({byte:source[cursor],origin:cursor});
  return output;
}
function* words(depth,prefix='') {
  yield prefix;
  if(depth)for(const symbol of ['a','é','🙂'])yield* words(depth-1,prefix+symbol);
}
function* shapes(points,n,start=0) {
  if(!n){yield [];return;}
  for(let a=start;a<points.length-1;a++)for(let b=a+1;b<points.length;b++)
    for(const tail of shapes(points,n-1,b))yield [{lo:points[a],hi:points[b]},...tail];
}
function* variants(shape) {
  if(!shape.length){yield [];return;}
  for(const ins of [[],bytes('X'),bytes('🙂')])for(const tail of variants(shape.slice(1)))yield [{...shape[0],ins},...tail];
}
export function checkMappingProfiles() {
  let sources=0,batches=0,pointCases=0,targetCases=0;
  for(const text of words(3)) {
    sources++;
    const source=bytes(text),points=[0];
    for(const c of text)points.push(points.at(-1)+bytes(c).length);
    const all=[[],...points.flatMap(p=>[[],bytes('X'),bytes('🙂')].map(ins=>[{lo:p,hi:p,ins}]))];
    for(let n=1;n<=2;n++)for(const shape of shapes(points,n))for(const edits of variants(shape))all.push(edits);
    for(const edits of all) {
      batches++;validateCanonicalBatch(source,edits);
      const output=emitByOrigin(source,edits),origins=provenance(source,edits);
      assert.deepEqual(origins.map(e=>e.byte),output,'provenance bytes diverge');
      for(const p of points)for(const affinity of ['left','right']) {
        const q=mapPointReverse(p,edits,affinity);
        assert.equal(q,mapPointForward(p,edits,affinity),'batch mapping composition');
        assert(validBoundary(output,q),'mapped point bounds/scalar boundary');pointCases++;
      }
      for(let a=0;a<points.length;a++)for(let b=a;b<points.length;b++)for(const profile of ['position','content'])for(const affinity of ['left','right']) {
        const target={profile,affinity,lo:points[a],hi:points[b],evidence:'mapped',body:'authored comment',version:7};
        const before=structuredClone({source,edits,target});
        const mapped=mapAttachment(source,edits,target);
        assert.deepEqual({source,edits,target},before,'mapping mutated inputs');
        assert.equal(mapped.body,target.body);assert.equal(mapped.version,target.version);
        if(profile==='content'&&a<b) {
          const survives=origins.some(e=>e.origin!==null&&target.lo<=e.origin&&e.origin<target.hi);
          assert.equal(mapped.evidence==='detached',!survives,'range deletion/provenance disagreement');
        }
        if(mapped.evidence!=='detached') {
          assert(mapped.location.lo<=mapped.location.hi,'inverted mapped range');
          assert(validBoundary(output,mapped.location.lo)&&validBoundary(output,mapped.location.hi),'range boundary');
          if(a===b)assert.equal(mapped.location.lo,mapped.location.hi,'collapsed point widened');
        }
        targetCases++;
      }
    }
  }
  const controls=[];
  const detect=(name,condition)=>{assert(condition,`control escaped: ${name}`);controls.push(name);};
  const source=bytes('abcd');
  const target={profile:'content',affinity:'right',lo:1,hi:3,evidence:'mapped',body:'keep me',version:4};
  const apply=edits=>mapAttachment(source,edits,target);
  const start=apply([{lo:1,hi:1,ins:bytes('X')}]),end=apply([{lo:3,hi:3,ins:bytes('X')}]);
  detect('start boundary absorbs neighbor',start.location.lo===2&&start.location.hi===4);
  detect('end boundary absorbs neighbor',end.location.lo===1&&end.location.hi===3);
  const removed=apply([{lo:1,hi:3,ins:[]}]);
  detect('universal clamping claims target survived',removed.evidence==='detached'&&removed.location===null);
  detect('target deletion erases authored body',removed.body===target.body&&removed.version===target.version);
  const equal=apply([{lo:1,hi:3,ins:bytes('bc')}]);
  detect('equal replacement bytes prove identity',equal.evidence==='detached');
  const adjacent=apply([{lo:1,hi:2,ins:bytes('X')},{lo:2,hi:3,ins:bytes('Y')}]);
  detect('adjacent edits evade complete removal',adjacent.evidence==='detached');
  const partial=apply([{lo:1,hi:2,ins:bytes('X')}]);
  detect('partial change claims semantic validity',partial.evidence==='affected');
  detect('empty insertion manufactures affected evidence',apply([{lo:2,hi:2,ins:[]}]).evidence==='mapped');
  const insertion={lo:2,hi:2,ins:bytes('X')};
  detect('collapsed opposing affinities invert',pointStep(2,insertion,'right')>pointStep(2,insertion,'left'));
  const collapsed=mapAttachment(source,[insertion],{...target,lo:2,hi:2});
  detect('collapsed target silently widened',collapsed.location.lo===collapsed.location.hi);
  const detached={...target,evidence:'detached',location:null};
  detect('undo or equal quote silently reattaches',mapAttachment(source,[],detached).evidence==='detached');
  const unavailable={...target,evidence:'unavailable',location:null};
  detect('missing history guessed from coordinates',mapAttachment(source,[],unavailable).evidence==='unavailable');
  const proposal={expectedRevision:0,lo:target.lo,hi:target.hi};
  const proposedLocation=partial.location;
  const preview={...proposal,location:proposedLocation};
  detect('mapped proposal refreshes revision',proposedLocation!==null&&
    !proposalEligible(preview,1,true)&&proposalEligible(preview,0,true)&&!proposalEligible(preview,0,false));
  assert.throws(()=>mapAttachment(source,[],{...target,profile:'callback'}),/unsupported private profile/);
  controls.push('unknown profile rejected');
  assert.throws(()=>mapAttachment(source,[],{...target,affinity:undefined}),/missing private point affinity/);
  controls.push('implicit affinity rejected');
  return {status:'finite-mapping-profiles-only',sources,batches,pointCases,targetCases,controls,
    scope:'Same-author finite byte/provenance/reference mapping; representative replacement/point rules, not production schemas, D45 defaults, semantic identity or independent certification'};
}
if(process.argv[1]?.endsWith('/mapping-profiles.mjs'))console.log(JSON.stringify(checkMappingProfiles(),null,2));
