import {pathToFileURL} from 'node:url';
import {checkCombinedSearchLifecycle} from './native-combined-search-lifecycle-checks.mjs';
export const checkRetainedAnnotationLifecycle=()=>checkCombinedSearchLifecycle(true);
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)console.log(JSON.stringify(checkRetainedAnnotationLifecycle()));
