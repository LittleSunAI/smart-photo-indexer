import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useTourDisclosure} from '../help/HelpPresentation';
import {useEffect,useRef,useState} from 'react';
import {invoke} from "../../shared/tauri/invoke";
import {Hint} from '../../shared/help/Hint';
import {HintTitle} from '../../shared/ui/InfoHint';
import {Icon} from '../../shared/ui/Icon';

interface BackupInfo{fileCount?:number;imageCount?:number;pathCount?:number}
interface RestoreStatus{pending:boolean;error:string|null}
interface Preview{path:string;kind:'full'|'index';info:BackupInfo}
export function IndexBackupPanel({ready}:{ready:boolean}){
  useCurrentLocale();
 const disclosure=useTourDisclosure('program-data-backup');
 const [busy,setBusy]=useState(false),[message,setMessage]=useState(''),[error,setError]=useState(''),[status,setStatus]=useState<RestoreStatus>({pending:false,error:null}),[preview,setPreview]=useState<Preview|null>(null);
 const alive=useRef(false),pending=useRef(false),statusGeneration=useRef(0);
 useEffect(()=>{alive.current=true;const read=()=>{const generation=++statusGeneration.current;void invoke<RestoreStatus>('get_index_restore_status').then(value=>{if(alive.current&&generation===statusGeneration.current)setStatus(value);}).catch(error=>{if(alive.current&&generation===statusGeneration.current)setError(String(error));});};read();window.addEventListener('smart-photo-indexer:backup-staged',read);return()=>{alive.current=false;statusGeneration.current++;window.removeEventListener('smart-photo-indexer:backup-staged',read);};},[]);
 const run=async(action:()=>Promise<void>)=>{if(pending.current)return;pending.current=true;setBusy(true);setError('');setMessage('');try{await action();}catch(error){if(alive.current)setError(String(error));}finally{pending.current=false;if(alive.current)setBusy(false);}};
 const pick=async()=>{
  setPreview(null);const path=await invoke<string|null>('choose_index_backup_path',{restore:true,includeFull:true});if(!path||!alive.current)return;
  const kind=path.toLowerCase().endsWith('.json')?'full':path.toLowerCase().endsWith('.sqlite')?'index':null;if(!kind)throw Error(localeText("details", "ui.IndexBackupPanel.5b0328b4b98eef2d"));
  const info=kind==='full'?await invoke<BackupInfo>('editor_full_backup',{action:'inspect',path}):await invoke<BackupInfo>('inspect_index_backup',{sourcePath:path});
  if(alive.current)setPreview({path,kind,info});
 };
 const stage=async()=>{if(!preview)return;statusGeneration.current++;if(preview.kind==='full')await invoke('editor_full_backup',{action:'stage',path:preview.path});else await invoke('stage_index_restore',{sourcePath:preview.path});if(alive.current){setPreview(null);setStatus({pending:true,error:null});setMessage(localeText("details", "ui.IndexBackupPanel.1d455d93c1325c5e"));}window.dispatchEvent(new Event('smart-photo-indexer:backup-staged'));};
 return <details {...disclosure} data-tour-id="main.library.backup" className="details-collapsible-card program-data-backup" data-help-id="program-data-backup">
  <Hint message={{id:'library.backup-full'}}><summary><HintTitle title={localeText("details", "ui.IndexBackupPanel.2576458786f1eb56")} message={{id:'library.backup-full'}}/><Icon name="down"/></summary></Hint>
  <div className="details-collapsible-card__body">
   <p className="metadata-edit-note">{localeText("details", "ui.IndexBackupPanel.1c8edb9cc956d0ab")}</p>
   <div className="program-data-backup__actions"><Hint message={{id:'library.backup-full-save'}}><button type="button" className="secondary-action" disabled={!ready||busy} onClick={()=>void run(async()=>{const result=await invoke<{path:string;fileCount:number}|null>('editor_full_backup',{action:'export'});if(result&&alive.current)setMessage(localeText("details", "ui.IndexBackupPanel.a954d2524a95f10e", {"value1": (result.fileCount), "value2": (result.path)}));})}>{localeText("details", "ui.IndexBackupPanel.40c9d642a5205e9b")}</button></Hint><Hint message={{id:'library.backup-full-pick'}}><button type="button" className="secondary-action" disabled={!ready||busy||status.pending} onClick={()=>void run(pick)}>{localeText("details", "ui.IndexBackupPanel.98c04ffbe380b0ff")}</button></Hint></div>
   {preview&&!status.pending?<div className="index-database-card program-data-backup__preview">
    <span className="editor-copyable" title={preview.path}>{preview.path}</span>
    <strong>{preview.kind==='full'?localeText("details", "ui.IndexBackupPanel.b1d7ca27f9087987", {"value1": (preview.info.fileCount)}):localeText("details", "ui.IndexBackupPanel.ec943596e13f1e31", {"value1": (preview.info.imageCount), "value2": (preview.info.pathCount)})}</strong>
    <p>{preview.kind==='full'?localeText("details", "ui.IndexBackupPanel.b3c6999a8f80a829"):localeText("details", "ui.IndexBackupPanel.56ad28fcf83bf2e7")} {' '}{localeText("details", "ui.IndexBackupPanel.c8a372b97ca0f8b9")}</p>
    <div className="program-data-backup__actions"><Hint message={{id:preview.kind==='full'?'library.backup-full-stage':'library.restore-stage'}}><button type="button" className="secondary-action" disabled={busy} onClick={()=>void run(stage)}>{localeText("details", "ui.IndexBackupPanel.8fe1799c5534c1ce")}</button></Hint><Hint message={{id:'library.restore-cancel-pick'}}><button type="button" className="secondary-action" disabled={busy} onClick={()=>setPreview(null)}>{localeText("details", "ui.IndexBackupPanel.6bfb220540c962db")}</button></Hint></div>
   </div>:null}
   {status.pending?<div><p>{localeText("details", "ui.IndexBackupPanel.0da49d9bd58f9fe3")}</p><Hint message={{id:'library.restore-cancel'}}><button type="button" className="secondary-action" disabled={busy} onClick={()=>void run(async()=>{statusGeneration.current++;await invoke('cancel_index_restore');if(alive.current){setStatus({pending:false,error:null});setMessage(localeText("details", "ui.IndexBackupPanel.582015c49e3077bc"));}})}>{localeText("details", "ui.IndexBackupPanel.63679a96e2fcac50")}</button></Hint></div>:null}
   {busy?<p role="status">{localeText("details", "ui.IndexBackupPanel.30c7a9b12aeb960b")}</p>:null}
   {message?<p role="status" className="editor-copyable">{message}</p>:null}
   {error||status.error?<p role="alert" className="danger">{error||status.error}</p>:null}
  </div>
 </details>;
}
