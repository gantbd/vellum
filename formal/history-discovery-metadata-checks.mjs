import {checkHistoryDiscoveryProducts} from './history-discovery-product-checks.mjs';
import {checkHistoryDiscovery} from './history-discovery-checks.mjs';
import {checkHistoryDiscoveryBoundaries} from './history-discovery-boundaries.mjs';
console.log(JSON.stringify({scope:'handler-bound-history-discovery-metadata',products:checkHistoryDiscoveryProducts(),discovery:checkHistoryDiscovery(),boundaries:checkHistoryDiscoveryBoundaries()}));
