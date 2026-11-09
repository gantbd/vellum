import {checkSettingsQueryGateway} from './settings-query-gateway-checks.mjs';
import {checkSettingsQueryGatewayFaults} from './settings-query-gateway-faults.mjs';
console.log(JSON.stringify({scope:'actual-parent-F07-decoder-corpus',products:checkSettingsQueryGateway(),faults:checkSettingsQueryGatewayFaults()}));
