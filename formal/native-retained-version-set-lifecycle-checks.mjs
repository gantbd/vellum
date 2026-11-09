import {checkNativeVersionSetLifecycle} from './native-version-set-lifecycle-checks.mjs';
console.log(JSON.stringify({installed:checkNativeVersionSetLifecycle(false,true),search:checkNativeVersionSetLifecycle(true,true)}));
