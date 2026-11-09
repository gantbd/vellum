import {checkNativeRetainedVersionSets} from './native-retained-version-set-checks.mjs';
import {checkNativeRetainedVersionSetBoundaries} from './native-retained-version-set-boundaries.mjs';
import {checkNativeVersionSetLifecycle} from './native-version-set-lifecycle-checks.mjs';
console.log(JSON.stringify({ownership:checkNativeRetainedVersionSets(),boundaries:checkNativeRetainedVersionSetBoundaries(),lifecycle:{installed:checkNativeVersionSetLifecycle(false,true),search:checkNativeVersionSetLifecycle(true,true)}}));
