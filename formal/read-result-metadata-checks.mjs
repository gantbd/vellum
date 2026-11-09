import {checkReadResultProducts} from './read-result-product-checks.mjs';
import {checkReadResultDiscovery} from './read-result-discovery-checks.mjs';
import {checkReadResultDiscoveryBoundaries} from './read-result-discovery-boundaries.mjs';
console.log(JSON.stringify({scope:'handler-bound-read-result-metadata-and-discovery',products:checkReadResultProducts(),discovery:checkReadResultDiscovery(),boundaries:checkReadResultDiscoveryBoundaries()}));
