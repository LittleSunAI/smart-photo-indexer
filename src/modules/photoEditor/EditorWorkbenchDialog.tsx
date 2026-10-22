import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useState} from 'react';
import {invoke} from "../../shared/tauri/invoke";
import {EditorModal} from './EditorModal';
import {MeasuredCardList} from '../../shared/ui/MeasuredCardList';
import {EditorProjectCard,type EditorProjectSummary,type EditorProjectPreview} from './EditorProjectCard';
import{ProjectCleanup}from'./ProjectCleanup';

const loadPreview=(projectId:string)=>invoke<EditorProjectPreview>('get_editor_workbench',{projectId});
export function EditorWorkbenchDialog({onClose,selectedPaths,cleanupInitiallyOpen=false}:{onClose():void;selectedPaths?:string[];cleanupInitiallyOpen?:boolean}){
  useCurrentLocale();
 const [projects,setProjects]=useState<EditorProjectSummary[]>([]),[error,setError]=useState(''),[busy,setBusy]=useState(false),[loading,setLoading]=useState(true),[query,setQuery]=useState(''),[period,setPeriod]=useState('all');
 const[refresh,setRefresh]=useState(0);
 useEffect(()=>{let alive=true;void invoke<{projects:EditorProjectSummary[]}>('get_editor_workbench').then(v=>{if(alive)setProjects(v.projects);}).catch(e=>{if(alive)setError(String(e));}).finally(()=>{if(alive)setLoading(false);});return()=>{alive=false;};},[refresh]);
 const open=async(projectId:string)=>{setBusy(true);setError('');try{await invoke('open_editor_project',{projectId});onClose();}catch(e){setError(String(e));}finally{setBusy(false);}};
 const text=query.trim().toLocaleLowerCase('ru-RU'),since=Date.now()/1000-7*86400;
 const visible=projects.filter(p=>(period==='all'||p.updatedAtUnix>=since)&&(!text||`${p.title} ${p.sourceName}`.toLocaleLowerCase('ru-RU').includes(text)));
 return <EditorModal title={localeText("editor", "ui.EditorWorkbenchDialog.c5a99c6f501de104")} onClose={onClose} busy={busy}>
  <div className="editor-project-browser__filters"><Hint message={{id:"project.search"}}><input type="search" aria-label={localeText("editor", "ui.EditorWorkbenchDialog.65c562d3e43664dc")} placeholder={localeText("editor", "ui.EditorWorkbenchDialog.124b2c5bca0e85ad")} value={query} onChange={e=>setQuery(e.target.value)}/></Hint><Hint message={{id:"project.period"}}><select aria-label={localeText("editor", "ui.EditorWorkbenchDialog.b7336cc971a29a90")} value={period} onChange={e=>setPeriod(e.target.value)}><option value="all">{localeText("editor", "ui.EditorWorkbenchDialog.158a380bab6cbc1b")}</option><option value="week">{localeText("editor", "ui.EditorWorkbenchDialog.121a2c30c845f7c7")}</option></select></Hint></div>
  <MeasuredCardList items={visible} itemKey={p=>p.id} maxHeight={420} label={localeText("editor", "ui.EditorWorkbenchDialog.6415426e09a5bd9b")} estimate={70} render={p=><EditorProjectCard project={p} loadPreview={loadPreview} busy={busy} onOpen={()=>void open(p.id)}/>}/>
  {!visible.length?<p>{loading?localeText("editor", "ui.EditorWorkbenchDialog.9a602002b48f8c08"):projects.length?localeText("editor", "ui.EditorWorkbenchDialog.562ebf6a5f361ef4"):localeText("editor", "ui.EditorWorkbenchDialog.4686305a9b614729")}</p>:null}
  <ProjectCleanup onBusyChange={setBusy} selectedPaths={selectedPaths} initialOpen={cleanupInitiallyOpen} onChanged={()=>setRefresh(v=>v+1)}/>
  {error?<p role="alert">{error}</p>:null}
 </EditorModal>;
}
