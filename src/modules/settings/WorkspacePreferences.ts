import type { SortMode, ViewSize } from '../../app/types';

const key='smart-photo-indexer.workspace.v1';
export interface WorkspacePreferences { folderGrouping:'foldersFirst'|'filesFirst'|'mixed'; lastPath:string|null; viewSize:ViewSize; sortMode:SortMode; includeSubfolders:boolean; showImagesOnly:boolean; recentPaths:string[]; refreshMode:'folder'|'index' }
const defaults:WorkspacePreferences={folderGrouping:'foldersFirst',lastPath:null,viewSize:'large',sortMode:'dateTakenNewest',includeSubfolders:false,showImagesOnly:true,recentPaths:[],refreshMode:'folder'};
export function loadWorkspacePreferences():WorkspacePreferences{
  try{
    const raw=JSON.parse(localStorage.getItem(key)??'null');if(!raw || typeof raw!=='object')return {...defaults};
    return {
      folderGrouping:'foldersFirst',
      lastPath:typeof raw.lastPath==='string' && raw.lastPath.length<=32768 ? raw.lastPath : null,
      viewSize:['table','list','small','medium','large','extraLarge'].includes(raw.viewSize)?raw.viewSize:defaults.viewSize,
      sortMode:['readyFirst','readyLast','errorsFirst','errorsLast','analyzedAsc','analyzedDesc','dateTakenNewest','dateTakenOldest','nameAsc','nameDesc','status','statusDesc','type','typeDesc','sizeAsc','sizeDesc','dimensionsAsc','dimensionsDesc','modifiedAsc','modifiedDesc','cameraAsc','cameraDesc','pathAsc','pathDesc','tagsAsc','tagsDesc','favoriteAsc','favoriteDesc'].includes(raw.sortMode)?raw.sortMode:defaults.sortMode,
      includeSubfolders:raw.includeSubfolders===true,showImagesOnly:raw.showImagesOnly!==false,
      refreshMode:raw.refreshMode==='index'?'index':'folder',
      recentPaths:Array.isArray(raw.recentPaths)?raw.recentPaths.filter((path:unknown)=>typeof path==='string' && path.length<=32768).slice(0,10):[],
    };
  }catch{return {...defaults};}
}
export function saveWorkspacePreferences(input:Omit<WorkspacePreferences,'recentPaths'|'refreshMode'>):void{
  try{
    const old=loadWorkspacePreferences();
    const lastPath=input.lastPath || old.lastPath;
    const paths=[lastPath,...old.recentPaths].filter((path):path is string=>Boolean(path));
    const seen=new Set<string>();const recentPaths=paths.filter(path=>{const normalized=path.replaceAll('/','\\').toLocaleLowerCase();if(seen.has(normalized))return false;seen.add(normalized);return true;}).slice(0,10);
    localStorage.setItem(key,JSON.stringify({...input,folderGrouping:'foldersFirst',lastPath,recentPaths,refreshMode:old.refreshMode}));
  }catch{ /* The workspace remains usable when preference storage is unavailable. */ }
}
export function saveRefreshMode(refreshMode:WorkspacePreferences['refreshMode']):void{
  localStorage.setItem(key,JSON.stringify({...loadWorkspacePreferences(),refreshMode}));
}
