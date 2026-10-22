import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,useState} from 'react';
import {useAppDispatch,useAppSelector} from '../../app/state/AppStateContext';
import {nativeQueueService} from '../taskQueue/NativeQueueService';

export function CloudConcurrencyField(){
  useCurrentLocale();
  const dispatch=useAppDispatch();
  const value=useAppSelector(s=>s.settings.processingLimits.cloudConcurrency);
  const [draft,setDraft]=useState(String(value)),[error,setError]=useState(''),[busy,setBusy]=useState(false);
  const submitting=useRef(false);
  useEffect(()=>setDraft(String(value)),[value]);
  const commit=async()=>{
    if(submitting.current)return;
    const requested=Number(draft);
    if(!draft.trim()||!Number.isInteger(requested)||requested<1||requested>100){setError(localeText("settings", "ui.CloudConcurrencyField.810e3cb93bfc3a79"));return;}
    submitting.current=true;setBusy(true);setError('');
    try{
      // Resolve the active native job independently of the history currently
      // displayed in Tasks. Never change a selected completed/local history job.
      const job=(await nativeQueueService.snapshot()).job;
      if(job&&job.config.providerMode!=='local'&&['prepared','queued','running','paused','interrupted'].includes(job.status)){
        if((job.config.requestedConcurrency??job.config.concurrency)!==requested)await nativeQueueService.setConcurrency(job.jobId,requested);
      }
      dispatch({type:'settings/setAnalysisLimit',name:'cloudConcurrency',value:requested});
    }catch(e){setError(localeText("settings", "ui.CloudConcurrencyField.451739673b3470de", {"value1": (String(e))}));}finally{submitting.current=false;setBusy(false);}
  };
  return <><Hint message={{id:"settings.concurrency"}}><label className="settings-field"><span>{localeText("settings", "ui.CloudConcurrencyField.a88cccffa25018a5")}</span>
    <input aria-label={localeText("settings", "ui.CloudConcurrencyField.ed6777c1d8399cf2")} title={localeText("settings", "ui.CloudConcurrencyField.ac1e5a57c7320c8a")} type="number" min={1} max={100} value={draft} disabled={busy} onChange={e=>setDraft(e.target.value)} onBlur={()=>void commit()} onKeyDown={e=>{if(e.key==='Enter'){e.preventDefault();void commit();}}}/>
  </label></Hint>{error?<small role="alert" className="danger settings-field-error">{error}</small>:null}</>;
}
