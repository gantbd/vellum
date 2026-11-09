import {checkNativeVersionSetCapture} from './native-version-set-capture-checks.mjs';
import {checkVersionSetCaptureBoundaries} from './native-version-set-capture-boundaries.mjs';
import {checkVersionSetHandoff} from './native-version-set-handoff-checks.mjs';
console.log(JSON.stringify({ownership:checkNativeVersionSetCapture(),boundaries:checkVersionSetCaptureBoundaries(),handoff:checkVersionSetHandoff()}));
