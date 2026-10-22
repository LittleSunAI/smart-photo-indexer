import type {AppState} from '../types';
import type {ExplorerItem,ImageItem} from '../../modules/explorer/types';
import type {FileOutcome} from '../../modules/filesystem/FileActions';
import {reconcileExplorerOrder} from './ExplorerOrder';

export const filePathKey=(path:string)=>path.replace(/^\\\\\?\\/,'').replace(/\//g,'\\').replace(/\\+$/,'').toLocaleLowerCase();
const beneath=(path:string,folder:string)=>filePathKey(path).startsWith(filePathKey(folder)+'\\');
export function applyFileOutcomes(state:AppState,outcomes:FileOutcome[],images:ImageItem[]):AppState{
 const applied=outcomes.filter(row=>row.filesystemApplied),byOld=new Map(applied.filter(row=>['moved','trashed'].includes(row.status)).map(row=>[filePathKey(row.oldPath),row]));
 const fresh=new Map(images.map(image=>[filePathKey(image.path),image])),idMap=new Map<string,string|null>();
 const remap=(item:ExplorerItem):ExplorerItem|null=>{
  const change=byOld.get(filePathKey(item.path));
  if(change?.status==='trashed'){idMap.set(item.id,null);return null;}
  const path=change?.newPath??item.path,newImage=fresh.get(filePathKey(path));
  if(!newImage){if(change){idMap.set(item.id,null);return null;}return item;}
  const id=item.id.startsWith('search:')?`search:${newImage.indexedImageId??newImage.id}:${newImage.path.toLowerCase()}`:newImage.id;
  idMap.set(item.id,id);return {...newImage,id};
 };
 const list=<T extends ExplorerItem>(items:T[],accept:(item:ExplorerItem)=>boolean=()=>true):T[]=>items.map(remap).filter((item):item is ExplorerItem=>Boolean(item)&&accept(item!)) as T[];
 const inExplorer=(item:ExplorerItem)=>!state.currentPath||beneath(item.path,state.currentPath)&&(state.includeSubfolders||filePathKey(item.path.slice(0,Math.max(item.path.lastIndexOf('\\'),item.path.lastIndexOf('/'))))===filePathKey(state.currentPath));
 const explorerItems=list(state.explorerItems,inExplorer),present=new Set(explorerItems.map(item=>filePathKey(item.path)));
 for(const image of images)if(inExplorer(image)&&!present.has(filePathKey(image.path))){explorerItems.push(image);present.add(filePathKey(image.path));}
 const search=(value:AppState['search']):AppState['search']=>{
  const results=list(value.results,item=>!value.rootPath||beneath(item.path,value.rootPath));
  return {...value,results,resultCount:Math.max(0,value.resultCount-value.results.length+results.length),needsRefresh:true,queryId:undefined,lastInput:undefined,selectionSnapshot:undefined};
 };
 const searchState=search(state.search),dragItems=list(state.dragDrop.items);
 const contexts={...state.workspaceContexts};
 for(const [key,context] of Object.entries(contexts))if(context)contexts[key as keyof typeof contexts]={...context,search:search(context.search)};
 const inspectedImage=state.inspectedImage?(remap(state.inspectedImage) as ImageItem|null)??undefined:undefined;
 const known=new Set([...explorerItems,...searchState.results,...dragItems,...(inspectedImage?[inspectedImage]:[])].map(item=>item.id));
 const selection=(value:{selectedItemIds:string[];selectedItemId:string|null;selectionAnchorItemId:string|null},restrict=false)=>{
  const ids=[...new Set(value.selectedItemIds.map(id=>idMap.has(id)?idMap.get(id):id).filter((id):id is string=>Boolean(id)&&(!restrict||known.has(id!))))];
  const primary=value.selectedItemId?(idMap.has(value.selectedItemId)?idMap.get(value.selectedItemId):value.selectedItemId):null;
  const anchor=value.selectionAnchorItemId?(idMap.has(value.selectionAnchorItemId)?idMap.get(value.selectionAnchorItemId):value.selectionAnchorItemId):null;
  return {selectedItemIds:ids,selectedItemId:primary&&(!restrict||known.has(primary))?primary:ids[0]??null,selectionAnchorItemId:anchor??ids[0]??null};
 };
 for(const [key,context] of Object.entries(contexts))if(context)contexts[key as keyof typeof contexts]={...context,...selection(context)};
 const quickSearch=state.quickSearch?{...state.quickSearch,search:search(state.quickSearch.search),baseSelection:state.quickSearch.baseSelection?{...state.quickSearch.baseSelection,...selection(state.quickSearch.baseSelection),inspectedImage:state.quickSearch.baseSelection.inspectedImage?(remap(state.quickSearch.baseSelection.inspectedImage) as ImageItem|null)??undefined:undefined}:undefined}:undefined;
 return {...state,...selection(state),explorerItems,...reconcileExplorerOrder(explorerItems,state.explorerDisplayOrder,state.explorerSortSnapshot,state.sortMode,state.folderGrouping),search:searchState,quickSearch,inspectedImage,
  workspaceContexts:contexts,dragDrop:{...state.dragDrop,items:dragItems,imageCount:dragItems.filter(item=>item.kind==='image').length},contextMenu:{...state.contextMenu,isOpen:false,itemId:null}};
}
