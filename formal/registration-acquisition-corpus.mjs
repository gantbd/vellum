import {checkRegistrationAcquisition} from './registration-acquisition-checks.mjs';
import {checkRegistrationAcquisitionFaults} from './registration-acquisition-faults.mjs';
console.log(JSON.stringify({scope:'resource-catalog-acquisition-result-corpus',products:checkRegistrationAcquisition(),faults:checkRegistrationAcquisitionFaults()}));
