import {checkSearchProducts} from './search-product-checks.mjs';
import {checkSearchDiscovery} from './search-discovery-checks.mjs';
import {checkSearchDiscoveryBoundaries} from './search-discovery-boundaries.mjs';
import {checkSearchValueShapes} from './search-value-shape-checks.mjs';
console.log(JSON.stringify({scope:'isolated-search-product-and-versioned-discovery',products:checkSearchProducts(),discovery:checkSearchDiscovery(),boundaries:checkSearchDiscoveryBoundaries(),valueShapes:checkSearchValueShapes()}));
