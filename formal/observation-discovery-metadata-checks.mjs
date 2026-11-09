import {checkObservationDiscoveryProducts} from './observation-discovery-product-checks.mjs';
import {checkObservationDiscovery} from './observation-discovery-checks.mjs';
import {checkObservationDiscoveryBoundaries} from './observation-discovery-boundaries.mjs';
console.log(JSON.stringify({scope:'handler-bound-observation-discovery-metadata',products:checkObservationDiscoveryProducts(),discovery:checkObservationDiscovery(),boundaries:checkObservationDiscoveryBoundaries()}));
