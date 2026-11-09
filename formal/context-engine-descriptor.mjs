// Fixed known first-party declarations; never caller-selected handlers.
import {engineSettingsInputForms} from './engine-settings-state.mjs';
import {registrationInputForms} from './registration-invocation-state.mjs';
import {sourceSerializationInputForm,scopedSourceSerializationInputForm} from './source-serialization.mjs';
import {invocationTerminalForms} from './invocation-publication.mjs';
import {outerInputSchema,inputSchemaCoverage} from './input-schema-profiles.mjs';
const contract='engine-context-declaration/1',variants=['semantic-settings','supported-registry'];
const freeze=v=>{if(v&&typeof v==='object'){Object.values(v).forEach(freeze);Object.freeze(v);}return v;};
export function contextEngineDescriptor(){
 const forms=[...engineSettingsInputForms(),...registrationInputForms(),sourceSerializationInputForm(),scopedSourceSerializationInputForm()];
 if(forms.some(f=>f.family!=='F16'||!variants.includes(f.variant)))return null;
 const rows=[];
 for(const id of variants){const input=outerInputSchema(forms.filter(f=>f.variant===id));if(!input)return null;
  rows.push({id,input:{contract,variant:id,coverage:inputSchemaCoverage,schema:input},
   outcomes:{qualifiedLocalTerminalForms:invocationTerminalForms(),effect:'none',release:'captured-current-recipient-intersection; not guaranteed disclosure'},
   requirements:{currentPredicates:'owning-profile-context-authority-dependencies-and-state-specific',inputCoverage:inputSchemaCoverage,notEstablished:['semantic-enablement','complete-value-codec','public-wire','installed-all-family-dispatch']},
   limits:{values:'current-owning-request-preparation-and-execution-profiles'},
   cancellation:'owned-prepublication-stop; accepted-publication-is-not-retracted'});
 }
 return freeze({family:'F16',contract,version:1,kind:'localCommand',variants:rows,presentation:{}});
}
