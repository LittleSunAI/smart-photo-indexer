import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useEffect,useRef,useState} from 'react';
import {useAppState,useAppDispatch} from '../../app/state/AppStateContext';
import {isTauriRuntime} from '../../shared/tauri/runtime';
import {fileActionsService,useFileActions} from './FileActionsService';
import {fileCommandEvent,selectedFileSource,pasteFolder,operationSummary,type FileIntent,type FileOutcome} from './FileActions';
import {fileSystemService} from './FileSystemService';
import {indexService} from '../index/IndexService';
import {appErrorService} from '../errors/AppError';
import {appLogService} from '../logs/AppLogService';
import type {ImageItem} from '../explorer/types';
import {FileConflictDialog} from './FileConflictDialog';

export function FileActionsHost(){
  useCurrentLocale();
 const state=useAppState(),dispatch=useAppDispatch(),latest=useRef(state);latest.current=state;
 const {operation}=useFileActions(),[decisionBusy,setDecisionBusy]=useState(false);
 const error=(value:unknown)=>{const message=String(value);dispatch({type:'app/setStatusMessage',message});dispatch({type:'appError/report',issue:appErrorService.createIssue(value,{domain:'filesystem',operation:localeText("main", "ui.FileActionsHost.fbc196ddad50ac76")})});appLogService.error('ui',localeText("main", "ui.FileActionsHost.5254c54e8ef4b080"),message);};
 const fail=useRef(error);fail.current=error;
 useEffect(()=>{
  if(!isTauriRuntime())return;
  let disposed=false,stop:(()=>void)|undefined;
  const applied=async(rows:FileOutcome[])=>{
   const paths=[...new Set(rows.filter(row=>row.filesystemApplied&&row.newPath).map(row=>row.newPath!))];
   let images:ImageItem[]=[];
   // UI reads actual files. A failed metadata commit is never retried as a file mutation.
   if(paths.length){try{
    images=(await fileSystemService.inspectDroppedPaths(paths)).items.filter((item):item is ImageItem=>item.kind==='image');
    const indexedPaths=new Set(rows.filter(row=>row.indexApplied).map(row=>row.newPath));
    const indexed=images.filter(image=>indexedPaths.has(image.path));
    if(indexed.length){const sync=await indexService.syncScannedImages(indexed,true),metadata=new Map(sync.indexedImages.map(image=>[image.path,image]));
     images=images.map(image=>{const value=metadata.get(image.path);return value?{...image,...value,id:image.id,kind:'image',name:value.fileName,indexedImageId:value.imageId,description:value.description??'',sourceRevision:value.sourceRevision??image.sourceRevision}:image;});}
   }catch(value){fail.current(localeText("main", "ui.FileActionsHost.c576945b0ac154a8", {"value1": (String(value))}));}}
   if(!disposed)dispatch({type:'filesystem/actionsApplied',outcomes:rows,images});
   for(const row of rows)if(row.message){appLogService.info('ui',`${row.oldPath}: ${row.message}`);if(row.status==='failed'||row.status==='partial'||row.filesystemApplied&&!row.indexApplied)fail.current(`${row.oldPath}: ${row.message}`);}
   window.dispatchEvent(new Event('smart-photo-indexer:processing-wake'));
  };
  void fileActionsService.connect(applied,view=>dispatch({type:'app/setStatusMessage',message:operationSummary(view),severity:'error'}),value=>fail.current(value)).then(unsubscribe=>{if(disposed)unsubscribe();else stop=unsubscribe;}).catch(value=>fail.current(value));
  const command=async(event:Event)=>{
   const intent=(event as CustomEvent<FileIntent>).detail;if(!intent)return;
   const snapshot=latest.current,source=intent.source??selectedFileSource(snapshot);
   dispatch({type:'explorer/closeContextMenu'});
   document.querySelector<HTMLElement>('.explorer-grid')?.focus({preventScroll:true});
   try{
    if(intent.command==='copy'||intent.command==='cut'){
     dispatch({type:'app/setStatusMessage',message:localeText("main", "ui.FileActionsHost.f4ec970bbfe9840f")});
     const clip=await fileActionsService.capture(intent.command==='copy'?'copy':'move',source);
     if(!clip)return;
     dispatch({type:'app/setStatusMessage',message:localeText("main", "ui.FileActionsHost.cff3bad2052e893a", {"value1": (intent.command==='copy'?localeText("main", "ui.FileActionsHost.1e56dce988696199"):localeText("main", "ui.FileActionsHost.9a58517288ac55bd")), "value2": (clip.count)})});return;
    }
    if(intent.command==='paste'){
     const folder=pasteFolder(snapshot),clip=await fileActionsService.clipboard();
     if(!clip.ready||!clip.selectionId)throw new Error(localeText("main", "ui.FileActionsHost.9643e5200c2c5586"));
     if(!folder)throw new Error(localeText("main", "ui.FileActionsHost.15836becbfeef1ae"));
     await fileActionsService.start(clip.kind==='move'?'move':'copy',{type:'clipboard',selectionId:clip.selectionId,revision:clip.revision},folder);return;
    }
    if(intent.command==='trash'){await fileActionsService.start('trash',source);return;}
    const folder=intent.destination??await fileSystemService.chooseFolder(snapshot.currentPath);
    if(folder)await fileActionsService.start(intent.command==='copyTo'?'copy':'move',source,folder);
   }catch(value){fail.current(value);}
  };
  window.addEventListener(fileCommandEvent,command);
  return()=>{disposed=true;stop?.();window.removeEventListener(fileCommandEvent,command);};
 },[dispatch]);
 const decide=async(choice:'replace'|'keepBoth'|'skip',all:boolean)=>{setDecisionBusy(true);try{await fileActionsService.resolve(choice,all);}catch(value){error(value);}finally{setDecisionBusy(false);}};
 const cancel=async()=>{setDecisionBusy(true);try{await fileActionsService.cancel();}catch(value){error(value);}finally{setDecisionBusy(false);}};
 return operation?.state==='awaitingConflict'&&operation.conflict?<FileConflictDialog key={operation.conflict.conflictId} conflict={operation.conflict} busy={decisionBusy} onDecision={decide} onCancel={cancel}/>:null;
}
