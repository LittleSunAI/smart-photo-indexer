import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useEffect,useRef,useState} from 'react';
import {EditorModal} from '../photoEditor/EditorModal';
import {Hint} from '../../shared/help/Hint';
import {formatBytes,formatUnixTime} from '../../shared/utils/fileFormatting';
import type {FileConflict,FileInfo} from './FileActions';
import './file-actions.css';

export function FileConflictDialog({conflict,busy,onDecision,onCancel}:{conflict:FileConflict;busy:boolean;onDecision(choice:'replace'|'keepBoth'|'skip',all:boolean):void;onCancel():void}){
  useCurrentLocale();
 const [all,setAll]=useState(false),safe=useRef<HTMLButtonElement>(null);
 useEffect(()=>safe.current?.focus(),[]);
 const grid=document.querySelector<HTMLElement>('.explorer-grid');
 return <EditorModal title={localeText("main", "ui.FileConflictDialog.a172ce5e5f40b2b5")} onClose={onCancel} busy={busy} className="file-conflict-dialog" returnFocus={grid}>
  <p>{conflict.requiresFreshDecision?localeText("main", "ui.FileConflictDialog.8ddb79328805ac98"):localeText("main", "ui.FileConflictDialog.10d29460a99c27d1")}</p>
  <FileBlock label={localeText("main", "ui.FileConflictDialog.627af276ee740808")} info={conflict.source}/>
  <FileBlock label={localeText("main", "ui.FileConflictDialog.231364c936a6d5a2")} info={conflict.target}/>
  {!conflict.replaceAllowed?<p className="file-conflict-note">{localeText("main", "ui.FileConflictDialog.a1274d6802be3aac")}</p>:null}
  <Hint message={{id:'files.apply-all'}}><label className="editor-inline-field file-conflict-all"><input type="checkbox" checked={all} disabled={busy} onChange={event=>setAll(event.target.checked)}/>{localeText("main", "ui.FileConflictDialog.10b03899ef45fa67")}</label></Hint>
  <footer><Hint message={{id:'files.cancel-remaining'}}><button type="button" disabled={busy} onClick={onCancel}>{localeText("main", "ui.FileConflictDialog.0461a2cf97c80d81")}</button></Hint>
   <Hint message={{id:conflict.replaceAllowed?'files.replace':'files.replace-source-disabled'}}><button type="button" disabled={busy||!conflict.replaceAllowed} onClick={()=>onDecision('replace',all)}>{localeText("main", "ui.FileConflictDialog.8aea640890080581")}</button></Hint>
   <Hint message={{id:'files.keep-both'}}><button ref={safe} className="editor-primary" type="button" disabled={busy} onClick={()=>onDecision('keepBoth',all)}>{localeText("main", "ui.FileConflictDialog.8098a004275e620b")}</button></Hint>
   <Hint message={{id:'files.skip'}}><button type="button" disabled={busy} onClick={()=>onDecision('skip',all)}>{localeText("main", "ui.FileConflictDialog.353c9c8a6539f1ba")}</button></Hint>
  </footer>
 </EditorModal>;
}
function FileBlock({label,info}:{label:string;info:FileInfo}){
  useCurrentLocale();return <section className="file-conflict-file"><small>{label}</small><strong>{info.name}</strong><span className="editor-copyable">{info.path}</span><small>{formatBytes(info.sizeBytes)} · {info.modifiedAt?formatUnixTime(info.modifiedAt):localeText("main", "ui.FileConflictDialog.1937c9d2144c9aee")}</small></section>;}
