import {checkRetainedSource} from './native-retained-source-checks.mjs';
import {checkRetainedSourceBoundaries} from './native-retained-source-boundaries.mjs';
import {checkRetainedSourceLifecycle} from './native-retained-source-lifecycle-checks.mjs';
console.log(JSON.stringify({ownership:checkRetainedSource(),boundaries:checkRetainedSourceBoundaries(),lifecycle:checkRetainedSourceLifecycle()}));
