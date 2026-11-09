import {checkRegistrationQueryResults} from './registration-query-result-checks.mjs';
import {checkRegistrationQueryResultFaults} from './registration-query-result-faults.mjs';
console.log(JSON.stringify({scope:'native-registration-query-result-corpus',products:checkRegistrationQueryResults(),faults:checkRegistrationQueryResultFaults()}));
