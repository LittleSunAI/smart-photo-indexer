import {modelChoices,choiceFor} from './EditorModelCatalog';
import type {EditorDraft,EditorProfile} from './EditorContracts';
import {parameterMode} from './EditorParameterState';
export function profileParameters(model:string,profile:EditorProfile):Record<string,unknown>{
  const choice=modelChoices.find(c=>c.route===model&&c.tier===profile);if(choice)return choice.preset;
  if(model==='fal-ai/object-removal/mask')return {model:profile==='economy'?'low_quality':profile==='balanced'?'medium_quality':'best_quality'};
  if(model==='fal-ai/flux-2-pro/outpaint')return {mode:profile==='economy'?'fast':'high'};
  return {};
}
export function effectiveParameter(d:EditorDraft,key:string,fallback:unknown):unknown {
  const mode=parameterMode(d,key);
  if(mode==='provider-default')return fallback;
  if(Object.hasOwn(d.formValues,key))return d.formValues[key];
  return choiceFor(d)?.preset[key]??profileParameters(d.modelId??'',d.profile)[key]??fallback;
}
