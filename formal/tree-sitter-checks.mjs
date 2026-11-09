import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {publishAnnotationPlan} from './annotation-compounds.mjs';
import {committedSourceTransforms,prepareParserSource} from './parser-source.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./tree-sitter-reference.json',import.meta.url),'utf8'));
const copy=structuredClone,bytes=s=>Array.from(Buffer.from(s,'utf8'));
const document={authority:'a',id:'A',incarnation:1};
const host={authorized:true,bounded:true,notStopped:true,interpretationCurrent:true,companionsReady:true,versionsAvailable:true,historyReady:true,completeIdentity:true,maxParticipants:4,
  parserSourceReserved:true,parserSourceLimits:{maxDocuments:4,maxRootBytes:32768,maxBatches:32,maxEdits:64,maxJournalBytes:262144}};
const limits={maxRootBytes:32768,maxReceipts:16,maxBatches:32,maxEdits:64,maxCoordinate:4294967295,maxPlanBytes:262144};
const capture={authorized:true,sourceReadable:true,continuous:true,live:true,notStopped:true,captureReserved:true};
function nativeCase(language,text,batches,fresh=false){
  const root={authority:'a',publication:0,documents:[{document,revision:1,source:bytes(text),annotationSet:1,records:[]}],groups:[],frontiers:{}};
  let current=root;const refs=[];
  for(let i=0;i<batches.length;i++){
    const d=current.documents[0],edits=batches[i].map(e=>({...e,expected:d.source.slice(e.lo,e.hi)}));
    const result=publishAnnotationPlan(current,{contract:'owned-annotation-plan/1',invocation:`native-${i}`,descriptor:`native-${i}`,
      sources:[{document,expectedSource:d.revision,expectedAnnotationSet:d.annotationSet,edits}],annotations:[]},host);
    assert(['applied','unchanged'].includes(result.status));if(result.status==='applied')refs.push(...committedSourceTransforms(result.state));current=result.state;
  }
  const snap=s=>({document,revision:s.documents[0].revision,source:s.documents[0].source});
  const p=prepareParserSource(fresh?null:snap(root),snap(current),refs,capture,limits);assert(p);
  return {language,before:root.documents[0].source,source:p.source,edits:p.edits,mode:p.mode};
}
export function treeSitterCases(){
  const corpora={
    markdown:['# Titel é🙂\n\nProse **bold** and *emphasis*.\n','a\r\nb\rc\n','```js\nconst value = 1;\n```\n\n```mermaid\ngraph LR\n```\n','- item\n  prose\n'],
    javascript:["const café = '🙂';\nfoo(café);\n",'function f(x) {\r\n return x + 1;\r\n}\n','// prose é\nlet value = `🙂`;\n','function (']};
  const cases=[];
  for(const [language,texts] of Object.entries(corpora))for(const text of texts){
    let offset=0;const ps=[0,...[...text].map(c=>(offset+=Buffer.byteLength(c,'utf8')))];
    const chosen=[...new Set([0,ps[Math.floor(ps.length/3)],ps[Math.floor(ps.length*2/3)],ps.at(-1)])];
    for(const lo of chosen)for(const insert of ['X','🙂','\n','\r\n','const z = 2;\n','```']){
      const hi=ps[ps.indexOf(lo)+1]??lo;cases.push(nativeCase(language,text,[[{lo,hi,ins:bytes(insert)}]]));
    }
  }
  const coalesced=nativeCase('javascript','const foo = 1;\n',[[{lo:6,hi:9,ins:bytes('Foo')}],[{lo:0,hi:0,ins:bytes('// é🙂\n')}]]);cases.push(coalesced);
  cases.push(nativeCase('javascript','let abc = 1;\n',[[{lo:4,hi:7,ins:bytes('ABC')}]]));
  cases.push(nativeCase('markdown','a🙂\r\nZ\n',[[{lo:0,hi:1,ins:bytes('# title\n')},{lo:7,hi:8,ins:bytes('**bold**')}]]));
  cases.push(nativeCase('javascript','const x=1;',[[{lo:6,hi:7,ins:bytes('name')}]],true));
  const mixed=[
    '# Prose\n\nA **bold** word.\n\n```js\nconst café = "🙂";\n```\n\n```js\nlet second = 2;\n```\n\n```mermaid\ngraph LR\n```\n',
    '```js\nfunction f() {\n```\n\n```js\n}\n```\n',
    '> ```js\n> const x = 1;\n> // one\n> x();\n> ```\n',
    '  ```javascript\r\n  const x = 1;\r\n  ```\r\n',
    '```JS\nconst x = 1;\n```\n\n```../evil\nx();\n```\n',
    '```js\nconst x = 1;\n',
    '```js\nconst x = 1;\n```\n',
    '```js\n\n```\n\n```unknown\ndata\n```\n',
    '# New\n\n```js\nconst newName = 1;\n```\n',
    '# New\n\n```mermaid\nconst newName = 1;\n```\n'
  ].map((text,i)=>({source:bytes(text),available:i===6?[]:['javascript']}));
  mixed.push({source:bytes(Array.from({length:12},(_,i)=>`\`\`\`${i%2?'javascript':'js'}\nconst x${i} = ${i};\n\`\`\`\n`).join('\n')),available:['javascript'],childBudget:2});
  mixed.push({source:bytes('```unknown\ndata\n```\n\n```js\nconst x = 1;\n```\n'),available:['javascript']});
  mixed.push({source:bytes('```js\nconsole.log("x");\n```\n'),available:['javascript']});
  mixed.push({source:bytes('```js\n```\n'),available:['javascript']});
  return {operation:'run',cases,mixed};
}
export function runTreeSitterReference(request,sourceFile=new URL('./tree-sitter-reference.py',import.meta.url)){
  const python=process.env.VELLUM_TREE_SITTER_PYTHON;assert(python,'VELLUM_TREE_SITTER_PYTHON is required');
  const input=JSON.stringify(request);assert(Buffer.byteLength(input,'utf8')<=8*1024*1024);
  const p=spawnSync(python,[fileURLToPath(sourceFile)],{input,encoding:'utf8',timeout:15000,maxBuffer:8*1024*1024});
  if(p.error||p.signal||p.status!==0){
    if(!p.error&&!p.signal&&p.status===2&&p.stderr.startsWith('REFERENCE_ASSERTION:'))assert.fail(`native semantic check failed: ${p.stderr.slice(0,1000)}`);
    throw Error(`native runner failure (${p.status}, ${p.signal}): ${(p.stderr||String(p.error)).slice(0,1000)}`);
  }
  return JSON.parse(p.stdout);
}
function verify(request,result){
  assert.deepEqual(result.metadata,catalog.environment);
  assert.equal(result.rows.length,request.cases.length);assert.equal(result.mixed.length,request.mixed.length);
  for(const row of result.rows)assert(row.equivalent&&row.publishedTreePreserved&&row.capturesConsistent&&row.nodes>0);
  const controls=[],control=(name,ok)=>{assert(ok,name);controls.push(name);};
  control('native-errors-remain-editable-data',result.rows.some(r=>r.hasError));
  control('actual-coalesced-commits-reach-native-parser',request.cases.at(-4).edits.length===2&&result.rows.at(-4).steps===2);
  control('changed-ranges-are-not-highlight-invalidation-oracle',result.rows.at(-3).changedRanges===0&&result.rows.at(-3).capturesChanged);
  control('actual-paired-source-edits-reach-native-parser',result.rows.at(-2).steps===2);
  control('missing-journal-explicit-fresh-native-parse',request.cases.at(-1).mode==='fresh'&&result.rows.at(-1).steps===0);
  const [normal,separate,quote,indented,aliases,incomplete,missing,empty,oldLabel,newLabel]=result.mixed;
  control('both-independent-same-label-fences-parse',normal.regions.length===3&&normal.regions.slice(0,2).every(r=>r.kind==='parsed'&&!r.hasError&&r.captures.length>0));
  control('mermaid-truthful-fallback-no-diagram-execution',normal.regions[2].label==='mermaid'&&normal.regions[2].kind==='unsupported-language');
  control('incomplete-programs-not-concatenated-across-fences',separate.regions.length===2&&separate.regions.every(r=>r.kind==='parsed'&&r.hasError));
  control('host-inline-grammar-really-highlights-prose',normal.inlineCaptures.some(([name])=>name==='text.strong'));
  for(const r of normal.regions.filter(r=>r.kind==='parsed'))control(`native-child-captures-inside-own-fence-${controls.length}`,r.captures.every(([,lo,hi])=>r.body[0]<=lo&&hi<=r.body[1]));
  control('quoted-fence-excludes-host-continuation-prefixes',quote.regions[0].kind==='parsed'&&!quote.regions[0].hasError&&quote.regions[0].ranges.length>=2);
  const quoteText=Buffer.from(request.mixed[2].source).toString('utf8'),quotePrefix=Buffer.byteLength(quoteText.slice(0,quoteText.indexOf('> //')),'utf8');
  control('host-prefix-styling-is-not-erased-by-child',quote.display.some(([layer,,lo,hi])=>layer==='host-block'&&lo<=quotePrefix&&quotePrefix<hi)&&!quote.regions[0].captures.some(([,lo,hi])=>lo<=quotePrefix&&quotePrefix<hi));
  control('indented-crlf-fence-native-points-and-map',indented.regions[0].kind==='parsed'&&!indented.regions[0].hasError);
  control('only-registered-case-normalized-alias-loads',aliases.regions[0].kind==='parsed'&&aliases.regions[1].kind==='unsupported-language');
  control('incomplete-fence-fallback-does-not-claim-child-parse',incomplete.regions[0].kind==='incomplete-fence'&&incomplete.regions[0].captures.length===0);
  control('missing-grammar-fallback-is-distinct',missing.regions[0].kind==='grammar-unavailable');
  control('empty-fence-does-not-disable-unrelated-fallback',empty.regions.length===2&&empty.regions[1].kind==='unsupported-language');
  control('label-change-rebuilds-membership-not-source-ownership',oldLabel.regions[0].kind==='parsed'&&newLabel.regions[0].kind==='unsupported-language');
  for(const [caseIndex,row] of result.mixed.entries())for(const [,,lo,hi] of row.display)assert(lo>=0&&lo<hi&&hi<=request.mixed[caseIndex].source.length);
  control('host-literal-capture-does-not-mask-child-body',normal.regions.filter(r=>r.kind==='parsed').every(r=>!normal.display.some(([layer,,lo,hi])=>layer==='host-block'&&r.ranges.some(([a,b])=>lo<b&&a<hi))));
  const budget=result.mixed[10];
  control('tiny-fences-and-aliases-share-document-budget',budget.partial&&budget.childAnalyses===2&&budget.regions.filter(r=>r.kind==='parsed').length===2&&budget.regions.filter(r=>r.kind==='region-budget').length===10);
  control('budget-fallback-preserves-host-styling',budget.display.some(([layer])=>layer==='host-block'));
  control('unknown-region-does-not-disable-supported-neighbor',result.mixed[11].regions[0].kind==='unsupported-language'&&result.mixed[11].regions[1].kind==='parsed');
  control('locals-dependent-patterns-explicitly-excluded',result.omittedPropertyPatterns===2&&!result.mixed[12].regions[0].captures.some(([name])=>name==='variable.builtin'));
  control('empty-supported-fence-not-mislabeled-unsupported',result.mixed[13].regions[0].kind==='empty-body');
  control('native-invalid-query-preserves-prior-query-and-tree',result.nativeControls.invalidQueryPreservesPrior);
  control('native-query-only-update-does-not-reparse-tree',result.nativeControls.queryOnlyPreservesTree);
  const firstKeyword=normal.regions[0].captures.find(([name])=>name==='keyword')[1];
  control('resolved-child-keyword-wins-in-body',normal.resolved.some(([layer,name,lo,hi])=>layer==='child'&&name==='keyword'&&lo<=firstKeyword&&firstKeyword<hi));
  const bold=Buffer.byteLength(Buffer.from(request.mixed[0].source).toString('utf8').split('bold')[0],'utf8');
  control('resolved-inline-prose-overrides-block-literal',normal.resolved.some(([layer,name,lo,hi])=>layer==='host-inline'&&name==='text.strong'&&lo<=bold&&bold<hi));
  control('resolved-prefix-stays-host-owned',quote.resolved.some(([layer,,lo,hi])=>layer==='host-block'&&lo<=quotePrefix&&quotePrefix<hi));
  control('resolved-unknown-body-remains-literal',normal.resolved.some(([layer,name,lo,hi])=>layer==='host-block'&&name==='text.literal'&&lo<=normal.regions[2].body[0]&&normal.regions[2].body[0]<hi));
  control('deterministic-layer-span-name-ties-checked',result.nativeControls.displayTiesChecked);
  return controls;
}
export function checkTreeSitterReference(sourceFile){
  const request=treeSitterCases(),result=runTreeSitterReference(request,sourceFile),controls=verify(request,result);
  return {status:'pinned-native-tree-sitter-reference-only',incrementalCases:request.cases.filter(c=>c.mode!=='fresh').length,freshCases:request.cases.filter(c=>c.mode==='fresh').length,
    mixedCases:request.mixed.length,controls,metadata:result.metadata,...catalog.scope};
}
export function checkTreeSitterFingerprintControl(){
  const directory=fs.mkdtempSync(path.join(os.tmpdir(),'vellum-parser-fingerprint-'));
  fs.copyFileSync(new URL('./tree-sitter-reference.py',import.meta.url),path.join(directory,'tree-sitter-reference.py'));
  const invalid=copy(catalog);invalid.environment.distributions['tree-sitter'].files['tree_sitter/__init__.py']='0'.repeat(64);
  fs.writeFileSync(path.join(directory,'tree-sitter-reference.json'),JSON.stringify(invalid));
  let failed=false;try{runTreeSitterReference({operation:'run',cases:[],mixed:[]},pathToFileURL(path.join(directory,'tree-sitter-reference.py')));}catch(e){assert.equal(e.code,'ERR_ASSERTION');assert(e.message.includes('fingerprint mismatch'));failed=true;}
  assert(failed);return true;
}
export function checkTreeSitterMutations(){
  const source=fs.readFileSync(new URL('./tree-sitter-reference.py',import.meta.url),'utf8'),directory=fs.mkdtempSync(path.join(os.tmpdir(),'vellum-parser-mutants-')),killed=[];
  fs.copyFileSync(new URL('./tree-sitter-reference.json',import.meta.url),path.join(directory,'tree-sitter-reference.json'));
  const mutations=[
    ['omit-actual-native-tree-edit','edited.edit(**edit)','pass # edit omitted'],
    ['mutate-published-native-tree','edited = old.copy()','edited = old'],
    ['combine-independent-fence-programs','child_ranges = ranges','child_ranges = all_body_ranges'],
    ['unregistered-label-loads-javascript','ALIASES.get(label.lower() if label.isascii() else label)','ALIASES.get(label.lower() if label.isascii() else label, "javascript")'],
    ['host-prefix-map-holes-ignored','ranges = subtract((body.start_byte, body.end_byte), holes)','ranges = [(body.start_byte, body.end_byte)]'],
    ['crossing-edit-target-clipped','return (lo, hi) if any(a <= lo <= hi <= b for a, b in ranges) else None','return intersections(lo, hi, ranges)[0] if intersections(lo, hi, ranges) else None'],
    ['host-layer-retains-child-body-mask','subtract((lo, hi), supported_bodies)','[(lo, hi)]'],
    ['source-position-shifted-to-child-local','projected = [(name, a, b)','projected = [(name, a-body.start_byte, b-body.start_byte)'],
    ['inline-grammar-loses-meaningful-delimiters','for n in inline.named_children','for n in inline.children'],
    ['tiny-fence-resets-budget','child_count += 1\n                tree = child_tree','child_count = 0\n                tree = child_tree'],
    ['locals-predicate-assumed-true','if compiled.pattern_assertions(pattern):','if False:'],
    ['host-layer-wins-over-child','-priorities[row[0]]','priorities[row[0]]'],
    ['wider-capture-wins-tie','row[3] - row[2]','row[2] - row[3]']
  ];
  for(const [name,from,to] of mutations){
    assert(source.includes(from),name);const file=path.join(directory,name+'.py');fs.writeFileSync(file,source.replace(from,to));
    let failed=false;try{checkTreeSitterReference(pathToFileURL(file));}catch(e){assert.equal(e.code,'ERR_ASSERTION',name);
      // Parse/import/C-API failures are not counted as semantic mutation kills.
      assert(!e.message.includes('SyntaxError')&&!e.message.includes('ImportError')&&!e.message.includes('ValueError')&&!e.message.includes('signal'),name);failed=true;}
    assert(failed,name);killed.push(name);
  }
  return killed;
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify({...checkTreeSitterReference(),fingerprintControl:checkTreeSitterFingerprintControl(),mutations:checkTreeSitterMutations()},null,2));
