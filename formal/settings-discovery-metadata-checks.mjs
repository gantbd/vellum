import {checkSettingsDiscoveryProducts} from './settings-discovery-product-checks.mjs';
import {checkSettingsDiscovery} from './settings-discovery-checks.mjs';
import {checkSettingsDiscoveryBoundaries} from './settings-discovery-boundaries.mjs';
console.log(JSON.stringify({scope:'handler-bound-settings-discovery-metadata',products:checkSettingsDiscoveryProducts(),discovery:checkSettingsDiscovery(),boundaries:checkSettingsDiscoveryBoundaries()}));
