import {checkRetainedAnnotations} from './native-retained-annotation-checks.mjs';
import {checkRetainedAnnotationBoundaries} from './native-retained-annotation-boundaries.mjs';
import {checkCombinedSearchLifecycle} from './native-combined-search-lifecycle-checks.mjs';
console.log(JSON.stringify({ownership:checkRetainedAnnotations(),boundaries:checkRetainedAnnotationBoundaries(),lifecycle:checkCombinedSearchLifecycle(true)}));
