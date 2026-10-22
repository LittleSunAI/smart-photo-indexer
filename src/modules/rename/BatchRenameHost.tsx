import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {isQueryView} from '../../app/state/QuickSearch';
import {useEffect,useRef,useState} from 'react';
import {listen} from '@tauri-apps/api/event';
import {useAppActions,useAppState} from '../../app/state/AppStateContext';
import {selectSelectedItems,selectWorkspaceItems} from '../../app/state/appSelectors';
import {isTauriRuntime} from '../../shared/tauri/runtime';
import {BatchRenameDialog} from './BatchRenameDialog';
import {batchRenameService,type BatchRenamePlan} from './BatchRenameService';

export function BatchRenameHost(){
 const state=useAppState(),actions=useAppActions(),current=useRef(state),completed=useRef(new Set<string>());
 current.current=state;
 const [paths,setPaths]=useState<string[]|null>(null),[failure,setFailure]=useState('');
 const opening=useRef(0);
 const apply=(id:string,reverse:boolean,mappings:Array<[string,string]>)=>{const token=`${id}:${reverse}`;if(completed.current.has(token))return;completed.current.add(token);actions.applyBatchFileRenames(mappings);window.dispatchEvent(new Event('smart-photo-indexer:processing-wake'));};
 const applyRef=useRef(apply);applyRef.current=apply;
 useEffect(()=>{
  const open=async(event:Event)=>{const generation=++opening.current;const explicit=(event as CustomEvent<{paths?:string[]}>).detail?.paths;
   const snapshot=current.current,selected=selectSelectedItems(snapshot),ids=new Set(selected.map(item=>item.id));
   const ordered=selectWorkspaceItems(snapshot).filter(item=>ids.has(item.id));
   const seen=new Set(ordered.map(item=>item.id));
   const snapshotSelection=isQueryView(snapshot)?snapshot.search.selectionSnapshot:undefined;
   let allSelected:string[]|undefined;
   if(!explicit&&snapshotSelection?.input.queryId){try{allSelected=await batchRenameService.selectionPaths(snapshotSelection.input.queryId,snapshotSelection.excludedPaths);}catch(error){setFailure(String(error));return;}}
   if(generation!==opening.current||snapshotSelection!==current.current.search.selectionSnapshot&&snapshotSelection)return;
   const chosen=explicit??allSelected??[...ordered,...selected.filter(item=>!seen.has(item.id))].map(item=>item.path);
   const source=chosen.length?chosen:snapshot.currentPath?[snapshot.currentPath]:[];
   if(source.length)setPaths(source);
  };
  window.addEventListener('smart-photo-indexer:open-batch-rename',open);
  let disposed=false,unlisten:(()=>void)|undefined;
  if(isTauriRuntime())void listen<{id:string;reverse:boolean;mappings:Array<[string,string]>}>('smart-photo-indexer:batch-renamed',event=>applyRef.current(event.payload.id,event.payload.reverse,event.payload.mappings)).then(stop=>{if(disposed)stop();else unlisten=stop;});
  return()=>{disposed=true;opening.current++;unlisten?.();window.removeEventListener('smart-photo-indexer:open-batch-rename',open);};
 },[]);
 const applied=(plan:BatchRenamePlan)=>apply(plan.id,plan.reverse,plan.rows.filter(row=>row.status!=='unchanged').map(row=>[row.oldPath,row.newPath]));
 return <>{paths?<BatchRenameDialog paths={paths} onClose={()=>setPaths(null)} onApplied={applied}/>:null}{failure?<RenameOpenError message={failure} onClose={()=>setFailure('')}/>:null}</>;
}
function RenameOpenError({message,onClose}:{message:string;onClose():void}){
  useCurrentLocale();const ref=useRef<HTMLDialogElement>(null);useEffect(()=>{ref.current?.showModal();return()=>ref.current?.close();},[]);return <dialog ref={ref} className="batch-rename-error" aria-label={localeText("rename", "ui.BatchRenameHost.df774ec3d95c376c")} onCancel={event=>{event.preventDefault();onClose();}} onKeyDown={event=>event.stopPropagation()}><strong>{localeText("rename", "ui.BatchRenameHost.0c78e7e55d944e69")}</strong><p>{message}</p><Hint message={{id:"common.acknowledge"}}><button type="button" onClick={onClose}>{localeText("rename", "ui.BatchRenameHost.e5c4bf213eddc221")}</button></Hint></dialog>;}
