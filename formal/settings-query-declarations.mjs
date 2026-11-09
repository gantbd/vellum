// Immutable existing F07 field declarations consumed by the native request guard.
const common=Object.freeze(['contract','operation','context','target','profile']),forms=Object.freeze(['effective-configuration','permitted-origin','capabilities'].map(operation=>Object.freeze({family:'F07',contract:'configuration-query/1',operation,variant:operation,fields:Object.freeze([...common,...(operation==='capabilities'?[]:['key'])])})));
export function settingsQueryInputForms(){return forms;}
