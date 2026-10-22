import {text as localeText} from "../i18n/Runtime";
import mappings from './parameter-help.json';
import type {HelpMessage} from './HelpMessage';
type ParameterId=Extract<HelpMessage['id'],`parameter.${string}`>;
/** Route meaning takes precedence. Only reviewed advanced fields have a binding. */
export function parameterHelp(key:string,routeId:string|undefined):HelpMessage{
 const id=(mappings.routes as Record<string,ParameterId>)[`${routeId}|${key}`]??(mappings.keys as Record<string,ParameterId>)[key];
 if(!id)throw Error(localeText("common", "ui.ParameterHelp.b8046c1513d2430d", {"value1": (routeId), "value2": (key)}));
 // The compiler validates every generated mapping as an argument-free catalog entry.
 return {id} as HelpMessage;
}
