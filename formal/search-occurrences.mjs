// Internal owned-result composition only, not a plugin ABI or issuer/security boundary.
import {isDeepStrictEqual as equal} from 'node:util';
const results=new WeakMap(),occurrences=new WeakMap(),copy=structuredClone;
const gates=['grant','scope','continuity','capacity','retained','notStopped'];
export const searchContextCurrent=(request,current)=>current&&equal(current.request,request)&&gates.every(k=>current[k]);
export function retainSearchResult(result,capture){results.set(result,{...capture,tickets:new Map()});}
export const searchResultCapture=result=>results.get(result);
// Protected internal cleanup. Prepared plans may still own the captured value;
// remove lookup paths without mutating a value retained by another owner.
export function discardSearchResultCapture(result,g){
 const capture=results.get(result);if(!capture||!['ownerControlAllowed','retentionReleased','deliveryDrained'].every(k=>g?.[k]===true))return false;
 for(const ticket of capture.tickets.values())occurrences.delete(ticket);
 capture.tickets.clear();return results.delete(result);
}
export function bindSearchOccurrence(result,index,current){
  const capture=results.get(result);
  if(!capture||!searchContextCurrent(capture.request,current)||!Number.isSafeInteger(index)||index<0||index>=result.matches.length)return null;
  if(capture.tickets.has(index))return capture.tickets.get(index);
  // At most one ticket per retained match; no unbounded repeated-lookup issuance.
  const ticket=Object.freeze({kind:'owned-search-occurrence/1'});
  const {lo,hi}=result.matches[index];
  occurrences.set(ticket,{capture,range:{lo,hi}});capture.tickets.set(index,ticket);return ticket;
}
export function resolveSearchOccurrence(ticket,current){
  const owned=occurrences.get(ticket);if(!owned||!searchContextCurrent(owned.capture.request,current))return null;
  const {capture,range}=owned;
  return {ref:copy(capture.snapshot.ref),context:capture.request.context,generation:capture.request.generation,range:copy(range)};
}
