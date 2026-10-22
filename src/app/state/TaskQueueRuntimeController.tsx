import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {loadUiPreferences,saveUiPreferences,manualQueueTargetVersion} from '../../shared/UiPreferences';
import { useEffect, useRef, type Dispatch } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { AppState } from '../types';
import type { AppAction } from './appActions';
import { nativeQueueService } from '../../modules/taskQueue/NativeQueueService';
import { isTauriRuntime } from '../../shared/tauri/runtime';
import type { IndexMetadataUpdateResult } from '../../modules/index/types';
import type { SmartRenameResult } from '../../modules/rename/RenameService';
import type { AppErrorContext, AppErrorIssue } from '../../modules/errors/AppError';
import {invoke} from "../../shared/tauri/invoke";

interface Props { state: AppState; dispatch: Dispatch<AppAction>; reportAppError(error: unknown, context: AppErrorContext): AppErrorIssue }
export function TaskQueueRuntimeController({ state, dispatch }: Props) {
  useCurrentLocale();
  const current=useRef(state);current.current=state;
  const intent=state.taskQueue.pendingPreparation,jobId=state.taskQueue.nativeJobId;
  const started=useRef<string|undefined>(undefined);
  useEffect(()=>{
    if(!isTauriRuntime()||!intent||started.current===intent.key)return;
    started.current=intent.key;
    const {key,tasks,config,targetJobId,targetVersion}=intent;
    const active=()=>current.current.taskQueue.preparationState!=='cancelled'&&current.current.taskQueue.preparationKey===key;
    const prepare=async()=>{if(config.providerMode==='local')await invoke('prepare_ollama_runtime',{endpoint:config.endpoint??null});
      if(!active())throw new Error(localeText("main", "ui.TaskQueueRuntimeController.84fa3cbbda1f617b"));
      return nativeQueueService.prepare(tasks,config,targetJobId);};
    void prepare().then(result=>{
      if(!active()){void nativeQueueService.cancelPreparation(key);return;}
      if(config.queueMode==='manual'&&result.totalCount>0&&targetVersion===manualQueueTargetVersion()&&(loadUiPreferences().queueManualTarget??null)===(targetJobId??null)){try{saveUiPreferences({queueManualTarget:result.jobId});}catch(error){dispatch({type:'app/setStatusMessage',message:localeText("main", "ui.TaskQueueRuntimeController.c9e80473dc790408", {"value1": (String(error))}),severity:'error'});}}
      dispatch({type:'taskQueue/nativePrepared',preparationKey:key,jobId:result.jobId,result});
      window.dispatchEvent(new Event('smart-photo-indexer:processing-wake'));
      if(config.autoStart&&!result.alreadyReady){
        dispatch({type:'taskQueue/nativeCommandPending',jobId:result.jobId,pending:true});
        void nativeQueueService.command(result.jobId,'start',key).then(()=>dispatch({type:'taskQueue/nativeCommandPending',jobId:result.jobId,pending:false}))
          .catch(error=>dispatch({type:'taskQueue/nativeCommandPending',jobId:result.jobId,pending:false,error:String(error)}));
      }
    }).catch(error=>{if(active())dispatch({type:'taskQueue/createBlocked',reason:localeText("main", "ui.TaskQueueRuntimeController.d92ffbebb877e696", {"value1": (String(error))})});});
  },[intent?.key,dispatch]);

  useEffect(()=>{
    if(!isTauriRuntime())return;
    let cancelled=false,pending=false,timer:number|undefined,last='',wakePending=false,active=false;
    let selected:string|undefined,cursor=0,observedMode:string|undefined;
    const tick=async()=>{
      if(cancelled)return;
      if(pending){wakePending=true;return;}pending=true;timer=undefined;
      try{
        const mode=current.current.settings.processingLimits.manualQueue?'manual':'direct';
        if(observedMode!==mode){observedMode=mode;selected=undefined;cursor=0;last='';}
        const requested=mode==='direct'?(current.current.taskQueue.nativeJobId??selected):current.current.taskQueue.nativeJobId;
        if(requested!==selected){selected=requested;cursor=0;last='';}
        const snapshot=await nativeQueueService.snapshot(selected,cursor,Boolean(current.current.taskQueue.hideFinished),mode);
        if(cancelled)return;
        active=snapshot.aggregate?Boolean(snapshot.aggregate.active||snapshot.aggregate.preflight||snapshot.aggregate.otherActive):[...snapshot.jobs,...(snapshot.workingJobs??[])].some(job=>['queued','running'].includes(job.status))||Boolean(snapshot.job&&['queued','running'].includes(snapshot.job.status));
        if(mode!==(current.current.settings.processingLimits.manualQueue?'manual':'direct')||mode==='manual'&&requested!==current.current.taskQueue.nativeJobId){timer=window.setTimeout(()=>void tick(),20);return;}
        if(snapshot.job){selected=snapshot.job.jobId;cursor=snapshot.revision;}
        const summary=JSON.stringify(snapshot);
        if(summary!==last){last=summary;dispatch({type:'taskQueue/nativeObserved',snapshot});}
        if(snapshot.hasMore){timer=window.setTimeout(()=>void tick(),20);return;}
      }catch(error){active=true;if(!cancelled){const id=current.current.taskQueue.nativeJobId;
        if(id)dispatch({type:'taskQueue/nativeCommandPending',jobId:id,pending:false,error:localeText("main", "ui.TaskQueueRuntimeController.e9c7c6cadb7c5c4c", {"value1": (String(error))})});
        else dispatch({type:'app/setStatusMessage',message:localeText("main", "ui.TaskQueueRuntimeController.2569b84f6107432d", {"value1": (String(error))}),severity:'error'});}}
      finally{pending=false;if(wakePending&&!cancelled){wakePending=false;if(timer!==undefined)window.clearTimeout(timer);timer=window.setTimeout(()=>void tick(),0);}}
      if(!cancelled&&active&&timer===undefined)timer=window.setTimeout(()=>void tick(),750);
    };
    const wake=()=>{last='';if(timer!==undefined)window.clearTimeout(timer);void tick();};
    const selectJob=(event:Event)=>{const detail=(event as CustomEvent<string|{jobId:string;queueMode?:'manual'|'direct'}>).detail;if(typeof detail!=='string'&&detail.queueMode)dispatch({type:'settings/setManualQueue',value:detail.queueMode==='manual'});dispatch({type:'taskQueue/selectNativeJob',jobId:typeof detail==='string'?detail:detail.jobId});wake();};
    window.addEventListener('smart-photo-indexer:processing-wake',wake);
    window.addEventListener('smart-photo-indexer:select-processing-job',selectJob);
    const cleanups:Array<()=>void>=[];
    const attach=<T,>(event:string,callback:(payload:T)=>void)=>listen<T>(event,value=>{if(!cancelled)callback(value.payload);}).then(cleanup=>cancelled?cleanup():cleanups.push(cleanup));
    void Promise.all([
      attach<IndexMetadataUpdateResult>('processing-metadata',result=>dispatch({type:'index/metadataUpdateSucceeded',result})),
      attach<Array<{imageId:string;paths?:string[];status:import('../../modules/explorer/types').ProcessingStatus}>>('processing-statuses',changes=>dispatch({type:'index/statusesUpdated',changes})),
      attach<SmartRenameResult>('processing-renamed',result=>dispatch({type:'processing/fileRenamed',result})),
      attach<{indexResult:import('../../modules/index/types').IndexSyncResult}>('smart-photo-indexer:editor-original-replaced',payload=>dispatch({type:'index/syncSucceeded',result:payload.indexResult})),
      attach('processing-changed',wake),
    ]).then(()=>void tick());
    return()=>{cancelled=true;if(timer!==undefined)window.clearTimeout(timer);window.removeEventListener('smart-photo-indexer:processing-wake',wake);window.removeEventListener('smart-photo-indexer:select-processing-job',selectJob);cleanups.forEach(cleanup=>cleanup());};
  },[dispatch]);
   useEffect(()=>{window.dispatchEvent(new Event('smart-photo-indexer:processing-wake'));},[jobId,state.taskQueue.hideFinished,state.settings.processingLimits.manualQueue]);
  return null;
}
