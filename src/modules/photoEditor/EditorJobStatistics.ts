import {localeTag as displayLocaleTag} from "../../shared/i18n/Formatting";
import {text as localeText} from "../../shared/i18n/Runtime";
import type {EditorJob} from './EditorContracts';
export const finishedJob=(job:EditorJob)=>job.readOnly||['completed','failed','cancelled'].includes(job.state);
export function jobDate(seconds:number){return new Date(seconds*1000).toLocaleString(displayLocaleTag(),{day:'2-digit',month:'2-digit',year:'numeric',hour:'2-digit',minute:'2-digit'});}
export function jobDuration(job:EditorJob,now=Math.floor(Date.now()/1000)){
 const seconds=Math.max(0,(finishedJob(job)?job.completedAtUnix??job.updatedAtUnix:now)-job.createdAtUnix);
 if(seconds<60)return localeText("editor", "ui.EditorJobStatistics.914526f79961207e", {"value1": (seconds)});
 if(seconds<3600)return localeText("editor", "ui.EditorJobStatistics.8acec7f722488d31", {"value1": (Math.floor(seconds/60)), "value2": (seconds%60)});
 return localeText("editor", "ui.EditorJobStatistics.2192a1f8c2534e48", {"value1": (Math.floor(seconds/3600)), "value2": (Math.floor(seconds%3600/60))});
}
export function jobProcessingDuration(job:EditorJob){return job.startedAtUnix?jobDuration({...job,createdAtUnix:job.startedAtUnix}):localeText("editor", "ui.EditorJobStatistics.0b356913ed176e28");}
export function jobSize(value:unknown):string{if(value&&typeof value==='object'&&'width' in value&&'height' in value)return `${value.width}×${value.height}`;return typeof value==='string'?value:localeText("editor", "ui.EditorJobStatistics.4655fd9f493d942e");}
