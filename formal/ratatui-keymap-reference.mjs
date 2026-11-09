// Ratatui-example input reference orchestration; not a delivered Rust UI or engine API.
import {checkKeymapProducts,checkKeymapActivation,checkKeymapMutations} from './frontend-keymaps.mjs';
import * as configuration from './frontend-configuration-reference.mjs';
console.log(JSON.stringify({...checkKeymapProducts(),activationControls:checkKeymapActivation(configuration),mutations:await checkKeymapMutations()},null,2));
