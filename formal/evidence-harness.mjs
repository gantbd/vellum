// Verification infrastructure only. A frozen copy is not an execution sandbox,
// an atomic filesystem snapshot, or independent certification of the toolchain.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';

const extensions=new Set(['.md','.mjs','.json','.ttl','.tla','.cfg','.smt2','.txt','.java','.py']);
const manifestName='.vellum-input-snapshot.json';
const digest=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const transientCaptures=[];
const captureScopes=new WeakMap();
let captureCleanupRegistered=false;
const captureRecord=directory=>{const stat=fs.lstatSync(directory);return Object.freeze({directory,device:stat.dev,inode:stat.ino});};
function removeCapturedDirectory(record){let stat;try{stat=fs.lstatSync(record.directory);}catch(error){if(error.code==='ENOENT')return;throw error;}
 if(!stat.isDirectory()||stat.isSymbolicLink()||stat.dev!==record.device||stat.ino!==record.inode||fs.realpathSync(record.directory)!==record.directory)throw Error('CAPTURE_CLEANUP_FAILURE: captured directory identity changed');
 fs.rmSync(record.directory,{recursive:true,force:false});
}
export function createCaptureScope(parent=os.tmpdir()){
 const directory=fs.realpathSync(fs.mkdtempSync(path.join(parent,'vellum-check-captures-'))),scope=Object.freeze({directory});captureScopes.set(scope,captureRecord(directory));return scope;
}
export function releaseCaptureScope(scope,passed){const record=captureScopes.get(scope);if(!record||passed!==true)return false;removeCapturedDirectory(record);captureScopes.delete(scope);return true;}
function trackTransientCapture(directory){
 transientCaptures.push(captureRecord(directory));
 if(captureCleanupRegistered)return;captureCleanupRegistered=true;
 process.once('exit',code=>{
  if(code!==0)return;
  try{for(const record of transientCaptures)removeCapturedDirectory(record);}catch(error){console.error('CAPTURE_CLEANUP_FAILURE: '+error.message);process.exitCode=1;}
 });
}
export const fileHash=file=>digest(fs.readFileSync(file));
export function projectInputs(root) {
  const files=[];
  for(const entry of fs.readdirSync(root,{withFileTypes:true})) {
    if(!entry.name.endsWith('.md'))continue;
    if(!entry.isFile())throw Error(`Unsupported source entry ${entry.name}`);
    files.push(entry.name);
  }
  for(const entry of fs.readdirSync(path.join(root,'formal'),{withFileTypes:true})) {
    if(entry.name==='results.json'||entry.name.startsWith('.'))continue;
    if(!entry.isFile()||!extensions.has(path.extname(entry.name)))
      throw Error(`Unsupported formal input entry ${entry.name}`);
    files.push(`formal/${entry.name}`);
  }
  return files.sort();
}
export function captureInputs(root,{retain=false}={}) {
  const transient=process.env.VELLUM_TRANSIENT_PROOF_CAPTURES==='1'&&retain!==true,scope=transient?process.env.VELLUM_TRANSIENT_PROOF_CAPTURE_SCOPE:null;
  const directory=fs.realpathSync(fs.mkdtempSync(path.join(scope||os.tmpdir(),'vellum-proof-inputs-')));
  if(transient&&!scope)trackTransientCapture(directory);
  fs.mkdirSync(path.join(directory,'formal'));
  const hashes={};
  for(const relative of projectInputs(root)) {
    const bytes=fs.readFileSync(path.join(root,relative));
    hashes[relative]=digest(bytes);
    fs.writeFileSync(path.join(directory,relative),bytes);
  }
  fs.writeFileSync(path.join(directory,manifestName),JSON.stringify({version:1,hashes}));
  return {directory,hashes};
}
export function compareInputs(root,hashes) {
  let inventory;
  try {inventory=projectInputs(root);}catch{return {matches:false,changed:['input-inventory']};}
  const expected=Object.keys(hashes).sort();
  const changed=new Set([...inventory.filter(p=>!expected.includes(p)),...expected.filter(p=>!inventory.includes(p))]);
  for(const relative of inventory) {
    try {if(hashes[relative]!==fileHash(path.join(root,relative)))changed.add(relative);}
    catch {changed.add(relative);}
  }
  return {matches:changed.size===0,changed:[...changed].sort()};
}
export function validateFrozenInputs(root) {
  const manifest=JSON.parse(fs.readFileSync(path.join(root,manifestName),'utf8'));
  if(manifest.version!==1||!manifest.hashes||!compareInputs(root,manifest.hashes).matches)
    throw Error('Frozen input manifest mismatch');
  return manifest;
}
export function exactSmtOutput(output,cases) {
  const expected=cases.flatMap(c=>[c.id,...c.expected]);
  const actual=output.trim().split(/\r?\n/).map(s=>s.trim()).filter(Boolean);
  return JSON.stringify(expected)===JSON.stringify(actual);
}
export function evaluateToolResult(p,expected,phrase,validate=()=>true) {
  const output=(p.stdout||'')+(p.stderr||'');
  const normal=!p.error&&!p.signal&&Number.isInteger(p.status)&&p.status>=0;
  let validated=false;
  try {validated=validate(output)===true;}catch {validated=false;}
  const exitMatches=expected==='success'?p.status===0:
    ['counterexample','witness'].includes(expected)&&p.status>0;
  return {output,passed:Boolean(normal&&exitMatches&&output.includes(phrase)&&validated)};
}
export function runFrozenVerification({root,args,env=process.env,execute=spawnSync,log=console.log}) {
  const captured=captureInputs(root,{retain:true});
  const worker=execute(process.execPath,[path.join(captured.directory,'formal/check.mjs'),
    '--frozen-worker',...args],{env:{...env,VELLUM_FROZEN_ROOT:captured.directory},stdio:'inherit'});
  const normal=!worker.error&&!worker.signal&&worker.status===0;
  const live=compareInputs(root,captured.hashes);
  const graph='formal/architecture.ttl';
  const workerReport=path.join(captured.directory,'formal/results.json');
  const generationOnly=args.includes('--write')&&!env.VELLUM_SHACL&&!env.VELLUM_TLA_JAR&&!env.VELLUM_Z3&&!env.VELLUM_JSONSCHEMA_PYTHON&&!env.VELLUM_ICU4J_JAR&&!env.VELLUM_TREE_SITTER_PYTHON;
  if(normal&&live.matches&&args.includes('--write')&&fs.existsSync(path.join(captured.directory,graph)))
    fs.copyFileSync(path.join(captured.directory,graph),path.join(root,graph));
  if(generationOnly&&normal&&live.matches&&!fs.existsSync(workerReport)) {
    log(`Generation only from captured inputs: ${captured.directory}`);
    return 0;
  }
  let report,reportPresent=false;
  try {
    report=JSON.parse(fs.readFileSync(workerReport,'utf8'));
    if(!report||typeof report.passed!=='boolean'||typeof report.completeToolRun!=='boolean'||
        typeof report.inputsUnchanged!=='boolean'||typeof report.selection!=='string'||!report.selection||
        !Array.isArray(report.checks)||!report.inputs||typeof report.inputs!=='object'||Array.isArray(report.inputs))
      throw Error('Malformed worker report');
    reportPresent=true;
  }
  catch {report={scope:'Verification failed before a complete worker report.',passed:false,
    completeToolRun:false,inputsUnchanged:false,inputs:captured.hashes,checks:[]};}
  const current=report.inputs&&typeof report.inputs==='object'?compareInputs(root,report.inputs):{matches:false,changed:['report-inputs']};
  report.snapshot={mode:'frozen-project-copy',directory:captured.directory,
    sourceHashes:captured.hashes,workerExitCode:worker.status??null,
    liveSourceMatchesCapture:live.matches,changedSourceInputs:live.changed,
    currentInputsMatchReport:current.matches,
    limit:'Endpoint equality and isolated verification bytes; not atomic capture, source-history/ABA detection, toolchain isolation or a sandbox.'};
  report.inputsUnchanged=Boolean(report.inputsUnchanged&&live.matches&&current.matches);
  const checksPassed=report.checks.length>0&&report.checks.every(c=>c.passed===true);
  report.passed=Boolean(normal&&report.passed&&report.completeToolRun===true&&report.selection==='all'&&
    report.inputsUnchanged&&checksPassed);
  if(!normal&&(!Number.isInteger(worker.status)||worker.status===0))report.completeToolRun=false;
  fs.writeFileSync(path.join(root,'formal/results.json'),JSON.stringify(report,null,2)+'\n');
  if(!live.matches)log(`FAIL live source differs from captured input: ${live.changed.join(', ')}`);
  if(!current.matches)log('FAIL current inputs do not match the recorded verification bytes.');
  log(`Frozen input verification: ${report.passed?'current complete pass':'not a current complete pass'}; ${captured.directory}`);
  const partial=report.selection&&report.selection!=='all'&&report.completeToolRun===false&&checksPassed;
  return normal&&reportPresent&&live.matches&&current.matches&&(report.passed||partial)?0:1;
}

if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  if(!process.argv.includes('--self-test'))throw Error('Use --self-test for harness controls');
  const checks=[];
  const check=(name,ok)=>{if(!ok)throw Error(`Harness control failed: ${name}`);checks.push(name);};
  const good={status:0,stdout:'PASS',stderr:''};
  check('success accepted',evaluateToolResult(good,'success','PASS').passed);
  check('counterexample accepted',evaluateToolResult({...good,status:12},'counterexample','PASS').passed);
  check('witness accepted',evaluateToolResult({...good,status:12},'witness','PASS').passed);
  check('tool error rejected',!evaluateToolResult({...good,error:new Error('launch')},'success','PASS').passed);
  check('signal rejected',!evaluateToolResult({...good,signal:'SIGTERM'},'success','PASS').passed);
  check('missing exit rejected',!evaluateToolResult({...good,status:null},'success','PASS').passed);
  check('nonzero success rejected',!evaluateToolResult({...good,status:1},'success','PASS').passed);
  check('zero counterexample rejected',!evaluateToolResult(good,'counterexample','PASS').passed);
  check('unknown expected role rejected',!evaluateToolResult({...good,status:1},'unknown','PASS').passed);
  check('validator exception rejected',!evaluateToolResult(good,'success','PASS',()=>{throw Error('parse');}).passed);
  check('false validator rejected',!evaluateToolResult(good,'success','PASS',()=>false).passed);
  check('wrong property rejected',!evaluateToolResult(good,'success','WRONG').passed);
  const cases=[{id:'A',expected:['sat','unsat']},{id:'B',expected:['sat']}];
  const smt='A\nsat\nunsat\nB\nsat\n';
  check('exact SMT accepted',exactSmtOutput(smt,cases));
  for(const [name,out] of [
    ['missing condition',smt.replace('B\nsat\n','')],['duplicate condition',smt+'B\nsat\n'],
    ['wrong condition order','B\nsat\nA\nsat\nunsat\n'],['unknown SMT result',smt.replace('unsat','unknown')],
    ['solver diagnostic',smt+'(error "parse")\n'],['wrong SMT conclusion',smt.replace('unsat','sat')]])
    check(`${name} rejected`,!exactSmtOutput(out,cases));
  const fixture=()=>{
    const directory=fs.mkdtempSync(path.join(os.tmpdir(),'vellum-harness-control-'));
    fs.mkdirSync(path.join(directory,'formal'));
    fs.writeFileSync(path.join(directory,'model.md'),'original');
    fs.writeFileSync(path.join(directory,'formal/architecture.ttl'),'graph');
    fs.writeFileSync(path.join(directory,'formal/BoundaryFixture.java'),'class BoundaryFixture {}');
    fs.writeFileSync(path.join(directory,'formal/ParserFixture.py'),'print("fixture")');
    fs.writeFileSync(path.join(directory,'formal/check.mjs'),`import fs from 'node:fs';
import path from 'node:path';import crypto from 'node:crypto';import {fileURLToPath} from 'node:url';
const root=path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const m=JSON.parse(fs.readFileSync(path.join(root,'${manifestName}'),'utf8'));
if(fs.readFileSync(path.join(root,'model.md'),'utf8')!=='original')process.exit(2);
const inputs=Object.fromEntries(Object.keys(m.hashes).map(p=>[p,crypto.createHash('sha256').update(fs.readFileSync(path.join(root,p))).digest('hex')]));
fs.writeFileSync(path.join(root,'formal/results.json'),JSON.stringify({passed:true,completeToolRun:true,selection:'all',inputsUnchanged:true,inputs,checks:[{name:'fixture',passed:true}]}));`);
    return directory;
  };
  const live=fixture(),captured=captureInputs(live);
  check('snapshot manifest accepted',validateFrozenInputs(captured.directory).version===1);
  check('Java verification source captured',captured.hashes['formal/BoundaryFixture.java']===fileHash(path.join(captured.directory,'formal/BoundaryFixture.java')));
  fs.writeFileSync(path.join(live,'formal/BoundaryFixture.java'),'class BoundaryFixture { int changed; }');
  check('changed Java verification source detected',!compareInputs(live,captured.hashes).matches);
  fs.writeFileSync(path.join(live,'formal/BoundaryFixture.java'),'class BoundaryFixture {}');
  check('Python verification source captured',captured.hashes['formal/ParserFixture.py']===fileHash(path.join(captured.directory,'formal/ParserFixture.py')));
  fs.writeFileSync(path.join(live,'formal/ParserFixture.py'),'print("changed")');
  check('changed Python verification source detected',!compareInputs(live,captured.hashes).matches);
  fs.writeFileSync(path.join(live,'formal/ParserFixture.py'),'print("fixture")');
  fs.writeFileSync(path.join(live,'model.md'),'changed before late hashing');
  check('prevalidation source change detected',!compareInputs(live,captured.hashes).matches);
  check('captured bytes remain original',fs.readFileSync(path.join(captured.directory,'model.md'),'utf8')==='original');
  fs.writeFileSync(path.join(live,'model.md'),'original');
  check('endpoint ABA limit explicit',compareInputs(live,captured.hashes).matches);
  fs.writeFileSync(path.join(live,'new.md'),'new');
  check('added source detected',!compareInputs(live,captured.hashes).matches);
  fs.unlinkSync(path.join(live,'new.md'));
  fs.unlinkSync(path.join(live,'model.md'));
  check('removed source detected',!compareInputs(live,captured.hashes).matches);
  fs.writeFileSync(path.join(captured.directory,'model.md'),'tampered');
  let rejected=false;try{validateFrozenInputs(captured.directory);}catch{rejected=true;}
  check('frozen input tamper rejected',rejected);
  const runFixture=(mutate,changeExit=false,missingReport=false,patchReport=null)=>{
    const directory=fixture();
    const code=runFrozenVerification({root:directory,args:[],env:{},log:()=>{},
      execute:(command,args,options)=>{
        if(mutate)mutate(directory);
        if(missingReport)return {status:0};
        const result=spawnSync(command,args,options);
        if(patchReport) {
          const reportFile=path.join(path.dirname(args[0]),'results.json');
          const report=JSON.parse(fs.readFileSync(reportFile,'utf8'));
          patchReport(report);fs.writeFileSync(reportFile,JSON.stringify(report));
        }
        return changeExit?{...result,status:1}:result;
      }});
    return {code,directory,report:JSON.parse(fs.readFileSync(path.join(directory,'formal/results.json'),'utf8'))};
  };
  const stable=runFixture();
  check('actual frozen child accepted',stable.code===0&&stable.report.passed&&stable.report.snapshot.currentInputsMatchReport);
  const race=runFixture(directory=>fs.writeFileSync(path.join(directory,'model.md'),'concurrent change'));
  check('child uses frozen source during live edit',race.report.checks.length===1&&race.report.snapshot.workerExitCode===0);
  check('live edit invalidates current report',race.code===1&&!race.report.passed&&!race.report.inputsUnchanged);
  const failed=runFixture(null,true);
  check('failed child cannot publish pass',failed.code===1&&!failed.report.passed);
  const absent=runFixture(null,false,true);
  check('missing worker report rejected',absent.code===1&&!absent.report.passed&&!absent.report.completeToolRun);
  const direct=spawnSync(process.execPath,[path.join(path.dirname(fileURLToPath(import.meta.url)),'check.mjs'),
    '--frozen-worker'],{encoding:'utf8',env:{...process.env,VELLUM_FROZEN_ROOT:''}});
  check('direct internal worker refused',direct.status!==0&&direct.stderr.includes('captured source tree'));
  const malformedRoot=fixture();
  const malformed=runFrozenVerification({root:malformedRoot,args:[],env:{},log:()=>{},
    execute:(_command,args)=>{
      fs.writeFileSync(path.join(path.dirname(args[0]),'results.json'),'{broken');
      return {status:0};
    }});
  check('malformed worker report rejected',malformed===1&&!JSON.parse(fs.readFileSync(path.join(malformedRoot,'formal/results.json'),'utf8')).passed);
  const partialRoot=fixture();
  const partialCode=runFrozenVerification({root:partialRoot,args:[],env:{},log:()=>{},
    execute:(command,args,options)=>{
      const result=spawnSync(command,args,options);
      const reportFile=path.join(path.dirname(args[0]),'results.json');
      const report=JSON.parse(fs.readFileSync(reportFile,'utf8'));
      Object.assign(report,{passed:false,completeToolRun:false,selection:'History'});
      fs.writeFileSync(reportFile,JSON.stringify(report));return result;
    }});
  const partial=JSON.parse(fs.readFileSync(path.join(partialRoot,'formal/results.json'),'utf8'));
  check('partial run stays noncomplete',partialCode===0&&!partial.passed&&!partial.completeToolRun);
  const generationRoot=fixture();
  fs.writeFileSync(path.join(generationRoot,'formal/results.json'),'existing-evidence');
  const generationCode=runFrozenVerification({root:generationRoot,args:['--write'],env:{},log:()=>{},
    execute:(_command,args)=>{
      fs.writeFileSync(path.join(path.dirname(args[0]),'architecture.ttl'),'generated');
      return {status:0};
    }});
  check('generation only creates no proof report',generationCode===0&&
    fs.readFileSync(path.join(generationRoot,'formal/results.json'),'utf8')==='existing-evidence'&&
    fs.readFileSync(path.join(generationRoot,'formal/architecture.ttl'),'utf8')==='generated');
  const schemaOnlyRoot=fixture();
  const schemaOnly=runFrozenVerification({root:schemaOnlyRoot,args:['--write'],env:{VELLUM_JSONSCHEMA_PYTHON:'selected'},log:()=>{},
    execute:(_command,args)=>{fs.writeFileSync(path.join(path.dirname(args[0]),'architecture.ttl'),'generated');return {status:0};}});
  check('schema-only request is not generation-only evidence',schemaOnly===1&&
    !JSON.parse(fs.readFileSync(path.join(schemaOnlyRoot,'formal/results.json'),'utf8')).passed);
  const unicodeOnlyRoot=fixture();
  const unicodeOnly=runFrozenVerification({root:unicodeOnlyRoot,args:['--write'],env:{VELLUM_ICU4J_JAR:'selected'},log:()=>{},
    execute:(_command,args)=>{fs.writeFileSync(path.join(path.dirname(args[0]),'architecture.ttl'),'generated');return {status:0};}});
  check('Unicode-only request is not generation-only evidence',unicodeOnly===1&&
    !JSON.parse(fs.readFileSync(path.join(unicodeOnlyRoot,'formal/results.json'),'utf8')).passed);
  const parserOnlyRoot=fixture();
  const parserOnly=runFrozenVerification({root:parserOnlyRoot,args:['--write'],env:{VELLUM_TREE_SITTER_PYTHON:'selected'},log:()=>{},
    execute:(_command,args)=>{fs.writeFileSync(path.join(path.dirname(args[0]),'architecture.ttl'),'generated');return {status:0};}});
  check('native-parser-only request is not generation-only evidence',parserOnly===1&&
    !JSON.parse(fs.readFileSync(path.join(parserOnlyRoot,'formal/results.json'),'utf8')).passed);
  const incomplete=runFixture(null,false,false,r=>r.completeToolRun=false);
  check('incomplete full pass rejected',incomplete.code===1&&!incomplete.report.passed);
  const checkFailure=runFixture(null,false,false,r=>r.checks[0].passed=false);
  check('failed check overrides pass flag',checkFailure.code===1&&!checkFailure.report.passed);
  const empty=runFixture(null,false,false,r=>r.checks=[]);
  check('empty checks cannot claim pass',empty.code===1&&!empty.report.passed);
  const promotedPartial=runFixture(null,false,false,r=>Object.assign(r,{selection:'History',completeToolRun:false}));
  check('partial pass flag demoted',promotedPartial.code===0&&!promotedPartial.report.passed);
  const noCompletion=runFixture(null,false,false,r=>delete r.completeToolRun);
  check('missing completion metadata rejected',noCompletion.code===1&&!noCompletion.report.passed);
  const badDigest=runFixture(null,false,false,r=>r.inputs['model.md']='invalid');
  check('malformed input digest rejected',badDigest.code===1&&!badDigest.report.passed);
  const noSelection=runFixture(null,false,false,r=>delete r.selection);
  check('missing selection metadata rejected',noSelection.code===1&&!noSelection.report.passed);
  console.log(JSON.stringify({status:'verification-harness-controls-only',controls:checks.length,checks,
    limit:'Fixture/control evidence, not a proof of the host, solver or full verifier.'},null,2));
}
