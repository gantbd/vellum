// D03 finite design reference, not a public ticket format or runtime backend.
import assert from 'node:assert/strict';
const clone=structuredClone;
const key=JSON.stringify;
const ticket=s=>s.epoch*2+s.generation;
const initial=()=>({epoch:0,generation:0,phase:'free',intent:-1,result:'none',
  retained:false,grant:true,dispatched:false,starts:[0,0,0,0],pubs:[0,0,0,0],
  dispatches:[0,0,0,0],outstanding:[false,false,false,false]});

// All actions are serialized abstract decisions. Two submit routes share them.
export function step(state,event) {
  const s=clone(state),i=ticket(s);
  let observation='none';
  switch(event.kind) {
    case 'submit':
      if(event.ticket!==i||s.phase==='retired')observation='invalid';
      else if(!s.grant)observation='unavailable';
      else if(s.phase==='free') {
        s.phase='pending';s.intent=event.intent;s.starts[i]++;observation='accepted';
      } else if(s.intent!==event.intent)observation='mismatch';
      else observation=s.phase==='pending'?'pending':s.retained?s.result:'unavailable';
      break;
    case 'finish':
      if(s.phase==='pending'&&event.ticket===i) {
        s.phase='done';s.retained=true;
        s.result=!s.grant?'rejected':event.result;
        if(s.result==='applied')s.pubs[i]++;
      }
      break;
    case 'forget':
      if(s.phase==='done')s.retained=false;
      break;
    case 'retire':
      if(s.phase==='done') {s.phase='retired';s.retained=false;}
      break;
    case 'next':
      if(s.phase==='retired'&&s.generation<1) {
        s.generation++;s.phase='free';s.intent=-1;s.result='none';s.dispatched=false;
      }
      break;
    case 'epoch':
      if(s.epoch<1) {
        s.epoch++;s.generation=0;s.phase='free';s.intent=-1;s.result='none';
        s.retained=false;s.dispatched=false;
      }
      break;
    case 'grant':s.grant=event.value;break;
    case 'dispatch':
      if(s.phase==='pending'&&s.grant&&!s.dispatched) {
        s.dispatched=true;s.dispatches[i]++;s.outstanding[i]=true;
      }
      break;
    case 'discharge':s.outstanding[event.ticket]=false;break;
    default:throw Error('unknown private event');
  }
  return {state:s,observation};
}

const events=[...Array.from({length:4},(_,i)=>[0,1].flatMap(intent=>
  ['host','agent'].map(route=>({kind:'submit',ticket:i,intent,route})))).flat(),
  ...Array.from({length:4},(_,i)=>['applied','unchanged','rejected','stopped']
    .map(result=>({kind:'finish',ticket:i,result}))).flat(),
  ...['forget','retire','next','epoch','dispatch'].map(kind=>({kind})),
  ...[true,false].map(value=>({kind:'grant',value})),
  ...Array.from({length:4},(_,i)=>({kind:'discharge',ticket:i}))];

export function checkReplayReference() {
  // Unbounded-length exploration of this finite fixture, not arbitrary identities.
  const queue=[initial()],seen=new Set([key(queue[0])]);let transitions=0;
  const witnesses=new Set();
  for(let cursor=0;cursor<queue.length;cursor++) {
    const before=queue[cursor],i=ticket(before);
    assert(before.starts.every(n=>n<=1)&&before.pubs.every((n,j)=>n<=before.starts[j]),'duplicate execution');
    assert(before.dispatches.every(n=>n<=1),'duplicate effect dispatch');
    for(const event of events) {
      transitions++;
      const {state:after,observation}=step(before,event);
      if(event.kind==='submit'&&observation!=='accepted')assert.deepEqual(after,before,'retry mutated state');
      if(event.kind==='submit'&&event.ticket!==i)assert.equal(observation,'invalid','old or unissued ticket executed');
      if(event.kind==='finish'&&event.ticket!==i)assert.deepEqual(after,before,'late completion redirected');
      if(event.kind==='submit'&&!before.grant&&event.ticket===i&&before.phase!=='retired')
        assert.equal(observation,'unavailable','status disclosure after revocation');
      if(['forget','retire','next','epoch'].includes(event.kind)) {
        assert.deepEqual(after.outstanding,before.outstanding,'retention discharged effects');
        assert.deepEqual(after.pubs,before.pubs,'retention rewrote publication');
      }
      if(event.kind==='finish'&&before.phase==='pending'&&event.ticket===i&&after.result!=='applied')
        assert.deepEqual(after.pubs,before.pubs,'nonpublication manufactured commit');
      if(observation==='pending')witnesses.add('duplicate pending');
      if(observation==='mismatch')witnesses.add('intent mismatch');
      if(event.kind==='submit'&&before.phase==='done'&&!before.retained&&before.grant&&event.ticket===i&&event.intent===before.intent)
        {assert.equal(observation,'unavailable');witnesses.add('forgotten terminal not reexecuted');}
      if(before.phase==='done'&&before.outstanding.some(Boolean)&&event.kind==='retire')witnesses.add('retired with obligation held');
      if(before.phase==='retired'&&before.generation===1&&event.kind==='next')
        {assert.deepEqual(after,before);witnesses.add('generation exhaustion no wrap');}
      if(before.epoch===1&&event.kind==='submit'&&event.ticket<2)witnesses.add('old incarnation rejected');
      if(before.phase==='done'&&before.retained&&!before.grant&&observation==='unavailable')witnesses.add('retained result withheld');
      const encoded=key(after);
      if(!seen.has(encoded)) {seen.add(encoded);queue.push(after);}
    }
    assert(queue.length<=200000,'private exploration capacity exceeded');
  }
  const controls=[];
  const detect=(name,condition)=>{assert(condition,`control escaped: ${name}`);controls.push(name);};
  let cache=null;const executions={A:0,B:0};
  for(const id of ['A','B','A']) {if(cache!==id)executions[id]++;cache=id;}
  detect('cache miss treated fresh',executions.A===2);
  let s=step(initial(),{kind:'submit',ticket:0,intent:0}).state;
  const mismatch=step(s,{kind:'submit',ticket:0,intent:1});
  detect('identity rebound to changed intent',mismatch.observation==='mismatch'&&mismatch.state.intent===0);
  s=step(s,{kind:'dispatch'}).state;s=step(s,{kind:'finish',ticket:0,result:'applied'}).state;
  const forgotten=step(s,{kind:'forget'}).state;
  detect('forgotten status reported not applied',forgotten.pubs[0]===1&&!forgotten.retained);
  const retired=step(s,{kind:'retire'}).state;
  detect('retirement drops unresolved charge',retired.outstanding[0]&&retired.dispatches[0]===1);
  const revoked=step(s,{kind:'grant',value:false}).state;
  detect('cached result bypasses release gate',step(revoked,{kind:'submit',ticket:0,intent:0}).observation==='unavailable');
  const next=step(retired,{kind:'next'}).state;
  detect('generation alias accepts old retry',step(next,{kind:'submit',ticket:0,intent:0}).observation==='invalid');
  const newer=step(retired,{kind:'epoch'}).state;
  detect('incarnation alias accepts old retry',step(newer,{kind:'submit',ticket:0,intent:0}).observation==='invalid');
  const stopped=step(step(initial(),{kind:'submit',ticket:0,intent:0}).state,{kind:'finish',ticket:0,result:'stopped'}).state;
  detect('stopped invocation implicitly restarted',step(stopped,{kind:'submit',ticket:0,intent:0}).observation==='stopped');
  const newPending=step(newer,{kind:'submit',ticket:2,intent:1}).state;
  detect('late completion redirected to new request',key(step(newPending,{kind:'finish',ticket:0,result:'applied'}).state)===key(newPending));
  assert.equal(witnesses.size,7,'missing causal witness');
  return {status:'finite-replay-reference-only',states:seen.size,transitions,
    witnesses:[...witnesses].sort(),controls,
    scope:'Two incarnations/two generations/one slot/two intents; opaque publication and modeled discharge, not actual editor/provider/clock/schema or independent certification'};
}
if(process.argv[1]?.endsWith('/replay-reference.mjs'))console.log(JSON.stringify(checkReplayReference(),null,2));
