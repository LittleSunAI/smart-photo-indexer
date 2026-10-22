import {localeTag as displayLocaleTag} from "../../shared/i18n/Formatting";
import {commandErrorCode,commandErrorMessage} from '../../shared/tauri/CommandError';
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {helpPresentation,useHelpPresentation} from '../help/HelpPresentation';
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,useState} from 'react';
import {invoke} from "../../shared/tauri/invoke";
import {useAppActions,useAppSelector} from '../../app/state/AppStateContext';
import {loadUiPreferences,saveUiPreferences,UI_PREFERENCES_EVENT} from '../../shared/UiPreferences';
import {Icon} from '../../shared/ui/Icon';
import {nativeQueueService,type NativeJob} from '../taskQueue/NativeQueueService';
import type {QueueTask} from '../taskQueue/types';
import {QueueTaskList} from './QueueTaskList';
import {QueueRemoveButton} from './QueueRemoveButton';

const PAGE=20;
const states:Record<string,string>={get prepared() { return localeText("details", "ui.QueueCapsules.38a71d64621ee369"); },get queued() { return localeText("details", "ui.QueueCapsules.1346ddcd535321b8"); },get running() { return localeText("details", "ui.QueueCapsules.1c5cccd40f95ce6b"); },get paused() { return localeText("details", "ui.QueueCapsules.8373a8c071251b6f"); },get interrupted() { return localeText("details", "ui.QueueCapsules.68cb6ec364a91f0d"); },get waitingService() { return localeText("details", "ui.QueueCapsules.f5e51946d4f7437f"); },get completed() { return localeText("details", "ui.QueueCapsules.87996812d86aaac1"); },get stopped() { return localeText("details", "ui.QueueCapsules.052d3187c3b9bb87"); },get preparationFailed() { return localeText("details", "ui.QueueCapsules.e012d37ae7d19c12"); }};
const title=(job:NativeJob)=>job.queueMode==='direct'?localeText("details", "ui.QueueCapsules.516c92e67371b6c0"):localeText("details", "ui.QueueCapsules.407ba58b30266c2e", {"value1": (new Date(job.createdAtUnix*1000).toLocaleString(displayLocaleTag(),{day:'numeric',month:'short',hour:'2-digit',minute:'2-digit'}))});

export function QueueCapsules(){
 const manual=useAppSelector(s=>s.settings.processingLimits.manualQueue);
 return manual?<ManualQueueCapsules/>:<DirectQueueList/>;
}
function DirectQueueList(){
 useCurrentLocale();const queue=useAppSelector(s=>s.taskQueue);
 const known=queue.nativeWorkingJobs?.find(job=>job.queueMode==='direct');
 const [job,setJob]=useState<NativeJob|null>(known??null),[busy,setBusy]=useState(!known),[error,setError]=useState('');
 useEffect(()=>{let alive=true;if(known){setJob(known);setBusy(false);setError('');return;}
  setBusy(true);void nativeQueueService.snapshot(undefined,0,false,'direct').then(snapshot=>{if(alive){setJob(snapshot.job);setError('');}}).catch(e=>{if(alive)setError(commandErrorMessage(e));}).finally(()=>{if(alive)setBusy(false);});return()=>{alive=false;};
 },[known,queue.nativeWorkingJobs]);
 return <div data-help-id="queue-capsules" data-testid="direct-photo-list">{job?<QueueCapsule flat job={job} target={false} onChoose={()=>{}} onView={()=>{}} hide={Boolean(queue.hideFinished)}/>:busy?<p role="status">{localeText('details','ui.QueueCapsules.ef1ea944d0b52396')}</p>:<p className="task-list-empty">{localeText('details','queue.directEmpty')}</p>}{error?<p role="alert">{error}</p>:null}</div>;
}

function ManualQueueCapsules(){
  useCurrentLocale();
 const queue=useAppSelector(s=>s.taskQueue),actions=useAppActions();
 const jobs=(queue.nativeWorkingJobs??[]).filter(job=>job.queueMode!=='direct'),hide=Boolean(queue.hideFinished);
 const [target,setTarget]=useState(()=>loadUiPreferences().queueManualTarget??null);
 const [page,setPage]=useState(0),[older,setOlder]=useState<NativeJob[]>([]),[busy,setBusy]=useState(false),[error,setError]=useState(''),[reload,setReload]=useState(0);
 const request=useRef(0);
 useEffect(()=>{const update=()=>setTarget(loadUiPreferences().queueManualTarget??null);window.addEventListener(UI_PREFERENCES_EVENT,update);window.addEventListener('storage',update);return()=>{window.removeEventListener(UI_PREFERENCES_EVENT,update);window.removeEventListener('storage',update);};},[]);
 useEffect(()=>{setPage(0);},[hide]);
 // The shared queue observer supplies revisions. There is no per-card polling.
 useEffect(()=>{
  const generation=++request.current;if(page===0){setOlder([]);setBusy(false);return;}
  setBusy(true);setError('');void invoke<NativeJob[]>('get_processing_jobs',{offset:page*100,workingOnly:true,hideFinished:hide,queueMode:'manual'}).then(rows=>{if(generation!==request.current)return;if(!rows.length){setPage(p=>Math.max(0,p-1));return;}setOlder(rows);}).catch(e=>{if(generation===request.current)setError(commandErrorMessage(e));}).finally(()=>{if(generation===request.current)setBusy(false);});return()=>{request.current++;};
 },[page,hide,queue.nativeWorkingJobs,reload]);
 const choose=(id:string|null)=>{try{saveUiPreferences({queueManualTarget:id});setError('');}catch(e){setError(localeText("details", "ui.QueueCapsules.05ca41f36283ffbb", {"value1": (commandErrorMessage(e))}));}};
 useEffect(()=>{
  if(!target)return;let alive=true;
  // Validate the saved ID directly; it may be beyond the first registry page.
  void nativeQueueService.snapshot(target).then(snapshot=>{if(!alive||helpPresentation().open)return;const job=snapshot.job;if(!job||job.workVisible===false||job.queueMode!=='manual'||job.status==='preparing'){choose(null);return;}}).catch(e=>{if(alive){if(!helpPresentation().open&&commandErrorCode(e)==='queue.target-missing')choose(null);else setError(localeText("details", "ui.QueueCapsules.f3d76cefcdce1281", {"value1": (commandErrorMessage(e))}));}});return()=>{alive=false;};
 },[target,reload]);
 const rows=page===0?jobs:older;
 return <div className="queue-capsules" data-help-id="queue-capsules">
  {rows.length===0&&!busy?<p className="task-list-empty">{hide?localeText("details", "ui.QueueCapsules.31a45b29cfba1f43"):localeText("details", "ui.QueueCapsules.7b00174d0e8cd329")}</p>:null}
  {rows.map(job=><QueueCapsule key={job.jobId} job={job} target={target===job.jobId} onChoose={()=>choose(job.jobId)} hide={hide} accepted={queue.preparationResult?.jobId===job.jobId?queue.preparationResult:undefined} onView={()=>actions.selectNativeJob(job.jobId)}/>)}
  {busy?<p role="status">{localeText("details", "ui.QueueCapsules.eab96112536ca9cf")}</p>:null}
  {error?<p role="alert" className="queue-error">{error}<Hint message={{id:"common.retry-read",args:{раздел:localeText("details", "ui.QueueCapsules.974e703f36ebc219")}}}><button type="button" onClick={()=>setReload(v=>v+1)}>{localeText("details", "ui.QueueCapsules.85fe8c69934dc354")}</button></Hint></p>:null}
  {page>0||rows.length===100?<nav className="details-page-controls" aria-label={localeText("details", "ui.QueueCapsules.c3f12ccbbb01ae87")}><Hint message={{id:"common.page-prev",args:{список:localeText("details", "ui.QueueCapsules.334c424e57285610")}}}><button type="button" disabled={busy||page===0} onClick={()=>setPage(p=>p-1)}>{localeText("details", "ui.QueueCapsules.419c86e9677893c9")}</button></Hint><span>{localeText("details", "ui.QueueCapsules.aa4afd61c3fcc571")}{' '}{page+1}</span><Hint message={{id:"common.page-next",args:{список:localeText("details", "ui.QueueCapsules.334c424e57285610")}}}><button type="button" disabled={busy||rows.length<100} onClick={()=>setPage(p=>p+1)}>{localeText("details", "ui.QueueCapsules.7a924e60a42ad3c3")}</button></Hint></nav>:null}
 </div>;
}

interface PageTask{imageId?:string;id:string;path:string;status:string;completedPasses:number;metadataCommitted:boolean;error?:string|null;result?:{description?:string;tags?:string[]}|null}
function QueueCapsule({job,target,onChoose,onView,hide,accepted,flat=false}:{job:NativeJob;target:boolean;onChoose():void;onView():void;hide:boolean;accepted?:unknown;flat?:boolean}){
  useCurrentLocale();
 const help=useHelpPresentation(),viewed=useAppSelector(state=>state.taskQueue.nativeJobId===job.jobId);
 const [open,setOpen]=useState(target&&(job.total??0)===0),[page,setPage]=useState(0),[rows,setRows]=useState<QueueTask[]>([]),[more,setMore]=useState(false),[busy,setBusy]=useState(false),[commandBusy,setCommandBusy]=useState(false),[error,setError]=useState(''),[reload,setReload]=useState(0),actions=useAppActions();
 const expanded=flat||open||Boolean(help.open&&help.stepId==='main.queue.photo'&&viewed);
 const request=useRef(0);
 useEffect(()=>{if(accepted)setOpen(true);},[accepted]);
 useEffect(()=>{if(target&&(job.total??0)===0)setOpen(true);},[target,job.total]);
 useEffect(()=>setPage(0),[hide]);
 useEffect(()=>{
  if(!expanded)return;const generation=++request.current;setBusy(true);setError('');
  void invoke<PageTask[]>('get_processing_tasks',{jobId:job.jobId,offset:page*PAGE,limit:PAGE+1,includeExcluded:false,hideFinished:hide}).then(items=>{
   if(generation!==request.current)return;if(!items.length&&page>0){setPage(p=>p-1);return;}
   setMore(items.length>PAGE);setRows(items.slice(0,PAGE).map(item=>({id:item.id,indexedImageId:item.imageId,type:'aiAnalysis',targetPath:item.path,fileName:item.path.split(/[\\/]/).at(-1)??item.path,sourceItemId:null,status:item.status==='succeeded'?'done':item.status==='failed'?'failed':item.status==='cancelled'?'cancelled':item.status==='skipped'?'skipped':['preparing','running','committing'].includes(item.status)?'running':'pending',nativeStage:item.status,progress:item.status==='succeeded'?100:item.metadataCommitted?100:Math.round(item.completedPasses/Math.max(1,job.config.passes??1)*90),completedPasses:item.completedPasses,createdAt:new Date(job.createdAtUnix*1000).toISOString(),attemptCount:0,maxAttempts:1,estimatedCostUsd:0,errorMessage:item.error??undefined,analysisTags:item.result?.tags})));
  }).catch(e=>{if(generation===request.current)setError(commandErrorMessage(e));}).finally(()=>{if(generation===request.current)setBusy(false);});return()=>{request.current++;};
 },[expanded,page,hide,job.jobId,job.revision,job.status,reload]);
 const count=job.counters?.total??job.total??0,done=job.counters?.done??job.done??0;
 const startable=count>0&&['prepared','paused','interrupted','waitingService'].includes(job.status);
 const start=async()=>{setCommandBusy(true);setError('');try{await actions.commandQueueJob(job.jobId,job.status==='prepared'?'start':'resume');}catch(e){setError(commandErrorMessage(e));}finally{setCommandBusy(false);}};
 return <section data-tour-id={flat?"main.queue.photos":"main.queue.card-actions"} data-tour-current={viewed?'true':undefined} className={flat?'direct-queue':`queue-capsule${target?' is-target':''}`} data-job-id={job.jobId} aria-label={title(job)}>
  {!flat?<div className="queue-capsule__heading">
   {job.queueMode!=='direct'?<Hint message={{id:target?'queue.target-active':'queue.target'}}><button data-tour-id="main.queue.target" data-tour-current={target?'true':undefined} type="button" role="radio" aria-checked={target} aria-label={localeText("details", "ui.QueueCapsules.4557a7d95b2692c5", {"value1": (title(job))})} title={localeText("details", "ui.QueueCapsules.ebda6bcffbdfb634")} className="queue-capsule__target" onClick={onChoose}>{target?'●':'○'}</button></Hint>:null}
   <Hint message={{id:"queue.capsule-open"}}><button type="button" className="queue-capsule__disclosure" aria-label={localeText("details", "ui.QueueCapsules.b202b85e68d0b3c0", {"value1": (expanded?localeText("details", "ui.QueueCapsules.84d63302bd908e73"):localeText("details", "ui.QueueCapsules.de2b7d987fbc9c15")), "value2": (title(job))})} aria-expanded={expanded} onClick={()=>setOpen(v=>!v)}><Icon name={expanded?'down':'right'}/></button></Hint>
   <Hint message={{id:"queue.capsule-view"}}><button type="button" className="queue-capsule__label" title={localeText("details", "ui.QueueCapsules.1dd7a71ceb8d93f2")} onClick={onView}><strong>{title(job)}</strong><small>{count} {' '}{localeText("details", "ui.QueueCapsules.ab9224bc90be2b76")}{' '}{done} {' '}{localeText("details", "ui.QueueCapsules.34d60e00b4528b36")}{' '}{states[job.status]??job.status}</small></button></Hint>
   <Hint message={{id:"queue.capsule-start"}}><button type="button" className="secondary-action queue-capsule__start" aria-label={localeText("details", "ui.QueueCapsules.b202b85e68d0b3c0", {"value1": (job.status==='prepared'?localeText("details", "ui.QueueCapsules.74ce0c4bb4a397d7"):localeText("details", "ui.QueueCapsules.d18f448c51f6e105")), "value2": (title(job))})} title={startable?job.status==='prepared'?localeText("details", "ui.QueueCapsules.d18c2d38ca9ba876"):localeText("details", "ui.QueueCapsules.bf105fc6f5f657e4"):states[job.status]??job.status} disabled={commandBusy||!startable} onClick={()=>void start()}><Icon name="play"/></button></Hint>
   <QueueRemoveButton scope="job" jobId={job.jobId}/>
  </div>:null}
  {expanded?<div className="queue-capsule__body" aria-busy={busy}><QueueTaskList tasks={rows} jobId={job.jobId} identity={`${job.jobId}:${page}:${hide}`}/>{busy?<span role="status" className="queue-capsule__loading">{localeText("details", "ui.QueueCapsules.ef1ea944d0b52396")}</span>:null}{page>0||more?<nav className="details-page-controls" aria-label={localeText("details", "ui.QueueCapsules.04fd13262e9bdf15", {"value1": (job.jobId)})}><Hint message={{id:"common.page-prev",args:{список:localeText("details", "ui.QueueCapsules.b9528de4f0e1fd24")}}}><button type="button" disabled={busy||page===0} onClick={()=>setPage(p=>p-1)}>{localeText("details", "ui.QueueCapsules.80ad3fcce40e1b40")}</button></Hint><span>{localeText("details", "ui.QueueCapsules.5c30d7a8c592b67e")}{' '}{page+1}</span><Hint message={{id:"common.page-next",args:{список:localeText("details", "ui.QueueCapsules.b9528de4f0e1fd24")}}}><button type="button" disabled={busy||!more} onClick={()=>setPage(p=>p+1)}>{localeText("details", "ui.QueueCapsules.12358f85bcd42040")}</button></Hint></nav>:null}</div>:null}
  {error?<p role="alert" className="queue-error">{error}<Hint message={{id:"common.retry-read",args:{раздел:localeText("details", "ui.QueueCapsules.a5687a80d21fb1a9")}}}><button type="button" onClick={()=>setReload(v=>v+1)}>{localeText("details", "ui.QueueCapsules.20254d0c885f7be8")}</button></Hint></p>:null}
 </section>;
}
