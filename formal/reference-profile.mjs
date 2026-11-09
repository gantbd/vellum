// Private design reference. No production line index, JSON decoder or Unicode engine.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
const profile=JSON.parse(fs.readFileSync(new URL('./reference-profile.json',import.meta.url),'utf8'));
const encode=s=>Array.from(new TextEncoder().encode(s));
const clone=structuredClone;
function validBytes(bytes) {
  try {new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(Uint8Array.from(bytes));return true;}
  catch{return false;}
}
const boundary=(bytes,i)=>i===bytes.length||(bytes[i]&0xc0)!==0x80;

export function logicalLines(bytes) {
  if(!validBytes(bytes))return null;
  const out=[];let start=0;
  for(let i=0;i<bytes.length;i++) {
    if(bytes[i]!==10&&bytes[i]!==13)continue;
    const bodyEnd=i;
    if(bytes[i]===13&&bytes[i+1]===10)i++;
    out.push({start,bodyEnd,end:i+1});start=i+1;
  }
  out.push({start,bodyEnd:bytes.length,end:bytes.length});return out;
}
export function lineToByte(bytes,line,column) {
  const lines=logicalLines(bytes);
  if(!lines||!Number.isSafeInteger(line)||!Number.isSafeInteger(column)||line<0||column<0||line>=lines.length)return null;
  const row=lines[line],at=row.start+column;
  return at<=row.bodyEnd&&boundary(bytes,at)?at:null;
}
export function byteToLine(bytes,at) {
  const lines=logicalLines(bytes);
  if(!lines||!Number.isSafeInteger(at)||at<0||at>bytes.length||!boundary(bytes,at))return null;
  const line=lines.findIndex(row=>row.start<=at&&at<=row.bodyEnd);
  return line<0?null:{line,column:at-lines[line].start};
}

// Payload shape, exact scalar/number decoding, bounds and host evidence are supplied.
// The fixture checks the stage gate and ownership, not a hostile-input JSON codec.
export function stageAnnotation(decoded,current,gates) {
  if(decoded.version!==1||!['shape','authority','resources','sourceCurrent','associationCurrent','notStopped'].every(k=>gates[k]))
    return {status:'refused',current};
  if(!equal(decoded.document,current.document)||!equal(decoded.source,current.source))return {status:'refused',current};
  return {status:'staged',candidate:clone(decoded),current};
}
const equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b); // Fixed normalized fixture records only.

export function checkReferenceProfile() {
  assert.deepEqual(Object.keys(profile.selections).sort(),
    ['execution','text','history','annotations','assets','configuration','observation','packaging'].sort());
  assert.deepEqual(profile.selections,{
    execution:'closed-versioned-typed-products-with-authorized-metadata',
    text:'lossless-logical-lines-and-default-unicode-boundaries',
    history:'invocation-groups-with-explicit-bounded-typing-opt-in',
    annotations:'versioned-utf8-json-reference-host-core-storage-neutral',
    assets:'bundled-immutable-pins-and-validated-host-alternatives',
    configuration:'typed-deterministic-composition-and-adapter-bindings',
    observation:'content-free-bounded-nonblocking-capture-optional-drain',
    packaging:'minimal-core-separate-providers-adapters-and-both-apps'});
  assert.equal(profile.id,'v1-reference');assert.equal(profile.revision,1);
  assert.deepEqual(profile.lines,{delimiters:['CRLF','LF','CR'],origin:0,columnUnit:'utf8-byte',
    bodyEndIncludedAsCaret:true,emptySourceLines:1,trailingDelimiterAddsEmptyLine:true,
    crlfInterior:'unavailable',normalization:'none'});
  assert.deepEqual(profile.segmentation,{unicode:'18.0.0',uax29Revision:49,graphemes:'default-extended',
    words:'default',implicitTailoring:false,identifierCommands:'separately-named-supported-profile'});
  assert.deepEqual(profile.annotationStorage,{encoding:'UTF-8',syntax:'RFC8259-JSON',versionedSchema:true,
    defaultMetadata:'empty',duplicateNames:'reject',location:'host-supplied-binding',load:'owned-stage-validate-then-atomic-publication'});
  assert.deepEqual(profile.assurance,{numericLimitsMeasured:false,referenceUnicodeConformanceSelected:true,productionUnicodeConformanceExecuted:false,
    hostSupportVerified:false,concretePublicSchemaClosed:false});
  let lineCases=0,coordinateCases=0,stageCases=0;const controls=[];
  const check=(name,ok)=>{assert(ok,name);controls.push(name);};
  const alphabet=['a','é','\r','\n','\u2028'];
  let texts=[''],frontier=[''];
  for(let length=1;length<=5;length++){frontier=frontier.flatMap(s=>alphabet.map(c=>s+c));texts.push(...frontier);}
  for(const text of texts) {
    const bytes=encode(text),before=clone(bytes),actual=logicalLines(bytes);
    // Independent string-splitting oracle; implementation scans UTF-8 bytes.
    const parts=text.split(/(\r\n|\r|\n)/),expected=[];let start=0;
    for(let i=0;i<parts.length;i+=2){const bodyEnd=start+encode(parts[i]).length,end=bodyEnd+encode(parts[i+1]??'').length;
      expected.push({start,bodyEnd,end});start=end;}
    assert.deepEqual(actual,expected);assert.deepEqual(bytes,before);
    assert.deepEqual(actual.flatMap(row=>bytes.slice(row.start,row.end)),bytes);
    for(let at=0;at<=bytes.length;at++) {
      const row=expected.findIndex(r=>r.start<=at&&at<=r.bodyEnd);
      // Decode prefix separately rather than share the implementation's boundary bit test.
      const want=row>=0&&validBytes(bytes.slice(0,at))?{line:row,column:at-expected[row].start}:null;
      const got=byteToLine(bytes,at);assert.deepEqual(got,want);
      if(got)assert.equal(lineToByte(bytes,got.line,got.column),at);
      coordinateCases++;
    }
    for(let line=0;line<actual.length;line++) {
      const width=actual[line].bodyEnd-actual[line].start;
      for(let col=0;col<=width;col++) {
        const at=actual[line].start+col,want=validBytes(bytes.slice(0,at))?at:null;
        assert.equal(lineToByte(bytes,line,col),want);
        if(want!==null)assert.deepEqual(byteToLine(bytes,want),{line,column:col});
      }
      assert.equal(lineToByte(bytes,line,width+1),null);
    }
    lineCases++;
  }
  check('empty-is-one-line',logicalLines([]).length===1);
  check('trailing-delimiter-empty-line',logicalLines(encode('a\r\n')).length===2);
  check('crlf-is-one-delimiter',logicalLines(encode('\r\n')).length===2);
  check('crlf-interior-not-rounded',byteToLine(encode('\r\n'),1)===null);
  check('unicode-separator-is-content',logicalLines(encode('a\u2028b')).length===1);
  check('bom-is-logical-content',logicalLines(encode('\ufeff')).at(-1).bodyEnd===3);
  check('invalid-utf8-refused',logicalLines([0xc3])===null);
  for(const value of [-1,0.5,NaN,Infinity,Number.MAX_SAFE_INTEGER])
    check(`invalid-line-or-column-${String(value)}`,lineToByte(encode('a'),value,0)===null&&lineToByte(encode('a'),0,value)===null);
  assert.throws(()=>assert.deepEqual(logicalLines(encode('\r\n')),[{start:0,bodyEnd:0,end:1},{start:1,bodyEnd:1,end:2},{start:2,bodyEnd:2,end:2}]));
  controls.push('split-crlf-mutant-detected');
  assert.throws(()=>assert.equal(byteToLine(encode('\r\n'),1)?.column,0));controls.push('clamp-mutant-detected');
  const current={document:{binding:'host-resource',incarnation:'1'},source:{evidence:'source-1'},records:[]};
  const payload={version:1,document:clone(current.document),source:clone(current.source),records:[
    {id:'note-1',incarnation:'1',contentVersion:'1',body:'',targetGeneration:'1',target:{kind:'document'}}]};
  const keys=['shape','authority','resources','sourceCurrent','associationCurrent','notStopped'];
  const gates=Object.fromEntries(keys.map(k=>[k,true]));
  for(let mask=0;mask<64;mask++)for(const binding of [true,false])for(const source of [true,false]) {
    const input=clone(payload);if(!binding)input.document.binding='different';if(!source)input.source.evidence='old';
    const gs=Object.fromEntries(keys.map((k,i)=>[k,!!(mask&(1<<i))]));
    const before=clone({input,current,gs}),result=stageAnnotation(input,current,gs);
    assert.equal(result.status,mask===63&&binding&&source?'staged':'refused');
    assert.deepEqual(result.current,current);assert.deepEqual({input,current,gs},before);stageCases++;
  }
  for(const body of ['', 'a\r\nb','é','\u0000','"\\','```mermaid\ngraph LR\n```']) {
    const p=clone(payload);p.records[0].body=body;
    assert.deepEqual(JSON.parse(JSON.stringify(p)),p);
  }
  check('unknown-storage-version-refused',stageAnnotation({...payload,version:2},current,gates).status==='refused');
  const staged=stageAnnotation(payload,current,gates);payload.records[0].body='changed caller';
  check('staged-payload-owned',staged.candidate.records[0].body==='');
  check('staging-is-not-publication',current.records.length===0&&staged.current.records.length===0);
  return {status:'reference-profile-boundary-only',selections:Object.keys(profile.selections).length,
    lineCases,coordinateCases,stageCases,controls,unicodeConformanceProved:false,
    hostileJsonDecoderProved:false,implementationRefinement:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkReferenceProfile(),null,2));
