import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {runUnicodeReference,unicodeReferenceCatalog as catalog} from './unicode-boundaries.mjs';
export function unicodeCases(file){
  return fs.readFileSync(new URL(file,import.meta.url),'utf8').split(/\r?\n/).map(s=>s.split('#')[0].trim()).filter(Boolean).map(s=>{
    let text='',offset=0;const expected=[];
    for(const token of s.split(/\s+/)){
      if(token==='÷')expected.push(offset);
      else if(token!=='×'){assert(/^[0-9A-F]{4,6}$/.test(token));const c=String.fromCodePoint(parseInt(token,16));text+=c;offset+=Buffer.byteLength(c,'utf8');}
    }
    return {text,expected};
  });
}
export function checkUnicodeConformance(sourceFile){
  for(const [file,metadata] of Object.entries(catalog.artifacts)){
    assert.equal(crypto.createHash('sha256').update(fs.readFileSync(new URL(file,import.meta.url))).digest('hex'),metadata.sha256,file);
  }
  const grapheme=unicodeCases('unicode18-grapheme-test.txt'),word=unicodeCases('unicode18-word-test.txt');
  assert.equal(grapheme.length,853);assert.equal(word.length,1944);
  const additional=['','ภาษาไทย','日本語','\uFEFF','e\u0301','é','a🇦\u200d🇧🇨b'];
  const result=runUnicodeReference([...grapheme,...word].map(r=>r.text).concat(additional),sourceFile);
  for(let i=0;i<grapheme.length;i++)assert.deepEqual(result.rows[i][0],grapheme[i].expected,`grapheme ${i}`);
  for(let i=0;i<word.length;i++)assert.deepEqual(result.rows[grapheme.length+i][1],word[i].expected,`word ${i}`);
  const rows=result.rows.slice(grapheme.length+word.length),controls=[];
  const control=(name,ok)=>{assert(ok,name);controls.push(name);};
  control('empty-source-one-boundary',JSON.stringify(rows[0])==='[[0],[0],[],[0]]');
  control('thai-default-not-dictionary',JSON.stringify(rows[1][1])==='[0,3,6,9,12,15,18,21]'&&JSON.stringify(rows[1][3])==='[0,12,21]');
  control('japanese-default-not-dictionary',JSON.stringify(rows[2][1])==='[0,3,6,9]'&&JSON.stringify(rows[2][3])==='[0,9]');
  control('bom-is-source-content',JSON.stringify(rows[3][0])==='[0,3]');
  control('no-canonical-normalization',JSON.stringify(rows[4][0])==='[0,3]'&&JSON.stringify(rows[5][0])==='[0,2]');
  control('raw-word-not-necessarily-grapheme-grid',rows[6][1].some(p=>!rows[6][0].includes(p))&&rows[6][2].flat().every(p=>rows[6][0].includes(p)));
  control('isolated-surrogate-refused',(()=>{try{runUnicodeReference(['\ud800']);return false;}catch{return true;}})());
  control('oversized-tool-batch-refused',(()=>{try{runUnicodeReference(Array(4097).fill(''));return false;}catch{return true;}})());
  return {status:'pinned-unicode-reference-conformance-only',graphemeCases:grapheme.length,wordCases:word.length,controls,
    ...result.version,jarSha256:result.jarSha256,...catalog.scope};
}
export function checkUnicodeMutations(){
  const source=fs.readFileSync(new URL('./UnicodeBoundary.java',import.meta.url),'utf8'),directory=fs.mkdtempSync(path.join(os.tmpdir(),'vellum-unicode-mutants-')),killed=[];
  const mutations=[
    ['ignore-format-rule','if(ignored(b))return false;','if(false)return false;'],
    ['regional-indicator-parity','if(count%2==1)return false;','if(count%2==0)return false;'],
    ['all-word-boundaries-joined','return true;                                          // WB999','return false;                                         // WB999'],
    ['utf16-used-as-utf8',"return s.substring(0,utf16).getBytes(StandardCharsets.UTF_8).length;",'return utf16;'],
    ['dictionary-is-default','List<Integer> w=words(s);','List<Integer> w=iterated(s,true);']
  ];
  for(const [name,from,to] of mutations){
    assert(source.includes(from),name);const file=path.join(directory,name+'.java');fs.writeFileSync(file,source.replace(from,to));
    let failed=false;try{checkUnicodeConformance(pathToFileURL(file));}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);failed=true;}
    assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkUnicodeConformance(),mutations:checkUnicodeMutations()},null,2));
