import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {nativeSourceSearchFixture as fixture,nativeSourceSearchCurrent as current} from './native-source-search-checks.mjs';
import {nativeSourceContextFacts as facts} from './native-source-context-checks.mjs';
import {nativeCombinedSearchFixture} from './native-combined-search-checks.mjs';
import {nativeVersionSetSearchFixture} from './native-version-set-search-checks.mjs';
import {invocationUsage} from './invocation-publication.mjs';
import {searchResultCapture,resolveSearchOccurrence,bindSearchOccurrence,discardSearchResultCapture} from './search-occurrences.mjs';
import {replacementResultFootprint,discardLiteralSearchResult,prepareReplacement} from './literal-search.mjs';
export function checkNativeSearchRetirement(){let cases=0,denials=0;
 let dependencyCases=0;
 for(const [kind,make]of [['source',fixture],['combined',nativeCombinedSearchFixture],['version-set',nativeVersionSetSearchFixture]]){
  const x=make(),result=x.query(),g={...facts},domain=x.domain??x.x.z.domain;let reads=0;assert.equal(result.status,'complete');Object.defineProperty(g,'ownerControlAllowed',{get(){reads++;if(reads===2)assert.equal(x.retire(result),true);return true;}});assert.equal(x.retire(result,g),true,kind);assert.equal(reads,1,kind+'/captured-once');assert.equal(x.usage().results,0);assert.equal(invocationUsage(domain).faulted,false);dependencyCases++;
  const y=make(),other=y.query(),h={...facts};Object.defineProperty(h,'ownerControlAllowed',{get(){assert.equal(y.retire(other),true);return true;}});let retired;assert.doesNotThrow(()=>{retired=y.retire(other,h);},kind);assert.equal(retired,false);assert.equal(y.usage().results,0);assert.equal(invocationUsage(y.domain??y.x.z.domain).faulted,false);dependencyCases++;
 }
 for(const stop of [false,true]){
  const z=fixture(),r=z.request(),q=current(r),out=z.query(r,q),capture=searchResultCapture(out),heldValue=structuredClone({request:capture.request,snapshot:capture.snapshot,limits:capture.limits});assert.equal(out.status,'complete');assert(out.matches.length>0);assert(replacementResultFootprint(out));assert(resolveSearchOccurrence(out.matches[0].reference,q));
  for(const field of ['ownerControlAllowed','retentionReleased','deliveryDrained'])for(const value of [false,undefined,1]){
   const g={...facts,[field]:value},before=z.usage();assert.equal(discardSearchResultCapture(out,g),false);assert.equal(discardLiteralSearchResult(out,g),false);assert.equal(z.retire(out,g),false);assert.deepEqual(z.usage(),before);assert.equal(searchResultCapture(out),capture);assert(resolveSearchOccurrence(out.matches[0].reference,q));assert(replacementResultFootprint(out));denials++;
  }
  if(stop)assert.equal(z.x.stop(z.reference,z.x.c,{ownerControlAllowed:true}).status,'stopped');
  assert.equal(z.retire(out),true);assert.equal(z.retire(out),false);assert.equal(searchResultCapture(out),undefined);assert.equal(replacementResultFootprint(out),null);for(const match of out.matches)assert.equal(resolveSearchOccurrence(match.reference,q),null);assert.equal(bindSearchOccurrence(out,0,q),null);assert.equal(prepareReplacement(out,{},q,z.x.z.root(),{}),null);assert.equal(discardSearchResultCapture(out,facts),false);assert.equal(discardLiteralSearchResult(out,facts),false);
  // A remaining prepared owner may hold this value. Retirement removes lookup
  // paths, not the immutable source/request/limit value it already captured.
  assert.deepEqual({request:capture.request,snapshot:capture.snapshot,limits:capture.limits},heldValue);assert.equal(capture.tickets.size,0);assert.equal(z.x.usage().holds,0);assert.equal(z.x.usage().retainedRoots,stop?0:1);cases++;
 }
 return {scope:'isolated-native-search-protected-registry-retirement',cases,dependencyCases,protectedDenials:denials,rawCaptureOccurrenceFootprintAndRebindRetired:true,retainedValueNotMutated:true,physicalReclamationOrIndependentLeaseProved:false};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkNativeSearchRetirement()));
