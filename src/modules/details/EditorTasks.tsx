import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {EditorPublicationCard,type EditorPublication} from './EditorPublicationCard';
import {MeasuredCardList} from '../../shared/ui/MeasuredCardList';
import {HintTitle} from '../../shared/ui/InfoHint';
import {sectionHints} from '../../shared/ui/SectionHints';
import {useEffect,useState} from 'react';
import {invoke} from "../../shared/tauri/invoke";
import {listen} from '@tauri-apps/api/event';
import {editorService} from '../photoEditor/EditorService';
import {EditorJobList} from '../photoEditor/EditorJobList';
import {EditorErrorDialog} from '../photoEditor/EditorErrorDialog';
import type {EditorJob} from '../photoEditor/EditorContracts';
type Publication=EditorPublication;
export function EditorTasks({excludeProjectId}:{excludeProjectId?:string}){
  useCurrentLocale();
 const [jobs,setJobs]=useState<EditorJob[]>([]),[publications,setPublications]=useState<Publication[]>([]),[error,setError]=useState(''),[reloadError,setReloadError]=useState('');
 const [expandedPublications,setExpandedPublications]=useState<Record<string,boolean>>({});
 useEffect(()=>{setExpandedPublications(previous=>Object.fromEntries(publications.map(p=>[p.id,previous[p.id]??Boolean(p.error)])));},[publications]);
 useEffect(()=>{let disposed=false,busy=false,again=false;const off:Array<()=>void>=[];const refresh=async()=>{if(busy){again=true;return;}busy=true;try{const {jobs,publications}=await invoke<{jobs:EditorJob[];publications:Publication[]}>('get_editor_workbench');if(!disposed){setJobs(jobs);setPublications(publications);setReloadError('');}}catch(e){if(!disposed)setReloadError(String(e));}finally{busy=false;if(again&&!disposed){again=false;void refresh();}}};void Promise.all(['smart-photo-indexer:editor-jobs-changed','smart-photo-indexer:editor-publications-changed'].map(event=>listen(event,()=>void refresh()).then(f=>disposed?f():off.push(f)))).then(()=>{if(!disposed)void refresh();}).catch(e=>{if(!disposed)setReloadError(String(e));});return()=>{disposed=true;off.forEach(f=>f());};},[]);
 const open=(projectId:string,stepId?:string)=>void invoke('open_editor_project',{projectId,stepId}).catch(e=>setError(String(e)));
 const otherJobs=jobs.filter(job=>job.projectId!==excludeProjectId);
 if(!otherJobs.length&&!publications.length&&!error&&!reloadError)return null;
 return <details className="details-collapsible-card task-editor-jobs"><Hint message={{id:"publication.other-projects"}}><summary><HintTitle title={localeText("details", "ui.EditorTasks.3843fd44a43ad839", {"value1": (otherJobs.length+publications.length)})}  message={{id:"publication.other-projects"}}/></summary></Hint><div className="details-collapsible-card__body"><EditorJobList projectOnly={Boolean(excludeProjectId)} jobs={otherJobs} onChange={setJobs} onOpenResult={id=>{const job=otherJobs.find(j=>j.results?.some(r=>r.stepId===id));if(job)open(job.projectId,id);}} onOpenProject={projectId=>open(projectId)}/>{publications.length?<details><Hint message={{id:"publication.section"}}><summary>{localeText("details", "ui.EditorTasks.10ad98dc2799a105")}{' '}{publications.length}</summary></Hint><MeasuredCardList items={publications} itemKey={p=>p.id} label={localeText("details", "ui.EditorTasks.3090626d0b9ea4a5")} render={p=><EditorPublicationCard readOnly={Boolean(excludeProjectId&&excludeProjectId!==p.projectId)} publication={p} open={expandedPublications[p.id]??Boolean(p.error)} onOpenProject={()=>open(p.projectId)} onToggle={()=>setExpandedPublications(previous=>({...previous,[p.id]:!(previous[p.id]??Boolean(p.error))}))} onAction={async(id,action)=>{try{await invoke(action==='cancel'?'cancel_editor_publication':'retry_editor_publication',{id});setPublications(await invoke<Publication[]>('get_editor_publications'));setError('');}catch(e){setError(String(e));}}}/>}/>
</details>:null}{error||reloadError?<EditorErrorDialog message={error||reloadError} onClose={()=>{setError('');setReloadError('');}}/>:null}</div></details>;
}
