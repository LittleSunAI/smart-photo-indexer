import {text as localeText} from "../i18n/Runtime";
import type {HelpMessage} from './HelpMessage';

export function outputHelp(mode:string):HelpMessage{
 return {id:mode==='tags'?'main.output-tags':mode==='description'?'main.output-description':'main.output-both'};
}
export function sortHelp(value:string):HelpMessage{
 const groups={readyFirst:'sort.ready-first',readyLast:'sort.ready-last',errorsFirst:'sort.errors-first',errorsLast:'sort.errors-last'} as const;
 if(value in groups)return {id:groups[value as keyof typeof groups]};
 const specific={dateTakenNewest:'sort.taken-new',takenDesc:'sort.taken-new',dateTakenOldest:'sort.taken-old',takenAsc:'sort.taken-old',modifiedDesc:'sort.modified-new',modifiedAsc:'sort.modified-old',analyzedDesc:'sort.analyzed-new',analyzedAsc:'sort.analyzed-old',nameAsc:'sort.name-asc',nameDesc:'sort.name-desc',status:'sort.status-ready',statusAsc:'sort.status-ready',statusDesc:'sort.status-error',relevance:'search.sort-relevance'} as const;
 if(value in specific)return {id:specific[value as keyof typeof specific]};
 const property=value.replace(/Asc$|Desc$/,'');
 const fields={size:'search.sort-size',dimensions:'search.sort-dimensions',type:'search.sort-type',camera:'search.sort-camera',path:'search.sort-path',tags:'search.sort-tags',favorite:'search.sort-favorite'} as const;
 if(property in fields)return {id:fields[property as keyof typeof fields]};
 const names:Record<string,string>={size:localeText("common", "ui.SelectionHelp.7a2c21372124d72c"),dimensions:localeText("common", "ui.SelectionHelp.01ae1486d064cbf7"),type:localeText("common", "ui.SelectionHelp.27c94684928b735f"),camera:localeText("common", "ui.SelectionHelp.66d950ed97fcee0e"),path:localeText("common", "ui.SelectionHelp.4bf6de7f853a480e"),tags:localeText("common", "ui.SelectionHelp.6797958e9c9909ac"),favorite:localeText("common", "ui.SelectionHelp.59dccd9ce6c51309")};
 return {id:'sort.other',args:{свойство:names[property]??localeText("common", "ui.SelectionHelp.bb83660a3dbe776c"),направление:value.endsWith('Desc')?localeText("common", "ui.SelectionHelp.3d17c4dc1c205d9b"):localeText("common", "ui.SelectionHelp.f8437f6d226f9311")}};
}
export function searchStatusHelp(value:string):HelpMessage{
 const states={all:'search.status-all',processed:'search.status-ready',queued:'search.status-queued',notProcessed:'search.status-not-ready',error:'search.status-error',missing:'search.status-missing',relocated:'search.status-relocated'} as const;
 return {id:states[value as keyof typeof states]??'search.status'};
}
export function sectionHelp(id:string|undefined):HelpMessage|undefined{
 const sections={'queue-items':'queue.section','queue-details':'queue.section','queue-history':'queue.history','library-roots':'library.connect','index-maintenance':'library.index','index-duplicates':'library.duplicates','index-backup':'library.backup-index','index-cache':'library.cache'} as const;
 return id&&id in sections?{id:sections[id as keyof typeof sections]}:undefined;
}
