import {checkNativeVersionSetInstallation} from './native-version-set-installation-checks.mjs';
import {checkNativeVersionSetInstallationBoundaries} from './native-version-set-installation-boundaries.mjs';
console.log(JSON.stringify({ownership:checkNativeVersionSetInstallation(),boundaries:checkNativeVersionSetInstallationBoundaries()}));
