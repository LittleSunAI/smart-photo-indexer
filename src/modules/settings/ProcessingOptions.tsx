import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {EditorErrorDialog} from '../photoEditor/EditorErrorDialog';
import {useState} from 'react';
import {useAppActions,useAppDispatch,useAppSelector} from '../../app/state/AppStateContext';
import {patchProfilePreferences,useProfilePreferences} from './ProfilePreferences';
import {Icon} from '../../shared/ui/Icon';

export function ProcessingOptions({surface='toolbar'}:{surface?:'toolbar'|'settings'}){
  useCurrentLocale();
 const [busy,setBusy]=useState(false),[error,setError]=useState('');
 const actions=useAppActions(),smart=useAppSelector(s=>s.smartRenameEnabled),profile=useProfilePreferences();
 const dispatch=useAppDispatch(),only=useAppSelector(s=>s.settings.processingLimits.onlyUnprocessed!==false);
 async function save(patch:Parameters<typeof patchProfilePreferences>[0]){setBusy(true);setError('');try{await patchProfilePreferences(patch);}catch(e){setError(String(e));}finally{setBusy(false);}}
 return <div className={'processing-options'+(surface==='settings'?' processing-options--settings':'')} role="group" aria-label={localeText("settings", "ui.ProcessingOptions.e4f0095b0be18141")}>
  <div className="processing-options__fields">
   <Hint message={{id:"main.recursive-process"}}><label data-tour-id={(surface==='toolbar'?'main':'settings')+'.processing.recursive'} title={localeText("settings", "ui.ProcessingOptions.7abbb092bcbed122")}><input aria-label={localeText("settings", "ui.ProcessingOptions.5135ddf86612b71f")} type="checkbox" checked={profile.recursiveProcessing} disabled={busy} onChange={e=>void save({recursiveProcessing:e.target.checked})}/><Icon name="folder"/><span>{localeText("settings", "ui.ProcessingOptions.ce5ca269664ed7be")}</span></label></Hint>
   <Hint message={{id:"main.smart-name"}}><label data-tour-id={(surface==='toolbar'?'main':'settings')+'.processing.smart-name'} className="processing-options__naming-start" title={localeText("settings", "ui.ProcessingOptions.ef08496b2eb180c1")}><input aria-label={localeText("settings", "ui.ProcessingOptions.78ebc823a57738df")} type="checkbox" checked={smart} onChange={e=>{actions.setSmartRename(e.target.checked);if(!e.target.checked&&profile.analysisCopy.enabled)void save({analysisCopy:{...profile.analysisCopy,enabled:false}});}}/><Icon name="tag"/><span>{localeText("settings", "ui.ProcessingOptions.aa98d1b463ffcdc0")}</span></label></Hint>
   <Hint message={{id:smart?'main.analysis-copy':'main.copy-disabled'}}><label data-tour-id={(surface==='toolbar'?'main':'settings')+'.processing.copy'} className="processing-options__naming-end" title={localeText("settings", "ui.ProcessingOptions.4ed59aa8230211ee")}><input aria-label={localeText("settings", "ui.ProcessingOptions.c58021bcc65caaa3")} type="checkbox" checked={smart&&profile.analysisCopy.enabled} disabled={busy||!smart} onChange={e=>void save({analysisCopy:{...profile.analysisCopy,enabled:e.target.checked}})}/><Icon name="copy"/><span>{localeText("settings", "ui.ProcessingOptions.dfedf8c9e85b532c")}</span></label></Hint>
   <Hint message={{id:"main.only-missing"}}><label data-tour-id={(surface==='toolbar'?'main':'settings')+'.processing.only-missing'} title={localeText("settings", "ui.ProcessingOptions.229b32e3ef0b5673")}><input aria-label={localeText("settings", "ui.ProcessingOptions.5ee24e1d5c6831f1")} type="checkbox" checked={only} onChange={e=>dispatch({type:'settings/setOnlyUnprocessed',value:e.target.checked})}/><Icon name="filter"/><span>{localeText("settings", "ui.ProcessingOptions.f5292e57df63ffa8")}</span></label></Hint>
  </div>
  {error?<EditorErrorDialog message={error} onClose={()=>setError('')}/>:null}
 </div>;
}
