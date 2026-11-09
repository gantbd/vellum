import {checkNativeVersionSetQueries} from './native-version-set-query-checks.mjs';
import {checkNativeVersionSetQueryBoundaries} from './native-version-set-query-boundaries.mjs';
console.log(JSON.stringify({ownership:checkNativeVersionSetQueries(),boundaries:checkNativeVersionSetQueryBoundaries()}));
