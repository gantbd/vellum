import {checkDiscoveryGateway} from './discovery-gateway-checks.mjs';
import {checkDiscoveryGatewayFaults} from './discovery-gateway-faults.mjs';
console.log(JSON.stringify({scope:'actual-F01-owner-gateway-products-and-fixed-faults',products:checkDiscoveryGateway(),faults:await checkDiscoveryGatewayFaults()}));
