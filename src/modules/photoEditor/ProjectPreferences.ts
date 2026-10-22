import{useEffect,useSyncExternalStore}from'react';
import{listen}from'@tauri-apps/api/event';
import{invoke}from'../../shared/tauri/invoke';
import{isTauriRuntime}from'../../shared/tauri/runtime';
export interface ProjectPreferences{version:1;revision:number;autosaveEnabled:boolean;intervalSeconds:30|60|180|300|600;alwaysSaveOnExit:boolean}
type Snapshot=ProjectPreferences&{ready:boolean;error:string};
let snapshot:Snapshot={version:1,revision:0,autosaveEnabled:false,intervalSeconds:60,alwaysSaveOnExit:false,ready:false,error:''};
const subscribers=new Set<()=>void>();let initialization:Promise<void>|undefined,tail:Promise<unknown>=Promise.resolve(),listening=false;
function publish(value:ProjectPreferences){if(value?.version!==1||typeof value.revision!=='number'||value.revision<snapshot.revision)return;snapshot={...value,ready:true,error:''};subscribers.forEach(f=>f());}
function failure(error:unknown){snapshot={...snapshot,error:String(error)};subscribers.forEach(f=>f());}
export function legacyProjectAutosave(raw:string|null):boolean|undefined{if(!raw)return;try{const p=JSON.parse(raw);return p?.version===1&&p.editorProjectSeed!=='fresh'?p.editorAutosave!==false:undefined;}catch{return;}}
export async function readProjectPreferences():Promise<ProjectPreferences>{
 if(!isTauriRuntime()){if(!snapshot.ready)publish(snapshot);return snapshot;}
 if(!initialization)initialization=(async()=>{const legacyAutosave=legacyProjectAutosave(localStorage.getItem('smart-photo-indexer.ui.v1'));publish(await invoke<ProjectPreferences>('initialize_project_preferences',{legacyAutosave}));})().catch(e=>{initialization=undefined;failure(e);throw e;});
 await initialization;return snapshot;
}
export function patchProjectPreferences(patch:Partial<Pick<ProjectPreferences,'autosaveEnabled'|'intervalSeconds'|'alwaysSaveOnExit'>>):Promise<ProjectPreferences>{
 const write=tail.then(async()=>{await readProjectPreferences();const value=isTauriRuntime()?await invoke<ProjectPreferences>('patch_project_preferences',{patch}):{...snapshot,...patch,revision:snapshot.revision+1};publish(value);return value;});tail=write.catch(e=>{failure(e);});return write;
}
function subscribe(callback:()=>void){subscribers.add(callback);return()=>{subscribers.delete(callback);};}
export function useProjectPreferences(){
 const value=useSyncExternalStore(subscribe,()=>snapshot);
 useEffect(()=>{void readProjectPreferences().catch(()=>{});if(!listening&&isTauriRuntime()){listening=true;void listen<ProjectPreferences>('smart-photo-indexer:project-preferences-changed',e=>publish(e.payload)).catch(e=>{listening=false;failure(e);});}},[]);
 return value;
}
