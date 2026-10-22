import {text} from './Runtime';
import type {ResourceKeys} from './generated/ResourceKeys';
import optionBindings from './model-option-bindings.json';

/** Display-only bindings: callers retain the original model IDs and enum values. */
export function recognitionModelLabel(provider:string,id:string,fallback:string){
 return text('models',`recognition.${provider}.${id}` as ResourceKeys['models'],{defaultValue:fallback,nsSeparator:false});
}
export function editorModelHelp(id:string,fallback:string){
 return text('models',`editor.help.${id}` as ResourceKeys['models'],{defaultValue:fallback});
}
export function modelOptionLabel(value:string){
 const key=(optionBindings as Record<string,string>)[value];
 return key?text('models',key as ResourceKeys['models']):value;
}
