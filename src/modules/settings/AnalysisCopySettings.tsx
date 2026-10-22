import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useState} from 'react';
import {fileSystemService} from '../filesystem/FileSystemService';
import {patchProfilePreferences,useProfilePreferences} from './ProfilePreferences';

export function AnalysisCopySettings(){
  useCurrentLocale();
 const profile=useProfilePreferences(),copy=profile.analysisCopy,[busy,setBusy]=useState(false),[error,setError]=useState('');
 async function change(destination:'beside'|'folder'){
  setBusy(true);setError('');try{let folder=copy.folder;if(destination==='folder'){const selected=await fileSystemService.chooseFolder(folder||null);if(!selected)return;folder=selected;}await patchProfilePreferences({analysisCopy:{...copy,destination,folder}});}catch(e){setError(String(e));}finally{setBusy(false);}
 }
 return <Hint message={{id:"settings.copy-destination"}}><label className="settings-field" data-tour-id="settings.storage.analysis-copy"><span>{localeText("settings", "ui.AnalysisCopySettings.d59c8685242181dd")}</span><select aria-label={localeText("settings", "ui.AnalysisCopySettings.44a1c65d6ebdb84f")} disabled={busy} value={copy.destination} onChange={e=>void change(e.target.value as 'beside'|'folder')}><option value="beside">{localeText("settings", "ui.AnalysisCopySettings.dde4bcc9c7486af9")}</option><option value="folder">{localeText("settings", "ui.AnalysisCopySettings.c95d58568ebb5a44")}</option></select>{copy.destination==='folder'?<span className="storage-path-row"><span className="storage-path" title={copy.folder}>{copy.folder||localeText("settings", "ui.AnalysisCopySettings.3520c088a1703051")}</span><Hint message={{id:"settings.copy-choose"}}><button type="button" disabled={busy} onClick={()=>void change('folder')}>{localeText("settings", "ui.AnalysisCopySettings.23548111f31c9c94")}</button></Hint></span>:null}{error?<span role="alert">{error}</span>:null}</label></Hint>;
}
