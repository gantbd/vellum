// Trusted serialized F18 composition, not a public token codec or preemption API.
import fs from 'node:fs';
import {invocationExecutionState,stopInvocation} from './invocation-publication.mjs';
const catalog=JSON.parse(fs.readFileSync(new URL('./invocation-control.json',import.meta.url)));
export function requestInvocationStop(domain,ticket,context,g){
 if(!catalog.gates.every(k=>g?.[k]===true)||!invocationExecutionState(domain,ticket,context,false))return {status:'unavailable'};
 return stopInvocation(domain,ticket,g);
}
