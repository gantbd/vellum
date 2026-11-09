import {checkSettingsQueryResults} from './settings-query-result-checks.mjs';
import {checkSettingsQueryResultFaults} from './settings-query-result-faults.mjs';
console.log(JSON.stringify({scope:'native-F07-result-and-fault-corpus',products:checkSettingsQueryResults(),faults:checkSettingsQueryResultFaults()}));
