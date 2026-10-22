import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useState} from 'react';
import {invoke} from "../../shared/tauri/invoke";
import {listen} from '@tauri-apps/api/event';
import {getCurrentWindow} from '@tauri-apps/api/window';
import {editorService} from './EditorService';
import {EditorWindow} from './EditorWindow';
import type {EditorSession} from './EditorContracts';
import './photo-editor.css';
export function EditorBootstrap(){
  useCurrentLocale();
  const [session,setSession]=useState<EditorSession|null>(null),[error,setError]=useState(''),[attempt,setAttempt]=useState(0);
  useEffect(()=>{if(session)return;let disposed=false;let off:(()=>void)|undefined;void listen('smart-photo-indexer:editor-close-requested',()=>void invoke('finish_editor_navigation',{epoch:null,revision:0,action:'close',discardUnsaved:true}),{target:getCurrentWindow().label}).then(f=>disposed?f():off=f);return()=>{disposed=true;off?.();};},[session]);
  useEffect(()=>{let cancelled=false;void editorService.session().then(value=>{if(!cancelled)setSession(value);}).catch(e=>{if(!cancelled)setError(String(e));});return()=>{cancelled=true;};},[attempt]);
  if(session)return <EditorWindow key={session.epoch} initialSession={session} onSessionChange={setSession}/>;
  return <main className="photo-editor-window editor-bootstrap"><strong>{error?localeText("editor", "ui.EditorBootstrap.e9e5e0d8feb06298"):localeText("editor", "ui.EditorBootstrap.075bc46b00bf33a9")}</strong>{error?<><p role="alert">{error}</p><Hint message={{id:"common.retry-read",args:{раздел:localeText("editor", "ui.EditorBootstrap.23851e47ced8dbf9")}}}><button type="button" onClick={()=>{setError('');setAttempt(n=>n+1);}}>{localeText("editor", "ui.EditorBootstrap.c35d829943ed5fab")}</button></Hint><Hint message={{id:"project.close-error"}}><button type="button" onClick={()=>void invoke('finish_editor_navigation',{epoch:null,revision:0,action:'close',discardUnsaved:true})}>{localeText("editor", "ui.EditorBootstrap.4a6e0ab68cab8f2e")}</button></Hint></>:null}</main>;
}
