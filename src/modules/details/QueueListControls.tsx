import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useRef,useState} from 'react';
import {nativeQueueService} from '../taskQueue/NativeQueueService';
import {useAppDispatch,useAppSelector} from '../../app/state/AppStateContext';
import {Icon} from '../../shared/ui/Icon';
export function QueueListControls({hideFinished,onToggle}:{hideFinished:boolean;onToggle():void}){
  useCurrentLocale();
 const dispatch=useAppDispatch();
 const manual=useAppSelector(s=>s.settings.processingLimits.manualQueue);
 const pending=useRef(false),[busy,setBusy]=useState(false);
 const create=async()=>{if(pending.current)return;pending.current=true;setBusy(true);try{const result=await nativeQueueService.createEmpty(crypto.randomUUID());if(result.preferenceError)dispatch({type:'app/setStatusMessage',message:localeText("details", "ui.QueueListControls.dd545ef89e6c2778", {"value1": (result.preferenceError)})});}catch(error){dispatch({type:'app/setStatusMessage',message:localeText("details", "ui.QueueListControls.341abab858794b1f", {"value1": (String(error))}),severity:'error'});}finally{pending.current=false;setBusy(false);}};
 const label=hideFinished?localeText("details", "ui.QueueListControls.2f2819198294ab32"):localeText("details", "ui.QueueListControls.bfe99cdcc4727672");
 return <span className="queue-list-controls">{manual?<Hint message={{id:"queue.new"}}><button data-tour-id="main.queue.new" type="button" className="secondary-action queue-new" aria-label={localeText("details", "ui.QueueListControls.49516e31ae71dd8c")} disabled={busy} aria-busy={busy} onClick={e=>{e.preventDefault();void create();}}><Icon name="plus"/></button></Hint>:null}<Hint message={{id:hideFinished?'queue.show-ready':'queue.hide-ready'}}><button type="button" className="queue-hide-done" aria-label={label} title={localeText("details", "ui.QueueListControls.59d734a4c61e0b2e", {"value1": (label)})} aria-pressed={hideFinished} onClick={onToggle}><Icon name={hideFinished?'eye':'eyeOff'}/><span>{label}</span></button></Hint></span>;
}
