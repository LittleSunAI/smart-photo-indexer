import {plural} from '../../shared/i18n/Formatting';
import {text as localeText} from "../../shared/i18n/Runtime";
import {isQueryView,quickSearchView} from './QuickSearch';
import type { AppState } from '../types';
import type { ExplorerItem, ImageItem } from '../../modules/explorer/types';
import { messages, pluralizeRu } from '../../shared/i18n';
import { estimateOpenRouterCostPerImageUsd } from '../../modules/taskQueue/TaskQueueService';
import {analysisLimits} from '../../shared/ai/AnalysisLimits';

const STATUS_ORDER: Record<string, number> = {
  processed: 1,
  queued: 2,
  notProcessed: 3,
  relocated: 4,
  missing: 5,
  error: 6,
};

export function selectSelectedItem(state: AppState): ExplorerItem | null {
  state=quickSearchView(state);
  if (!state.selectedItemId) return null;
  return getKnownWorkspaceItems(state).find((item) => item.id === state.selectedItemId) ?? null;
}

export function selectSelectedItems(state: AppState): ExplorerItem[] {
  state=quickSearchView(state);
  const selectedIds = new Set(state.selectedItemIds);
  return getKnownWorkspaceItems(state).filter((item) => selectedIds.has(item.id));
}

export function selectSelectedImageItems(state: AppState): ImageItem[] {
  return selectSelectedItems(state).filter((item): item is ImageItem => item.kind === 'image');
}

export function selectSelectionCost(state:AppState) {
  state=quickSearchView(state);
  const provider=state.settings.aiProviderMode,limits=state.settings.processingLimits;
  const model=provider==='local'?state.settings.localAi.selectedModel:provider==='fal'?state.settings.fal.selectedModel:state.settings.openRouter.selectedModel;
  const selected=isQueryView(state)&&state.search.selectionSnapshot
    ? Math.max(0,state.search.selectionSnapshot.totalCount-state.search.selectionSnapshot.excludedPaths.length)
    : selectSelectedImageItems(state).length;
  const count=selected;
  const effective=analysisLimits(provider,model,limits.outputMode,limits.maxTokens,limits.imageMaxSide);
  const perPass=estimateOpenRouterCostPerImageUsd(model,provider,effective.maxTokens,limits.passes);
  return {provider,model,selected,count,passes:limits.passes,perPass,totalUsd:count*limits.passes*perPass,budgetUsd:limits.maxEstimatedCostPerRunUsd};
}

export function selectionCostEqual(a:ReturnType<typeof selectSelectionCost>,b:ReturnType<typeof selectSelectionCost>) {
  return a.provider===b.provider&&a.model===b.model&&a.selected===b.selected&&a.count===b.count&&a.passes===b.passes&&a.perPass===b.perPass&&a.totalUsd===b.totalUsd&&a.budgetUsd===b.budgetUsd;
}

export function selectContextMenuItem(state: AppState): ExplorerItem | null {
  state=quickSearchView(state);
  if (!state.contextMenu.itemId) return null;
  return getKnownWorkspaceItems(state).find((item) => item.id === state.contextMenu.itemId) ?? null;
}

export function selectWorkspaceItems(state: AppState): ExplorerItem[] {
  state=quickSearchView(state);
  const query = (state.filenameFilter??'').toLocaleLowerCase();
  const baseItems = isQueryView(state)
    ? state.search.results
    : state.explorerItems;

  const filteredByMode = state.showImagesOnly
    ? baseItems.filter((item) => item.kind === 'folder' || item.kind === 'image')
    : baseItems;

  const searched = isQueryView(state)
    ? filteredByMode
    : query.length > 0
      ? filteredByMode.filter((item) => item.name.toLocaleLowerCase().includes(query))
      : filteredByMode;

  // Native search orders the full archive by relevance before paging.
  if(isQueryView(state))return searched;
  if(!state.explorerDisplayOrder.length)return sortExplorerItems(searched,state.sortMode,state.folderGrouping);
  const byId=new Map(searched.map(item=>[item.id,item])),ordered:ExplorerItem[]=[];
  for(const id of state.explorerDisplayOrder){const item=byId.get(id);if(item){ordered.push(item);byId.delete(id);}}
  ordered.push(...byId.values());
  return ordered;
}

export function selectViewerImages(state: AppState): ImageItem[] {
  return selectWorkspaceItems(state).filter((item): item is ImageItem => item.kind === 'image');
}

export function selectWorkspaceSummary(state:AppState):AppState['taskSummary']{
  state=quickSearchView(state);
  if(!isQueryView(state)){
    const images=selectWorkspaceItems(state).filter((item):item is ImageItem=>item.kind==='image');
    return {...state.taskSummary,totalItems:selectWorkspaceItems(state).length,processed:images.filter(i=>i.status==='processed').length,
      inQueue:images.filter(i=>i.status==='queued').length,notProcessed:images.filter(i=>['notProcessed','missing','relocated'].includes(i.status)).length,errors:images.filter(i=>i.status==='error').length};
  }
  return {...state.taskSummary,totalItems:state.search.resultCount,...(state.search.statusCounts??{
    processed:state.search.results.filter(i=>i.status==='processed').length,inQueue:state.search.results.filter(i=>i.status==='queued').length,
    notProcessed:state.search.results.filter(i=>['notProcessed','missing','relocated'].includes(i.status)).length,errors:state.search.results.filter(i=>i.status==='error').length,
  })};
}

export function selectWorkspaceTitle(state: AppState): string {
  state=quickSearchView(state);
  if (isQueryView(state)) {
    const query = state.searchQuery.trim();
    return query.length > 0 ? messages().workspace.searchResultsTitle(query) : messages().workspace.titleWhenSearching;
  }

  return state.currentPath ?? state.pathSegments.join(' > ');
}

export function selectProcessButtonLabel(state:AppState):string{return state.settings.processingLimits.manualQueue?localeText("main", "ui.appSelectors.90abf7c79e932920"):localeText("main", "ui.appSelectors.822f592a5b57344e");}

export interface SelectionCounts{total:number;images:number;folders:number;files:number;query:boolean}
export function selectSelectionCounts(state:AppState):SelectionCounts{
 state=quickSearchView(state);const snapshot=state.search.selectionSnapshot;
 if(snapshot&&isQueryView(state)){const total=Math.max(0,snapshot.totalCount-snapshot.excludedPaths.length);return {total,images:total,folders:0,files:0,query:true};}
 const selected=selectSelectedItems(state);return {total:selected.length,images:selected.filter(i=>i.kind==='image').length,folders:selected.filter(i=>i.kind==='folder').length,files:selected.filter(i=>i.kind==='file').length,query:false};
}
export function formatSelectionSummary(counts:SelectionCounts,includeLabel=true):string{
 if(!counts.total)return messages().workspace.noSelection;
 const parts=[counts.images?plural('images',counts.images):'',counts.folders?plural('folders',counts.folders):'',counts.files?plural('files',counts.files):''].filter(Boolean);
 const value=parts.length===1?parts[0]:counts.total+' — '+parts.join(', ');
 return includeLabel?localeText("main","ui.appSelectors.abcd709b65248a8e",{value1:value}):value;
}
export function selectSelectionSummary(state:AppState):string{return formatSelectionSummary(selectSelectionCounts(state));}

// Explicit Windows-style Latin/Cyrillic ordering matches native photo_sort_key.
const collator = new Intl.Collator('en-US', { numeric: true, sensitivity: 'base' });
export function explorerItemComparator(sortMode: AppState['sortMode'],grouping:AppState['folderGrouping']='foldersFirst'): (left:ExplorerItem,right:ExplorerItem)=>number {
  const foldersFirst = (left: ExplorerItem, right: ExplorerItem) => {
    return left.kind === 'folder' ? (right.kind==='folder'?0:-1) : right.kind==='folder'?1:0;
  };

  const compare=(left:ExplorerItem,right:ExplorerItem)=>{
    const folderOrder = foldersFirst(left, right);
    if (folderOrder !== 0) return folderOrder;

    switch (sortMode) {
      case 'nameAsc':
        return collator.compare(left.name, right.name);
      case 'nameDesc':
        return collator.compare(right.name, left.name);
      case 'readyFirst': case 'readyLast': {
        const ready=(item:ExplorerItem)=>Number(item.kind==='image'&&item.status==='processed');
        return (sortMode==='readyFirst'?-1:1)*(ready(left)-ready(right))||collator.compare(left.name,right.name);
      }
      case 'errorsFirst': case 'errorsLast': {
        const error=(item:ExplorerItem)=>Number(item.kind==='image'&&item.status==='error');
        return (sortMode==='errorsFirst'?-1:1)*(error(left)-error(right))||collator.compare(left.name,right.name);
      }
      case 'status':
        return getStatusOrder(left) - getStatusOrder(right) || collator.compare(left.name, right.name);
      case 'statusDesc':
        return getStatusOrder(right) - getStatusOrder(left) || collator.compare(left.name,right.name);
      case 'type':
        return collator.compare(getTypeLabel(left), getTypeLabel(right)) || collator.compare(left.name, right.name);
      case 'typeDesc':
        return collator.compare(getTypeLabel(right),getTypeLabel(left)) || collator.compare(left.name,right.name);
      case 'sizeAsc': case 'sizeDesc':
        return compareOptional(left.kind==='folder'?null:left.sizeBytes,right.kind==='folder'?null:right.sizeBytes,sortMode.endsWith('Desc')) || collator.compare(left.name,right.name);
      case 'dimensionsAsc': case 'dimensionsDesc': {
        const area=(item:ExplorerItem)=>item.kind==='image' && item.fileMetadata?.width && item.fileMetadata.height ? item.fileMetadata.width*item.fileMetadata.height : null;
        return compareOptional(area(left),area(right),sortMode.endsWith('Desc')) || collator.compare(left.name,right.name);
      }
      case 'analyzedAsc': case 'analyzedDesc':
        return compareOptional(left.kind==='image'?left.aiProcessedAtUnix:null,right.kind==='image'?right.aiProcessedAtUnix:null,sortMode.endsWith('Desc')) || collator.compare(left.name,right.name);
      case 'modifiedAsc': case 'modifiedDesc':
        return compareOptional(left.kind==='folder'?null:left.modifiedAtUnix,right.kind==='folder'?null:right.modifiedAtUnix,sortMode.endsWith('Desc')) || collator.compare(left.name,right.name);
      case 'cameraAsc': case 'cameraDesc': {
        const camera=(item:ExplorerItem)=>item.kind==='image'?localeText("main", "ui.appSelectors.31160329a3f1c9d5", {"value1": (item.fileMetadata?.cameraMake??''), "value2": (item.fileMetadata?.cameraModel??'')}).trim():'';
        return (sortMode.endsWith('Desc')?-1:1)*collator.compare(camera(left),camera(right)) || collator.compare(left.name,right.name);
      }
      case 'pathAsc': case 'pathDesc':
        return (sortMode.endsWith('Desc')?-1:1)*collator.compare(left.path,right.path);
      case 'tagsAsc': case 'tagsDesc':
        return (sortMode.endsWith('Desc')?-1:1)*((left.kind==='image'?left.tags.length:0)-(right.kind==='image'?right.tags.length:0)) || collator.compare(left.name,right.name);
      case 'favoriteAsc': case 'favoriteDesc':
        return (sortMode.endsWith('Desc')?-1:1)*(Number(left.kind==='image'&&left.isFavorite)-Number(right.kind==='image'&&right.isFavorite)) || collator.compare(left.name,right.name);
      case 'dateTakenOldest':
        return compareCaptureDates(left,right,false) || collator.compare(left.name,right.name);
      case 'dateTakenNewest':
      default:
        return compareCaptureDates(left,right,true) || collator.compare(left.name,right.name);
    }
  };
  return (left,right)=>compare(left,right)||collator.compare(left.path,right.path)||left.id.localeCompare(right.id);
}

export function sortExplorerItems(items: ExplorerItem[],sortMode:AppState['sortMode'],grouping:AppState['folderGrouping']='foldersFirst'):ExplorerItem[]{
  return [...items].sort(explorerItemComparator(sortMode,grouping));
}

/** A metadata event only checks adjacent keys; it never sorts the displayed list. */
export function explorerOrderNeedsRefresh(items:ExplorerItem[],sortMode:AppState['sortMode'],grouping:AppState['folderGrouping']):boolean{
  const compare=explorerItemComparator(sortMode,grouping);
  for(let index=1;index<items.length;index++)if(compare(items[index-1],items[index])>0)return true;
  return false;
}

function getStatusOrder(item: ExplorerItem): number {
  return item.kind === 'image' ? STATUS_ORDER[item.status] ?? 99 : 0;
}

function compareCaptureDates(left:ExplorerItem,right:ExplorerItem,descending:boolean):number{
  const a=left.kind==='image' ? left.fileMetadata?.dateTakenLocal : null;
  const b=right.kind==='image' ? right.fileMetadata?.dateTakenLocal : null;
  if(!a || !b)return a ? -1 : b ? 1 : 0;
  return (descending ? -1 : 1)*(a<b ? -1 : a>b ? 1 : 0);
}

function compareOptional(a:number|null|undefined,b:number|null|undefined,descending:boolean):number{
  if(a==null || b==null)return a==null?(b==null?0:1):-1;
  return (descending?-1:1)*(a-b);
}

function getTypeLabel(item: ExplorerItem): string {
  if (item.kind === 'folder') return 'folder';
  if (item.kind === 'image') return item.extension ?? 'image';
  return item.fileType;
}


function getKnownWorkspaceItems(state: AppState): ExplorerItem[] {
  const byId = new Map<string, ExplorerItem>();
  for (const item of state.explorerItems) byId.set(item.id, item);
  for (const item of state.search.results) byId.set(item.id, item);
  for (const item of state.dragDrop.items) byId.set(item.id, item);
  if(state.inspectedImage)byId.set(state.inspectedImage.id,state.inspectedImage);
  return [...byId.values()];
}
