import {checkReadProducts} from './read-product-checks.mjs';
import {checkReadDiscovery} from './read-discovery-checks.mjs';
import {checkReadDiscoveryBoundaries} from './read-discovery-boundaries.mjs';
console.log(JSON.stringify({scope:'isolated-read-product-and-versioned-discovery',products:checkReadProducts(),discovery:checkReadDiscovery(),boundaries:checkReadDiscoveryBoundaries()}));
