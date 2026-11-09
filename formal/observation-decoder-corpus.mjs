import {checkObservationDecoder} from './observation-decoder-checks.mjs';
import {checkObservationDecoderFaults} from './observation-decoder-faults.mjs';
console.log(JSON.stringify({scope:'shared-prepaid-F06-decoder-corpus',products:checkObservationDecoder(),faults:await checkObservationDecoderFaults()}));
