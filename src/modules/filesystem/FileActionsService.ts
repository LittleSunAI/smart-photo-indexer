import {invoke} from "../../shared/tauri/invoke";
import {listen} from '@tauri-apps/api/event';
import {useSyncExternalStore} from 'react';
import type {FileActionKind,FileActionSource,FileOperation,FileOutcome,FileSelection} from './FileActions';

const empty:FileSelection={selectionId:null,revision:0,kind:null,count:0,ready:false};
let snapshot:{clipboard:FileSelection;operation:FileOperation|null;preparing:boolean}={clipboard:empty,operation:null,preparing:false};
const subscribers=new Set<()=>void>();
const wakeListeners=new Set<()=>void>();
const wake=()=>wakeListeners.forEach(fn=>fn());
let synchronize:()=>Promise<void>=async()=>{};
function update(patch:Partial<typeof snapshot>){snapshot={...snapshot,...patch};subscribers.forEach(fn=>fn());}
const operation=(input:object)=>invoke<FileOperation|null>('explorer_file_operation',{input});
let captureGeneration=0;
export const fileActionsService={
 subscribe:(fn:()=>void)=>{subscribers.add(fn);return()=>{subscribers.delete(fn);};},
 getSnapshot:()=>snapshot,
 async clipboard(){const clipboard=await invoke<FileSelection>('explorer_file_selection',{input:{action:'inspect'}});if(!snapshot.preparing)update({clipboard});return clipboard;},
 async capture(kind:'copy'|'move',source:FileActionSource){
  const generation=++captureGeneration;update({clipboard:empty,preparing:true});
  try{const clipboard=await invoke<FileSelection>('explorer_file_selection',{input:{action:'capture',kind,source}});
   if(generation!==captureGeneration)return null;
   update({clipboard});wake();return clipboard;
  }catch(error){if(generation!==captureGeneration)return null;throw error;}
  finally{if(generation===captureGeneration)update({preparing:false});}
 },
 async start(kind:FileActionKind,source:FileActionSource,destinationFolder?:string){
  await synchronize();
  const view=await operation({action:'start',requestId:crypto.randomUUID(),kind,source,destinationFolder});if(view){update({operation:view});wake();}return view;
 },
 async resolve(choice:'replace'|'keepBoth'|'skip',applyToAll:boolean){const view=snapshot.operation;if(!view?.conflict)return;
  const next=await operation({action:'resolveConflict',operationId:view.operationId,conflictId:view.conflict.conflictId,choice,applyToAll});if(next){update({operation:next});wake();}
 },
 async cancel(){const view=snapshot.operation;if(!view)return;const next=await operation({action:'cancelRemaining',operationId:view.operationId});if(next){update({operation:next});wake();}},
 async connect(applied:(rows:FileOutcome[])=>Promise<void>,finished:(view:FileOperation)=>void,failed:(error:unknown)=>void){
  let disposed=false,running=false,dirty=false,timer:ReturnType<typeof setTimeout>|undefined,flight:Promise<void>=Promise.resolve();
  const offsets=new Map<string,number>(),done=new Set<string>();
  const execute=async()=>{
   timer=undefined;if(disposed)return;if(running){dirty=true;return;}running=true;
   try{let view=await operation({action:'inspect',offset:0});
    if(view&&!disposed){
     let offset=offsets.get(view.operationId)??0;
     let page=offset?await operation({action:'inspect',operationId:view.operationId,offset}):view;
     while(page&&!disposed){if(page.changedPaths.length){await applied(page.changedPaths);offset+=page.changedPaths.length;offsets.set(view.operationId,offset);}
      if(!page.hasMore)break;page=await operation({action:'inspect',operationId:view.operationId,offset});}
     update({operation:view});
     if(!['running','awaitingConflict'].includes(view.state)&&!done.has(view.operationId)){done.add(view.operationId);finished(view);}
    }
    const clipboard=await invoke<FileSelection>('explorer_file_selection',{input:{action:'inspect'}});if(!disposed&&!snapshot.preparing)update({clipboard});
   }catch(error){if(!disposed)failed(error);}finally{running=false;if(dirty&&!disposed){dirty=false;schedule();}}
  };
  const pump=()=>{if(running){dirty=true;return flight;}flight=execute();return flight;};
  const flush=async()=>{if(running)await flight;await pump();};
  const schedule=()=>{if(!disposed&&!timer)timer=setTimeout(()=>{timer=undefined;void pump();},80);};
  const stop=await listen('smart-photo-indexer:file-actions-changed',schedule);
  wakeListeners.add(schedule);synchronize=flush;schedule();
  return()=>{disposed=true;stop();wakeListeners.delete(schedule);if(synchronize===flush)synchronize=async()=>{};if(timer)clearTimeout(timer);};
 },
};
export function useFileActions(){return useSyncExternalStore(fileActionsService.subscribe,fileActionsService.getSnapshot,fileActionsService.getSnapshot);}
