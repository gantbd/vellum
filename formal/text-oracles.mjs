// Design conformance oracles, NOT production editor code or independently
// certified expectations. No imports from the text models or candidate engine.
const encoder=new TextEncoder();
// Canonical text already has character identity: U+FEFF is not a file signature.
const decoder=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true});
const bytes=text=>Array.from(encoder.encode(text));
const equal=(a,b)=>{
  if(a.length!==b.length)return false;
  for(let i=0;i<a.length;i++)
    if(!Object.hasOwn(a,i)||!Object.hasOwn(b,i)||a[i]!==b[i])return false;
  return true;
};
const assert=(ok,message)=>{if(!ok)throw Error(message);};
const validBytes=data=>{
  if(!Array.isArray(data))return false;
  for(let i=0;i<data.length;i++)
    if(!Object.hasOwn(data,i)||!Number.isInteger(data[i])||data[i]<0||data[i]>255)return false;
  try {decoder.decode(new Uint8Array(data));return true;}catch{return false;}
};
const boundary=(data,i)=>Number.isSafeInteger(i)&&i>=0&&i<=data.length&&
  (i===data.length||(data[i]&0xc0)!==0x80);

export function validateCanonicalBatch(source,edits) {
  assert(validBytes(source),'invalid source bytes');
  assert(Array.isArray(edits),'invalid edit list');
  let prior;
  for(const e of edits) {
    assert(e&&boundary(source,e.lo)&&boundary(source,e.hi)&&e.lo<=e.hi,'invalid source boundary');
    assert(validBytes(e.ins),'invalid inserted bytes');
    if(prior) {
      assert(prior.hi<=e.lo,'unsorted or overlapping ranges');
      assert(prior.hi!==e.lo||(prior.lo<prior.hi&&e.lo<e.hi),'unresolved insertion tie');
    }
    prior=e;
  }
}

// Original-position emitter: no splices, cumulative deltas or inverse logic.
export function emitByOrigin(source,edits) {
  const result=[];
  let pos=0,index=0;
  while(pos<=source.length) {
    const edit=edits[index];
    if(edit&&edit.lo===pos) {
      result.push(...edit.ins);index++;
      if(edit.hi>pos){pos=edit.hi;continue;}
    }
    if(pos===source.length)break;
    result.push(source[pos++]);
  }
  return result;
}

// Destructive edits on a PRIVATE flat copy, from highest original offset down.
export function applyRightToLeft(source,edits) {
  const result=source.slice();
  for(const e of [...edits].reverse())result.splice(e.lo,e.hi-e.lo,...e.ins);
  return result;
}

// Left-to-right sequential coordinates; inverses capture actual replaced bytes.
export function applySequentialWithInverses(source,edits) {
  const result=source.slice(),inverses=[];
  let delta=0;
  for(const e of edits) {
    const lo=e.lo+delta,hi=e.hi+delta;
    const removed=result.slice(lo,hi);
    result.splice(lo,hi-lo,...e.ins);
    inverses.push({lo,hi:lo+e.ins.length,ins:removed});
    delta+=e.ins.length-(e.hi-e.lo);
  }
  return {result,inverses};
}
export function restoreInReverse(result,inverses) {
  const restored=result.slice();
  for(const e of [...inverses].reverse())restored.splice(e.lo,e.hi-e.lo,...e.ins);
  return restored;
}

const alphabet=['a','é','🙂','\r','\n'];
const insertions=['','X','é','🙂','\r\n','\uFEFF'].map(bytes);
const maxSourceScalars=3;
function* words(max,prefix='') {
  yield prefix;
  if(max)for(const symbol of alphabet)yield* words(max-1,prefix+symbol);
}
function scalarBoundaries(text) {
  const positions=[0];
  for(const scalar of text)positions.push(positions.at(-1)+bytes(scalar).length);
  return positions;
}
function* rangeShapes(positions,count,start=0) {
  if(!count){yield [];return;}
  for(let lo=start;lo<positions.length-1;lo++)
    for(let hi=lo+1;hi<positions.length;hi++)
      for(const tail of rangeShapes(positions,count-1,hi))
        yield [{lo:positions[lo],hi:positions[hi]},...tail];
}
function* insertVariants(shape,index=0) {
  if(index===shape.length){yield [];return;}
  for(const ins of insertions)
    for(const tail of insertVariants(shape,index+1))yield [{...shape[index],ins},...tail];
}

const golden=[
  {name:'adjacent length change',source:[97,98,99,100],edits:[
    {lo:0,hi:1,ins:[88,88]},{lo:1,hi:2,ins:[89]}],expected:[88,88,89,99,100]},
  {name:'multibyte byte range',source:[65,240,159,153,130,66],
    edits:[{lo:1,hi:5,ins:[195,169]}],expected:[65,195,169,66]},
  {name:'literal FEFF and exact line endings',source:[239,187,191,97,13,10,98,9],
    edits:[{lo:3,hi:4,ins:[120]}],expected:[239,187,191,120,13,10,98,9]},
  {name:'scalar edit inside grapheme',source:[101,204,129,33],
    edits:[{lo:0,hi:1,ins:[97]}],expected:[97,204,129,33]},
  {name:'insert into empty text',source:[],edits:[{lo:0,hi:0,ins:[240,159,153,130]}],
    expected:[240,159,153,130]},
  {name:'delete through final byte',source:[97,195,169],edits:[{lo:1,hi:3,ins:[]}],expected:[97]},
];

export function checkTextOracles() {
  let cases=0,generatedCases=0,coalescingCases=0,sources=0;
  const batchSizes={0:0,1:0,2:0,3:0};
  const verify=(source,edits,expected=null)=>{
    validateCanonicalBatch(source,edits);
    const original=source.slice();
    const origin=emitByOrigin(source,edits),rtl=applyRightToLeft(source,edits);
    const sequential=applySequentialWithInverses(source,edits);
    assert(equal(origin,rtl)&&equal(origin,sequential.result),'forward oracle disagreement');
    if(expected)assert(equal(origin,expected),'literal golden mismatch');
    assert(origin.length===source.length+edits.reduce((n,e)=>n+e.ins.length-(e.hi-e.lo),0),'length law');
    assert(equal(restoreInReverse(sequential.result,sequential.inverses),source),'inverse recovery');
    assert(equal(source,original),'source snapshot changed');
    assert(validBytes(origin),'result is not valid UTF-8');
    for(let i=1;i<edits.length;i++) {
      const a=edits[i-1],b=edits[i];
      if(a.hi!==b.lo)continue;
      const combined=[...edits.slice(0,i-1),{lo:a.lo,hi:b.hi,ins:[...a.ins,...b.ins]},...edits.slice(i+1)];
      validateCanonicalBatch(source,combined);
      assert(equal(emitByOrigin(source,combined),origin),'adjacent coalescing law');
      coalescingCases++;
    }
    cases++;
  };
  for(const g of golden)verify(g.source,g.edits,g.expected);
  for(const text of words(maxSourceScalars)) {
    sources++;
    const source=bytes(text),positions=scalarBoundaries(text);
    const generated=edits=>{verify(source,edits);generatedCases++;batchSizes[edits.length]++;};
    generated([]);
    for(const pos of positions)for(const ins of insertions)generated([{lo:pos,hi:pos,ins}]);
    for(let count=1;count<=3;count++)
      for(const shape of rangeShapes(positions,count))for(const edits of insertVariants(shape))generated(edits);
  }
  const domainControls=[];
  const reject=(name,source,edits,reason)=>{
    let rejected=false;
    try{validateCanonicalBatch(source,edits);}catch(error){
      assert(error instanceof Error&&error.message===reason,`wrong rejection: ${name}`);rejected=true;
    }
    assert(rejected,`invalid domain accepted: ${name}`);domainControls.push(name);
  };
  reject('continuation-byte start',bytes('🙂'),[{lo:1,hi:4,ins:[]}],'invalid source boundary');
  reject('continuation-byte end',bytes('🙂'),[{lo:0,hi:2,ins:[]}],'invalid source boundary');
  reject('out-of-range end',bytes('a'),[{lo:0,hi:2,ins:[]}],'invalid source boundary');
  reject('negative boundary',bytes('a'),[{lo:-1,hi:0,ins:[]}],'invalid source boundary');
  reject('nonintegral boundary',bytes('a'),[{lo:0.5,hi:1,ins:[]}],'invalid source boundary');
  reject('invalid source UTF-8',[0xc0,0xaf],[],'invalid source bytes');
  reject('surrogate source encoding',[0xed,0xa0,0x80],[],'invalid source bytes');
  reject('out-of-range scalar source',[0xf4,0x90,0x80,0x80],[],'invalid source bytes');
  reject('sparse source bytes',Array(1),[],'invalid source bytes');
  reject('invalid inserted UTF-8',bytes('a'),[{lo:0,hi:1,ins:[0xff]}],'invalid inserted bytes');
  reject('non-byte inserted value',bytes('a'),[{lo:0,hi:1,ins:[256]}],'invalid inserted bytes');
  reject('sparse inserted bytes',bytes('a'),[{lo:0,hi:1,ins:Array(1)}],'invalid inserted bytes');
  reject('overlapping replacements',bytes('abc'),[{lo:0,hi:2,ins:[]},{lo:1,hi:3,ins:[]}],'unsorted or overlapping ranges');
  reject('unsorted oracle input',bytes('abc'),[{lo:2,hi:3,ins:[]},{lo:0,hi:1,ins:[]}],'unsorted or overlapping ranges');
  reject('coincident insertion tie',bytes('a'),[{lo:0,hi:0,ins:[88]},{lo:0,hi:0,ins:[89]}],'unresolved insertion tie');
  reject('touching insertion tie',bytes('ab'),[{lo:0,hi:1,ins:[88]},{lo:1,hi:1,ins:[89]}],'unresolved insertion tie');
  const mutationControls=[];
  const detect=(name,failed)=>{assert(failed,`mutation escaped: ${name}`);mutationControls.push(name);};
  detect('sparse equality false positive',!equal(Array(1),[0])&&!equal([0],Array(1))&&!equal(Array(1),Array(1)));
  const adjacent=golden[0],multi=golden[1],fidelity=golden[2],combining=golden[3];
  const unshifted=adjacent.source.slice();
  for(const e of adjacent.edits)unshifted.splice(e.lo,e.hi-e.lo,...e.ins);
  detect('unshifted sequential coordinates',!equal(unshifted,adjacent.expected));
  detect('adjacent edit omitted',!equal(applyRightToLeft(adjacent.source,[adjacent.edits[0]]),adjacent.expected));
  const e=multi.edits[0],text=decoder.decode(new Uint8Array(multi.source));
  const wrongUnits=bytes(text.slice(0,e.lo)+decoder.decode(new Uint8Array(e.ins))+text.slice(e.hi));
  detect('UTF-16 used for byte coordinates',!equal(wrongUnits,multi.expected));
  const fidelityOut=applyRightToLeft(fidelity.source,fidelity.edits);
  const combiningOut=applySequentialWithInverses(combining.source,combining.edits).result;
  detect('implicit newline rewriting',!equal(bytes(decoder.decode(new Uint8Array(fidelityOut)).replace(/\r\n/g,'\n')),fidelity.expected));
  detect('implicit Unicode normalization',!equal(bytes(decoder.decode(new Uint8Array(combiningOut)).normalize('NFC')),combining.expected));
  detect('literal leading FEFF stripped',!equal(fidelityOut.slice(3),fidelity.expected));
  detect('suffix omitted',!equal(emitByOrigin(adjacent.source,adjacent.edits).slice(0,-1),adjacent.expected));
  const seq=applySequentialWithInverses(adjacent.source,adjacent.edits),wrongInverse=seq.result.slice();
  for(const inv of seq.inverses)wrongInverse.splice(inv.lo,inv.hi-inv.lo,...inv.ins);
  detect('inverse chain applied forwards',!equal(wrongInverse,adjacent.source));
  const aliased=adjacent.source.slice(),before=aliased.slice();
  for(const edit of [...adjacent.edits].reverse())aliased.splice(edit.lo,edit.hi-edit.lo,...edit.ins);
  detect('source snapshot mutated',!equal(aliased,before));
  return {status:'finite-canonical-byte-oracles-only',sources,generatedCases,goldenCases:golden.length,
    cases,coalescingCases,batchSizes,domainControls,mutationControls,
    domain:{alphabet,maxSourceScalars,insertionChoices:insertions.length,maxEdits:3,
      multiedit:'canonical nonempty source ranges; adjacent replacements included',
      insertion:'single insertion at every scalar boundary; unresolved ties excluded'},
    scope:'Three algorithmic constructions plus literal byte fixtures. Same author and trusted Node codec; not independent certification, arbitrary-length proof, production refinement or policy approval.'};
}

// Deliberately no project-file or production-algorithm dependencies.
if(process.argv[1]?.endsWith('/text-oracles.mjs'))console.log(JSON.stringify(checkTextOracles(),null,2));
