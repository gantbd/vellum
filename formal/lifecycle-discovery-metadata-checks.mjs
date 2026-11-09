import {checkLifecycleDiscoveryProducts} from './lifecycle-discovery-product-checks.mjs';
import {checkLifecycleDiscovery} from './lifecycle-discovery-checks.mjs';
import {checkLifecycleDiscoveryBoundaries} from './lifecycle-discovery-boundaries.mjs';
console.log(JSON.stringify({scope:'handler-bound-lifecycle-discovery-metadata',products:checkLifecycleDiscoveryProducts(),discovery:checkLifecycleDiscovery(),boundaries:checkLifecycleDiscoveryBoundaries()}));
