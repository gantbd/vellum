// Finite authorized definition graph, not a schema evaluator or URI resolver.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';

export function definitionClosure(nodes,root,budget) {
  const seen=new Set(),pending=[root];let spent=0;
  while(pending.length) {
    if(spent===budget)return {status:'exhausted'};
    spent++;const id=pending.pop();if(seen.has(id))continue;
    const n=nodes.find(n=>n.id===id);
    if(!n)return {status:'unavailable'};
    if(!n.authorized)return {status:'denied'};
    seen.add(id);pending.push(...n.imports);
  }
  return {status:'complete',definitions:[...seen].sort().map(id=>({id,version:nodes.find(n=>n.id===id).version}))};
}
export function checkDefinitionClosure() {
  let graphCases=0;const controls=[];
  // Oracle enumerates all subsets and intersects the import-closed supersets.
  for(let edges=0;edges<512;edges++)for(let root=0;root<3;root++) {
    const nodes=Array.from({length:3},(_,i)=>({id:i,version:1,authorized:true,
      imports:Array.from({length:3},(_,j)=>j).filter(j=>edges&(1<<(i*3+j)))}));
    const supersets=Array.from({length:8},(_,i)=>i).filter(mask=>(mask&(1<<root))&&
      nodes.every(n=>!(mask&(1<<n.id))||n.imports.every(j=>mask&(1<<j))));
    const least=supersets.reduce((a,b)=>a&b,7);
    const expected=nodes.filter(n=>least&(1<<n.id)).map(n=>({id:n.id,version:1}));
    const before=structuredClone(nodes),actual=definitionClosure(nodes,root,20);
    assert.deepEqual(actual,{status:'complete',definitions:expected});assert.deepEqual(nodes,before);graphCases++;
  }
  const nodes=[{id:0,version:1,authorized:true,imports:[1]},{id:1,version:3,authorized:true,imports:[0]}];
  const control=(name,ok)=>{assert(ok,name);controls.push(name);};
  control('cycle-is-bounded-closure-not-recursive-validation',definitionClosure(nodes,0,10).status==='complete');
  control('exhaustion-not-partial-success',definitionClosure(nodes,0,1).status==='exhausted');
  control('missing-import-not-auto-fetch',definitionClosure(nodes.slice(0,1),0,10).status==='unavailable');
  control('unauthorized-import-not-valid',definitionClosure([nodes[0],{...nodes[1],authorized:false}],0,10).status==='denied');
  const changed=[nodes[0],{...nodes[1],version:4}];
  control('import-version-part-of-closure',JSON.stringify(definitionClosure(nodes,0,10))!==JSON.stringify(definitionClosure(changed,0,10)));
  control('unrelated-definition-not-captured',definitionClosure([...nodes,{id:2,version:9,authorized:true,imports:[]}],0,10).definitions.length===2);
  return {status:'finite-definition-closure-only',graphCases,controls,validatorTerminationProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)
  console.log(JSON.stringify(checkDefinitionClosure(),null,2));
