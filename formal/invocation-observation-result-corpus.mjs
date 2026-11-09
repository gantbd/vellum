import {checkInvocationObservationResults} from './invocation-observation-result-checks.mjs';
import {checkInvocationObservationResultFaults} from './invocation-observation-result-faults.mjs';
console.log(JSON.stringify({scope:'closed-native-F06-result-and-fault-corpus',products:checkInvocationObservationResults(),faults:await checkInvocationObservationResultFaults()}));
