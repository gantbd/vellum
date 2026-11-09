// Design reference only. Fixture fields/limits are not a public engine schema.
import assert from 'node:assert/strict';
import {validateCanonicalBatch,emitByOrigin,applySequentialWithInverses,restoreInReverse} from './text-oracles.mjs';

const encoder=new TextEncoder();
const bytes=text=>Array.from(encoder.encode(text));
const dense=a=>Array.isArray(a)&&Array.from({length:a.length},(_,i)=>Object.hasOwn(a,i)).every(Boolean);
const sameOrder=(a,b)=>dense(a)&&dense(b)&&a.length===b.length&&a.every((e,i)=>e===b[i]);
const checkRecords=(source,context,edits,fixtureLimit)=>{
  assert(Number.isSafeInteger(fixtureLimit)&&fixtureLimit>=0,'invalid fixture limit');
  assert(Array.isArray(edits),'invalid fixture edit list');
  assert(edits.length<=fixtureLimit,'fixture normalization budget');
  assert(dense(edits),'invalid fixture edit list');
  validateCanonicalBatch(source,[]);
  for(const e of edits) {
    assert(e&&e.context===context,'wrong fixture source context');
    validateCanonicalBatch(source,[e]);
    assert(dense(e.expected)&&e.expected.every((v,i)=>Number.isInteger(v)&&v>=0&&v<=255&&
      v===source[e.lo+i])&&e.expected.length===e.hi-e.lo,'expected bytes mismatch');
  }
};

// Construction A: copy then sort, followed by ordered-domain validation.
export function normalizeBySort(source,context,edits,fixtureLimit) {
  checkRecords(source,context,edits,fixtureLimit);
  const result=edits.slice().sort((a,b)=>a.lo-b.lo);
  validateCanonicalBatch(source,result);
  return result;
}

// Construction B: reject pairwise ambiguity in input order, then assign ranks.
// No sort or ordered batch validator is used for the normalization relation.
export function normalizeByRank(source,context,edits,fixtureLimit) {
  checkRecords(source,context,edits,fixtureLimit);
  for(let i=0;i<edits.length;i++)for(let j=i+1;j<edits.length;j++) {
    const a=edits[i],b=edits[j];
    assert(a.lo!==b.lo,'unresolved pairwise tie');
    const [left,right]=a.lo<b.lo?[a,b]:[b,a];
    assert(left.hi<=right.lo,'pairwise overlap');
    assert(left.hi!==right.lo||(left.lo<left.hi&&right.lo<right.hi),'unresolved pairwise endpoint');
  }
  const result=new Array(edits.length);
  for(const e of edits)result[edits.filter(other=>other.lo<e.lo).length]=e;
  return result;
}

const alphabet=['a','é','🙂'];
const insertions=['','X','🙂'].map(bytes);
function* words(max,prefix='') {
  yield prefix;
  if(max)for(const symbol of alphabet)yield* words(max-1,prefix+symbol);
}
function positions(text) {
  const result=[0];
  for(const scalar of text)result.push(result.at(-1)+bytes(scalar).length);
  return result;
}
function* shapes(points,count,start=0) {
  if(!count){yield [];return;}
  for(let lo=start;lo<points.length-1;lo++)for(let hi=lo+1;hi<points.length;hi++)
    for(const tail of shapes(points,count-1,hi))yield [{lo:points[lo],hi:points[hi]},...tail];
}
function* variants(shape,index=0) {
  if(index===shape.length){yield [];return;}
  for(const ins of insertions)for(const tail of variants(shape,index+1))yield [{...shape[index],ins},...tail];
}
function* permutations(records) {
  if(!records.length){yield [];return;}
  for(let i=0;i<records.length;i++)for(const tail of permutations([...records.slice(0,i),...records.slice(i+1)]))
    yield [records[i],...tail];
}
const fixtureRecord=(source,e,tag,context='A@0')=>({...e,context,tag,
  expected:source.slice(e.lo,e.hi),meta:{origin:`fixture-${tag}`,plan:[tag,tag+1]}});

export function checkBatchNormalization() {
  let sources=0,canonicalBatches=0,permutationCases=0;
  const sizes={0:0,1:0,2:0,3:0};
  const verify=(source,edits)=>{
    canonicalBatches++;
    const records=edits.map((e,i)=>fixtureRecord(source,e,i));
    const expected=emitByOrigin(source,records);
    for(const input of permutations(records)) {
      const before=structuredClone({source,input});
      const sorted=normalizeBySort(source,'A@0',input,3);
      const ranked=normalizeByRank(source,'A@0',input,3);
      assert(sameOrder(sorted,records)&&sameOrder(ranked,records),'permutation or record identity lost');
      assert(sameOrder(normalizeBySort(source,'A@0',sorted,3),sorted),'sort idempotence');
      assert(sameOrder(normalizeByRank(source,'A@0',ranked,3),ranked),'rank idempotence');
      assert.deepEqual(emitByOrigin(source,sorted),expected,'forward meaning changed');
      const sequential=applySequentialWithInverses(source,ranked);
      assert.deepEqual(sequential.result,expected,'sequential meaning changed');
      assert.deepEqual(restoreInReverse(sequential.result,sequential.inverses),source,'inverse meaning changed');
      assert.deepEqual({source,input},before,'input/source/metadata mutated');
      sizes[records.length]++;permutationCases++;
    }
  };
  for(const text of words(3)) {
    sources++;
    const source=bytes(text),points=positions(text);
    verify(source,[]);
    for(const pos of points)for(const ins of insertions)verify(source,[{lo:pos,hi:pos,ins}]);
    for(let n=1;n<=3;n++)for(const shape of shapes(points,n))for(const edits of variants(shape))verify(source,edits);
  }
  const domainControls=[];
  const reject=(name,source,input,sortReason,rankReason=sortReason,limit=3)=>{
    const before=structuredClone({source,input});
    for(const [i,normalizer] of [normalizeBySort,normalizeByRank].entries()) {
      const reason=i===0?sortReason:rankReason;
      assert.throws(()=>normalizer(source,'A@0',input,limit),error=>error instanceof Error&&error.message===reason,name);
      assert.deepEqual({source,input},before,`rejection mutated input: ${name}`);
    }
    domainControls.push(name);
  };
  const source=bytes('abcd');
  const record=(lo,hi,ins=[],tag=lo)=>fixtureRecord(source,{lo,hi,ins},tag);
  reject('overlap',source,[record(0,2),record(1,3)],'unsorted or overlapping ranges','pairwise overlap');
  reject('coincident insertion',source,[record(1,1,[88],0),record(1,1,[89],1)],'unresolved insertion tie','unresolved pairwise tie');
  reject('replacement start tie',source,[record(0,0,[88]),record(0,1,[89])],'unresolved insertion tie','unresolved pairwise tie');
  reject('replacement end tie',source,[record(0,1,[88]),record(1,1,[89])],'unresolved insertion tie','unresolved pairwise endpoint');
  reject('interior insertion',source,[record(0,3),record(1,1,[88])],'unsorted or overlapping ranges','pairwise overlap');
  reject('duplicate edit is not deduplication',source,[record(0,1,[88],0),record(0,1,[88],1)],'unsorted or overlapping ranges','unresolved pairwise tie');
  reject('mismatched expected bytes',source,[{...record(0,1),expected:[120]}],'expected bytes mismatch');
  reject('wrong context despite equal bytes',source,[{...record(0,1),context:'B@0'}],'wrong fixture source context');
  reject('invalid scalar boundary',bytes('🙂'),[fixtureRecord(bytes('🙂'),{lo:1,hi:4,ins:[]},0)],'invalid source boundary');
  reject('invalid inserted bytes',source,[record(0,1,[255])],'invalid inserted bytes');
  reject('sparse inserted bytes',source,[record(0,1,Array(1))],'invalid inserted bytes');
  reject('sparse expected bytes',source,[{...record(0,1),expected:Array(1)}],'expected bytes mismatch');
  reject('sparse edit list',source,Array(1),'invalid fixture edit list');
  reject('invalid normalization bound',source,[],'invalid fixture limit','invalid fixture limit',NaN);
  reject('normalization exhaustion',source,[record(0,1)],'fixture normalization budget','fixture normalization budget',0);
  const mutationControls=[];
  const detect=(name,condition)=>{assert(condition,`mutation escaped: ${name}`);mutationControls.push(name);};
  detect('sparse order comparison false positive',!sameOrder(Array(1),[undefined])&&!sameOrder([undefined],Array(1)));
  const canonical=[record(0,1,[88],0),record(1,2,[89],1)],reversed=[...canonical].reverse();
  detect('input order used as canonical order',!sameOrder(reversed,canonical));
  const mutating=[...reversed],old=JSON.stringify(mutating);mutating.sort((a,b)=>a.lo-b.lo);
  detect('in-place sorting mutates request',JSON.stringify(mutating)!==old);
  detect('metadata omitted',!sameOrder(canonical.map(({lo,hi,ins})=>({lo,hi,ins})),canonical));
  const swapped=canonical.map((e,i)=>({...e,meta:canonical[1-i].meta}));
  detect('metadata swapped independently',JSON.stringify(swapped)!==JSON.stringify(canonical));
  const emptyRecord=record(0,0,[],8),withEmpty=[emptyRecord,record(2,3,[88],9)];
  const normalizedEmpty=normalizeBySort(source,'A@0',withEmpty,3);
  detect('empty edit elided',!sameOrder(normalizedEmpty.filter(e=>e.lo!==e.hi||e.ins.length),withEmpty));
  const tied=[record(1,1,[88],0),record(1,1,[89],1)];
  const concat=edits=>edits.flatMap(e=>e.ins);
  detect('ties resolved by container order',JSON.stringify(concat(tied))!==JSON.stringify(concat([...tied].reverse())));
  // Existing representative left-affinity rule; deleted interiors detach.
  const map=(p,e)=>p<=e.lo?p:p>=e.hi?p+e.ins.length-(e.hi-e.lo):-1;
  const merged={lo:0,hi:2,ins:[88,89]};
  const separatePoint=map(map(1,canonical[1]),canonical[0]),mergedPoint=map(1,merged);
  assert.deepEqual(emitByOrigin(source,canonical),emitByOrigin(source,[merged]));
  detect('adjacent edits silently coalesced',separatePoint===1&&mergedPoint===-1);
  const docB=canonical.map(e=>({...e,context:'B@0'}));
  const a=normalizeBySort(source,'A@0',reversed,3),b=normalizeByRank(source,'B@0',[...docB].reverse(),3);
  assert(sameOrder(a,canonical)&&sameOrder(b,docB),'per-document coordinate separation');
  return {status:'finite-batch-normalization-only',sources,canonicalBatches,permutationCases,sizes,
    domainControls,mutationControls,crossDocumentCases:1,
    coalescingCounterexample:{sameBytes:true,separatePoint,mergedPoint},
    scope:'Same-author finite reference constructors with shared admission/byte oracles; not a production canonicalizer, public schema, default mapping, complete budgets/authority or multi-document publication proof'};
}

if(process.argv[1]?.endsWith('/batch-normalization.mjs'))console.log(JSON.stringify(checkBatchNormalization(),null,2));
