import {checkNativeCombinedSearch} from './native-combined-search-checks.mjs';
import {checkCombinedSearchBoundaries} from './native-combined-search-boundaries.mjs';
import {checkCombinedSearchLifecycle} from './native-combined-search-lifecycle-checks.mjs';
console.log(JSON.stringify({ownership:checkNativeCombinedSearch(),boundaries:checkCombinedSearchBoundaries(),lifecycle:checkCombinedSearchLifecycle()}));
