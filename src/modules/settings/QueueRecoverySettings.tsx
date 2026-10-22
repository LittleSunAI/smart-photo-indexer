import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {HintTitle} from '../../shared/ui/InfoHint';
import {sectionHints} from '../../shared/ui/SectionHints';
import {useEffect,useRef,useState} from 'react';
import {useAppSelector} from '../../app/state/AppStateContext';
import {JobCredentialPanel} from '../details/JobCredentialPanel';
import {UnknownAttemptsPanel} from '../details/UnknownAttemptsPanel';
import {nativeQueueService} from '../taskQueue/NativeQueueService';
import {Icon} from '../../shared/ui/Icon';

export function QueueRecoverySettings(){
  useCurrentLocale();
  const queue=useAppSelector(state=>state.taskQueue);
  const [busy,setBusy]=useState(false),[message,setMessage]=useState('');
  const generation=useRef(0);
  useEffect(()=>{generation.current++;setMessage('');setBusy(false);return()=>{generation.current++;};},[queue.nativeJobId]);
  const cloud=['fal','openrouter'].includes(queue.config.providerMode);
  const reconcile=async()=>{if(!queue.nativeJobId)return;const current=generation.current;setBusy(true);setMessage('');try{const result=await nativeQueueService.reconcile(queue.nativeJobId);if(current===generation.current)setMessage(localeText("settings", "ui.QueueRecoverySettings.eb669c28069d96d6", {"value1": (result.checked), "value2": (result.resolved), "value3": (result.errors.join(' '))}));}catch(e){if(current===generation.current)setMessage(String(e));}finally{if(current===generation.current)setBusy(false);}};
  return <details className="settings-card queue-recovery-settings" data-tour-id="settings.queue.recovery">
    <Hint message={{id:"recovery.section"}}><summary><HintTitle title={localeText("settings", "ui.QueueRecoverySettings.f021a94d33f06410")}  message={{id:"recovery.section"}}/><Icon name="down"/></summary></Hint>
    <p className="settings-muted">{localeText("settings", "ui.QueueRecoverySettings.1e471483ca42befa")}</p>
    {queue.nativeJobId&&cloud?<>
      <JobCredentialPanel jobId={queue.nativeJobId} provider={queue.config.providerMode} status={queue.nativeStatus??''}/>
      {queue.unresolvedRequests?<>
        {queue.config.providerMode==='fal'?<Hint message={{id:"recovery.fal"}}><button type="button" className="secondary-action" disabled={busy||queue.tasks.some(task=>task.status==='running')||['running','queued','prepared'].includes(queue.nativeStatus??'')} onClick={()=>void reconcile()}>{localeText("settings", "ui.QueueRecoverySettings.5d9a19aef6b371a7")}</button></Hint>:null}
        <UnknownAttemptsPanel jobId={queue.nativeJobId} status={queue.nativeStatus??''} count={queue.unresolvedRequests}/>
      </>:null}
    </>:<p className="settings-muted">{localeText("settings", "ui.QueueRecoverySettings.752303bfdb718e42")}</p>}
    {message?<p role="status">{message}</p>:null}
  </details>;
}
