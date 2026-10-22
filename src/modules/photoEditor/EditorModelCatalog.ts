import {editorModelHelp} from '../../shared/i18n/ModelLabels';
import {text as localeText} from "../../shared/i18n/Runtime";
import catalog from './catalog.json';
import type {EditorDraft,EditorProfile} from './EditorContracts';
export interface ModelChoice {id:string;name:string;route:string;tier:EditorProfile;preset:Record<string,unknown>;advancedFields:string[];help:string;localPolicies:Record<string,unknown>}
export const modelChoices=(catalog.choices as unknown as ModelChoice[]).map(choice=>({...choice,get help(){return editorModelHelp(choice.id,choice.help);}}));
export const operationChoices=catalog.operationChoices as Record<string,Record<EditorProfile,string[]>>;
export function choicesFor(operation:string):ModelChoice[]{const ids=Object.values(operationChoices[operation]??{}).flat();return ids.map(id=>modelChoices.find(c=>c.id===id)).filter((c):c is ModelChoice=>Boolean(c));}
export function choiceFor(d:EditorDraft):ModelChoice|undefined{const choices=choicesFor(d.operation);return choices.find(c=>c.id===d.formValues.__choiceId&&c.route===d.modelId&&c.tier===d.profile)??choices.find(c=>c.route===d.modelId&&c.tier===d.profile)??choices.find(c=>c.route===d.modelId);}
export const tierName=(profile:EditorProfile)=>profile==='economy'?localeText("editor", "ui.EditorModelCatalog.33abf9fbda7d56c4"):profile==='balanced'?localeText("editor", "ui.EditorModelCatalog.a4b8720d6debc72b"):localeText("editor", "ui.EditorModelCatalog.187d55eb3dcc83e8");
export function conditionalField(key:string,values:Record<string,unknown>,model:string):boolean{
 if(key==='output_compression')return ['jpeg','webp'].includes(String(values.output_format));
 if(key.startsWith('face_enhancement_'))return values.face_enhancement===true;
 if(key==='strength'&&model.startsWith('topaz/'))return values.model==='Text Refine';
 if(key==='fix_compression')return values.model!=='CGI';
 if(key==='highres_denoise')return values.enable_hr_fix===true;
 if(model==='fal-ai/seedvr/upscale/image')return !(key==='upscale_factor'&&values.upscale_mode==='target'||key==='target_resolution'&&values.upscale_mode!=='target');
 return true;
}
