import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useState} from 'react';
import {useAppActions,useAppDispatch,useAppSelector} from '../../app/state/AppStateContext';
import {nativeQueueService} from '../taskQueue/NativeQueueService';
import {Icon} from '../../shared/ui/Icon';
export function QueueRemoveButton({scope,jobId,taskId,fileName,disabled=false,queueMode:explicitMode}:{scope:'photo'|'job'|'allWorking';jobId?:string;taskId?:string;fileName?:string;disabled?:boolean;queueMode?:'direct'|'manual'}){
  useCurrentLocale();
 const [busy,setBusy]=useState(false),actions=useAppActions(),dispatch=useAppDispatch();
 const currentMode=useAppSelector(s=>s.settings.processingLimits.manualQueue?'manual':'direct'),queueMode=explicitMode??currentMode;
 const preparing=useAppSelector(s=>s.taskQueue.preparationState==='preparing'&&s.taskQueue.pendingPreparation?.config.queueMode===(s.settings.processingLimits.manualQueue?'manual':'direct')?s.taskQueue.preparationKey:undefined);
 const label=scope==='allWorking'?localeText("details", "ui.QueueRemoveButton.de003171a2a4477d"):scope==='job'?localeText("details", "ui.QueueRemoveButton.4257ca7c0f24031c"):localeText("details", "ui.QueueRemoveButton.fc8fe15fd8da1381", {"value1": (fileName??'')});
 return <Hint message={{id:scope==='photo'?'queue.remove-photo':scope==='job'?'queue.remove-job':'queue.remove-all'}}><button type="button" className={`secondary-action queue-remove queue-remove--${scope}`} data-queue-remove={scope} aria-label={label} title={label} disabled={disabled||busy||scope!=='allWorking'&&!jobId} onClick={async e=>{
  e.preventDefault();e.stopPropagation();setBusy(true);
  try{const result=await nativeQueueService.remove(scope,jobId,taskId,queueMode);if(scope==='allWorking'&&preparing){await nativeQueueService.cancelPreparation(preparing);dispatch({type:'taskQueue/preparationCancelled',preparationKey:preparing});}actions.setStatusMessage(localeText("details", "ui.QueueRemoveButton.fe01e0d90c92cc78", {"value1": (result.removedCount), "value2": (result.preferenceError?localeText("details", "ui.QueueRemoveButton.6b0d69a76aa1cb9b", {"value1": (result.preferenceError)}):'')}));}
  catch(error){actions.setStatusMessage(localeText("details", "ui.QueueRemoveButton.e0f2664312cc4003", {"value1": (String(error))}), 'error');}finally{setBusy(false);}
 }}><Icon name="trash"/></button></Hint>;
}
