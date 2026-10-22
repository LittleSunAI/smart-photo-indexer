import {text as localeText} from "../../shared/i18n/Runtime";
import {initialQuickSearch,quickSearchView,selectionOf} from './QuickSearch';
import {applyFileOutcomes} from './FileActionsProjection';
import {reconcileExplorerOrder} from './ExplorerOrder';
import {normalizeSearchFields,legacySearchFields} from '../../modules/search/SearchFieldsContract';
import type { AppState, ProcessingSelectionDraft, RenamePreviewState } from '../types';
import type { ExplorerItem, ImageItem } from '../../modules/explorer/types';
import type { QueueTaskStatus, TaskQueueState, TaskSummary } from '../../modules/taskQueue/types';
import type { AppAction } from './appActions';
import type { IndexMetadataUpdateResult, IndexSyncResult } from '../../modules/index/types';
import { mapIndexSearchResultsToImages } from '../../modules/index/IndexService';
import type { AppSettings } from '../../modules/settings/Settings.types';
import { settingsService } from '../../modules/settings/SettingsService';
import { resolveLocalAiModel, resolvePresetModel } from '../../modules/settings/modelPresets';
import { estimateQueueCost, summarizeTaskQueue } from '../../modules/taskQueue/TaskQueueService';
import { messages } from '../../shared/i18n';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { SmartRenameResult } from '../../modules/rename/RenameService';
import { captureDateLabel,imageDimensions } from '../../modules/filesystem/ImageMetadata';
import { sortExplorerItems } from './appSelectors';
import { switchWorkspace } from './WorkspaceContexts';

const MAX_RECENT_SEARCHES = 10;

function parseSearchTagFilters(value: string): string[] {
  const seen = new Set<string>();
  const tags: string[] = [];
  for (const raw of value.split(/[\n,;]+/)) {
    const tag = raw.trim().replace(/^#+/, '').toLowerCase();
    if (!tag || seen.has(tag)) continue;
    seen.add(tag);
    tags.push(tag);
  }
  return tags;
}

function hasActiveSearchFilters(state: AppState): boolean {
  return state.search.statusFilter !== 'all'
    || parseSearchTagFilters(state.search.tagFilterText).length > 0
    || state.search.dateFrom.trim().length > 0
    || state.search.dateTo.trim().length > 0
    || Boolean(state.search.collectionId || state.search.rootPath)
    || state.search.favoriteOnly || state.search.untaggedOnly===true;
}

function quickDateToday(): string {
  const date=new Date();return `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,'0')}-${String(date.getDate()).padStart(2,'0')}`;
}

function normalizeDeletedImagePath(path: string): string {
  return path.trim().replace(/\\/g, '/').toLowerCase();
}

function itemMatchesDeletedImage(item: ExplorerItem, itemId: string | null | undefined, path: string): boolean {
  if (item.kind !== 'image') return false;

  const cleanItemId = itemId?.trim();
  if (cleanItemId && item.id === cleanItemId) {
    return true;
  }

  const cleanPath = normalizeDeletedImagePath(path);
  if (!cleanPath) return false;

  return normalizeDeletedImagePath(item.path) === cleanPath;
}

function removeDeletedImageFromItems<T extends ExplorerItem>(
  items: T[],
  itemId: string | null | undefined,
  path: string,
): T[] {
  return items.filter((item) => !itemMatchesDeletedImage(item, itemId, path));
}

function collectDeletedImageIds(items: ExplorerItem[], itemId: string | null | undefined, path: string): Set<string> {
  const ids = new Set<string>();

  for (const item of items) {
    if (itemMatchesDeletedImage(item, itemId, path)) {
      ids.add(item.id);
    }
  }

  const cleanItemId = itemId?.trim();
  if (cleanItemId) {
    ids.add(cleanItemId);
  }

  return ids;
}

function reducePresentedState(state: AppState, action: AppAction): AppState {
  if(action.type==='index/statusesUpdated'){
    const changes=new Map(action.changes.map(c=>[c.imageId,c])),paths=new Map(action.changes.flatMap(c=>(c.paths??[]).map(path=>[path.replaceAll('/','\\').toLowerCase(),c] as const)));
    const patch=(items:ExplorerItem[])=>items.map(item=>{if(item.kind!=='image')return item;const change=changes.get(item.indexedImageId??'')??paths.get(item.path.replaceAll('/','\\').toLowerCase());return change?{...item,indexedImageId:change.imageId,status:change.status}:item;});
    const search=(s:AppState['search'])=>{const results=patch(s.results) as ImageItem[];return {...s,results,dataRevision:(s.dataRevision??0)+1,needsRefresh:Boolean(s.queryId)||s.needsRefresh,statusCounts:s.resultCount===s.results.length?{processed:results.filter(i=>i.status==='processed').length,inQueue:results.filter(i=>i.status==='queued').length,notProcessed:results.filter(i=>['notProcessed','missing','relocated'].includes(i.status)).length,errors:results.filter(i=>i.status==='error').length}:s.statusCounts};};
    const contexts={...state.workspaceContexts};for(const [mode,context]of Object.entries(contexts))if(context)contexts[mode as keyof typeof contexts]={...context,search:search(context.search)};
    return {...state,explorerItems:patch(state.explorerItems),search:search(state.search),workspaceContexts:contexts,quickSearch:state.quickSearch?{...state.quickSearch,dataRevision:(state.quickSearch.dataRevision??0)+1,search:search(state.quickSearch.search)}:undefined};
  }
  if(action.type==='quickSearch/update'){
    const quick=state.quickSearch??initialQuickSearch(state);let search=action.field?{...quick.search,[action.field]:action.field==='fields'?normalizeSearchFields(JSON.parse(action.value??'[]')):action.value??''}:quick.search;
    if(action.field||(action.query!==undefined&&action.query.trim()!==quick.query.trim()))search={...search,selectionSnapshot:undefined,errorMessage:null};
    return {...state,quickSearch:{...quick,search,query:action.query??quick.query,composing:action.composing??quick.composing}};
  }
  if(action.type==='quickSearch/clear'){
    const quick=state.quickSearch;if(!quick)return state;
    return {...state,...(quick.active?quick.baseSelection:{}),quickSearch:{...quick,query:'',active:false,baseSelection:undefined,search:{...quick.search,results:[],resultCount:0,queryId:undefined,lastInput:undefined,selectionSnapshot:undefined,isSearching:false,isLoadingPage:false,errorMessage:null}}};
  }
  const quickAction=action.type==='quickSearch/apply';
  const selectionAction=state.quickSearch?.active&&(action.type==='explorer/inspectImage'||action.type==='search/selectSnapshot'||['explorer/selectItem','explorer/toggleSelection','explorer/selectRange','explorer/selectAllVisible','explorer/clearSelection','explorer/openContextMenu','explorer/closeContextMenu'].includes(action.type));
  if(quickAction||selectionAction){
    const q=state.quickSearch??initialQuickSearch(state),inner=quickAction?action.action:action;
    const projected={...state,quickSearch:undefined,underlyingSearch:undefined,activeWorkspace:'search' as const,search:q.search,searchQuery:q.query};
    const next=appReducer(projected,inner);
    if(quickAction&&inner.type==='search/succeeded'){
      if(action.dataRevision!==undefined&&action.dataRevision!==(q.dataRevision??0))next.search={...next.search,needsRefresh:true};
      if(q.search.selectionSnapshot)next.search={...next.search,selectionSnapshot:q.search.selectionSnapshot};
    }
    const activate=q.active||inner.type==='search/start';
    return {...state,...selectionOf(next),contextMenu:next.contextMenu,statusMessage:next.statusMessage,quickSearch:{...q,active:activate,search:next.search,baseSelection:q.baseSelection??(activate?selectionOf(state):undefined)}};
  }

  if(action.type==='explorer/inspectImage')return {...state,inspectedImage:action.image,selectedItemId:action.image.id,selectedItemIds:[action.image.id],selectionAnchorItemId:action.image.id,search:{...state.search,selectionSnapshot:undefined},contextMenu:closedContextMenu(),statusMessage:''};

  if(action.type==='filesystem/actionsApplied')return applyFileOutcomes(state,action.outcomes,action.images);
  if(action.type==='filesystem/batchRenamed'){
    const mapping=new Map(action.mappings);
    const patch=<T extends ExplorerItem>(items:T[]):T[]=>items.map(item=>{
      const path=mapping.get(item.path);if(!path)return item;
      const name=path.split(/[\\/]/).at(-1)??item.name;
      return {...item,path,name,...(item.kind==='image'?{previewUrl:convertFileSrc(path)}:{})};
    });
    const search=(value:AppState['search']):AppState['search']=>({...value,results:patch(value.results),needsRefresh:true,
      queryId:undefined,lastInput:undefined,selectionSnapshot:undefined});
    const contexts={...state.workspaceContexts};for(const [mode,context] of Object.entries(contexts))if(context)contexts[mode as keyof typeof contexts]={...context,search:search(context.search)};
    const explorerItems=patch(state.explorerItems);
    const explorerSortSnapshot=state.explorerSortSnapshot?Object.fromEntries(patch(Object.values(state.explorerSortSnapshot)).map(item=>[item.id,item])):undefined;
    const explorerDisplayOrder=sortExplorerItems(explorerItems,state.sortMode,state.folderGrouping).map(item=>item.id);
    return {...state,explorerItems,explorerSortSnapshot,explorerDisplayOrder,search:search(state.search),dragDrop:{...state.dragDrop,items:patch(state.dragDrop.items)},workspaceContexts:contexts,
      contextMenu:closedContextMenu(),statusMessage:localeText("main", "ui.appReducer.e786c9afd24a37a6", {"value1": (action.mappings.length)})};
  }
  if(action.type==='workspace/setActive')return switchWorkspace(state,action.mode);
  if(action.type==='workspace/openCollection'){
    if(action.collectionId&&!state.collections.items.some(item=>item.id===action.collectionId))return state;
    const next=switchWorkspace(state,'collections');
    return {...next,selectedItemId:null,selectedItemIds:[],selectionAnchorItemId:null,searchQuery:'',
      search:{...next.search,queryId:undefined,lastInput:undefined,selectionSnapshot:undefined,results:[],resultCount:0,collectionId:action.collectionId,needsRefresh:false,isSearching:false,errorMessage:null},
      statusMessage:action.collectionId?localeText("main", "ui.appReducer.033d21c4e56294a4"):localeText("main", "ui.appReducer.6600a3162589d098")};
  }
  if (action.type === 'search/selectSnapshot') {
    if (state.activeWorkspace === 'explorer' || state.search.queryId !== action.sourceQueryId) return state;
    const results=state.search.results;
    return {...state, search:{...state.search,selectionSnapshot:{input:action.input,totalCount:action.totalCount,excludedPaths:[]}},
      selectedItemIds:results.map(item=>item.id),selectedItemId:results[0]?.id??null,selectionAnchorItemId:results[0]?.id??null,
      contextMenu:closedContextMenu(),statusMessage:''};
  }
  const selection=state.quickSearch?.active?undefined:state.search.selectionSnapshot;
  if (selection && action.type === 'explorer/toggleSelection' && state.activeWorkspace!=='explorer') {
    const item=state.search.results.find(candidate=>candidate.id===action.itemId);if(!item)return state;
    const excluded=new Set(selection.excludedPaths);
    if(excluded.has(item.path))excluded.delete(item.path);else excluded.add(item.path);
    const selectedItemIds=state.search.results.filter(candidate=>!excluded.has(candidate.path)).map(candidate=>candidate.id);
    return {...state,selectedItemIds,selectedItemId:item.id,selectionAnchorItemId:item.id,contextMenu:closedContextMenu(),
      search:{...state.search,selectionSnapshot:{...selection,excludedPaths:[...excluded]}},statusMessage:''};
  }
  const resetSelection = selection && (
    ['explorer/selectItem','explorer/selectRange','explorer/selectAllVisible','explorer/clearSelection',
      'filesystem/scanStarted','search/succeeded','search/clear','dragDrop/inspectStarted'].includes(action.type)
    || (action.type==='explorer/openContextMenu' && action.itemId && !state.selectedItemIds.includes(action.itemId))
  );
  let next=reduceAppState(resetSelection ? {...state,search:{...state.search,selectionSnapshot:undefined}} : state,action);
  if(action.type==='collections/loadSucceeded'){
    const ids=new Set(action.result.collections.map(item=>item.id));
    const clear=(search:AppState['search'])=>({...search,collectionId:'',results:[],resultCount:0,queryId:undefined,lastInput:undefined,selectionSnapshot:undefined,isSearching:false});
    if(next.activeWorkspace==='collections'&&next.search.collectionId&&!ids.has(next.search.collectionId))next={...next,search:clear(next.search),selectedItemId:null,selectedItemIds:[],selectionAnchorItemId:null};
    const cached=next.workspaceContexts?.collections;
    if(cached?.search.collectionId&&!ids.has(cached.search.collectionId))next={...next,workspaceContexts:{...next.workspaceContexts,collections:{...cached,search:clear(cached.search),selectedItemId:null,selectedItemIds:[],selectionAnchorItemId:null}}};
  }
  if(action.type==='collections/mutated'||action.type==='collections/addImagesSucceeded'){
    const cached=next.workspaceContexts?.collections;
    const id=action.type==='collections/mutated'?action.collectionId:action.result.collection?.id;
    const deleted=action.type==='collections/mutated'&&action.deleted;
    const changedMembers=action.type==='collections/addImagesSucceeded'||Boolean(action.imageIds)||deleted;
    if(cached){
      const changed=changedMembers&&cached.search.collectionId===id;
      const search=deleted&&cached.search.collectionId===id?{...cached.search,collectionId:'',results:[],resultCount:0,queryId:undefined,lastInput:undefined,selectionSnapshot:undefined,needsRefresh:true}:{...cached.search,selectionSnapshot:changed?undefined:cached.search.selectionSnapshot,needsRefresh:true};
      next={...next,workspaceContexts:{...next.workspaceContexts,collections:{...cached,search,selectedItemId:changed?null:cached.selectedItemId,selectedItemIds:changed?[]:cached.selectedItemIds}}};
    }
    if(next.activeWorkspace==='collections'){
      const changed=changedMembers&&state.search.collectionId===id;
      const search=deleted&&state.search.collectionId===id?{...next.search,collectionId:'',results:[],resultCount:0,queryId:undefined,lastInput:undefined,selectionSnapshot:undefined,needsRefresh:true}:{...next.search,selectionSnapshot:changed?undefined:next.search.selectionSnapshot,needsRefresh:true};
      next={...next,search,selectedItemId:changed?null:next.selectedItemId,selectedItemIds:changed?[]:next.selectedItemIds};
    }
  }
  if(action.type==='index/metadataUpdateSucceeded'||action.type==='collections/favoriteUpdated'){
    const patch=(search:AppState['search'],mode:string)=>{
      let results=action.type==='index/metadataUpdateSucceeded'?applyMetadataUpdateToExplorerItems(search.results,action.result) as ImageItem[]:applyFavoriteUpdateToItems(search.results,action.result);
      const removed=mode==='favorites'?results.filter(item=>!item.isFavorite):mode==='ready'?results.filter(item=>item.status!=='processed'):[];
      if(removed.length)results=results.filter(item=>!removed.includes(item));
      const statusCounts=search.resultCount===search.results.length?{
        processed:results.filter(item=>item.status==='processed').length,inQueue:results.filter(item=>item.status==='queued').length,
        notProcessed:results.filter(item=>['notProcessed','missing','relocated'].includes(item.status)).length,errors:results.filter(item=>item.status==='error').length,
      }:search.statusCounts;
      return {...search,results,statusCounts,dataRevision:(search.dataRevision??0)+1,resultCount:Math.max(0,search.resultCount-removed.length),
        selectionSnapshot:removed.length?undefined:search.selectionSnapshot,
        needsRefresh:Boolean(search.queryId)||mode==='favorites'||mode==='ready'||search.needsRefresh};
    };
    const contexts={...next.workspaceContexts};
    for(const [mode,context] of Object.entries(contexts))if(mode!=='explorer'&&mode!==next.activeWorkspace&&context){
      const search=patch(context.search,mode);contexts[mode as keyof typeof contexts]={...context,search,
        selectedItemIds:context.selectedItemIds.filter(id=>search.results.some(item=>item.id===id))};
    }
    const search=patch(next.search,next.activeWorkspace);
    const cleared=next.search.selectionSnapshot&&!search.selectionSnapshot;
    next={...next,workspaceContexts:contexts,search,
      selectedItemIds:state.quickSearch?.active?state.selectedItemIds:cleared?[]:next.selectedItemIds.filter(id=>next.activeWorkspace==='explorer'||next.inspectedImage?.id===id||search.results.some(item=>item.id===id)),
      statusMessage:cleared?localeText("main", "ui.appReducer.77fa8d16fc5c6dc7"):next.statusMessage};
  }
  if(action.type==='index/syncSucceeded'){
    const known=new Map<string,ImageItem>();
    for(const item of [...state.explorerItems,...state.search.results,...(state.quickSearch?.search.results??[]),...Object.values(state.workspaceContexts??{}).flatMap(context=>context?.search.results??[])])if(item.kind==='image')known.set(item.path,item);
    const changed=action.result.newImages>0||action.result.relocatedImages>0||action.result.indexedImages.some(image=>{
      const old=known.get(image.path);if(!old)return false;
      return old.status!==image.status||(old.description??'')!==(image.description??'')||
        old.tags.join('\n')!==image.tags.join('\n')||
        image.aiProcessedAtUnix!==undefined&&old.aiProcessedAtUnix!==image.aiProcessedAtUnix||
        image.fileMetadata?.modifiedAtUnix!==undefined&&old.modifiedAtUnix!==image.fileMetadata.modifiedAtUnix||
        image.fileMetadata?.dateTakenLocal!==undefined&&old.fileMetadata?.dateTakenLocal!==image.fileMetadata.dateTakenLocal;
    });
    const patch=(search:AppState['search'])=>({...search,results:applyIndexedImagesToExplorerItems(search.results,action.result) as ImageItem[],
      needsRefresh:search.needsRefresh||Boolean(changed&&search.queryId)});
    const contexts={...next.workspaceContexts};
    for(const [mode,context]of Object.entries(contexts))if(context)contexts[mode as keyof typeof contexts]={...context,search:patch(context.search)};
    next={...next,search:patch(next.search),workspaceContexts:contexts,
      quickSearch:next.quickSearch?{...next.quickSearch,search:patch(next.quickSearch.search)}:undefined};
  }
  if(state.quickSearch?.active&&action.type.startsWith('search/'))next={...next,...selectionOf(state),activeWorkspace:state.activeWorkspace,quickSearch:{...next.quickSearch!,baseSelection:selectionOf(next)}};
  if(next.quickSearch&&(action.type==='index/metadataUpdateSucceeded'||action.type==='collections/favoriteUpdated'||action.type==='explorer/removeImageByPath')){
    const q=next.quickSearch;let results=q.search.results;
    if(action.type==='index/metadataUpdateSucceeded')results=applyMetadataUpdateToExplorerItems(results,action.result) as ImageItem[];
    else if(action.type==='collections/favoriteUpdated')results=applyFavoriteUpdateToItems(results,action.result);
    else results=removeDeletedImageFromItems(results,action.itemId,action.path);
    next={...next,quickSearch:{...q,dataRevision:(q.dataRevision??0)+1,search:{...q.search,results,needsRefresh:Boolean(q.search.queryId)||q.search.needsRefresh,resultCount:Math.max(0,q.search.resultCount-(q.search.results.length-results.length))}}};
  }
  if(next.inspectedImage){
    let image=next.inspectedImage;
    if(action.type==='index/metadataUpdateSucceeded')image=applyMetadataUpdateToExplorerItems([image],action.result)[0] as ImageItem;
    if(action.type==='collections/favoriteUpdated')image=applyFavoriteUpdateToItems([image],action.result)[0];
    const clear=next.selectedItemId!==image.id||next.activeWorkspace!==state.activeWorkspace||action.type==='filesystem/scanStarted'||action.type==='explorer/removeImageByPath'&&itemMatchesDeletedImage(image,action.itemId,action.path);
    next={...next,inspectedImage:clear?undefined:image};
  }
  if(next.explorerItems!==state.explorerItems&&next.sortMode===state.sortMode&&next.folderGrouping===state.folderGrouping&&action.type!=='filesystem/scanSucceeded'){next={...next,...reconcileExplorerOrder(next.explorerItems,state.explorerDisplayOrder,state.explorerSortSnapshot,state.sortMode,state.folderGrouping)};}
  if(!next.quickSearch?.active&&next.search.selectionSnapshot && next.activeWorkspace!=='explorer' &&
    (next.search.results!==state.search.results || next.search.selectionSnapshot!==state.search.selectionSnapshot || next.selectedItemIds!==state.selectedItemIds)) {
    const excluded=new Set(next.search.selectionSnapshot.excludedPaths);
    const ids=next.search.results.filter(item=>!excluded.has(item.path)).map(item=>item.id);
    if(ids.length!==next.selectedItemIds.length || ids.some((id,index)=>id!==next.selectedItemIds[index]))return {...next,selectedItemIds:ids};
  }
  return next;
}

function reduceAppState(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case 'processing/fileRenamed': {
      return { ...state, explorerItems: applyFileRename(state.explorerItems, action.result), search: { ...state.search, results: applyFileRename(state.search.results, action.result) }, dragDrop: { ...state.dragDrop, items: applyFileRename(state.dragDrop.items, action.result) } };
    }
    case 'taskQueue/nativePrepared': {
      if (state.taskQueue.preparationKey !== action.preparationKey || state.taskQueue.preparationState==='cancelled') return state;
      const pending=state.taskQueue.pendingPreparation,result=action.result;
      const autoStart=pending?.config.autoStart&&!result?.alreadyReady;
      const message=result?.message||((result?.totalCount??1)>0?localeText("main", "ui.appReducer.8dbe08c5ddca8ec0", {"value1": (result?.totalCount??pending?.tasks.length??0), "value2": (result?.readyCount??0), "value3": (result?.duplicateCount??0)}):localeText("main", "ui.appReducer.d962699fc3229c02", {"value1": (result?.readyCount??0), "value2": (result?.duplicateCount??0)}));
      // A different capsule may have been opened while this command was preparing.
      const retainView=Boolean(state.taskQueue.nativeJobId&&state.taskQueue.nativeJobId!==action.jobId)||Boolean(pending&&pending.config.queueMode!==(state.settings.processingLimits.manualQueue?'manual':'direct'));
      return {...state,processingSelectionDraft:emptyProcessingDraft(),statusMessage:message,taskQueue:{...state.taskQueue,
        pendingPreparation:undefined,preparationResult:result,preparationState:autoStart?'starting':undefined,lastMessage:message,
        ...(retainView?{}:{tasks:(pending?.tasks??state.taskQueue.tasks).map(task=>({...task,pendingNativeIdentity:true})),config:pending?.config??state.taskQueue.config,nativeJobId:action.jobId,nativeRevision:0,nativeStatus:result?.status??(state.taskQueue.nativeJobId===action.jobId?state.taskQueue.nativeStatus:undefined),nativeSnapshotPending:true}),
      }};
    }
    case 'taskQueue/preparationCancelled': {
      if(state.taskQueue.preparationKey!==action.preparationKey)return state;
      return {...state,processingSelectionDraft:emptyProcessingDraft(),taskQueue:{...state.taskQueue,tasks:state.taskQueue.nativeJobId?state.taskQueue.tasks:[],pendingPreparation:undefined,preparationState:'cancelled',lastMessage:localeText("main", "ui.appReducer.68875031bcb116c2")}};
    }
    case 'taskQueue/selectionPreparing': {
      return {...state,activeDetailsTab:'tasks',taskQueue:{...state.taskQueue,preparationKey:action.preparationKey,preparationState:'preparing',lastErrorMessage:null}};
    }
    case 'taskQueue/toggleFinished': {
      return {...state,taskQueue:{...state.taskQueue,hideFinished:!state.taskQueue.hideFinished}};
    }
    case 'taskQueue/nativeCommandPending': {
      if(action.jobId==='*')return {...state,taskQueue:{...state.taskQueue,commandPending:action.pending,lastErrorMessage:action.error??(action.pending?null:state.taskQueue.lastErrorMessage)}};
      if(state.taskQueue.nativeJobId!==action.jobId){
        if(state.taskQueue.preparationResult?.jobId!==action.jobId)return state;
        return {...state,taskQueue:{...state.taskQueue,preparationState:action.pending?'starting':undefined,lastErrorMessage:action.error??state.taskQueue.lastErrorMessage}};
      }
      return {...state,taskQueue:{...state.taskQueue,commandPending:action.pending,
        preparationState:action.pending?state.taskQueue.preparationState:undefined,
        lastErrorMessage:action.error??(action.pending?null:state.taskQueue.lastErrorMessage)}};
    }
    case 'taskQueue/selectNativeJob': {
      if(state.taskQueue.nativeJobId===action.jobId)return state;
      return { ...state, taskQueue: { ...state.taskQueue, nativeJobId: action.jobId, nativeRevision: 0, nativeCounters: undefined, nativeSnapshotPending:false,tasks: [] } };
    }
    case 'taskQueue/nativeObserved': {
      const { snapshot } = action;
      if(snapshot.queueMode&&snapshot.queueMode!==(state.settings.processingLimits.manualQueue?'manual':'direct'))return state;
      if(snapshot.aggregate)state={...state,taskQueue:{...state.taskQueue,nativeAggregate:snapshot.aggregate}};
      const job = snapshot.job;
      if (!job) return {...state,taskQueue:{...state.taskQueue,nativeJobs:snapshot.jobs,nativeWorkingJobs:snapshot.workingJobs??state.taskQueue.nativeWorkingJobs}};
      if (state.taskQueue.nativeJobId && state.taskQueue.nativeJobId !== job.jobId) return state;
      // A draft has not yet been saved. Do not let an older job replace that selection.
      if (!state.taskQueue.nativeJobId && state.taskQueue.tasks.length && !state.taskQueue.nativeStatus) return state;
      const same = state.taskQueue.nativeJobId === job.jobId;
      if(same&&snapshot.revision<(state.taskQueue.nativeRevision??0))return state;
      const awaitingFirstSnapshot=same&&state.taskQueue.nativeSnapshotPending===true;
      const pendingByPath=new Map((awaitingFirstSnapshot?state.taskQueue.tasks:[]).map(task=>[task.targetPath,task]));
      const tasks = new Map((same ? state.taskQueue.tasks : []).filter(task=>!(awaitingFirstSnapshot&&(state.taskQueue.config.sourceRoots?.includes(task.targetPath)||Boolean(state.taskQueue.config.sourceQueryId)&&task.targetPath.startsWith("search:")))).map(task => [task.id, task]));
      for (const native of snapshot.tasks) {
        if(native.workVisible===false){tasks.delete(native.id);const draft=pendingByPath.get(native.path);if(draft)tasks.delete(draft.id);continue;}
        const context=native.config??job.config;
        const old = tasks.get(native.id)??pendingByPath.get(native.path);
        if(awaitingFirstSnapshot&&old&&old.id!==native.id)tasks.delete(old.id);
        const status = native.status === 'succeeded' ? 'done' : native.status === 'failed' ? 'failed' : native.status === 'cancelled' ? 'cancelled' : native.status==='skipped'?'skipped': ['preparing','running','committing'].includes(native.status) ? 'running' : 'pending';
        tasks.set(native.id, { ...old,pendingNativeIdentity:false,nativeOrdinal:native.ordinal??old?.nativeOrdinal, id: native.id, type: 'aiAnalysis', sourceItemId: native.sourceItemId, targetPath: native.path,
          fileName: native.path.split(/[\\/]/).pop() ?? native.path, status, nativeStage:native.status, completedPasses:native.completedPasses,
          currentStartedAtUnix:native.currentStartedAtUnix,
          progress: native.metadataCommitted ? 100 : Math.round(native.completedPasses / (context.passes ?? 1) * 90),
          createdAt: new Date(job.createdAtUnix * 1000).toISOString(), attemptCount: old?.attemptCount ?? 0, maxAttempts: (context.maxRetriesPerTask ?? 0) + 1,
          estimatedCostUsd: context.estimatedCostPerImageUsd ?? 0, errorMessage: native.error ?? undefined, providerMode: context.providerMode, model: context.model,
        });
      }
      if(awaitingFirstSnapshot&&!snapshot.hasMore)for(const [id,task] of tasks)if(task.pendingNativeIdentity)tasks.delete(id);
      const c = job.counters!;
      const finished = c.done + c.failed + c.cancelled;
      const runState = job.status === 'running' ? 'running' : job.status === 'stopped' ? 'stopped' : job.status === 'completed' ? 'completed' : job.status === 'prepared' ? 'idle' : 'paused';
      const taskQueue: TaskQueueState = { ...state.taskQueue, tasks: [...tasks.values()].sort((a,b)=>(a.nativeOrdinal??Number.MAX_SAFE_INTEGER)-(b.nativeOrdinal??Number.MAX_SAFE_INTEGER)), config: { ...state.taskQueue.config, ...job.config,queueMode:job.queueMode,outputMode:job.config.outputMode??'both' },
        nativeJobId: job.jobId, nativeStatus: job.status, nativeRevision: snapshot.revision, nativeJobs: snapshot.jobs,nativeWorkingJobs:snapshot.workingJobs??state.taskQueue.nativeWorkingJobs,nativeSnapshotPending:awaitingFirstSnapshot&&snapshot.hasMore,
        nativeCounters: { ...c, skipped: 0, finished, progressPercent: c.total ? Math.round(finished / c.total * 100) : 0 }, runState,
        estimatedRunCostUsd: job.spentUsd ?? 0, estimatedPendingUsd:job.estimatedPendingUsd, reservedCostUsd: job.reservedUsd ?? 0, unknownCosts: job.unknownCosts ?? 0, unresolvedRequests: job.unresolvedRequests ?? 0,
        lastMessage: job.notice??({prepared:localeText("main", "ui.appReducer.9b75cf7062e07cd5"),running:localeText("main", "ui.appReducer.3d7dadcf5b452947"),queued:localeText("main", "ui.appReducer.fb3198c6a810b06d"),paused:localeText("main", "ui.appReducer.b430666be43ac699"),interrupted:localeText("main", "ui.appReducer.cb70b3f5e4266086"),waitingService:localeText("main", "ui.appReducer.7ccbc5449a1b4d77"),stopped:localeText("main", "ui.appReducer.b44efc4abb7d58b0"),completed:localeText("main", "ui.appReducer.e1f7a6c3568fa6f1")} as Record<string,string>)[job.status]??job.status,
        lastErrorMessage: job.error,
      };
      // Native status projection considers every job for a photo. A viewed
      // cancelled/failed task must not overwrite another job's live status.
      return { ...state, taskQueue };
    }
    case 'settings/setCloudCredential': {
      const field = action.provider === 'fal' ? 'fal' : 'openRouter';
      return persistSettings(state, { ...state.settings, [field]: { ...state.settings[field], credentialRef: action.credentialRef, apiKey: '' } }, localeText("main", "ui.appReducer.b81a6820a8830cd5"));
    }
    case 'settings/setFalPreset': {
      const selectedModel = resolvePresetModel(action.presetId,'','fal');
      return persistSettings(state, { ...state.settings, fal: { ...state.settings.fal, selectedModel, selectedPresetId: action.presetId } }, `Fal: ${selectedModel}`);
    }
    case 'settings/setAnalysisLimit': {
      const bounds = { passes: [1, 5], cloudConcurrency: [1, 100], maxTokens: [1600, 4096], imageMaxSide: [1024, 4096] };
      const [min, max] = bounds[action.name];
      if (!Number.isFinite(action.value)) return state;
      return persistSettings(state, { ...state.settings, processingLimits: { ...state.settings.processingLimits, [action.name]: Math.round(Math.max(min, Math.min(max, action.value))) } }, localeText("main", "ui.appReducer.3a880a4da963d3f7"));
    }
    case 'settings/setAnalysisOutput': {
      if(!['both','tags','description'].includes(action.value))return state;
      return persistSettings(state,{...state.settings,processingLimits:{...state.settings.processingLimits,outputMode:action.value}},localeText("main", "ui.appReducer.395e562066bad2e9"));
    }

    case 'appError/report': {
      const deduped = state.appIssues.filter((issue) => issue.technicalMessage !== action.issue.technicalMessage || issue.operation !== action.issue.operation);
      return {
        ...state,
        appIssues: [action.issue, ...deduped].slice(0, 30),
        statusMessage: action.issue.userMessage,
      };
    }

    case 'appError/dismiss': {
      return {
        ...state,
        appIssues: state.appIssues.map((issue) => issue.id === action.issueId ? { ...issue, isDismissed: true } : issue),
      };
    }

    case 'appError/clearAll': {
      return {
        ...state,
        appIssues: state.appIssues.map((issue) => ({ ...issue, isDismissed: true })),
        statusMessage: localeText("main", "ui.appReducer.9cd3989134a2287b"),
      };
    }

    case 'workspace/setActive': {
      return {
        ...state,
        activeWorkspace: action.mode,
        contextMenu: closedContextMenu(),
        statusMessage: action.mode === 'search'
          ? localeText("main", "ui.appReducer.0272a93b042b2bff")
          : localeText("main", "ui.appReducer.6fe324b1388d2479"),
      };
    }

    case 'details/setActiveTab': {
      return { ...state, activeDetailsTab: action.tab };
    }

    case 'search/setQuery': {
      return { ...state, searchQuery: action.query };
    }

    case 'search/setStatusFilter': {
      return {
        ...state,
        search: { ...state.search, statusFilter: action.value },
        statusMessage: localeText("main", "ui.appReducer.16bfcfd47a30d636", {"value1": (action.value === 'all' ? localeText("main", "ui.appReducer.167cd3610d8b808f") : action.value)}),
      };
    }

    case 'search/setTagFilterText': {
      return {
        ...state,
        search: { ...state.search, tagFilterText: action.value },
        statusMessage: action.value.trim() ? localeText("main", "ui.appReducer.dde268295aba39f4") : localeText("main", "ui.appReducer.4cbfdbba946eeb7c"),
      };
    }

    case 'search/setTagMode': {
      return {
        ...state,
        search: { ...state.search, tagMode: action.value },
        statusMessage: action.value === 'all' ? localeText("main", "ui.appReducer.f0680702494e018f") : localeText("main", "ui.appReducer.a57f3cab2bd4b97d"),
      };
    }

    case 'search/addTagFilter': {
      const existing = parseSearchTagFilters(state.search.tagFilterText);
      const tag = action.tag.trim().replace(/^#+/, '').toLowerCase();
      if (!tag || existing.includes(tag)) return state;
      return {
        ...state,
        search: { ...state.search, tagFilterText: [...existing, tag].join(', ') },
        statusMessage: localeText("main", "ui.appReducer.135cf69393c692ee", {"value1": (tag)}),
      };
    }

    case 'search/removeTagFilter': {
      const tag = action.tag.trim().replace(/^#+/, '').toLowerCase();
      const tags = parseSearchTagFilters(state.search.tagFilterText).filter((item) => item !== tag);
      return {
        ...state,
        search: { ...state.search, tagFilterText: tags.join(', ') },
        statusMessage: tag ? localeText("main", "ui.appReducer.2e57b9b942d3ab2d", {"value1": (tag)}) : localeText("main", "ui.appReducer.a0d63954b2031738"),
      };
    }

    case 'explorer/setNameFilter':return {...state,filenameFilter:action.value};
    case 'search/setScope': {
      if(action.field==='fields'){let fields:unknown;try{fields=JSON.parse(action.value);}catch{return state;}return {...state,search:{...state.search,fields:normalizeSearchFields(fields)}};}
      if(action.field==='expandTerms')return {...state,search:{...state.search,expandTerms:action.value==='true'}};
      if(action.field==='favoriteOnly')return {...state,search:{...state.search,favoriteOnly:action.value==='true'}};
      if(action.field==='untaggedOnly')return {...state,search:{...state.search,untaggedOnly:action.value==='true'}};
      if(action.field==='dateField' && !['modified','taken','indexed','analyzed'].includes(action.value))return state;
      return {...state,search:{...state.search,[action.field]:action.value}};
    }

    case 'search/setDateFrom': {
      return {
        ...state,
        search: { ...state.search, dateFrom: action.value },
        statusMessage: action.value ? localeText("main", "ui.appReducer.b0387bef85140ad2", {"value1": (action.value)}) : localeText("main", "ui.appReducer.7ce636e3920b078a"),
      };
    }

    case 'search/setDateTo': {
      return {
        ...state,
        search: { ...state.search, dateTo: action.value },
        statusMessage: action.value ? localeText("main", "ui.appReducer.6449a05dba5bbe48", {"value1": (action.value)}) : localeText("main", "ui.appReducer.d9a12024a6e3c2d1"),
      };
    }


    case 'search/presetSaved': {
      return {
        ...state,
        searchPresets: action.presets,
        activeWorkspace: 'search',
        statusMessage: localeText("main", "ui.appReducer.10bf080309975ab8"),
      };
    }

    case 'search/presetApplied': {
      return {
        ...state,
        activeWorkspace: 'search',
        searchQuery: action.preset.query,
        searchPresets: action.presets,
        search: {
          ...state.search,
          statusFilter: action.preset.statusFilter,
          tagFilterText: action.preset.tagFilterText,
          tagMode: action.preset.tagMode,
          dateFrom: action.preset.dateFrom,
          dateTo: action.preset.dateTo,
          collectionId: action.preset.collectionId || '',
          rootPath: action.preset.rootPath || '',
          sortOrder: action.preset.sortOrder || 'relevance',
          dateField: action.preset.dateField || 'modified',
          fields:normalizeSearchFields(action.preset.fields,legacySearchFields),
          expandTerms: action.preset.expandTerms ?? false,
          favoriteOnly: action.preset.favoriteOnly,
          untaggedOnly: action.preset.untaggedOnly===true,
          errorMessage: null,
        },
        statusMessage: localeText("main", "ui.appReducer.d85ed67032347566", {"value1": (action.preset.name)}),
      };
    }

    case 'search/presetDeleted': {
      return {
        ...state,
        searchPresets: action.presets,
        statusMessage: localeText("main", "ui.appReducer.f0118d29759cddcc"),
      };
    }

    case 'search/presetFavoriteToggled': {
      return {
        ...state,
        searchPresets: action.presets,
        statusMessage: localeText("main", "ui.appReducer.dd32759f01323384"),
      };
    }

    case 'search/presetRenamed': {
      return {
        ...state,
        searchPresets: action.presets,
        statusMessage: localeText("main", "ui.appReducer.8e047b5ab24265e2"),
      };
    }

    case 'search/exportStarted': {
      return {
        ...state,
        activeWorkspace: 'search',
        search: {...state.search,operationFeedback:{message:localeText("main", "ui.appReducer.06241e172261ee9d")}},
        statusMessage: localeText("main", "ui.appReducer.ba325427f2ae3bdb", {"value1": (action.scope), "value2": (action.format.toUpperCase())}),
      };
    }

    case 'search/exportSucceeded': {
      return {
        ...state,
        activeWorkspace: 'search',
        search: {...state.search,operationFeedback:{message:localeText("main", "ui.appReducer.c45a308ab9eb1975", {"value1": (action.exportedCount), "value2": (action.fileName)})}},
        statusMessage: localeText("main", "ui.appReducer.2321c202b31611fe", {"value1": (action.exportedCount), "value2": (action.fileName)}),
      };
    }

    case 'search/exportFailed': {
      return {
        ...state,
        activeWorkspace: 'search',
        search: {...state.search,operationFeedback:{message:localeText("main", "ui.appReducer.5656da36822833b2", {"value1": (action.errorMessage)}),error:true}},
        statusMessage: localeText("main", "ui.appReducer.07e3813283d7c70c", {"value1": (action.errorMessage)}),
      };
    }

    case 'search/copyPathsSucceeded': {
      return {
        ...state,
        activeWorkspace: 'search',
        search: {...state.search,operationFeedback:{message:localeText("main", "ui.appReducer.02e8457d9b93e5f5", {"value1": (action.copiedCount)})}},
        statusMessage: localeText("main", "ui.appReducer.62da9754039a6bfd", {"value1": (action.copiedCount)}),
      };
    }

    case 'search/copyPathsFailed': {
      return {
        ...state,
        activeWorkspace: 'search',
        search: {...state.search,operationFeedback:{message:localeText("main", "ui.appReducer.b85b8193160b19e2", {"value1": (action.errorMessage)}),error:true}},
        statusMessage: localeText("main", "ui.appReducer.74e544c8515f1539", {"value1": (action.errorMessage)}),
      };
    }

    case 'search/exportCancelled': return {...state,search:{...state.search,operationFeedback:{message:localeText("main", "ui.appReducer.f99288dad95e3e21")}},statusMessage:localeText("main", "ui.appReducer.e5112c8b812b43bb")};

    case 'search/revealFoldersStarted': {
      return {
        ...state,
        activeWorkspace: 'search',
        search: {...state.search,operationFeedback:{message:localeText("main", "ui.appReducer.1676bba2f2a8a177", {"value1": (action.count)})}},
        statusMessage: localeText("main", "ui.appReducer.3a897e1bcf01f127", {"value1": (action.count)}),
      };
    }

    case 'search/revealFoldersSucceeded': {
      const message=localeText("main", "ui.appReducer.e169335991b22ce1", {"value1": (action.openedCount), "value2": (action.skippedCount>0 ? localeText("main", "ui.appReducer.771327873f0c60fc", {"value1": (action.skippedCount)}) : '')});
      return {
        ...state,
        activeWorkspace: 'search',
        search: {...state.search,operationFeedback:{message}},
        statusMessage: message,
      };
    }

    case 'search/revealFoldersFailed': {
      const message=localeText("main", "ui.appReducer.284b23e52551920c", {"value1": (action.errorMessage)});
      return {
        ...state,
        activeWorkspace: 'search',
        search: {...state.search,operationFeedback:{message,error:true}},
        statusMessage: message,
      };
    }

    case 'collections/loadStarted': {
      return {
        ...state,
        collections: { ...state.collections, isLoading: true, errorMessage: null },
      };
    }

    case 'collections/loadSucceeded': {
      return {
        ...state,
        collections: {
          ...state.collections,
          isLoading: false,
          errorMessage: null,
          items: action.result.collections,
          lastLoadedAtUnix: action.result.loadedAtUnix,
        },
        statusMessage: localeText("main", "ui.appReducer.8fa3b42333d1af6c", {"value1": (action.result.totalCount)}),
      };
    }

    case 'collections/loadFailed': {
      return {
        ...state,
        collections: { ...state.collections, isLoading: false, errorMessage: action.errorMessage },
        statusMessage: localeText("main", "ui.appReducer.1694d3dd468dfd9f", {"value1": (action.errorMessage)}),
      };
    }

    case 'collections/createSucceeded': {
      const nextItems = upsertCollection(state.collections.items, action.result.collection ?? null);
      return {
        ...state,
        collections: {
          ...state.collections,
          isLoading: false,
          errorMessage: null,
          items: nextItems,
          lastMutationMessage: action.result.collection ? localeText("main", "ui.appReducer.0b0751e48f6a590a", {"value1": (action.result.collection.name)}) : localeText("main", "ui.appReducer.449bc326ce1dfb6d"),
        },
        statusMessage: action.result.collection ? localeText("main", "ui.appReducer.12f338dff505ae17", {"value1": (action.result.collection.name)}) : localeText("main", "ui.appReducer.c30c586d131b08e5"),
      };
    }

    case 'collections/createFailed': {
      return {
        ...state,
        collections: { ...state.collections, errorMessage: action.errorMessage, lastMutationMessage: null },
        statusMessage: localeText("main", "ui.appReducer.f86b1df732ab87de", {"value1": (action.errorMessage)}),
      };
    }

    case 'collections/mutated': {
      const ids=action.imageIds ? new Set(action.imageIds) : null;
      const update=(items:ExplorerItem[])=>items.map(item=>{
        if(item.kind!=='image' || ids && !ids.has(item.indexedImageId ?? '')) return item;
        return {...item,collections:(item.collections ?? []).flatMap(name=>name===action.oldName ? action.newName ? [action.newName] : [] : [name])};
      });
      const message=action.deleted ? localeText("main", "ui.appReducer.d55ff3f27ce899c5") : action.newName ? localeText("main", "ui.appReducer.ea92f9001a2be3cc") : localeText("main", "ui.appReducer.87cdaf542a19346f", {"value1": (action.result.changedImages)});
      return {...state,explorerItems:update(state.explorerItems),dragDrop:{...state.dragDrop,items:update(state.dragDrop.items)},
        search:{...state.search,results:update(state.search.results) as typeof state.search.results,collectionId:action.deleted && state.search.collectionId===action.collectionId ? '' : state.search.collectionId},
        collections:{...state.collections,errorMessage:null,items:action.deleted ? state.collections.items.filter(item=>item.id!==action.collectionId) : upsertCollection(state.collections.items,action.result.collection ?? null),lastMutationMessage:message},statusMessage:message};
    }

    case 'collections/addImagesSucceeded': {
      const nextItems = upsertCollection(state.collections.items, action.result.collection ?? null);
      const collectionName = action.result.collection?.name ?? 'collection';
      const collectionNameForImages = action.result.collection?.name ?? null;
      const imageIdsForCollection = new Set(action.imageIds);
      return {
        ...state,
        explorerItems: collectionNameForImages ? applyCollectionNameToItems(state.explorerItems, imageIdsForCollection, collectionNameForImages) : state.explorerItems,
        search: {
          ...state.search,
          results: collectionNameForImages ? applyCollectionNameToItems(state.search.results, imageIdsForCollection, collectionNameForImages) as typeof state.search.results : state.search.results,
        },
        collections: {
          ...state.collections,
          errorMessage: null,
          items: nextItems,
          lastMutationMessage: `Added ${action.result.changedImages} image${action.result.changedImages === 1 ? '' : 's'} to ${collectionName}.`,
        },
        statusMessage: `Added ${action.result.changedImages} image${action.result.changedImages === 1 ? '' : 's'} to ${collectionName}${action.result.skippedImages > 0 ? `, skipped ${action.result.skippedImages}` : ''}.`,
      };
    }

    case 'collections/addImagesFailed': {
      return {
        ...state,
        collections: { ...state.collections, errorMessage: action.errorMessage, lastMutationMessage: null },
        statusMessage: localeText("main", "ui.appReducer.d4893c3b774e0494", {"value1": (action.errorMessage)}),
      };
    }

    case 'collections/favoriteUpdated': {
      return {
        ...state,
        explorerItems: applyFavoriteUpdateToItems(state.explorerItems, action.result),
        search: { ...state.search, results: applyFavoriteUpdateToItems(state.search.results, action.result) as typeof state.search.results },
        dragDrop: { ...state.dragDrop, items: applyFavoriteUpdateToItems(state.dragDrop.items, action.result) },
        statusMessage: `${action.result.isFavorite ? 'Added to favorites' : localeText("main", "ui.appReducer.2810ae2bdb5daef0")}: ${action.result.fileName}.`,
      };
    }

    case 'collections/favoriteFailed': {
      return {
        ...state,
        statusMessage: localeText("main", "ui.appReducer.cf89c4cf925b4eb6", {"value1": (action.errorMessage)}),
      };
    }

    case 'search/applyQuickFilter': {
      if (action.filter === 'processedToday') {
        return {
          ...state,
          activeWorkspace: 'search',
          search: { ...state.search, statusFilter: 'processed', dateField: 'analyzed', dateFrom: quickDateToday(), dateTo: quickDateToday() },
          statusMessage: localeText("main", "ui.appReducer.ebfbee683d2d0c91"),
        };
      }

      if (action.filter === 'missing') {
        return {
          ...state,
          activeWorkspace: 'search',
          search: { ...state.search, statusFilter: 'missing' },
          statusMessage: localeText("main", "ui.appReducer.aa7a9f77dfabaf95"),
        };
      }

      if (action.filter === 'favorites') {
        return {
          ...state,
          activeWorkspace: 'search',
          search: { ...state.search, favoriteOnly: true },
          statusMessage: localeText("main", "ui.appReducer.2ac298a20fdafd22"),
        };
      }

      if (action.filter === 'errors') {
        return {
          ...state,
          activeWorkspace: 'search',
          search: { ...state.search, statusFilter: 'error' },
          statusMessage: localeText("main", "ui.appReducer.0fc8dda2bce9d32e"),
        };
      }

      return {
        ...state,
        activeWorkspace: 'search',
        search: { ...state.search, tagFilterText: '', statusFilter: 'all', untaggedOnly: true },
        statusMessage: localeText("main", "ui.appReducer.d531a7d71f0aa337"),
      };
    }

    case 'search/submit': {
      const query = state.searchQuery.trim();
      if (query.length === 0 && !hasActiveSearchFilters(state)) {
        return {
          ...state,
          activeWorkspace: 'search',
          contextMenu: closedContextMenu(),
          search: { ...state.search, isSearching: false, errorMessage: localeText("main", "ui.appReducer.515f9b658c3e9468") },
          statusMessage: localeText("main", "ui.appReducer.91a290229249160c"),
        };
      }

      const recentSearches = query
        ? [query, ...state.recentSearches.filter((item) => item.toLowerCase() !== query.toLowerCase())].slice(0, MAX_RECENT_SEARCHES)
        : state.recentSearches;

      return {
        ...state,
        activeWorkspace: 'search',
        recentSearches,
        contextMenu: closedContextMenu(),
        statusMessage: localeText("main", "ui.appReducer.a7f5b699e566f6b0", {"value1": (query)}),
      };
    }

    case 'search/start': {
      return {
        ...state,
        activeWorkspace: action.workspace??(state.activeWorkspace==='explorer'?'search':state.activeWorkspace),
        contextMenu: closedContextMenu(),
        search: {
          ...state.search,
          isSearching: true,
          errorMessage: null,
          operationFeedback:undefined,
          isLoadingPage:false,pageError:null,
        },
        statusMessage: localeText("main", "ui.appReducer.f22292e925cc11e0"),
      };
    }

    case 'search/pageStarted': return { ...state, search: { ...state.search, isLoadingPage: true, pageError: null } };
    case 'search/pageFailed': return { ...state, search: { ...state.search, isLoadingPage: false, pageError: action.errorMessage } };
    case 'search/pageSucceeded': {
      const existing = new Set(state.search.results.map(item => item.path));
      const next = mapIndexSearchResultsToImages(action.result).filter(item => !existing.has(item.path));
      return { ...state, search: { ...state.search, isLoadingPage: false, pageError: null,
        results: [...state.search.results, ...next], resultCount: action.result.totalCount } };
    }
    case 'search/succeeded': {
      const query = action.result.query.trim();
      const results = mapIndexSearchResultsToImages(action.result);
      const recentSearches = action.result.recentSearches.length > 0
        ? action.result.recentSearches.slice(0, MAX_RECENT_SEARCHES)
        : [query, ...state.recentSearches.filter((item) => item.toLowerCase() !== query.toLowerCase())].slice(0, MAX_RECENT_SEARCHES);

      return {
        ...state,
        activeWorkspace: action.workspace??(state.activeWorkspace==='explorer'?'search':state.activeWorkspace),
        recentSearches,
        search: {
          ...state.search,
          isSearching: false,
          errorMessage: null,
          results,
          isLoadingPage:false,pageError:null,
          lastInput:action.input?{...action.input,queryId:action.result.queryId}:state.search.lastInput,
          statusCounts:action.result.statusCounts,
          needsRefresh:action.dataRevision!==undefined&&action.dataRevision!==(state.search.dataRevision??0),
          resultCount: action.result.totalCount,
          appliedSortOrder:action.sortOrder ?? state.search.sortOrder,
          queryId: action.result.queryId ?? undefined,
          lastQuery: query,
          lastAppliedTags: action.result.appliedTags,
          lastAppliedStatuses: action.result.appliedStatuses,
          lastAppliedDateFromUnix: action.result.appliedDateFromUnix ?? null,
          lastAppliedDateToUnix: action.result.appliedDateToUnix ?? null,
          lastAppliedFavoriteOnly: action.result.appliedFavoriteOnly,
          lastSearchedAtUnix: action.result.searchedAtUnix,
        },
        selectedItemId: results[0]?.id ?? null,
        selectedItemIds: results[0] ? [results[0].id] : [],
        selectionAnchorItemId: results[0]?.id ?? null,
        taskSummary: summarizeImages(results, localeText("main", "ui.appReducer.1a3cd700d65d9c50", {"count": (action.result.totalCount)})),
        statusMessage: action.result.totalCount > 0
          ? localeText("main", "ui.appReducer.385c5c555d1ace08", {"value1": (action.result.totalCount)})
          : localeText("main", "ui.appReducer.95d5bf359fcfe68d", {"value1": (query)}),
      };
    }

    case 'search/failed': {
      return {
        ...state,
        activeWorkspace: state.activeWorkspace==='explorer'?'search':state.activeWorkspace,
        search: { ...state.search, isSearching: false, errorMessage: action.errorMessage },
        statusMessage: localeText("main", "ui.appReducer.d7d67810f422adab", {"value1": (action.errorMessage)}),
      };
    }

    case 'search/clear': {
      return {
        ...state,
        searchQuery: '',
        activeWorkspace: action.keepWorkspace?state.activeWorkspace:'explorer',
        selectedItemId: action.keepWorkspace?null:state.explorerItems[0]?.id ?? null,
        selectedItemIds: !action.keepWorkspace&&state.explorerItems[0] ? [state.explorerItems[0].id] : [],
        selectionAnchorItemId: action.keepWorkspace?null:state.explorerItems[0]?.id ?? null,
search: { ...state.search, isSearching: false, errorMessage: null, operationFeedback:undefined, results: [], resultCount: 0, queryId:undefined,lastInput:undefined,selectionSnapshot:undefined,lastSearchedAtUnix:null,statusCounts:undefined,isLoadingPage:false,pageError:null,lastQuery: '', statusFilter: 'all', tagFilterText: '', tagMode: 'any', dateFrom: '', dateTo: '', collectionId: '', rootPath: '', sortOrder: 'relevance', appliedSortOrder:'relevance', dateField:'modified', lastAppliedTags: [], lastAppliedStatuses: [], lastAppliedDateFromUnix: null, lastAppliedDateToUnix: null, lastAppliedFavoriteOnly: false, favoriteOnly: false, untaggedOnly: false },
        contextMenu: closedContextMenu(),
        statusMessage: localeText("main", "ui.appReducer.b903ae2ae51c4a39"),
      };
    }

    case 'explorer/selectItem': {
      if (action.itemId === null) {
        return {
          ...state,
          selectedItemId: null,
          selectedItemIds: [],
          selectionAnchorItemId: null,
          contextMenu: closedContextMenu(),
          statusMessage: '',
        };
      }

      const selected = findKnownItemById(state, action.itemId);
      return {
        ...state,
        selectedItemId: selected?.id ?? null,
        selectedItemIds: selected ? [selected.id] : [],
        selectionAnchorItemId: selected?.id ?? null,
        contextMenu: closedContextMenu(),
        statusMessage: '',
      };
    }

    case 'explorer/toggleSelection': {
      const target = findKnownItemById(state, action.itemId);
      if (!target) return state;

      const isAlreadySelected = state.selectedItemIds.includes(action.itemId);
      const selectedItemIds = isAlreadySelected
        ? state.selectedItemIds.filter((itemId) => itemId !== action.itemId)
        : [...state.selectedItemIds, action.itemId];
      const selectedItemId = isAlreadySelected
        ? selectedItemIds[selectedItemIds.length - 1] ?? null
        : action.itemId;

      return {
        ...state,
        selectedItemId,
        selectedItemIds,
        selectionAnchorItemId: action.itemId,
        contextMenu: closedContextMenu(),
        statusMessage: buildSelectionStatus(selectedItemIds, getActiveItems(state)),
      };
    }

    case 'explorer/selectRange': {
      const targetIndex = action.visibleItemIds.indexOf(action.itemId);
      if (targetIndex < 0) return state;

      const anchorId = state.selectionAnchorItemId && action.visibleItemIds.includes(state.selectionAnchorItemId)
        ? state.selectionAnchorItemId
        : state.selectedItemId && action.visibleItemIds.includes(state.selectedItemId)
          ? state.selectedItemId
          : action.visibleItemIds[0] ?? action.itemId;
      const anchorIndex = action.visibleItemIds.indexOf(anchorId);
      const from = Math.min(anchorIndex, targetIndex);
      const to = Math.max(anchorIndex, targetIndex);
      const selectedItemIds = action.visibleItemIds.slice(from, to + 1);

      return {
        ...state,
        selectedItemId: action.itemId,
        selectedItemIds,
        selectionAnchorItemId: anchorId,
        contextMenu: closedContextMenu(),
        statusMessage: buildSelectionStatus(selectedItemIds, getActiveItems(state)),
      };
    }

    case 'explorer/selectAllVisible': {
      const selectedItemIds = keepKnownItemIds(action.visibleItemIds, getActiveItems(state));
      return {
        ...state,
        selectedItemId: selectedItemIds[0] ?? null,
        selectedItemIds,
        selectionAnchorItemId: selectedItemIds[0] ?? null,
        contextMenu: closedContextMenu(),
        statusMessage: selectedItemIds.length > 0 ? buildSelectionStatus(selectedItemIds, getActiveItems(state)) : localeText("main", "ui.appReducer.e9747aaa7828df2b"),
      };
    }

    case 'explorer/clearSelection': {
      return {
        ...state,
        selectedItemId: null,
        selectedItemIds: [],
        selectionAnchorItemId: null,
        contextMenu: closedContextMenu(),
        statusMessage: '',
      };
    }

    case 'explorer/openFolder': {
      const folder = state.explorerItems.find((item) => item.id === action.folderId && item.kind === 'folder');
      return {
        ...state,
        selectedItemId: folder?.id ?? state.selectedItemId,
        selectedItemIds: folder ? [folder.id] : state.selectedItemIds,
        selectionAnchorItemId: folder?.id ?? state.selectionAnchorItemId,
        contextMenu: closedContextMenu(),
        statusMessage: folder ? localeText("main", "ui.appReducer.0a8baec332c4700d", {"value1": (folder.name)}) : localeText("main", "ui.appReducer.1ac19b8581048da1"),
      };
    }

    case 'explorer/openContextMenu': {
      const target = action.itemId ? findKnownItemById(state, action.itemId) : null;
      const selectedItemIds = target && !state.selectedItemIds.includes(target.id) ? [target.id] : state.selectedItemIds;
      const selectedItemId = target?.id ?? state.selectedItemId;

      return {
        ...state,
        selectedItemId,
        selectedItemIds,
        selectionAnchorItemId: target?.id ?? state.selectionAnchorItemId,
        contextMenu: {
          isOpen: true,
          x: action.x,
          y: action.y,
          itemId: target?.id ?? null,
        },
      };
    }

    case 'explorer/closeContextMenu': {
      return { ...state, contextMenu: closedContextMenu() };
    }

    case 'explorer/removeImageByPath': {
      const allKnownItems = [
        ...state.explorerItems,
        ...state.search.results,
        ...state.dragDrop.items,
      ];

      const deletedIds = collectDeletedImageIds(allKnownItems, action.itemId ?? null, action.path);

      const explorerItems = removeDeletedImageFromItems(state.explorerItems, action.itemId ?? null, action.path);
      const searchResults = removeDeletedImageFromItems(state.search.results, action.itemId ?? null, action.path) as typeof state.search.results;
      const dragDropItems = removeDeletedImageFromItems(state.dragDrop.items, action.itemId ?? null, action.path);

      const selectedItemIds = state.selectedItemIds.filter((id) => !deletedIds.has(id));
      const activeItems = state.activeWorkspace !== 'explorer' ? searchResults : explorerItems;

      const selectedItemId = state.selectedItemId && !deletedIds.has(state.selectedItemId)
        ? state.selectedItemId
        : selectedItemIds[0] ?? activeItems[0]?.id ?? null;

      const nextSelectedItemIds = selectedItemIds.length > 0
        ? selectedItemIds
        : selectedItemId
          ? [selectedItemId]
          : [];

      return {
        ...state,
        explorerItems,
        search: {
          ...state.search,
          results: searchResults,
          resultCount: Math.max(0, state.search.resultCount - (state.search.results.length - searchResults.length)),
        },
        dragDrop: {
          ...state.dragDrop,
          items: dragDropItems,
          imageCount: Math.max(0, state.dragDrop.imageCount - (state.dragDrop.items.length - dragDropItems.length)),
        },
        selectedItemId,
        selectedItemIds: nextSelectedItemIds,
        selectionAnchorItemId: selectedItemId,
        contextMenu: closedContextMenu(),
        taskSummary: summarizeImages(activeItems, state.taskSummary.currentTask),
        statusMessage: localeText("main", "ui.appReducer.ab96799396d28c67"),
      };
    }

    case 'dragDrop/setOver': {
      return {
        ...state,
        dragDrop: { ...state.dragDrop, isDraggingOver: action.value },
        statusMessage: action.value ? localeText("main", "ui.appReducer.1b1b6e12faabb6f9") : state.statusMessage,
      };
    }

    case 'dragDrop/inspectStarted': {
      return {
        ...state,
        dragDrop: {
          ...emptyDragDropState(),
          isDraggingOver: true,
          isInspecting: true,
          paths: action.paths,
        },
        contextMenu: closedContextMenu(),
        statusMessage: localeText("main", "ui.appReducer.5e08620a13822e95", {"count": (action.paths.length)}),
      };
    }

    case 'dragDrop/inspectSucceeded': {
      const selectedImageIds = action.items.filter((item): item is ImageItem => item.kind === 'image').map((item) => item.id);
      return {
        ...state,
        selectedItemId: selectedImageIds[0] ?? state.selectedItemId,
        selectedItemIds: selectedImageIds.length > 0 ? selectedImageIds : state.selectedItemIds,
        selectionAnchorItemId: selectedImageIds[0] ?? state.selectionAnchorItemId,
        dragDrop: {
          isDraggingOver: false,
          isInspecting: false,
          paths: action.paths,
          items: action.items,
          imageCount: action.imageCount,
          folderCount: action.folderCount,
          fileCount: action.fileCount,
          unsupportedCount: action.unsupportedCount,
          errorMessage: null,
          droppedAt: new Date().toISOString(),
        },
        statusMessage: localeText("main", "ui.appReducer.5b4a7354d7aeead7", {"folders": (action.folderCount), "images": (action.imageCount), "files": (action.fileCount)}),
      };
    }

    case 'dragDrop/inspectFailed': {
      return {
        ...state,
        dragDrop: { ...emptyDragDropState(), errorMessage: action.errorMessage },
        statusMessage: localeText("main", "ui.appReducer.e40770b662ac07f8", {"value1": (action.errorMessage)}),
      };
    }

    case 'dragDrop/clear': {
      return {
        ...state,
        dragDrop: emptyDragDropState(),
        statusMessage: localeText("main", "ui.appReducer.f1e439f2fa8af19b"),
      };
    }

    case 'processing/queueDroppedImages': {
      return {
        ...state,
        activeDetailsTab: 'tasks',
        statusMessage: localeText("main", "ui.appReducer.79c7551ed8e17295"),
      };
    }

    case 'processing/queueSelectedImages': {
      const selectedIds = state.selectedItemIds.length > 0 ? state.selectedItemIds : state.selectedItemId ? [state.selectedItemId] : [];
      const selectedImages = getImagesById(state.explorerItems, selectedIds);
      const skippedCount = selectedIds.length - selectedImages.length;

      if (selectedImages.length === 0) {
        return {
          ...state,
          contextMenu: closedContextMenu(),
          processingSelectionDraft: emptyProcessingDraft(),
          statusMessage: selectedIds.length > 0
            ? localeText("main", "ui.appReducer.f4f98518fdc98970")
            : localeText("main", "ui.appReducer.72c10ecbbc8669b8"),
        };
      }

      const imageIdSet = new Set(selectedImages.map((item) => item.id));
      const explorerItems = state.explorerItems.map((item) => (
        item.kind === 'image' && imageIdSet.has(item.id)
          ? { ...item, status: item.status==='processed'?'processed' as const:'queued' as const }
          : item
      ));
      const taskSummary = summarizeImages(explorerItems, localeText("main", "ui.appReducer.ee88998f2f836b91", {"count": (selectedImages.length)}));

      return {
        ...state,
        explorerItems,
        contextMenu: closedContextMenu(),
        processingSelectionDraft: {
          imageIds: [...imageIdSet],
          imageCount: selectedImages.length,
          skippedCount,
          smartRenameEnabled: state.smartRenameEnabled,
          createdAt: new Date().toISOString(),
        },
        taskSummary,
        statusMessage: skippedCount > 0
          ? localeText("main", "ui.appReducer.8cad1e17ccc80e30", {"count": (selectedImages.length), "skipped": (skippedCount)})
          : localeText("main", "ui.appReducer.90317cb42f2c7b8f", {"count": (selectedImages.length)}),
      };
    }


    case 'renamePreview/start': {
      return {
        ...state,
        activeDetailsTab: 'tasks',
        contextMenu: closedContextMenu(),
        renamePreview: {
          isPreparing: true,
          result: null,
          errorMessage: null,
          imageIds: action.imageIds,
          skippedNonImageCount: action.skippedNonImageCount,
          skippedByLimitCount: action.skippedByLimitCount,
          skippedByCostCount: action.skippedByCostCount,
          config: action.config,targetJobId:action.targetJobId,targetVersion:action.targetVersion,
          createdAt: new Date().toISOString(),
          confirmedAt: null,
        },
        processingSelectionDraft: {
          imageIds: action.imageIds,
          imageCount: action.imageIds.length,
          skippedCount: action.skippedNonImageCount + action.skippedByLimitCount + action.skippedByCostCount,
          smartRenameEnabled: true,
          createdAt: new Date().toISOString(),
        },
        statusMessage: localeText("main", "ui.appReducer.2d54b66a4686d65a", {"count": (action.imageIds.length)}),
      };
    }

    case 'renamePreview/succeeded': {
      const preview = action.result;
      const blocker = preview.errors > 0 || preview.conflicts > 0;
      const message = blocker
        ? localeText("main", "ui.appReducer.c718ef69a690b0b5", {"errors": (preview.errors), "conflicts": (preview.conflicts)})
        : localeText("main", "ui.appReducer.8fa3debcd548579c", {"value1": (preview.ready), "value2": (preview.unchanged)});

      return {
        ...state,
        activeDetailsTab: 'tasks',
        renamePreview: {
          ...state.renamePreview,
          isPreparing: false,
          result: preview,
          errorMessage: null,
        },
        statusMessage: message,
      };
    }

    case 'renamePreview/failed': {
      return {
        ...state,
        activeDetailsTab: 'tasks',
        renamePreview: {
          ...state.renamePreview,
          isPreparing: false,
          result: null,
          errorMessage: action.errorMessage,
        },
        statusMessage: localeText("main", "ui.appReducer.cb4489e4d56ed73c", {"value1": (action.errorMessage)}),
      };
    }

    case 'renamePreview/cancel': {
      return {
        ...state,
        renamePreview: emptyRenamePreview(),
        processingSelectionDraft: emptyProcessingDraft(),
        statusMessage: localeText("main", "ui.appReducer.d1545f111949ec6c"),
      };
    }

    case 'renamePreview/accepted': {
      return {
        ...state,
        renamePreview: {
          ...state.renamePreview,
          confirmedAt: new Date().toISOString(),
        },
        statusMessage: localeText("main", "ui.appReducer.6c876fb0e2b92775"),
      };
    }


    case 'taskQueue/createFromSelection': {
      if (action.tasks.length === 0) {
        return {
          ...state,
          contextMenu: closedContextMenu(),
          taskQueue: {
            ...state.taskQueue,
            runState: 'idle',
            activeTaskId: null,
            lastMessage: localeText("main", "ui.appReducer.d6fd45ee1c889b0f"),
            skippedNonImageCount: action.skippedNonImageCount,
            skippedByLimitCount: action.skippedByLimitCount,
            skippedByCostCount: action.skippedByCostCount,
            config: action.config,
          },
          processingSelectionDraft: emptyProcessingDraft(),
          statusMessage: localeText("main", "ui.appReducer.6aa09924091c0ed4"),
        };
      }

      const queuedItemIds = new Set(action.tasks.map((task) => task.sourceItemId).filter(Boolean));
      const explorerItems = state.explorerItems.map((item) => (
        item.kind === 'image' && queuedItemIds.has(item.id)
          ? { ...item, status: item.status==='processed'?'processed' as const:'queued' as const }
          : item
      ));
      const droppedItems = state.dragDrop.items.map((item) => (
        item.kind === 'image' && queuedItemIds.has(item.id)
          ? { ...item, status: 'queued' as const }
          : item
      ));
      const queue = buildTaskQueue(state.taskQueue, action.tasks, action.config, action.skippedNonImageCount, action.skippedByLimitCount, action.skippedByCostCount);
      const taskSummary = summarizeImages(explorerItems, localeText("main", "ui.appReducer.072589b51d888563", {"count": (action.tasks.length)}));
      const limitSuffix = action.skippedByLimitCount > 0 ? localeText("main", "ui.appReducer.070dc129f4d2f740", {"count": (action.skippedByLimitCount)}) : '';
      const costSuffix = action.skippedByCostCount > 0 ? localeText("main", "ui.appReducer.2d5a4a991d4f6a39", {"count": (action.skippedByCostCount)}) : '';
      const nonImageSuffix = action.skippedNonImageCount > 0 ? localeText("main", "ui.appReducer.0a794f7f077cfe99", {"count": (action.skippedNonImageCount)}) : '';

      return {
        ...state,
        activeDetailsTab: 'tasks',
        explorerItems,
        dragDrop: { ...state.dragDrop, items: droppedItems },
        taskQueue: { ...queue, nativeJobId: undefined, nativeStatus: undefined, nativeRevision: 0, nativeCounters: undefined,nativeSnapshotPending:false,
          preparationKey:action.tasks[0]?.id,pendingPreparation:{key:action.tasks[0].id,tasks:action.tasks,config:action.config,targetJobId:action.targetJobId,targetVersion:action.targetVersion},preparationResult:undefined,preparationState:'preparing',hideFinished:state.taskQueue.hideFinished,nativeJobs:state.taskQueue.nativeJobs,nativeWorkingJobs:state.taskQueue.nativeWorkingJobs,
          priorStatuses:Object.fromEntries(action.tasks.map(task=>[task.targetPath,task.sourceSnapshot?.status??'notProcessed'])) },
        renamePreview: action.config.smartRenameEnabled ? emptyRenamePreview() : state.renamePreview,
        taskSummary,
        contextMenu: closedContextMenu(),
        processingSelectionDraft: {
          imageIds: action.tasks.map((task) => task.sourceItemId).filter((id): id is string => Boolean(id)),
          imageCount: action.tasks.length,
          skippedCount: action.skippedNonImageCount + action.skippedByLimitCount + action.skippedByCostCount,
          smartRenameEnabled: action.config.smartRenameEnabled,
          createdAt: queue.preparedAt,
        },
        statusMessage: localeText("main", "ui.appReducer.5931a79a6aa21036", {"count": (action.tasks.length), "limits": (limitSuffix), "budget": (costSuffix), "other": (nonImageSuffix)}),
      };
    }

    case 'taskQueue/createBlocked': {
      return {
        ...state,
        activeDetailsTab: 'tasks',
        contextMenu: closedContextMenu(),
        taskQueue: {
          ...state.taskQueue,
          lastMessage: action.reason,
          lastErrorMessage:action.reason,pendingPreparation:undefined,
          preparationState:state.taskQueue.preparationState==='preparing'?'failed':state.taskQueue.preparationState,
        },
        renamePreview: emptyRenamePreview(),
        statusMessage: action.reason,
      };
    }

    case 'taskQueue/consentCancelled': {
      return {...state,taskQueue:{...state.taskQueue,preparationState:state.taskQueue.preparationState==='preparing'?undefined:state.taskQueue.preparationState,lastErrorMessage:null,lastMessage:localeText("main", "ui.appReducer.02612faffe6f7f5b")},statusMessage:localeText("main", "ui.appReducer.9c0760be0ef1080b")};
    }

    case 'taskQueue/start': {
      const counters = summarizeTaskQueue(state.taskQueue);
      if (state.taskQueue.tasks.length === 0) {
        return {
          ...state,
          activeDetailsTab: 'tasks',
          statusMessage: localeText("main", "ui.appReducer.6ace438cd4d4983a"),
          taskQueue: { ...state.taskQueue, lastMessage: localeText("main", "ui.appReducer.4c43e57174deb651") },
        };
      }

      if (counters.pending === 0 && counters.running === 0) {
        return {
          ...state,
          activeDetailsTab: 'tasks',
          statusMessage: localeText("main", "ui.appReducer.3d040d2d89115353"),
          taskQueue: { ...state.taskQueue, runState: 'completed', lastMessage: localeText("main", "ui.appReducer.33e06d9cb99dad05") },
        };
      }

      return {
        ...state,
        activeDetailsTab: 'tasks',
        taskQueue: {
          ...state.taskQueue,
          runState: 'running',
          startedAt: state.taskQueue.startedAt ?? new Date().toISOString(),
          finishedAt: null,
          lastMessage: localeText("main", "ui.appReducer.2e8599281f856547"),
        },
        statusMessage: localeText("main", "ui.appReducer.586bdee6b9d796cd"),
      };
    }

    case 'taskQueue/pause': {
      if (state.taskQueue.runState !== 'running') return state;
      return {
        ...state,
        taskQueue: { ...state.taskQueue, runState: 'paused', lastMessage: localeText("main", "ui.appReducer.c3948c6ae06f107c") },
        statusMessage: localeText("main", "ui.appReducer.e95e0782b5b15fa8"),
      };
    }

    case 'taskQueue/resume': {
      if (state.taskQueue.runState !== 'paused') return state;
      return {
        ...state,
        taskQueue: { ...state.taskQueue, runState: 'running', lastMessage: localeText("main", "ui.appReducer.4c1205964d433099") },
        statusMessage: localeText("main", "ui.appReducer.a14a90490122c929"),
      };
    }

    case 'taskQueue/stop': {
      const taskQueue = updateQueueTasks(state.taskQueue, (task) => (
        task.status === 'pending' || task.status === 'running'
          ? { ...task, status: 'cancelled', progress: task.progress, finishedAt: new Date().toISOString(), errorMessage: localeText("main", "ui.appReducer.c00e4b985372c10c") }
          : task
      ));
      const cancelledItemIds = new Set(taskQueue.tasks.filter((task) => task.status === 'cancelled').map((task) => task.sourceItemId).filter(Boolean));
      const explorerItems = state.explorerItems.map((item) => (
        item.kind === 'image' && cancelledItemIds.has(item.id) && item.status === 'queued'
          ? { ...item, status: 'notProcessed' as const }
          : item
      ));

      return {
        ...state,
        explorerItems,
        taskSummary: summarizeImages(explorerItems, localeText("main", "ui.appReducer.7a034e1539279880")),
        taskQueue: {
          ...taskQueue,
          runState: 'stopped',
          activeTaskId: null,
          finishedAt: new Date().toISOString(),
          lastMessage: localeText("main", "ui.appReducer.46f473d9599060e4"),
        },
        statusMessage: localeText("main", "ui.appReducer.e084ee23e7fd15bc"),
      };
    }

    case 'taskQueue/clearFinished': {
      const tasks = state.taskQueue.tasks.filter((task) => task.status === 'pending' || task.status === 'running');
      const countersBefore = summarizeTaskQueue(state.taskQueue);
      return {
        ...state,
        taskQueue: {
          ...state.taskQueue,
          tasks,
          runState: tasks.length > 0 ? state.taskQueue.runState : 'idle',
          activeTaskId: tasks.find((task) => task.status === 'running')?.id ?? null,
          lastMessage: countersBefore.finished > 0 ? localeText("main", "ui.appReducer.ddee8c723e0fc205", {"count": (countersBefore.finished)}) : localeText("main", "ui.appReducer.b9cb1c64bdb20f8a"),
        },
        statusMessage: countersBefore.finished > 0 ? localeText("main", "ui.appReducer.f278b3e6824008ef", {"count": (countersBefore.finished)}) : localeText("main", "ui.appReducer.e368c2f2c6396cb6"),
      };
    }

    case 'taskQueue/taskStarted': {
      const task = state.taskQueue.tasks.find((candidate) => candidate.id === action.taskId) ?? null;
      if (!task || task.status !== 'pending') return state;
      const explorerItems = state.explorerItems.map((item) => (
        item.kind === 'image' && item.id === task.sourceItemId
          ? { ...item, status: 'queued' as const }
          : item
      ));

      return {
        ...state,
        explorerItems,
        taskQueue: updateQueueTask(state.taskQueue, action.taskId, {
          status: 'running',
          progress: 5,
          startedAt: new Date().toISOString(),
          nextRetryAt: undefined,
          attemptCount: task.attemptCount + 1,
        }, { activeTaskId: action.taskId, lastMessage: `Running: ${task.fileName}` }),
        taskSummary: summarizeImages(explorerItems, localeText("main", "ui.appReducer.3157c0ee646b0e6b", {"value1": (task.fileName)})),
        statusMessage: localeText("main", "ui.appReducer.5304afa6431a0ce9", {"value1": (task.fileName)}),
      };
    }

    case 'taskQueue/taskProgress': {
      const task = state.taskQueue.tasks.find((candidate) => candidate.id === action.taskId) ?? null;
      if (!task || task.status !== 'running') return state;
      return {
        ...state,
        taskQueue: updateQueueTask(state.taskQueue, action.taskId, { progress: clampInteger(action.progress, 0, 99) }, { lastMessage: `Processing ${task.fileName}: ${clampInteger(action.progress, 0, 99)}%` }),
      };
    }

    case 'taskQueue/taskSucceeded': {
      const task = state.taskQueue.tasks.find((candidate) => candidate.id === action.taskId) ?? null;
      if (!task || task.status !== 'running') return state;
      const explorerItems = state.explorerItems.map((item) => (
        item.kind === 'image' && item.id === task.sourceItemId
          ? {
              ...item,
              status: 'processed' as const,
              description: action.result?.description?.trim() || item.description?.trim() || localeText("main", "ui.appReducer.557e445499b4d97b"),
              tags: action.result?.tags?.length ? action.result.tags : item.tags,
            }
          : item
      ));
      const nextQueue = updateQueueTask(state.taskQueue, action.taskId, {
        status: 'done',
        progress: 100,
        finishedAt: new Date().toISOString(),
        analysisDescription: action.result?.description,
        analysisTags: action.result?.tags,
        analysisConfidence: action.result?.confidence,
        analysisSource: action.result?.source,
        errorMessage: undefined,
        errorCode: undefined,
        retryable: undefined,
        nextRetryAt: undefined,
      });
      const counters = summarizeTaskQueue(nextQueue);
      const runState = counters.pending === 0 && counters.running === 0 ? 'completed' : nextQueue.runState;

      return {
        ...state,
        explorerItems,
        taskSummary: summarizeImages(explorerItems, counters.pending === 0 ? localeText("main", "ui.appReducer.00a5bf1b5e0b3742") : `Completed: ${task.fileName}`),
        taskQueue: {
          ...nextQueue,
          runState,
          activeTaskId: null,
          finishedAt: runState === 'completed' ? new Date().toISOString() : nextQueue.finishedAt,
          lastMessage: runState === 'completed' ? localeText("main", "ui.appReducer.5536ae2e7b332c05", {"value1": (counters.done), "value2": (counters.failed)}) : `Completed: ${task.fileName}`,
          lastErrorMessage: runState === 'completed' && counters.failed === 0 ? null : nextQueue.lastErrorMessage,
          lastErrorCode: runState === 'completed' && counters.failed === 0 ? null : nextQueue.lastErrorCode,
          consecutiveErrorCount: 0,
        },
        statusMessage: runState === 'completed' ? localeText("main", "ui.appReducer.fb2a7ce9999a9649", {"value1": (counters.done), "value2": (counters.failed)}) : `Completed AI analysis: ${task.fileName}`,
      };
    }

    case 'taskQueue/taskRenameSucceeded': {
      const task = state.taskQueue.tasks.find((candidate) => candidate.id === action.taskId) ?? null;
      if (!task || task.status !== 'running') return state;

      const explorerItems = applyFileRename(state.explorerItems, action.result);
      const searchResults = applyFileRename(state.search.results, action.result);
      const droppedItems = applyFileRename(state.dragDrop.items, action.result);
      const nextQueue = updateQueueTask(state.taskQueue, action.taskId, {
        renameWasAttempted: true,
        renameWasRenamed: action.result.wasRenamed,
        renameOldPath: action.result.oldPath,
        renameNewPath: action.result.newPath,
        renameOldFileName: action.result.oldFileName,
        renameNewFileName: action.result.newFileName,
        targetPath: action.result.newPath,
        fileName: action.result.newFileName,
      }, {
        lastMessage: action.result.wasRenamed
          ? `Renamed: ${action.result.oldFileName} → ${action.result.newFileName}`
          : action.result.message,
      });

      return {
        ...state,
        explorerItems,
        selectedItemId: state.selectedItemId === task.sourceItemId ? task.sourceItemId : state.selectedItemId,
        taskQueue: nextQueue,
        taskSummary: summarizeImages(explorerItems, action.result.wasRenamed ? `Renamed: ${action.result.newFileName}` : action.result.message),
        search: { ...state.search, results: searchResults },
        dragDrop: { ...state.dragDrop, items: droppedItems },
        statusMessage: action.result.wasRenamed ? localeText("main", "ui.appReducer.a77d824ccf2550f9", {"value1": (action.result.newFileName)}) : action.result.message,
      };
    }

    case 'taskQueue/taskFailed': {
      const task = state.taskQueue.tasks.find((candidate) => candidate.id === action.taskId) ?? null;
      if (!task || task.status !== 'running') return state;

      const retryable = action.retryable === true;
      const attemptsLeft = task.attemptCount < task.maxAttempts;
      const shouldAutoRetry = retryable && attemptsLeft;
      const retryDelayMs = shouldAutoRetry
        ? clampInteger(action.retryAfterMs ?? state.taskQueue.config.requestDelayMs * 2, 500, 120000)
        : 0;
      const nextRetryAt = shouldAutoRetry ? new Date(Date.now() + retryDelayMs).toISOString() : undefined;

      if (shouldAutoRetry) {
        const retryQueue = updateQueueTask(state.taskQueue, action.taskId, {
          status: 'pending',
          progress: 0,
          finishedAt: undefined,
          errorMessage: action.errorMessage,
          errorCode: action.errorCode,
          retryable: true,
          nextRetryAt,
        }, {
          activeTaskId: null,
          lastErrorMessage: action.errorMessage,
          lastErrorCode: action.errorCode ?? null,
          consecutiveErrorCount: state.taskQueue.consecutiveErrorCount + 1,
          lastMessage: localeText("main", "ui.appReducer.e5be988a76dfa322", {"value1": (task.fileName), "value2": (task.attemptCount + 1), "value3": (task.maxAttempts), "value4": (Math.round(retryDelayMs / 1000))}),
        });

        return {
          ...state,
          taskQueue: retryQueue,
          statusMessage: `Temporary AI error. Retrying ${task.fileName} in ${Math.round(retryDelayMs / 1000)} sec.` ,
        };
      }

      const explorerItems = state.explorerItems.map((item) => (
        item.kind === 'image' && item.id === task.sourceItemId
          ? { ...item, status: 'error' as const }
          : item
      ));
      const failedQueue = updateQueueTask(state.taskQueue, action.taskId, {
        status: 'failed',
        progress: 100,
        finishedAt: new Date().toISOString(),
        errorMessage: action.errorMessage,
        errorCode: action.errorCode,
        retryable,
        nextRetryAt: undefined,
      }, {
        lastErrorMessage: action.errorMessage,
        lastErrorCode: action.errorCode ?? null,
        consecutiveErrorCount: state.taskQueue.consecutiveErrorCount + 1,
      });
      const shouldStop = failedQueue.config.stopOnFirstError || (task.smartRenameEnabled === true && failedQueue.config.renameStopOnFirstError);
      const stoppedQueue = shouldStop
        ? updateQueueTasks(failedQueue, (candidate) => candidate.status === 'pending'
          ? { ...candidate, status: 'cancelled' as QueueTaskStatus, finishedAt: new Date().toISOString(), errorMessage: localeText("main", "ui.appReducer.307c27f621e775ab") }
          : candidate)
        : failedQueue;
      const counters = summarizeTaskQueue(stoppedQueue);
      const hasMoreWork = counters.pending > 0 || counters.running > 0;
      const runState = shouldStop || !hasMoreWork ? 'completed' : stoppedQueue.runState;
      const retryHint = retryable && !attemptsLeft ? localeText("main", "ui.appReducer.2d75012eb8b824d6", {"value1": (task.maxAttempts), "value2": (task.maxAttempts)}) : '';

      return {
        ...state,
        explorerItems,
        taskSummary: summarizeImages(explorerItems, `Failed: ${task.fileName}`),
        taskQueue: {
          ...stoppedQueue,
          runState,
          activeTaskId: null,
          finishedAt: runState === 'completed' ? new Date().toISOString() : stoppedQueue.finishedAt,
          lastMessage: shouldStop ? localeText("main", "ui.appReducer.d2437c8085ac1bac", {"value1": (action.errorMessage), "value2": (retryHint)}) : localeText("main", "ui.appReducer.0ab88f717daa304b", {"value1": (action.errorMessage), "value2": (retryHint)}),
        },
        statusMessage: shouldStop ? localeText("main", "ui.appReducer.367c3c43c9c6193f", {"value1": (action.errorMessage), "value2": (retryHint)}) : localeText("main", "ui.appReducer.8f7b5ade3908162b", {"value1": (action.errorMessage), "value2": (retryHint)}),
      };
    }

    case 'taskQueue/retryFailed': {
      const failedTasks = state.taskQueue.tasks.filter((task) => task.status === 'failed');
      if (failedTasks.length === 0) {
        return {
          ...state,
          taskQueue: { ...state.taskQueue, lastMessage: localeText("main", "ui.appReducer.e9dfdf710fd2662f") },
          statusMessage: localeText("main", "ui.appReducer.27297ce58b2bea72"),
        };
      }

      const failedItemIds = new Set(failedTasks.map((task) => task.sourceItemId).filter(Boolean));
      const explorerItems = state.explorerItems.map((item) => (
        item.kind === 'image' && failedItemIds.has(item.id)
          ? { ...item, status: 'queued' as const }
          : item
      ));
      const tasks = state.taskQueue.tasks.map((task) => task.status === 'failed'
        ? {
            ...task,
            status: 'pending' as QueueTaskStatus,
            progress: 0,
            startedAt: undefined,
            finishedAt: undefined,
            errorMessage: undefined,
            errorCode: undefined,
            retryable: undefined,
            nextRetryAt: undefined,
            attemptCount: 0,
            maxAttempts: Math.max(1, state.taskQueue.config.maxRetriesPerTask + 1),
          }
        : task);

      return {
        ...state,
        explorerItems,
        activeDetailsTab: 'tasks',
        taskSummary: summarizeImages(explorerItems, localeText("main", "ui.appReducer.a03f118b7396bc06", {"count": (failedTasks.length)})),
        taskQueue: {
          ...state.taskQueue,
          tasks,
          runState: 'idle',
          activeTaskId: null,
          finishedAt: null,
          lastErrorMessage: null,
          lastErrorCode: null,
          consecutiveErrorCount: 0,
          lastMessage: localeText("main", "ui.appReducer.e6c7eb0a8a5537da", {"count": (failedTasks.length)}),
        },
        statusMessage: localeText("main", "ui.appReducer.d253c2d0e3e3c261", {"count": (failedTasks.length)}),
      };
    }

    case 'filesystem/scanStarted': {
      return {
        ...state,
        activeWorkspace: 'explorer',
        filenameFilter:action.requestedPath===state.currentPath?state.filenameFilter:'',
        isScanningFolder: true,
        isFolderScanActive: true,
        filesystemError: null,
        selectedItemId: action.preserveSelection ? state.selectedItemId : null,
        selectedItemIds: action.preserveSelection ? state.selectedItemIds : [],
        selectionAnchorItemId: action.preserveSelection ? state.selectionAnchorItemId : null,
        contextMenu: closedContextMenu(),
        statusMessage: action.requestedPath ? localeText("main", "ui.appReducer.e69f43c1d0960768", {"value1": (action.requestedPath)}) : localeText("main", "ui.appReducer.9ca7eca4e540d394"),
      };
    }

    case 'filesystem/scanFinished': {
      if(state.currentPath!==action.requestedPath)return state;
      const known=new Set(state.explorerItems.map(item=>item.id));
      const selectedItemIds=state.selectedItemIds.filter(id=>known.has(id));
      const selectedItemId=state.selectedItemId && known.has(state.selectedItemId) ? state.selectedItemId : selectedItemIds[0]??null;
      return {...state,isFolderScanActive:false,selectedItemIds,selectedItemId,selectionAnchorItemId:state.selectionAnchorItemId && known.has(state.selectionAnchorItemId) ? state.selectionAnchorItemId : selectedItemId};
    }
    case 'filesystem/pageSucceeded': {
      if (state.currentPath !== action.result.currentPath) return state;
      const paths = new Set(state.explorerItems.map(item => item.path));
      const ids = new Set(state.explorerItems.map(item => item.id));
      const added=action.result.items.filter(item => {
        if(paths.has(item.path)||ids.has(item.id))return false;
        paths.add(item.path);ids.add(item.id);return true;
      });
      const items = [...state.explorerItems, ...added];
      return { ...state, explorerItems: items, explorerSortSnapshot:{...state.explorerSortSnapshot,...Object.fromEntries(added.map(item=>[item.id,item]))},explorerDisplayOrder:state.explorerDisplayOrder, scannedDirectories: action.result.scannedDirectories,
        skippedEntries: action.result.skippedEntries, taskSummary: summarizeImages(items, localeText("main", "ui.appReducer.ca154b13e3cb48c5")),
        statusMessage: localeText("main", "ui.appReducer.36c76f16746c986c", {"value1": (items.length)}) };
    }
    case 'filesystem/scanSucceeded': {
      const firstSelectable = action.result.items.find((item) => item.kind === 'image')
        ?? action.result.items.find((item) => item.kind === 'folder')
        ?? action.result.items[0]
        ?? null;

      const taskSummary = summarizeImages(action.result.items, localeText("main", "ui.appReducer.776f7ff6760c0870"));
      const folderCount = action.result.items.filter((item) => item.kind === 'folder').length;
      const imageCount = action.result.items.filter((item) => item.kind === 'image').length;
      const fileCount = action.result.items.filter((item) => item.kind === 'file').length;
      const truncatedSuffix = action.result.truncated ? localeText("main", "ui.appReducer.a51b82d789a8206c") : '';

      return {
        ...state,
        currentPath: action.result.currentPath,
        search:state.search.rootPath===state.currentPath&&Boolean(state.search.rootPath)?{...state.search,rootPath:action.result.currentPath??''}:state.search,
        parentPath: action.result.parentPath,
        pathSegments: action.result.pathSegments,
        breadcrumbs: action.result.breadcrumbs,
        ...buildNavigationState(state, action.result.currentPath, action.navigationMode, action.historyTargetIndex),
        explorerItems: action.result.items,
        explorerDisplayOrder: sortExplorerItems(action.result.items,state.sortMode,state.folderGrouping).map(item=>item.id),
        explorerSortSnapshot:Object.fromEntries(action.result.items.map(item=>[item.id,item])),
        selectedItemId: action.preserveSelection || state.activeWorkspace!=='explorer' ? state.selectedItemId : firstSelectable?.id ?? null,
        selectedItemIds: action.preserveSelection || state.activeWorkspace!=='explorer' ? state.selectedItemIds : firstSelectable ? [firstSelectable.id] : [],
        selectionAnchorItemId: action.preserveSelection || state.activeWorkspace!=='explorer' ? state.selectionAnchorItemId : firstSelectable?.id ?? null,
        contextMenu: closedContextMenu(),
        isScanningFolder: false,
        filesystemError: null,
        scannedDirectories: action.result.scannedDirectories,
        skippedEntries: action.result.skippedEntries,
        scanWasTruncated: action.result.truncated,
        taskSummary,
        statusMessage: localeText("main", "ui.appReducer.0689dbe66e9a3b16", {"value1": (folderCount), "value2": (imageCount), "value3": (fileCount), "value4": (action.result.currentPath), "value5": (truncatedSuffix)}),
      };
    }

    case 'filesystem/scanFailed': {
      return {
        ...state,
        isScanningFolder: false,
        isFolderScanActive: false,
        filesystemError: action.errorMessage,
        contextMenu: closedContextMenu(),
        statusMessage: localeText("main", "ui.appReducer.07bb1e0f9e772601", {"value1": (action.errorMessage)}),
      };
    }


    case 'index/countsUpdated': {
      return { ...state, indexDatabase: { ...state.indexDatabase, ...action.counts } };
    }
    case 'index/initializeStarted': {
      return {
        ...state,
        indexDatabase: {
          ...state.indexDatabase,
          isInitializing: true,
          errorMessage: null,
        },
        statusMessage: localeText("main", "ui.appReducer.9647872b176756c8"),
      };
    }

    case 'index/initializeSucceeded': {
      return {
        ...state,
        indexDatabase: {
          ...state.indexDatabase,
          isInitializing: false,
          isReady: true,
          errorMessage: null,
          databasePath: action.info.databasePath,
          schemaVersion: action.info.schemaVersion,
          migrationsApplied: action.info.migrationsApplied,
          tables: action.info.tables,
          imageCount: action.info.imageCount,
          pathCount: action.info.pathCount,
          tagCount: action.info.tagCount,
        },
        statusMessage: action.info.migrationsApplied > 0
          ? localeText("main", "ui.appReducer.b1cc2a17c225c015", {"count": (action.info.migrationsApplied)})
          : localeText("main", "ui.appReducer.ff975e666eada3ec", {"count": (action.info.imageCount)}),
      };
    }

    case 'index/initializeFailed': {
      return {
        ...state,
        indexDatabase: {
          ...state.indexDatabase,
          isInitializing: false,
          isReady: false,
          errorMessage: action.errorMessage,
        },
        statusMessage: localeText("main", "ui.appReducer.7ec6b37acb85da73", {"value1": (action.errorMessage)}),
      };
    }

    case 'index/syncStarted': {
      return {
        ...state,
        indexDatabase: {
          ...state.indexDatabase,
          errorMessage: null,
        },
        statusMessage: localeText("main", "ui.appReducer.29edbffea114f955", {"count": (action.imageCount)}),
      };
    }

    case 'index/syncSucceeded': {
      const explorerItems = applyIndexedImagesToExplorerItems(state.explorerItems, action.result);
      const droppedItems = applyIndexedImagesToExplorerItems(state.dragDrop.items, action.result);
      const taskSummary = summarizeImages(explorerItems, localeText("main", "ui.appReducer.7eeab59904f2d94a"));

      return {
        ...state,
        search:{...state.search,results:applyIndexedImagesToExplorerItems(state.search.results,action.result) as ImageItem[]},
        explorerItems,
        dragDrop: { ...state.dragDrop, items: droppedItems },
        taskSummary,
        indexDatabase: {
          ...state.indexDatabase,
          isReady: true,
          errorMessage: null,
          databasePath: action.result.databasePath,
          schemaVersion: action.result.schemaVersion,
          imageCount: action.result.imageCount,
          pathCount: action.result.pathCount,
          lastSyncAtUnix: action.result.syncedAtUnix,
          lastSyncedImages: action.result.insertedOrUpdatedImages,
          lastKnownImages: action.result.knownImages,
          lastNewImages: action.result.newImages,
          lastRelocatedImages: action.result.relocatedImages,
        },
        statusMessage: localeText("main", "ui.appReducer.b0479ec7faf8d2fe", {"value1": (action.result.newImages), "value2": (action.result.relocatedImages)}),
      };
    }

    case 'index/syncFailed': {
      return {
        ...state,
        indexDatabase: {
          ...state.indexDatabase,
          errorMessage: action.errorMessage,
        },
        statusMessage: localeText("main", "ui.appReducer.9ef1cc9ee5ca8d2e", {"value1": (action.errorMessage)}),
      };
    }

    case 'index/rescanStarted': {
      return {
        ...state,
        indexDatabase: {
          ...state.indexDatabase,
          errorMessage: null,
        },
        statusMessage: action.rootPath
          ? localeText("main", "ui.appReducer.40bb70d3efcbe8cc", {"value1": (action.rootPath)})
          : localeText("main", "ui.appReducer.30df389bc4f39d92"),
      };
    }

    case 'index/rescanSucceeded': {
      return {
        ...state,
        indexDatabase: {
          ...state.indexDatabase,
          isReady: true,
          errorMessage: null,
          databasePath: action.result.databasePath,
          imageCount: action.result.imageCount,
          pathCount: action.result.pathCount,
          lastRescanAtUnix: action.result.rescannedAtUnix,
          lastRescanCheckedPaths: action.result.checkedPaths,
          lastMissingImages: action.result.missingPaths,
          lastRevivedImages: action.result.revivedPaths,
        },
        statusMessage: localeText("main", "ui.appReducer.8daaafcaaea1f630", {"value1": (action.result.checkedPaths), "value2": (action.result.missingPaths), "value3": (action.result.revivedPaths)}),
      };
    }

    case 'index/rescanFailed': {
      return {
        ...state,
        indexDatabase: {
          ...state.indexDatabase,
          errorMessage: action.errorMessage,
        },
        statusMessage: localeText("main", "ui.appReducer.dc0cf03971d58f94", {"value1": (action.errorMessage)}),
      };
    }



    case 'index/metadataUpdateStarted': {
      return {
        ...state,
        statusMessage: localeText("main", "ui.appReducer.77b1a109382c1936", {"value1": (action.imageName)}),
      };
    }

    case 'index/metadataUpdateSucceeded': {
      const explorerItems = applyMetadataUpdateToExplorerItems(state.explorerItems, action.result);
      const droppedItems = applyMetadataUpdateToExplorerItems(state.dragDrop.items, action.result);
      const searchResults = applyMetadataUpdateToExplorerItems(state.search.results, action.result) as typeof state.search.results;
      const taskSummary = summarizeImages(explorerItems, localeText("main", "ui.appReducer.6a247ba66cc3d955"));

      return {
        ...state,
        explorerItems,
        search: { ...state.search, results: searchResults },
        dragDrop: { ...state.dragDrop, items: droppedItems },
        taskSummary,
        indexDatabase: {
          ...state.indexDatabase,
          imageCount: action.result.imageCount,
          pathCount: action.result.pathCount,
          tagCount: action.result.tagCount,
        },
        statusMessage: localeText("main", "ui.appReducer.67068f9c72ba9a3a", {"value1": (action.result.fileName)}),
      };
    }

    case 'index/metadataUpdateFailed': {
      return {
        ...state,
        statusMessage: localeText("main", "ui.appReducer.0c913dff317094d5", {"value1": (action.errorMessage)}),
      };
    }

    case 'toolbar/setIncludeSubfolders': {
      return {
        ...state,
        includeSubfolders: action.value,
        statusMessage: action.value ? localeText("main", "ui.appReducer.33ab2721626958c9") : localeText("main", "ui.appReducer.2bf34741921fd6b2"),
      };
    }

    case 'toolbar/setShowImagesOnly': {
      return {
        ...state,
        showImagesOnly: action.value,
        statusMessage: action.value ? localeText("main", "ui.appReducer.4fe328a82843591f") : localeText("main", "ui.appReducer.5a23fcbcf1784306"),
      };
    }

    case 'toolbar/setSmartRename': {
      return persistSettings(state, {
        ...state.settings,
        rename: { ...state.settings.rename, enabled: action.value },
      }, action.value
        ? localeText("main", "ui.appReducer.2018c55c0ecb02e8")
        : localeText("main", "ui.appReducer.ce9c5d969174fc79"));
    }

    case 'toolbar/setViewSize': {
      return { ...state, viewSize: action.value };
    }

    case 'toolbar/setFolderGrouping':return {...state,folderGrouping:'foldersFirst',explorerDisplayOrder:sortExplorerItems(state.explorerItems,state.sortMode,'foldersFirst').map(item=>item.id),explorerSortSnapshot:Object.fromEntries(state.explorerItems.map(item=>[item.id,item]))};
    case 'toolbar/setSortMode': {
      return { ...state, sortMode: action.value, explorerDisplayOrder:sortExplorerItems(state.explorerItems,action.value,state.folderGrouping).map(item=>item.id),explorerSortSnapshot:Object.fromEntries(state.explorerItems.map(item=>[item.id,item])) };
    }

    case 'privacy/setAiConsent': {
      const acceptedAt = action.value ? new Date().toISOString() : null;
      return persistSettings(state, {
        ...state.settings,
        privacy: {
          ...state.settings.privacy,
          aiConsentAccepted: action.value,
          consentAcceptedAt: acceptedAt,
          externalAiProviderNoticeSeen: action.value || state.settings.privacy.externalAiProviderNoticeSeen,
        },
      }, action.value
        ? localeText("main", "ui.appReducer.a58587bcde71125b")
        : localeText("main", "ui.appReducer.1892f1afe32b8781"));
    }

    case 'settings/togglePanel': {
      return {
        ...state,
        settingsPanelOpen: !state.settingsPanelOpen,
        statusMessage: state.settingsPanelOpen ? messages().settings.messages.panelClosed : messages().settings.messages.panelOpened,
      };
    }

    case 'settings/setPanelOpen': {
      return {
        ...state,
        settingsPanelOpen: action.value,
        statusMessage: action.value ? messages().settings.messages.panelOpened : messages().settings.messages.panelClosed,
      };
    }

    case 'settings/setAiProviderMode': {
      return persistSettings(state, { ...state.settings, aiProviderMode: action.value }, messages().settings.messages.aiProviderModeSet(action.value));
    }

    case 'settings/setOpenRouterApiKey': {
      return persistSettings(state, {
        ...state.settings,
        openRouter: { ...state.settings.openRouter, apiKey: action.value },
      }, action.value.trim() ? messages().settings.messages.apiKeySaved : messages().settings.messages.apiKeyCleared, { apiKeyCheckResult: null });
    }

    case 'settings/setOpenRouterPreset': {
      const selectedModel = resolvePresetModel(action.presetId, state.settings.openRouter.customModel);
      return persistSettings(state, {
        ...state.settings,
        openRouter: {
          ...state.settings.openRouter,
          selectedPresetId: action.presetId,
          selectedModel,
          useFreeRouter: selectedModel === 'openrouter/free' || action.presetId.endsWith(':free'),
        },
      }, messages().settings.messages.presetSelected(selectedModel));
    }

    case 'settings/setOpenRouterCustomModel': {
      const selectedModel = state.settings.openRouter.selectedPresetId === 'custom'
        ? action.value.trim()
        : state.settings.openRouter.selectedModel;
      return persistSettings(state, {
        ...state.settings,
        openRouter: { ...state.settings.openRouter, customModel: action.value, selectedModel },
      }, selectedModel ? localeText("main", "ui.appReducer.e27a72e8e5314b9d", {"value1": (selectedModel)}) : localeText("main", "ui.appReducer.ee010d62eb05583a"));
    }

    case 'settings/setLocalAiPreset': {
      const selectedModel = resolveLocalAiModel(action.presetId, state.settings.localAi.customModel);
      return persistSettings(state, {
        ...state.settings,
        localAi: {
          ...state.settings.localAi,
          selectedPresetId: action.presetId,
          selectedModel,
        },
      }, selectedModel ? localeText("main", "ui.appReducer.e9ed75a42e6d1d63", {"value1": (selectedModel)}) : localeText("main", "ui.appReducer.5d80d7671c461822"));
    }

    case 'settings/setLocalAiCustomModel': {
      const selectedModel = state.settings.localAi.selectedPresetId === 'custom'
        ? action.value.trim()
        : state.settings.localAi.selectedModel;
      return persistSettings(state, {
        ...state.settings,
        localAi: {
          ...state.settings.localAi,
          customModel: action.value,
          selectedModel,
        },
      }, selectedModel ? localeText("main", "ui.appReducer.e6f5869ffc72b38a", {"value1": (selectedModel)}) : localeText("main", "ui.appReducer.0a0df7661f20c2d7"));
    }

    case 'settings/setLocalAiEndpoint': {
      return persistSettings(state, {
        ...state.settings,
        localAi: {
          ...state.settings.localAi,
          endpoint: action.value,
        },
      }, action.value.trim() ? localeText("main", "ui.appReducer.2bda6fe435f9ac2e", {"value1": (action.value.trim())}) : localeText("main", "ui.appReducer.08edacf789d53c25"));
    }

    case 'settings/setShowApiKey': {
      return persistSettings(state, {
        ...state.settings,
        openRouter: { ...state.settings.openRouter, showApiKey: action.value },
      }, action.value ? localeText("main", "ui.appReducer.544171ffbe361bed") : localeText("main", "ui.appReducer.f2189dbfc0981fb6"));
    }

    case 'settings/setMaxImagesPerRun': {
      const value = clampInteger(action.value, 1, 5000);
      return persistSettings(state, {
        ...state.settings,
        openRouter: { ...state.settings.openRouter, maxImagesPerRun: value },
        processingLimits: { ...state.settings.processingLimits, maxImagesPerRun: value },
      }, localeText("main", "ui.appReducer.fa1c57d11587902b", {"value1": (value)}));
    }

    case 'settings/setRequestDelayMs': {
      const value = clampInteger(action.value, 0, 60000);
      return persistSettings(state, {
        ...state.settings,
        openRouter: { ...state.settings.openRouter, requestDelayMs: value },
        processingLimits: { ...state.settings.processingLimits, requestDelayMs: value },
      }, localeText("main", "ui.appReducer.82105e1758479015", {"value1": (value)}));
    }

    case 'settings/setMaxCostUsd': {
      const value = clampNumber(action.value, 0, 1000);
      return persistSettings(state, {
        ...state.settings,
        openRouter: { ...state.settings.openRouter, maxEstimatedCostPerRunUsd: value },
        processingLimits: { ...state.settings.processingLimits, maxEstimatedCostPerRunUsd: value },
      }, localeText("main", "ui.appReducer.80ba7eccc55535d0", {"value1": (value.toFixed(2))}));
    }

    case 'settings/setStopOnFirstAiError': {
      return persistSettings(state, {
        ...state.settings,
        processingLimits: { ...state.settings.processingLimits, stopOnFirstAiError: action.value },
      }, action.value ? localeText("main", "ui.appReducer.0b653d8397af82d1") : localeText("main", "ui.appReducer.e24e6bc67da4848d"));
    }
    case 'settings/setOnlyUnprocessed': {
      return persistSettings(state,{...state.settings,processingLimits:{...state.settings.processingLimits,onlyUnprocessed:action.value}},action.value?localeText("main", "ui.appReducer.553c359025bdd88c"):localeText("main", "ui.appReducer.df8aba00a8ad3b86"));
    }

    case 'settings/setMaxRetriesPerTask': {
      const value = clampInteger(action.value, 0, 5);
      return persistSettings(state, {
        ...state.settings,
        processingLimits: { ...state.settings.processingLimits, maxRetriesPerTask: value },
      }, localeText("main", "ui.appReducer.5c6ba99a2d8e689a", {"value1": (value)}));
    }

    case 'settings/setRenameEnabled': {
      return persistSettings(state, {
        ...state.settings,
        rename: { ...state.settings.rename, enabled: action.value },
      }, action.value ? localeText("main", "ui.appReducer.103ccaa450c32dde") : localeText("main", "ui.appReducer.c88d01464ed1d4b6"));
    }

    case 'settings/setRenameStopOnError': {
      return persistSettings(state, {
        ...state.settings,
        rename: { ...state.settings.rename, stopOnFirstError: action.value },
      }, action.value ? localeText("main", "ui.appReducer.a95564ca63c2ec11") : localeText("main", "ui.appReducer.0d77f40decbbe842"));
    }

    case 'settings/applySettings': {
      return persistSettings(state, action.settings, localeText("main", "ui.appReducer.180affa1358d9e56"));
    }

    case 'settings/checkOpenRouterKey': {
      const result = settingsService.checkOpenRouterKey(state.settings.openRouter.apiKey);
      return { ...state, apiKeyCheckResult: result, statusMessage: result.message };
    }

    case 'settings/resetApiKeyCheck': {
      return { ...state, apiKeyCheckResult: null };
    }

    case 'settings/setLocalAiStatus': {
      return { ...state, localAiStatus: action.status };
    }

    case 'settings/setManualQueue': {
      const projection=state.settings.processingLimits.manualQueue===action.value?state:{...state,taskQueue:{...state.taskQueue,nativeJobId:undefined,nativeStatus:undefined,nativeRevision:0,nativeCounters:undefined,nativeAggregate:undefined,nativeWorkingJobs:undefined,nativeSnapshotPending:false,commandPending:false,tasks:[]}};
      return persistSettings(projection, { ...state.settings, processingLimits: { ...state.settings.processingLimits, manualQueue: action.value } },
        action.value ? localeText("main", "ui.appReducer.6887aea2ce0901c6") : localeText("main", "ui.appReducer.3b583f82432170a8"));
    }

    case 'settings/setOpenRouterStatus': {
      return { ...state, openRouterStatus: action.status };
    }

    case 'app/setStatusMessage': {
      return { ...state, statusMessage: action.message };
    }

    default: {
      return state;
    }
  }
}


function persistSettings(
  state: AppState,
  settings: AppSettings,
  statusMessage: string,
  extra: Partial<AppState> = {},
): AppState {
  settingsService.saveSettings(settings);

  return {
    ...state,
    ...extra,
    settings,
    aiConsentAccepted: settings.privacy.aiConsentAccepted,
    smartRenameEnabled: settings.rename.enabled,
    statusMessage,
  };
}

function formatSizeLabel(sizeBytes: number): string {
  if (!Number.isFinite(sizeBytes) || sizeBytes < 0) return localeText("main", "ui.appReducer.f2c66ac5794e3730");
  if (sizeBytes < 1024) return `${sizeBytes} B`;
  if (sizeBytes < 1024 * 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`;
  if (sizeBytes < 1024 * 1024 * 1024) return `${(sizeBytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(sizeBytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

function clampInteger(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}

function clampNumber(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, value));
}

function buildNavigationState(
  state: AppState,
  currentPath: string,
  navigationMode: 'push' | 'replace' | 'history',
  historyTargetIndex?: number,
): Pick<AppState, 'navigationHistory' | 'navigationIndex'> {
  if (!currentPath.trim()) {
    return { navigationHistory: state.navigationHistory, navigationIndex: state.navigationIndex };
  }

  if (navigationMode === 'history') {
    const index = typeof historyTargetIndex === 'number'
      ? historyTargetIndex
      : state.navigationHistory.findIndex((item) => item === currentPath);

    if (index >= 0 && index < state.navigationHistory.length) {
      return { navigationHistory: state.navigationHistory, navigationIndex: index };
    }

    return buildNavigationState(state, currentPath, 'push');
  }

  if (navigationMode === 'replace') {
    if (state.navigationHistory.length === 0) {
      return { navigationHistory: [currentPath], navigationIndex: 0 };
    }

    const navigationHistory = [...state.navigationHistory];
    const safeIndex = state.navigationIndex >= 0 ? state.navigationIndex : navigationHistory.length - 1;
    navigationHistory[safeIndex] = currentPath;
    return { navigationHistory, navigationIndex: safeIndex };
  }

  if (state.navigationHistory[state.navigationIndex] === currentPath) {
    return { navigationHistory: state.navigationHistory, navigationIndex: state.navigationIndex };
  }

  const beforeCurrent = state.navigationIndex >= 0
    ? state.navigationHistory.slice(0, state.navigationIndex + 1)
    : [];
  const navigationHistory = [...beforeCurrent, currentPath];

  return { navigationHistory, navigationIndex: navigationHistory.length - 1 };
}

function findKnownItemById(state: AppState, itemId: string | null): ExplorerItem | null {
  if (!itemId) return null;
  return [...(state.inspectedImage?[state.inspectedImage]:[]),...state.explorerItems, ...state.search.results, ...state.dragDrop.items].find((item) => item.id === itemId) ?? null;
}

function getActiveItems(state: AppState): ExplorerItem[] {
  const baseItems = state.activeWorkspace !== 'explorer' ? state.search.results : state.explorerItems;
  return [...baseItems, ...state.dragDrop.items];
}

function closedContextMenu(): AppState['contextMenu'] {
  return { isOpen: false, x: 0, y: 0, itemId: null };
}

function emptyDragDropState(): AppState['dragDrop'] {
  return {
    isDraggingOver: false,
    isInspecting: false,
    paths: [],
    items: [],
    imageCount: 0,
    folderCount: 0,
    fileCount: 0,
    unsupportedCount: 0,
    errorMessage: null,
    droppedAt: null,
  };
}

function emptyRenamePreview(): RenamePreviewState {
  return {
    isPreparing: false,
    result: null,
    errorMessage: null,
    imageIds: [],
    skippedNonImageCount: 0,
    skippedByLimitCount: 0,
    skippedByCostCount: 0,
    config: null,
    createdAt: null,
    confirmedAt: null,
  };
}


function emptyProcessingDraft(): ProcessingSelectionDraft {
  return {
    imageIds: [],
    imageCount: 0,
    skippedCount: 0,
    smartRenameEnabled: false,
    createdAt: null,
  };
}

function keepKnownItemIds(itemIds: string[], items: ExplorerItem[]): string[] {
  const knownIds = new Set(items.map((item) => item.id));
  return itemIds.filter((itemId) => knownIds.has(itemId));
}

function buildSelectionStatus(_selectedItemIds: string[], _items: ExplorerItem[]): string {
  // Selection has one dedicated footer owner; keep operation failures available there.
  return '';
}

function getItemsById(items: ExplorerItem[], itemIds: string[]): ExplorerItem[] {
  const selectedIds = new Set(itemIds);
  return items.filter((item) => selectedIds.has(item.id));
}

function getImagesById(items: ExplorerItem[], itemIds: string[]): ImageItem[] {
  return getItemsById(items, itemIds).filter((item): item is ImageItem => item.kind === 'image');
}

function applyIndexedImagesToExplorerItems(items: ExplorerItem[], result: IndexSyncResult): ExplorerItem[] {
  if (result.indexedImages.length === 0) {
    return items;
  }

  const byPath = new Map(result.indexedImages.map((image) => [image.path, image]));

  return items.map((item) => {
    if (item.kind !== 'image') return item;

    const indexed = byPath.get(item.path);
    if (!indexed) return item;

    return {
      ...item,
      status: indexed.status,
      sourceRevision: indexed.sourceRevision ?? item.sourceRevision,
      fileMetadata: indexed.fileMetadata,
      sizeBytes: indexed.fileMetadata?.sizeBytes??item.sizeBytes,
      sizeLabel: typeof indexed.fileMetadata?.sizeBytes==='number'?formatSizeLabel(indexed.fileMetadata.sizeBytes):item.sizeLabel,
      modifiedAtUnix:indexed.fileMetadata?.modifiedAtUnix??item.modifiedAtUnix,
      dateTaken: captureDateLabel(indexed.fileMetadata),
      dimensions: imageDimensions(indexed.fileMetadata),
      description: indexed.description ?? '',
      aiProcessedAtUnix: indexed.aiProcessedAtUnix === undefined ? item.aiProcessedAtUnix : indexed.aiProcessedAtUnix,
      descriptionVersion: indexed.descriptionVersion,
      descriptionFormat:indexed.descriptionFormat,
      tagsVersion: indexed.tagsVersion,
      tags: indexed.tags,
      isFavorite: indexed.isFavorite,
      collections: indexed.collections,
      indexState: indexed.indexState,
      indexedImageId: indexed.imageId,
      previousPath: indexed.previousPath ?? null,
      firstSeenAtUnix: indexed.firstSeenAtUnix ?? null,
      lastSeenAtUnix: indexed.lastSeenAtUnix ?? null,
    };
  });
}


function applyMetadataUpdateToExplorerItems(items: ExplorerItem[], result: IndexMetadataUpdateResult): ExplorerItem[] {
  return items.map((item) => {
    if (item.kind !== 'image') return item;
    const matchesIndexedId = item.indexedImageId && item.indexedImageId === result.imageId;
    const matchesPath = item.path === result.path;
    if (!matchesIndexedId && !matchesPath) return item;

    return {
      ...item,
      status: result.status,
      description: result.description?.trim() || '',
      tags: result.tags,
      indexedImageId: result.imageId,
      aiProcessedAtUnix: result.aiProcessedAtUnix === undefined ? item.aiProcessedAtUnix : result.aiProcessedAtUnix,
      descriptionVersion: result.descriptionVersion,
      descriptionFormat:result.descriptionFormat,
      tagsVersion: result.tagsVersion,
      indexState: item.indexState ?? 'known',
      lastSeenAtUnix: result.updatedAtUnix,
    };
  });
}

function applyFileRename<T extends ExplorerItem>(items: T[], result: SmartRenameResult): T[] {
  return items.map(item => {
    if (item.kind !== 'image' || item.path !== result.oldPath) return item;
    return { ...item, name: result.newFileName, path: result.newPath,
      previewUrl: convertFileSrc(result.newPath),
      sizeBytes: result.sizeBytes ?? item.sizeBytes,
      sizeLabel: result.sizeBytes ? formatSizeLabel(result.sizeBytes) : item.sizeLabel,
      modifiedAtUnix: result.modifiedAtUnix ?? item.modifiedAtUnix };
  });
}




function upsertCollection<T extends { id: string; updatedAtUnix: number; isFavorite: boolean; name: string }>(items: T[], collection: T | null): T[] {
  if (!collection) return items;
  const next = items.some((item) => item.id === collection.id)
    ? items.map((item) => item.id === collection.id ? collection : item)
    : [collection, ...items];
  return next.slice().sort((left, right) => Number(right.isFavorite) - Number(left.isFavorite) || right.updatedAtUnix - left.updatedAtUnix || left.name.localeCompare(right.name));
}

function applyCollectionNameToItems<T extends ExplorerItem>(items: T[], imageIds: Set<string>, collectionName: string): T[] {
  return items.map((item) => {
    if (item.kind !== 'image' || !item.indexedImageId || !imageIds.has(item.indexedImageId)) return item;
    const existing = item.collections ?? [];
    if (existing.includes(collectionName)) return item;
    return { ...item, collections: [...existing, collectionName].sort((left, right) => left.localeCompare(right)) };
  });
}

function applyFavoriteUpdateToItems<T extends ExplorerItem>(items: T[], result: { imageId: string; path: string; isFavorite: boolean }): T[] {
  return items.map((item) => {
    if (item.kind !== 'image') return item;
    const matchesImageId = item.indexedImageId && item.indexedImageId === result.imageId;
    const matchesPath = item.path === result.path;
    if (!matchesImageId && !matchesPath) return item;
    return { ...item, isFavorite: result.isFavorite };
  });
}

function buildTaskQueue(
  currentQueue: TaskQueueState,
  tasks: TaskQueueState['tasks'],
  config: TaskQueueState['config'],
  skippedNonImageCount: number,
  skippedByLimitCount: number,
  skippedByCostCount: number,
): TaskQueueState {
  const now = new Date().toISOString();
  return {
    ...currentQueue,
    tasks,
    runState: 'idle',
    activeTaskId: null,
    preparedAt: now,
    startedAt: null,
    finishedAt: null,
    skippedByLimitCount,
    skippedByCostCount,
    skippedNonImageCount,
    estimatedRunCostUsd: estimateQueueCost(tasks),
    lastErrorMessage: null,
    lastErrorCode: null,
    consecutiveErrorCount: 0,
    config,
    lastMessage: `${tasks.length} task${tasks.length === 1 ? '' : 's'} prepared.`,
  };
}

function updateQueueTask(
  queue: TaskQueueState,
  taskId: string,
  patch: Partial<TaskQueueState['tasks'][number]>,
  queuePatch: Partial<TaskQueueState> = {},
): TaskQueueState {
  return {
    ...queue,
    ...queuePatch,
    tasks: queue.tasks.map((task) => task.id === taskId ? { ...task, ...patch } : task),
  };
}

function updateQueueTasks(
  queue: TaskQueueState,
  updater: (task: TaskQueueState['tasks'][number]) => TaskQueueState['tasks'][number],
): TaskQueueState {
  return {
    ...queue,
    tasks: queue.tasks.map(updater),
  };
}

function summarizeImages(items: ExplorerItem[], currentTask: string): TaskSummary {
  const images = items.filter((item): item is ImageItem => item.kind === 'image');
  const processed = images.filter((item) => item.status === 'processed').length;
  const inQueue = images.filter((item) => item.status === 'queued').length;
  const errors = images.filter((item) => item.status === 'error').length;
  const notProcessed = images.filter((item) => item.status === 'notProcessed' || item.status === 'relocated' || item.status === 'missing').length;
  const done = processed + errors;

  return {
    totalItems: images.length,
    processed,
    inQueue,
    notProcessed,
    errors,
    currentTask,
    overallProgress: images.length > 0 ? Math.round((done / images.length) * 100) : 0,
  };
}

// Severity follows action data, never the language or spelling of a message.
export function appReducer(state:AppState,action:AppAction):AppState{
 const next=reducePresentedState(state,action);
 if(next===state||next.statusMessage===state.statusMessage&&!['appError/report','app/setStatusMessage'].includes(action.type))return next;
 const severity=action.type==='appError/report'?action.issue.severity:
  action.type==='app/setStatusMessage'?action.severity??'info':
  'errorMessage' in action&&Boolean(action.errorMessage)||'error' in action&&Boolean(action.error)?'error':'info';
 return {...next,statusSeverity:severity};
}
