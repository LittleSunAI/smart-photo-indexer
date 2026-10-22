import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {initialQuickSearch,isQueryView,quickSearchView} from './QuickSearch';
import {readProfilePreferences,initializeProfilePreferences,patchProfilePreferences,subscribeProfilePreferences,profilePreferencesSnapshot} from '../../modules/settings/ProfilePreferences';
import {loadUiPreferences,manualQueueTargetVersion,saveUiPreferences} from '../../shared/UiPreferences';
import {searchDraftInput,searchInputSignature} from '../../modules/search/SearchInput';
import { createContext, useContext, useEffect, useLayoutEffect, useMemo, useReducer, useRef, useSyncExternalStore, type Dispatch, type ReactNode } from 'react';
import { loadWorkspacePreferences, saveWorkspacePreferences } from '../../modules/settings/WorkspacePreferences';
import type { AppState } from '../types';
import { createInitialAppState } from './initialAppState';
import { appReducerWithLocalAiReadiness } from './appReducerWithLocalAiReadiness';
import type { AppActions, AppAction, NavigationMode } from './appActions';
import { fileSystemService } from '../../modules/filesystem/FileSystemService';
import { fileOpenService } from '../../modules/filesystem/FileOpenService';
import { indexService, toSearchImageItem, mapIndexSearchResultsToImages } from '../../modules/index/IndexService';
import type { IndexSearchInput,IndexSearchResultItem } from '../../modules/index/types';
import type { ExplorerItem, ImageItem } from '../../modules/explorer/types';
import { createAiAnalysisTasks, estimateOpenRouterCostPerImageUsd } from '../../modules/taskQueue/TaskQueueService';
import { renameService } from '../../modules/rename/RenameService';
import { buildPresetName, searchPresetService } from '../../modules/search/SearchPresetService';
import { searchExportService } from '../../modules/search/SearchExportService';
import { collectionService } from '../../modules/collections/CollectionService';
import {analysisLimits} from '../../shared/ai/AnalysisLimits';
import { appErrorService, type AppErrorContext, type AppErrorIssue } from '../../modules/errors/AppError';
import { broadcastImageFavoriteUpdated, listenForImageFavoriteUpdates } from '../../modules/imageViewerWindow/favoriteSync';
import { listenForViewerImageDeleted } from '../../modules/imageViewerWindow/viewerSession';
import { isTauriRuntime } from '../../shared/tauri/runtime';
import { messages } from '../../shared/i18n';
import { TaskQueueRuntimeController } from './TaskQueueRuntimeController';
import { IndexBackgroundJobController, requestIndexRescanJob } from './IndexBackgroundJobController';
import { nativeQueueService } from '../../modules/taskQueue/NativeQueueService';
import {invoke} from "../../shared/tauri/invoke";
import type { CollectionMutationResult } from '../../modules/collections/types';
import { createFolderSyncQueue } from './FolderSyncQueue';
import {CloudPhotoConsentModal,useCloudPhotoConsent} from '../../modules/settings/CloudPhotoConsent';
import {editorService} from '../../modules/photoEditor/EditorService';
import {selectSelectedImageItems} from './appSelectors';

const AppStateContext = createContext<AppState | null>(null);
const AppStateStoreContext = createContext<AppStateStore | null>(null);
const AppDispatchContext = createContext<Dispatch<AppAction> | null>(null);
const AppActionsContext = createContext<AppActions | null>(null);

type AppStateStoreListener = () => void;

interface AppStateStore {
  getState(): AppState;
  setState(state: AppState): void;
  subscribe(listener: AppStateStoreListener): () => void;
}

interface AppStateProviderProps {
  children: ReactNode;
}

function createAppStateStore(initialState: AppState): AppStateStore {
  let currentState = initialState;
  const listeners = new Set<AppStateStoreListener>();

  return {
    getState: () => currentState,
    setState: (nextState) => {
      if (Object.is(currentState, nextState)) return;
      currentState = nextState;
      listeners.forEach((listener) => listener());
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

function getSearchStatuses(filter: AppState['search']['statusFilter']): string[] {
  return filter === 'all' ? [] : [filter];
}

function parseSearchTags(value: string): string[] {
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

function dateInputToUnixStart(value: string): number | null {
  if (!value) return null;
  const time = new Date(`${value}T00:00:00`).getTime();
  return Number.isFinite(time) ? Math.floor(time / 1000) : null;
}

function dateInputToUnixEnd(value: string): number | null {
  if (!value) return null;
  const date = new Date(`${value}T00:00:00`); date.setDate(date.getDate()+1);
  const time = date.getTime();
  return Number.isFinite(time) ? Math.floor(time / 1000) : null;
}

function hasRunnableSearch(query: string, state: AppState): boolean {
  // Calendar bounds use the camera's recorded civil date without inventing an offset.
  return query.trim().length > 0
    || state.search.statusFilter !== 'all'
    || parseSearchTags(state.search.tagFilterText).length > 0
    || state.search.dateFrom.trim().length > 0
    || state.search.dateTo.trim().length > 0
    || Boolean(state.search.collectionId || state.search.rootPath)
    || state.search.favoriteOnly || state.search.untaggedOnly===true;
}

function findKnownItemById(state: AppState, itemId: string): ExplorerItem | null {
  return [...(state.inspectedImage?[state.inspectedImage]:[]),...state.explorerItems, ...state.search.results, ...state.dragDrop.items].find((item) => item.id === itemId) ?? null;
}

function nextCalendarDate(value:string):string|undefined{
  if(!value)return undefined;
  const date=new Date(`${value}T12:00:00`);if(!Number.isFinite(date.getTime()))return undefined;
  date.setDate(date.getDate()+1);
  return `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,'0')}-${String(date.getDate()).padStart(2,'0')}`;
}
function findSelectedIndexedImages(state: AppState): ImageItem[] {
  const selectedIds = state.selectedItemIds.length > 0 ? state.selectedItemIds : state.selectedItemId ? [state.selectedItemId] : [];
  const selected = new Set(selectedIds);
  const seen = new Set<string>();
  const images: ImageItem[] = [];
  for (const item of [...(state.inspectedImage?[state.inspectedImage]:[]),...state.explorerItems, ...state.search.results, ...state.dragDrop.items]) {
    if (item.kind !== 'image' || !selected.has(item.id)) continue;
    const indexedId = item.indexedImageId;
    if (!indexedId || seen.has(indexedId)) continue;
    seen.add(indexedId);
    images.push(item);
  }
  return images;
}


export function AppStateProvider({ children }: AppStateProviderProps) {
  useCurrentLocale();
  const searchRequestRef = useRef(0);
  const searchLaunchRef=useRef<{request:number;signature:string}|null>(null);
  const searchInputRef = useRef<IndexSearchInput | null>(null);
  const workspaceInputsRef=useRef<Record<string,IndexSearchInput|null>>({});
  const searchPagePendingRef = useRef(false);
  const folderRequestRef = useRef(0);
  const folderScanAbortRef = useRef<AbortController | null>(null);
  const selectionRequestRef = useRef(0);
  const selectionPendingRef = useRef(false);
  const [baseState, dispatch] = useReducer(appReducerWithLocalAiReadiness, undefined, createInitialAppState);
  const state=useMemo(()=>quickSearchView(baseState),[baseState]);
  const quickRequestRef=useRef(0),quickPageRef=useRef(false),quickTimerRef=useRef<ReturnType<typeof setTimeout>|null>(null),quickSignatureRef=useRef<string|null>(null);
  const quickInputRef=useRef<IndexSearchInput|null>(null);
  const queueSourceQueryRef=useRef<string|undefined>(undefined);
  const latestState=useRef(state);latestState.current=state;
  const appStateStoreRef = useRef<AppStateStore | null>(null);
  const renamePreviewRequestRef = useRef(0);
  const renamePreviewImagesRef = useRef<ImageItem[]>([]);
  const processingRequestRef=useRef(false);
  const processingGenerationRef=useRef(0);
  const nativeCommandRef=useRef(false);
  const nativeCommandGenerationRef=useRef(0);
  const cloudConsent=useCloudPhotoConsent();
  if (!appStateStoreRef.current) {
    appStateStoreRef.current = createAppStateStore(state);
  }

  const reportAppError = (error: unknown, context: AppErrorContext): AppErrorIssue => {
    const issue = appErrorService.createIssue(error, context);
    dispatch({ type: 'appError/report', issue });
    return issue;
  };

  useEffect(()=>{let disposed=false;const update=()=>{if(!disposed)dispatch({type:'privacy/setAiConsent',value:profilePreferencesSnapshot().cloudConsent});};const off=subscribeProfilePreferences(update);void initializeProfilePreferences(state.settings.privacy.aiConsentAccepted).then(update).catch(error=>{if(!disposed)reportAppError(error,{domain:'settings',operation:localeText("main", "ui.AppStateContext.20452c925ed932d2")});});return()=>{disposed=true;off();};},[]);

  const syncIndexForItems = async (items: ExplorerItem[],isCurrent:()=>boolean=()=>true) => {
    if(!isCurrent())return;
    const images = items.filter((item): item is ImageItem => item.kind === 'image');
    dispatch({ type: 'index/syncStarted', imageCount: images.length });

    try {
      const syncResult = await indexService.syncScannedImages(images, true);
      if(isCurrent())dispatch({ type: 'index/syncSucceeded', result: syncResult });
    } catch (indexError) {
      if(!isCurrent())return;
      const issue = reportAppError(indexError, { domain: 'sqlite', operation: localeText("main", "ui.AppStateContext.a51e0ae6513c9b9c") });
      dispatch({ type: 'index/syncFailed', errorMessage: issue.userMessage });
    }
  };

  const actions = useMemo<AppActions>(() => {
    const activeSearchInput=()=>state.quickSearch?.active?quickInputRef.current:searchInputRef.current;
    const clearQuick=()=>{quickRequestRef.current++;selectionRequestRef.current++;quickSignatureRef.current=null;quickPageRef.current=false;if(quickTimerRef.current!==null)clearTimeout(quickTimerRef.current);quickTimerRef.current=null;void indexService.closeSearchSnapshot(quickInputRef.current?.queryId).catch(()=>{});quickInputRef.current=null;dispatch({type:'quickSearch/clear'});};
    const submitQuick=(sortOrder?:string)=>{
      if(quickTimerRef.current!==null)clearTimeout(quickTimerRef.current);quickTimerRef.current=null;
      const q=latestState.current.quickSearch??initialQuickSearch(baseState);if(q.composing)return;
      if(!q.query.trim()){clearQuick();return;}
      const input=searchDraftInput(q.query,q.search);if(sortOrder)input.sortOrder=sortOrder;
      if(!input.fields?.length)return;
      const signature=searchInputSignature(input);if(quickSignatureRef.current===signature&&(q.search.isSearching||!q.search.needsRefresh))return;
      quickSignatureRef.current=signature;const request=++quickRequestRef.current;quickPageRef.current=false;selectionRequestRef.current++;
      dispatch({type:'quickSearch/apply',action:{type:'search/start',query:q.query.trim()}});
      void indexService.searchIndexedImages(input).then(result=>{
        if(request!==quickRequestRef.current){void indexService.closeSearchSnapshot(result.queryId).catch(()=>{});return;}
        const old=quickInputRef.current;quickInputRef.current={...input,queryId:result.queryId};
        if(old?.queryId&&old.queryId!==result.queryId&&latestState.current.quickSearch?.search.selectionSnapshot?.input.queryId!==old.queryId)void indexService.closeSearchSnapshot(old.queryId).catch(()=>{});
        dispatch({type:'quickSearch/apply',dataRevision:q.dataRevision??0,action:{type:'search/succeeded',result,sortOrder:input.sortOrder??'relevance',input}});
      }).catch(error=>{if(request!==quickRequestRef.current)return;quickSignatureRef.current=null;dispatch({type:'quickSearch/apply',action:{type:'search/failed',errorMessage:String(error)}});});
    };

    async function visitSearchPages(scope:'selected'|'all',consume:(images:ImageItem[])=>Promise<boolean|void>):Promise<void>{
      const selection=scope==='selected'?state.search.selectionSnapshot:undefined;
      const input={...(selection?.input??activeSearchInput())} as IndexSearchInput;
      if(!input.queryId)throw new Error(localeText("main", "ui.AppStateContext.6f4985bc5eaa894e"));
      const selected=new Set(state.selectedItemIds),excluded=new Set(selection?.excludedPaths??[]);
      const queryId=await indexService.forkSearchSnapshot(input.queryId);
      if(!queryId)throw new Error(localeText("main", "ui.AppStateContext.5fd8707d32175ac4"));
      try{for(let offset=0;;){
        const result=await indexService.searchIndexedImages({...input,queryId,offset,limit:128});
        const images=mapIndexSearchResultsToImages(result).filter(item=>scope==='all'||(selection?!excluded.has(item.path):selected.has(item.id)));
        if(await consume(images)===false)return;
        offset+=result.returnedCount;if(offset>=result.totalCount)return;
        if(!result.returnedCount)throw new Error(localeText("main", "ui.AppStateContext.8ec3b74e56d823c8"));
      }}finally{await indexService.closeSearchSnapshot(queryId);}
    }
    async function visitSelectedSearchPages(consume:(images:ImageItem[])=>Promise<boolean|void>):Promise<void>{return visitSearchPages('selected',consume);}
    async function resolveSelectedImages(maximum:number):Promise<ImageItem[]> {
      if(!Number.isInteger(maximum) || maximum<1)throw new Error(localeText("main", "ui.AppStateContext.34a039a7ecb993f3"));
      if(!state.search.selectionSnapshot || !isQueryView(state))return findSelectedIndexedImages(state).slice(0,maximum);
      const images:ImageItem[]=[];
      await visitSelectedSearchPages(async page=>{images.push(...page.slice(0,maximum-images.length));return images.length<maximum;});
      return images;
    }
    const getSearchSelection = async (scope: 'selected' | 'all'): Promise<ImageItem[]> => {
      if(scope==='selected' && state.search.selectionSnapshot) {
        const items:ImageItem[]=[];await visitSelectedSearchPages(async page=>{items.push(...page);});return items;
      }
      if (scope === 'selected' || !activeSearchInput()) return state.search.results;
      const input = { ...activeSearchInput()! };
      const queryId = await indexService.forkSearchSnapshot(input.queryId);
      if(!queryId)throw new Error(localeText("main", "ui.AppStateContext.2bca5928c3c8c114"));
      const results: ImageItem[] = [];
      let ownedId = queryId;
      try {
        for (let offset = 0; ; ) {
          const page = await indexService.searchIndexedImages({ ...input, queryId: ownedId ?? undefined, offset, limit: 500 });

          results.push(...mapIndexSearchResultsToImages(page));
          offset += page.returnedCount;
          if (offset >= page.totalCount || page.returnedCount === 0) return results;
        }
      } finally { await indexService.closeSearchSnapshot(ownedId); }
    };
    const loadFolder = async (
      path?: string | null,
      navigationMode: NavigationMode = 'push',
      historyTargetIndex?: number,
    ) => {
      clearQuick();
      const requestedPath = path && path.trim().length > 0 ? path.trim() : null;
      const preserveSelection=navigationMode==='replace' && requestedPath===state.currentPath;
      const request = ++folderRequestRef.current;
      folderScanAbortRef.current?.abort();
      const abort = new AbortController(); folderScanAbortRef.current = abort;
      let firstPage = true;
      const syncQueue=createFolderSyncQueue(items=>syncIndexForItems(items,()=>request===folderRequestRef.current && !abort.signal.aborted),abort.signal);
      dispatch({ type: 'filesystem/scanStarted', requestedPath, preserveSelection });

      try {
        await fileSystemService.scanDirectory({
          path: requestedPath,
          includeSubfolders: state.includeSubfolders,
          imagesOnly: state.showImagesOnly,
          readMetadata:true,
          signal: abort.signal,
          onPage: async result => {
            if (request !== folderRequestRef.current) return;
            if (firstPage) { firstPage = false; dispatch({ type: 'filesystem/scanSucceeded', result, navigationMode, historyTargetIndex, preserveSelection }); }
            else dispatch({ type: 'filesystem/pageSucceeded', result });
            await syncQueue.enqueue(result.items);
          },
        });
        await syncQueue.drain();
        if(request===folderRequestRef.current && !abort.signal.aborted)dispatch({type:'filesystem/scanFinished',requestedPath});
      } catch (error) {
        if (abort.signal.aborted || request !== folderRequestRef.current) return;
        const issue = reportAppError(error, { domain: 'filesystem', operation: localeText("main", "ui.AppStateContext.2f6a1b0b4495221b"), relatedPath: requestedPath });
        dispatch({ type: 'filesystem/scanFailed', errorMessage: issue.userMessage });
      }
    };

    const openHistoryIndex = async (index: number) => {
      const targetPath = state.navigationHistory[index];
      if (!targetPath) {
        dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.e481713c489e7e79") });
        return;
      }

      await loadFolder(targetPath, 'history', index);
    };

    const openItemExternally = async (itemId: string) => {
      const item = findKnownItemById(state, itemId);
      if (!item) {
        dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.5a8ddd8b3518a8b1") });
        return;
      }

      if (item.kind === 'folder') {
        await loadFolder(item.path, 'push');
        return;
      }

      dispatch({ type: 'explorer/selectItem', itemId });
      dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.3b169ff4540ace73", {"value1": (item.name)}) });

      try {
        await fileOpenService.openExternalPath(item.path);
        dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.f27af98f16d023b8", {"value1": (item.name)}) });
      } catch (error) {
        const issue = reportAppError(error, { domain: 'filesystem', operation: localeText("main", "ui.AppStateContext.03a96b34987b7a43"), relatedPath: item.path });
        dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.c0b5209b2252e881", {"value1": (item.name), "value2": (issue.userMessage)}) ,severity:'error'});
      }
    };


    const updateImageMetadata = async (input: import('../../modules/index/types').IndexMetadataUpdateInput) => {
      const target = state.explorerItems.find((item) => item.kind === 'image' && (item.path === input.path || item.indexedImageId === input.imageId)) as ImageItem | undefined;
      dispatch({ type: 'index/metadataUpdateStarted', imageName: target?.name ?? input.path });

      try {
        const result = await indexService.updateImageMetadata(input);
        dispatch({ type: 'index/metadataUpdateSucceeded', result });
      } catch (error) {
        const issue = reportAppError(error, { domain: 'sqlite', operation: localeText("main", "ui.AppStateContext.b8a08a74d3c7c951"), relatedPath: input.path });
        dispatch({ type: 'index/metadataUpdateFailed', errorMessage: issue.userMessage });
        throw error;
      }
    };

    const revealItemExternally = async (itemId: string) => {
      const item = findKnownItemById(state, itemId);
      if (!item) {
        dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.b5b0f194a58b44b3") });
        return;
      }

      dispatch({ type: 'explorer/selectItem', itemId });
      dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.68f26206debba652", {"value1": (item.name)}) });

      try {
        await fileOpenService.revealInFileManager(item.path);
        dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.32cea6474df7d69f", {"value1": (item.name)}) });
      } catch (error) {
        const issue = reportAppError(error, { domain: 'filesystem', operation: localeText("main", "ui.AppStateContext.960f39241920350e"), relatedPath: item.path });
        dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.74a3d3f5b0876eb3", {"value1": (item.name), "value2": (issue.userMessage)}) ,severity:'error'});
      }
    };

    const queueImagesForProcessing = async (images: ImageItem[], skippedNonImageCount: number, totalSelected = images.length, manualOnly=false, generation=processingGenerationRef.current, sourceRoots:string[]=[],sourceQueryId?:string,excludedSourcePaths:string[]=[],targetJobId:string|null=loadUiPreferences().queueManualTarget??null) => {
      const targetVersion=manualQueueTargetVersion();
      images=images.map(image=>({...image,tags:[...image.tags]}));
      const profile=await readProfilePreferences();
      if (images.length === 0&&sourceRoots.length===0&&!sourceQueryId) {
        dispatch({ type: 'taskQueue/createBlocked', reason: skippedNonImageCount > 0 ? messages().settings.openRouterErrors.noDroppedImages : messages().settings.openRouterErrors.noImagesSelected });
        return;
      }

      if (state.settings.aiProviderMode !== 'local' && !(await readProfilePreferences()).cloudConsent) {
        const provider=state.settings.aiProviderMode==='fal'?'fal':'openrouter';
        const accepted=await cloudConsent.request({provider,model:(provider==='fal'?state.settings.fal:state.settings.openRouter).selectedModel,photoCount:images.length,selectedCount:totalSelected});
        if(generation!==processingGenerationRef.current)return;
        if(!accepted){dispatch({type:'taskQueue/consentCancelled'});return;}
        await patchProfilePreferences({cloudConsent:true});
        dispatch({type:'privacy/setAiConsent',value:true});
      }

      const cloudSettings = state.settings.aiProviderMode === 'fal' ? state.settings.fal : state.settings.openRouter;
      if (state.settings.aiProviderMode !== 'local' && !cloudSettings.credentialRef) {
        dispatch({ type: 'taskQueue/createBlocked', reason: localeText("main", "ui.AppStateContext.704737b3413591da") });
        return;
      }

      const maxImagesPerRun = Math.max(1, Math.round(state.settings.processingLimits.maxImagesPerRun));
      const limitedImages = images;
      const skippedByLimitCount = 0;
      const selectedAiModel = state.settings.aiProviderMode === 'local'
        ? state.settings.localAi.selectedModel
        : cloudSettings.selectedModel;
      const outputMode=state.settings.processingLimits.outputMode;
      const effectiveLimits=analysisLimits(state.settings.aiProviderMode,selectedAiModel,outputMode,state.settings.processingLimits.maxTokens,state.settings.processingLimits.imageMaxSide);
      const estimatedCostPerImageUsd = estimateOpenRouterCostPerImageUsd(selectedAiModel, state.settings.aiProviderMode,effectiveLimits.maxTokens,state.settings.processingLimits.passes);
      const maxEstimatedCostUsd = Math.max(0, state.settings.processingLimits.maxEstimatedCostPerRunUsd);
      const costLimitedImages = limitedImages;
      const skippedByCostCount = limitedImages.length - costLimitedImages.length;
      const config = {
        queueContractVersion:2,onlyUnprocessed:state.settings.processingLimits.onlyUnprocessed!==false,
        sourceRoots,sourceQueryId,excludedSourcePaths,recursiveProcessing:profile.recursiveProcessing,analysisCopy:{...profile.analysisCopy,enabled:state.smartRenameEnabled&&profile.analysisCopy.enabled},
        outputMode,
        autoStart: !manualOnly && !state.settings.processingLimits.manualQueue,
        queueMode:(!manualOnly&&!state.settings.processingLimits.manualQueue?'direct':'manual') as 'direct'|'manual',
        credentialRef: state.settings.aiProviderMode === 'local' ? null : cloudSettings.credentialRef,
        endpoint: state.settings.localAi.endpoint,
        passes: state.settings.processingLimits.passes,
        concurrency: state.settings.aiProviderMode === 'local' ? 1 : state.settings.processingLimits.cloudConcurrency,
        maxTokens: effectiveLimits.maxTokens,
        imageMaxSide: effectiveLimits.imageMaxSide,
        providerMode: state.settings.aiProviderMode,
        model: selectedAiModel,
        maxImagesPerRun,
        requestDelayMs: Math.max(0, Math.round(state.settings.processingLimits.requestDelayMs)),
        maxEstimatedCostUsd,
        estimatedCostPerImageUsd,
        maxRetriesPerTask: Math.max(0, Math.round(state.settings.processingLimits.maxRetriesPerTask)),
        stopOnFirstError: state.settings.processingLimits.stopOnFirstAiError,
        smartRenameEnabled: state.smartRenameEnabled,
        renameStopOnFirstError: state.settings.rename.stopOnFirstError,
      };
      // Direct processing prepares file plans in the native queue and starts
      // immediately. Only manual queue preparation asks to review the plan.
      if (!config.autoStart&&config.smartRenameEnabled&&sourceRoots.length===0&&!sourceQueryId) {
        const request = ++renamePreviewRequestRef.current;
        renamePreviewImagesRef.current = costLimitedImages.map(image => ({ ...image, tags: [...image.tags] }));
        const imageIds = costLimitedImages.map((image) => image.id);
        dispatch({ type: 'renamePreview/start', imageIds, skippedNonImageCount, skippedByLimitCount, skippedByCostCount, config,targetJobId:config.queueMode==='manual'?targetJobId:undefined,targetVersion });
        renameService
          .previewSmartRenameImages(costLimitedImages,config.analysisCopy)
          .then((result) => { if (request === renamePreviewRequestRef.current) dispatch({ type: 'renamePreview/succeeded', result }); })
          .catch((error) => {
          if (request !== renamePreviewRequestRef.current) return;
          const issue = reportAppError(error, { domain: 'rename', operation: localeText("main", "ui.AppStateContext.4ac42b53888d9cf9") });
          dispatch({ type: 'renamePreview/failed', errorMessage: issue.userMessage });
        });
        return;
      }

      const tasks = createAiAnalysisTasks(costLimitedImages, config);
      if((sourceRoots.length||sourceQueryId)&&tasks.length===0)tasks.push({id:`sources-${Date.now()}-${generation}`,type:"aiAnalysis",sourceItemId:null,targetPath:sourceRoots[0]??`search:${sourceQueryId}`,fileName:localeText("main", "ui.AppStateContext.5d33472be13ddf74"),status:"pending",progress:0,createdAt:new Date().toISOString(),attemptCount:0,maxAttempts:1,estimatedCostUsd:0});
      dispatch({ type: 'taskQueue/createFromSelection', tasks, skippedNonImageCount, skippedByLimitCount, skippedByCostCount, config,targetJobId:config.queueMode==='manual'?targetJobId:undefined,targetVersion });
    };

    const commandNativeQueue = async (action: string,explicitId?:string) => {
      const general=!explicitId,id=explicitId??'*',interrupt=general&&['pause','stop'].includes(action);
      if(!interrupt&&(nativeCommandRef.current||processingRequestRef.current)){if(explicitId)throw new Error(localeText("main", "ui.AppStateContext.e0ead485d1538c46"));return;}
      const generation=++nativeCommandGenerationRef.current;nativeCommandRef.current=!interrupt;
      const current=()=>generation===nativeCommandGenerationRef.current;
      dispatch({type:'taskQueue/nativeCommandPending',jobId:id,pending:!interrupt});
      try{
        if(['start','resume','retry'].includes(action)){
          const contexts=await nativeQueueService.contexts(explicitId,action==='retry',state.settings.processingLimits.manualQueue?'manual':'direct');if(!current())return;
          const cloud=contexts.filter(c=>['fal','openrouter'].includes(c.providerMode));
          if(cloud.length&&!(await readProfilePreferences()).cloudConsent){
            const total=cloud.reduce((n,c)=>n+c.total,0),first=cloud[0];
            const accepted=await cloudConsent.request({provider:first.providerMode as 'fal'|'openrouter',model:first.model,photoCount:total,selectedCount:total,fromSavedJob:true});
            if(!accepted||!current())return;
            await patchProfilePreferences({cloudConsent:true});if(!current())return;dispatch({type:'privacy/setAiConsent',value:true});
          }
        }
        if(!current())return;
        if(general){const result=await nativeQueueService.commandAll(action,state.settings.processingLimits.manualQueue?'manual':'direct');if(current()&&result.errors?.length)dispatch({type:'taskQueue/nativeCommandPending',jobId:id,pending:false,error:result.errors.map(e=>e.error).join('; ')});}
        else await nativeQueueService.command(id,action);
      }catch(error){if(current())dispatch({type:'taskQueue/nativeCommandPending',jobId:id,pending:false,error:String(error)});if(explicitId)throw error;}
      finally{if(current()){nativeCommandRef.current=false;dispatch({type:'taskQueue/nativeCommandPending',jobId:id,pending:false});}}
    };

    return {
      setWorkspaceMode: (mode) => {
        clearQuick();
        selectionRequestRef.current++;searchRequestRef.current++;searchPagePendingRef.current=false;
        workspaceInputsRef.current[state.activeWorkspace]=searchInputRef.current;
        searchInputRef.current=workspaceInputsRef.current[mode]??state.workspaceContexts?.[mode]?.search.lastInput??null;
        dispatch({type:'workspace/setActive',mode});
        if((mode==='favorites'||mode==='ready'||mode==='collections'&&Boolean(state.workspaceContexts?.collections?.search.collectionId))&&(!state.workspaceContexts?.[mode]?.search.queryId||state.workspaceContexts?.[mode]?.search.needsRefresh)){
          window.setTimeout(()=>{if(appStateStoreRef.current?.getState().activeWorkspace===mode)liveActionsRef.current.submitSearch();},0);
        }
      },
      openCollection: (collectionId) => {
        clearQuick();
        selectionRequestRef.current++;searchRequestRef.current++;searchPagePendingRef.current=false;
        workspaceInputsRef.current[state.activeWorkspace]=searchInputRef.current;
        if(state.activeWorkspace==='collections')void indexService.closeSearchSnapshot(searchInputRef.current?.queryId).catch(()=>{});
        workspaceInputsRef.current.collections=null;
        dispatch({type:'workspace/openCollection',collectionId});searchInputRef.current=null;
        if(collectionId)window.setTimeout(()=>{const current=appStateStoreRef.current?.getState();if(current?.activeWorkspace==='collections'&&current.search.collectionId===collectionId)liveActionsRef.current.submitSearch();},0);
      },
      setDetailsTab: (tab) => dispatch({ type: 'details/setActiveTab', tab }),
      setSearchQuery: (query) => dispatch({ type: 'search/setQuery', query }),
      setSearchStatusFilter: (value) => dispatch({ type: 'search/setStatusFilter', value }),
      setSearchTagFilterText: (value) => dispatch({ type: 'search/setTagFilterText', value }),
      setSearchTagMode: (value) => dispatch({ type: 'search/setTagMode', value }),
      addSearchTagFilter: (tag) => dispatch({ type: 'search/addTagFilter', tag }),
      removeSearchTagFilter: (tag) => dispatch({ type: 'search/removeTagFilter', tag }),
      setSearchDateFrom: (value) => dispatch({ type: 'search/setDateFrom', value }),
      setSearchDateTo: (value) => dispatch({ type: 'search/setDateTo', value }),
      setFilenameFilter:value=>dispatch({type:'explorer/setNameFilter',value}),
      setSearchScope: (field, value) => dispatch({ type: 'search/setScope', field, value }),
      applySearchQuickFilter: (filter) => dispatch({ type: 'search/applyQuickFilter', filter }),
      saveCurrentSearchPreset: (name) => {
        const draft = {
          name,
          query: state.searchQuery,
          statusFilter: state.search.statusFilter,
          tagFilterText: state.search.tagFilterText,
          tagMode: state.search.tagMode,
          dateFrom: state.search.dateFrom,
          dateTo: state.search.dateTo,
          collectionId: state.search.collectionId,
          rootPath: state.search.rootPath,
          sortOrder: state.search.sortOrder,
          dateField: state.search.dateField,
          fields:state.search.fields,
          expandTerms: state.search.expandTerms,
          favoriteOnly: state.search.favoriteOnly,
          untaggedOnly: state.search.untaggedOnly===true,
        };
        const hasPresetContent = draft.query.trim().length > 0
          || draft.statusFilter !== 'all'
          || parseSearchTags(draft.tagFilterText).length > 0
          || draft.dateFrom.trim().length > 0
          || draft.dateTo.trim().length > 0
          || draft.favoriteOnly || Boolean(draft.collectionId || draft.rootPath);
        if (!hasPresetContent && !draft.untaggedOnly) {
          dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.524d952f6e90cc7a") });
          return;
        }
        const presets = searchPresetService.savePreset({ ...draft, name: name || buildPresetName(draft) }, state.searchPresets);
        dispatch({ type: 'search/presetSaved', presets });
      },
      applySearchPreset: (presetId) => {
        const preset = state.searchPresets.find((item) => item.id === presetId);
        if (!preset) {
          dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.6f4dd147d15e2c82") });
          return;
        }
        const presets = searchPresetService.markPresetUsed(presetId, state.searchPresets);
        dispatch({ type: 'search/presetApplied', preset, presets });
      },
      deleteSearchPreset: (presetId) => {
        const presets = searchPresetService.deletePreset(presetId, state.searchPresets);
        dispatch({ type: 'search/presetDeleted', presets });
      },
      toggleSearchPresetFavorite: (presetId) => {
        const presets = searchPresetService.toggleFavorite(presetId, state.searchPresets);
        dispatch({ type: 'search/presetFavoriteToggled', presets });
      },
      renameSearchPreset: (presetId, name) => {
        const presets = searchPresetService.renamePreset(presetId, name, state.searchPresets);
        dispatch({ type: 'search/presetRenamed', presets });
      },
      exportSearchResults: async (format, scope) => {
        const snapshot=scope==='selected'?state.search.selectionSnapshot:undefined;
        const selectedIds = state.selectedItemIds;
        const query = state.search.lastQuery || state.searchQuery;
        const candidateItems = searchExportService.pickItems(state.search.results, selectedIds, scope);
        if (candidateItems.length === 0 && !snapshot) {
          dispatch({ type: 'search/exportFailed', errorMessage: scope === 'selected' ? localeText("main", "ui.AppStateContext.64dfe8ca3db542d6") : localeText("main", "ui.AppStateContext.47196f848dbb6b13") });
          return;
        }

        dispatch({ type: 'search/exportStarted', scope, format });
        try {
          if (isTauriRuntime()) {
            const input = snapshot?.input ?? activeSearchInput();
            if (!input) throw new Error(localeText("main", "ui.AppStateContext.9143a128841cdb17"));
            const result = await indexService.exportSearchSnapshot({ ...input }, format, scope === 'selected' && !snapshot ? candidateItems.map(item => item.path) : undefined,snapshot?.excludedPaths);
            if (result.cancelled) dispatch({ type: 'search/exportCancelled' });
            else dispatch({ type: 'search/exportSucceeded', exportedCount: result.exportedCount, fileName: result.fileName, format: result.format });
            return;
          }
          const results = await getSearchSelection(scope);
          const result = searchExportService.exportResults({
            appName: state.appName,
            query,
            results,
            selectedIds,
            scope,
            format,
          });
          dispatch({ type: 'search/exportSucceeded', exportedCount: result.exportedCount, fileName: result.fileName, format: result.format });
        } catch (error) {
          const issue = reportAppError(error, { domain: 'search', operation: localeText("main", "ui.AppStateContext.17b10b3892c299f0") });
          dispatch({ type: 'search/exportFailed', errorMessage: issue.userMessage });
        }
      },
      copySearchResultPaths: async(scope)=>{
        try{
          const chunks:string[]=[];let count=0,bytes=0;
          await visitSearchPages(scope,async page=>{const text=page.map(item=>item.path).join('\n');bytes+=text.length*2+(chunks.length?2:0);if(bytes>64*1024*1024)throw new Error(localeText("main", "ui.AppStateContext.e29a47be608909d4"));if(text)chunks.push(text);count+=page.length;});
          await searchExportService.copyPathsText(chunks.join('\n'),count);
          dispatch({type:'search/copyPathsSucceeded',copiedCount:count,scope});
        }catch(error){dispatch({type:'search/copyPathsFailed',errorMessage:String(error)});}
      },
      revealSearchResultFolders: async(scope)=>{
        try{
          const seen=new Set<string>(),first:ImageItem[]=[];let bytes=0;
          await visitSearchPages(scope,async page=>{for(const item of page){const folder=item.path.replace(/[\\/][^\\/]+$/,'').toLowerCase();if(seen.has(folder))continue;seen.add(folder);bytes+=folder.length*2;if(bytes>64*1024*1024)throw new Error(localeText("main", "ui.AppStateContext.81b885492b64c193"));if(first.length<10)first.push(item);}});
          dispatch({type:'search/revealFoldersStarted',count:first.length,scope});
          for(const item of first)await fileOpenService.revealInFileManager(item.path);
          dispatch({type:'search/revealFoldersSucceeded',openedCount:first.length,skippedCount:seen.size-first.length});
        }catch(error){dispatch({type:'search/revealFoldersFailed',errorMessage:String(error)});}
      },
      loadCollections: async () => {
        dispatch({ type: 'collections/loadStarted' });
        try {
          const result = await collectionService.getCollections();
          dispatch({ type: 'collections/loadSucceeded', result });
        } catch (error) {
          const issue = reportAppError(error, { domain: 'collections', operation: localeText("main", "ui.AppStateContext.cef8ac0f89b8da4b") });
          dispatch({ type: 'collections/loadFailed', errorMessage: issue.userMessage });
        }
      },
      createCollection: async (name) => {
        try {
          const result = await collectionService.createCollection({ name });
          dispatch({ type: 'collections/createSucceeded', result });
          return result;
        } catch (error) {
          const issue = reportAppError(error, { domain: 'collections', operation: localeText("main", "ui.AppStateContext.0279bebadeb54797") });
          dispatch({ type: 'collections/createFailed', errorMessage: issue.userMessage });
          return null;
        }
      },
      addSelectedImagesToCollection: async (collectionId) => {
        if(isQueryView(state) && state.search.selectionSnapshot) {
          try {
            const snapshot=state.search.selectionSnapshot;
            const result=await invoke<CollectionMutationResult>('mutate_search_collection',{input:snapshot.input,collectionId,excludedPaths:snapshot.excludedPaths,remove:false});
            dispatch({type:'collections/addImagesSucceeded',result,imageIds:findSelectedIndexedImages(state).map(item=>item.indexedImageId!)});
            return result;
          }catch(error){dispatch({type:'collections/addImagesFailed',errorMessage:String(error)});return null;}
        }
        const images = findSelectedIndexedImages(state);
        if (images.length === 0) {
          dispatch({ type: 'collections/addImagesFailed', errorMessage: localeText("main", "ui.AppStateContext.b1b2565296a9847c") });
          return null;
        }

        try {
          const result = await collectionService.addImagesToCollection({ collectionId, imageIds: images.map((image) => image.indexedImageId!).filter(Boolean) });
          dispatch({ type: 'collections/addImagesSucceeded', result, imageIds: images.map((image) => image.indexedImageId!).filter(Boolean) });
          return result;
        } catch (error) {
          const issue = reportAppError(error, { domain: 'collections', operation: localeText("main", "ui.AppStateContext.3e93f6bc881e47fd") });
          dispatch({ type: 'collections/addImagesFailed', errorMessage: issue.userMessage });
          return null;
        }
      },
      renameCollection: async (collectionId, name) => {
        const oldName=state.collections.items.find(item=>item.id===collectionId)?.name ?? '';
        try {
          const result=await collectionService.renameCollection(collectionId,name);
          dispatch({type:'collections/mutated',collectionId,oldName,newName:result.collection?.name,result});
        } catch(error) { dispatch({type:'collections/createFailed',errorMessage:String(error)}); }
      },
      deleteCollection: async (collectionId) => {
        const oldName=state.collections.items.find(item=>item.id===collectionId)?.name ?? '';
        try {
          const result=await collectionService.deleteCollection(collectionId);
          dispatch({type:'collections/mutated',collectionId,oldName,deleted:true,result});
        } catch(error) { dispatch({type:'collections/createFailed',errorMessage:String(error)}); }
      },
      removeSelectedFromCollection: async (collectionId) => {
        const oldName=state.collections.items.find(item=>item.id===collectionId)?.name ?? '';
        const imageIds=Array.from(new Set(findSelectedIndexedImages(state).map(item=>item.indexedImageId!)));
        try {
          const snapshot=isQueryView(state)?state.search.selectionSnapshot:undefined;
          const result=snapshot
            ? await invoke<CollectionMutationResult>('mutate_search_collection',{input:snapshot.input,collectionId,excludedPaths:snapshot.excludedPaths,remove:true})
            : await collectionService.removeImagesFromCollection({collectionId,imageIds});
          dispatch({type:'collections/mutated',collectionId,oldName,imageIds,result});
        } catch(error) { dispatch({type:'collections/createFailed',errorMessage:String(error)}); }
      },
      toggleFavoriteForItem: async (itemId) => {
        const item = findKnownItemById(state, itemId);
        if (!item || item.kind !== 'image') {
          dispatch({ type: 'collections/favoriteFailed', errorMessage: localeText("main", "ui.AppStateContext.13b21d3455d1b42d") });
          return;
        }

        try {
          const result = await collectionService.setImageFavorite({
            imageId: item.indexedImageId ?? null,
            path: item.path,
            isFavorite: !(item.isFavorite ?? false),
          });
          dispatch({ type: 'collections/favoriteUpdated', result });
          void broadcastImageFavoriteUpdated(result);
        } catch (error) {
          const issue = reportAppError(error, { domain: 'collections', operation: localeText("main", "ui.AppStateContext.48908c46cd15eaf5") });
          dispatch({ type: 'collections/favoriteFailed', errorMessage: issue.userMessage });
        }
      },
      favoriteImagePath: async (path,imageId) => {
        const result=await collectionService.setImageFavorite({path,imageId:imageId??null,isFavorite:true});
        dispatch({type:'collections/favoriteUpdated',result});
        await broadcastImageFavoriteUpdated(result);
      },
      setQuickSearchQuery:(query)=>{if(quickTimerRef.current!==null)clearTimeout(quickTimerRef.current);quickTimerRef.current=null;if(query.trim()===(latestState.current.quickSearch?.query??'').trim()){dispatch({type:'quickSearch/update',query});return;}quickRequestRef.current++;quickSignatureRef.current=null;if(!query.trim()){clearQuick();return;}dispatch({type:'quickSearch/update',query});},
      setQuickSearchScope:(field,value)=>{quickRequestRef.current++;quickSignatureRef.current=null;dispatch({type:'quickSearch/update',field,value});},
      setQuickSearchComposing:(composing)=>{quickRequestRef.current++;quickSignatureRef.current=null;dispatch({type:'quickSearch/update',composing});},
      clearQuickSearch:clearQuick,
      submitToolbarSearch:()=>submitQuick(),
      submitSearch: (sortOrderOverride?:string) => {
        if(state.quickSearch?.active&&sortOrderOverride){submitQuick(sortOrderOverride);return;}
        clearQuick();
        const advanced=baseState;
        const planned=searchDraftInput(advanced.searchQuery,advanced.search);if(sortOrderOverride)planned.sortOrder=sortOrderOverride;
        if(searchLaunchRef.current?.signature===searchInputSignature(planned))return;
        const workspace=advanced.activeWorkspace==='explorer'?'search':advanced.activeWorkspace;
        selectionRequestRef.current++;
        const request = ++searchRequestRef.current;

        searchPagePendingRef.current = false;
        const query = advanced.searchQuery.trim();
        if(workspace==='search')dispatch({ type: 'search/submit' });
        if (!hasRunnableSearch(query, advanced)) return;
        const tags = parseSearchTags(advanced.search.tagFilterText);
        const dateFromUnix = dateInputToUnixStart(advanced.search.dateFrom);
        const dateToUnix = dateInputToUnixEnd(advanced.search.dateTo);
        dispatch({ type: 'search/start', query: query || localeText("main", "ui.AppStateContext.b1bc0fc57712ce13"),workspace });
        const input=searchDraftInput(query,advanced.search);
        if(sortOrderOverride)input.sortOrder=sortOrderOverride;
        const signature=searchInputSignature(input);
        searchLaunchRef.current={request,signature};
        void indexService.searchIndexedImages(input)
          .then((result) => {
            if (request !== searchRequestRef.current) { void indexService.closeSearchSnapshot(result.queryId); return; }
            const previous=searchInputRef.current;
            searchInputRef.current = { ...input, queryId: result.queryId };
            if(previous?.queryId&&previous.queryId!==result.queryId&&advanced.search.selectionSnapshot?.input.queryId!==previous.queryId)void indexService.closeSearchSnapshot(previous.queryId).catch(()=>{});
            workspaceInputsRef.current[workspace]=searchInputRef.current;
            dispatch({ type: 'search/succeeded', result, sortOrder:input.sortOrder ?? 'relevance',workspace,input,dataRevision:advanced.search.dataRevision??0 });
          })
          .catch((error) => {
            if (request !== searchRequestRef.current) return;
            const issue = reportAppError(error, { domain: 'sqlite', operation: localeText("main", "ui.AppStateContext.b775c28f5b38cf80") });
            dispatch({ type: 'search/failed', errorMessage: issue.userMessage });
          }).finally(()=>{if(searchLaunchRef.current?.request===request)searchLaunchRef.current=null;});
      },
      loadMoreSearchResults: () => {
        if(state.quickSearch?.active){
          const input=quickInputRef.current;if(!input||quickPageRef.current||state.search.isSearching||state.search.results.length>=state.search.resultCount)return;
          const request=quickRequestRef.current;quickPageRef.current=true;dispatch({type:'quickSearch/apply',action:{type:'search/pageStarted'}});
          void indexService.searchIndexedImages({...input,offset:state.search.results.length,limit:200}).then(result=>{if(request===quickRequestRef.current)dispatch({type:'quickSearch/apply',action:{type:'search/pageSucceeded',result}});}).catch(error=>{if(request===quickRequestRef.current)dispatch({type:'quickSearch/apply',action:{type:'search/pageFailed',errorMessage:String(error)}});}).finally(()=>{if(request===quickRequestRef.current)quickPageRef.current=false;});return;
        }

        if (!searchInputRef.current || searchPagePendingRef.current || state.search.isSearching
          || state.search.results.length >= state.search.resultCount) return;
        const request = searchRequestRef.current;
        searchPagePendingRef.current = true;
        dispatch({ type: 'search/pageStarted' });
        void indexService.searchIndexedImages({ ...searchInputRef.current, offset: state.search.results.length, limit: 200 })
          .then(result => { if (request === searchRequestRef.current) dispatch({ type: 'search/pageSucceeded', result }); })
          .catch(error => { if (request === searchRequestRef.current) dispatch({ type: 'search/pageFailed', errorMessage: String(error) }); })
          .finally(() => { if (request === searchRequestRef.current) searchPagePendingRef.current = false; });
      },
      clearSearch: (keepWorkspace=false) => { clearQuick();selectionRequestRef.current++;searchRequestRef.current += 1; void indexService.closeSearchSnapshot(searchInputRef.current?.queryId).catch(() => {}); searchInputRef.current = null; searchPagePendingRef.current = false; dispatch({ type: 'search/clear',keepWorkspace }); },
      selectItem: (itemId) => {selectionRequestRef.current++;dispatch({ type: 'explorer/selectItem', itemId });},
      toggleItemSelection: (itemId) => {selectionRequestRef.current++;dispatch({ type: 'explorer/toggleSelection', itemId });},
      selectRangeToItem: (itemId, visibleItemIds) => {selectionRequestRef.current++;dispatch({ type: 'explorer/selectRange', itemId, visibleItemIds });},
      selectAllVisible: (visibleItemIds) => {selectionRequestRef.current++;dispatch({ type: 'explorer/selectAllVisible', visibleItemIds });},
      resolveSelectedImages,
      openSearchViewer: async (itemId) => {
        const item=state.search.results.find(candidate=>candidate.id===itemId),input=activeSearchInput();
        if(!item || !input?.queryId)return;
        try {await invoke('open_search_viewer_window',{input:{...input},activePath:item.path});}
        catch(error){reportAppError(error,{domain:'filesystem',operation:localeText("main", "ui.AppStateContext.fb74b52d651652b9"),relatedPath:item.path});}
      },
      selectAllInWorkspace: (visibleItemIds) => {
        if(!isQueryView(state) || !activeSearchInput()?.queryId) {dispatch({type:'explorer/selectAllVisible',visibleItemIds});return;}
        if(state.search.selectionSnapshot) {
          selectionRequestRef.current++;
          dispatch({type:'search/selectSnapshot',input:state.search.selectionSnapshot.input,sourceQueryId:activeSearchInput()!.queryId!,totalCount:state.search.selectionSnapshot.totalCount});return;
        }
        if(selectionPendingRef.current)return;
        selectionPendingRef.current=true;
        const input={...activeSearchInput()!},sourceQueryId=input.queryId!,request=++selectionRequestRef.current;
        void indexService.forkSearchSnapshot(sourceQueryId).then(async queryId=>{
          if(!queryId)return;
          const latest=appStateStoreRef.current!.getState();
          if(request!==selectionRequestRef.current || !isQueryView(latest) || latest.search.queryId!==sourceQueryId){await indexService.closeSearchSnapshot(queryId);return;}
          dispatch({type:'search/selectSnapshot',input:{...input,queryId},sourceQueryId,totalCount:state.search.resultCount});
        }).catch(error=>dispatch({type:'app/setStatusMessage',message:String(error),severity:'error'})).finally(()=>{selectionPendingRef.current=false;});
      },
      clearSelection: () => {selectionRequestRef.current++;dispatch({ type: 'explorer/clearSelection' });},
      openFolder: (folderId) => {
        const folder = state.explorerItems.find((item) => item.id === folderId && item.kind === 'folder');
        dispatch({ type: 'explorer/openFolder', folderId });
        if (folder) {
          void loadFolder(folder.path, 'push');
        }
      },
      openSelectedFile: (itemId) => {
        void openItemExternally(itemId);
      },
      revealSelectedInFileManager: (itemId) => {
        void revealItemExternally(itemId);
      },
      openPath: (path, navigationMode = 'push', historyTargetIndex) => loadFolder(path, navigationMode, historyTargetIndex),
      openBreadcrumb: async (path) => {
        if (!path.trim()) {
          dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.40a3c9ce09fa1d86") });
          return;
        }

        await loadFolder(path, 'push');
      },
      openContextMenu: (itemId, x, y) => dispatch({ type: 'explorer/openContextMenu', itemId, x, y }),
      closeContextMenu: () => dispatch({ type: 'explorer/closeContextMenu' }),
      removeImageByPath: (itemId, path) => {
        dispatch({ type: 'explorer/removeImageByPath', itemId, path });
      },
      setDragOver: (value) => dispatch({ type: 'dragDrop/setOver', value }),
      handleDroppedPaths: async (paths) => {
        const cleanPaths = Array.from(new Set(paths.map((path) => path.trim()).filter(Boolean)));
        if (cleanPaths.length === 0) {
          dispatch({ type: 'dragDrop/inspectFailed', errorMessage: localeText("main", "ui.AppStateContext.bb01e6c829dd2600") });
          return;
        }

        dispatch({ type: 'dragDrop/inspectStarted', paths: cleanPaths });
        try {
          const result = await fileSystemService.inspectDroppedPaths(cleanPaths);
          dispatch({ type: 'dragDrop/inspectSucceeded', items: result.items, paths: result.paths, imageCount: result.imageCount, folderCount: result.folderCount, fileCount: result.fileCount, unsupportedCount: result.unsupportedCount });
          await syncIndexForItems(result.items);
        } catch (error) {
          {
            const issue = reportAppError(error, { domain: 'dragDrop', operation: localeText("main", "ui.AppStateContext.5f94ac98ac78a87c") });
            dispatch({ type: 'dragDrop/inspectFailed', errorMessage: issue.userMessage });
          }
        }
      },
      clearDroppedPaths: () => dispatch({ type: 'dragDrop/clear' }),
      openFirstDroppedFolder: async () => {
        const folder = state.dragDrop.items.find((item) => item.kind === 'folder');
        if (!folder) {
          dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.388bb241b6aa298d") });
          return;
        }
        await loadFolder(folder.path, 'push');
      },
      selectDroppedImages: () => {
        const imageIds = state.dragDrop.items.filter((item) => item.kind === 'image').map((item) => item.id);
        if (imageIds.length === 0) {
          dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.32f70812ad710711") });
          return;
        }
        dispatch({ type: 'explorer/selectAllVisible', visibleItemIds: imageIds });
      },
      queueSelectedForProcessing: (manualOnly=false) => {
        if(processingRequestRef.current || nativeCommandRef.current || ['preparing','starting'].includes(state.taskQueue.preparationState??'') || state.renamePreview.isPreparing)return;
        processingRequestRef.current=true;
        const request=++processingGenerationRef.current;
        if(isQueryView(state) && state.search.selectionSnapshot) {
          dispatch({type:'taskQueue/selectionPreparing',preparationKey:`selection-${Date.now()}-${request}`});
          const total=state.search.selectionSnapshot.totalCount-state.search.selectionSnapshot.excludedPaths.length;
          const queryId=state.search.selectionSnapshot.input.queryId;queueSourceQueryRef.current=queryId;
          void queueImagesForProcessing([],0,total,manualOnly,request,[],queryId??undefined,state.search.selectionSnapshot.excludedPaths)
            .catch(error=>{if(request===processingGenerationRef.current)dispatch({type:'taskQueue/createBlocked',reason:String(error)});})
            .finally(()=>{queueSourceQueryRef.current=undefined;processingRequestRef.current=false;});
          return;
        }
        const selectedIds = state.selectedItemIds.length > 0 ? state.selectedItemIds : state.selectedItemId ? [state.selectedItemId] : [];
        const selectedItems = [...(state.inspectedImage?[state.inspectedImage]:[]),...state.explorerItems, ...state.search.results, ...state.dragDrop.items].filter((item) => selectedIds.includes(item.id));
        const selectedImages = selectedItems.filter((item): item is ImageItem => item.kind === 'image');
        void queueImagesForProcessing(selectedImages, selectedItems.filter(item=>item.kind!=="image"&&item.kind!=="folder").length,selectedItems.length,manualOnly,request,selectedItems.filter(item=>item.kind==="folder").map(item=>item.path))
          .catch(error=>dispatch({type:'taskQueue/createBlocked',reason:String(error)}))
          .finally(()=>{processingRequestRef.current=false;});
      },
      queueDroppedImages: () => {
        if(processingRequestRef.current || nativeCommandRef.current || ['preparing','starting'].includes(state.taskQueue.preparationState??'') || state.renamePreview.isPreparing)return;
        processingRequestRef.current=true;const request=++processingGenerationRef.current;
        const droppedImages = state.dragDrop.items.filter((item): item is ImageItem => item.kind === 'image');
        dispatch({ type: 'processing/queueDroppedImages' });
        void queueImagesForProcessing(droppedImages, state.dragDrop.fileCount + state.dragDrop.folderCount + state.dragDrop.unsupportedCount,droppedImages.length,false,request)
          .catch(error=>dispatch({type:'taskQueue/createBlocked',reason:String(error)}))
          .finally(()=>{processingRequestRef.current=false;});
      },
      addDroppedImagesToCollection: async (collectionId) => {
        const imageIds = state.dragDrop.items
          .filter((item): item is ImageItem => item.kind === 'image' && Boolean(item.indexedImageId))
          .map((image) => image.indexedImageId!)
          .filter(Boolean);
        if (imageIds.length === 0) {
          dispatch({ type: 'collections/addImagesFailed', errorMessage: localeText("main", "ui.AppStateContext.a5f5ee1f1b7d2f9d") });
          return;
        }

        try {
          const result = await collectionService.addImagesToCollection({ collectionId, imageIds });
          dispatch({ type: 'collections/addImagesSucceeded', result, imageIds });
        } catch (error) {
          const issue = reportAppError(error, { domain: 'collections', operation: localeText("main", "ui.AppStateContext.18c8e77ad55bd43b") });
          dispatch({ type: 'collections/addImagesFailed', errorMessage: issue.userMessage });
        }
      },
      confirmRenamePreviewQueue: () => {
        const preview = state.renamePreview;
        if (!preview.result || !preview.config) {
          dispatch({ type: 'taskQueue/createBlocked', reason: localeText("main", "ui.AppStateContext.08c8e14a4601a22d") });
          return;
        }

        if (preview.result.errors > 0 || preview.result.conflicts > 0) {
          dispatch({ type: 'taskQueue/createBlocked', reason: localeText("main", "ui.AppStateContext.407ddc891798c296") });
          return;
        }

        const images = renamePreviewImagesRef.current.filter(item => preview.imageIds.includes(item.id));

        if (images.length === 0) {
          dispatch({ type: 'taskQueue/createBlocked', reason: localeText("main", "ui.AppStateContext.b18bae6b41b5ca23") });
          return;
        }

        const tasks = createAiAnalysisTasks(images, preview.config).map(task => ({
          ...task, renamePlanId: preview.result!.items.find(item => item.oldPath === task.targetPath)?.planId,
        }));
        dispatch({ type: 'renamePreview/accepted' });
        dispatch({
          type: 'taskQueue/createFromSelection',
          tasks,
          skippedNonImageCount: preview.skippedNonImageCount,
          skippedByLimitCount: preview.skippedByLimitCount,
          skippedByCostCount: preview.skippedByCostCount,
          config: preview.config,targetJobId:preview.targetJobId,targetVersion:preview.targetVersion,
        });
      },
      cancelRenamePreview: () => {
        renamePreviewRequestRef.current += 1;
        renamePreviewImagesRef.current = [];
        dispatch({ type: 'renamePreview/cancel' });
      },
      selectNativeJob: (jobId) => { dispatch({ type: 'taskQueue/selectNativeJob', jobId }); window.dispatchEvent(new CustomEvent('smart-photo-indexer:processing-wake')); },
      commandQueueJob:(jobId,action)=>commandNativeQueue(action,jobId),
      startTaskQueue: () => { void commandNativeQueue('start'); },
      pauseTaskQueue: () => { cloudConsent.settle(false);void commandNativeQueue('pause'); },
      resumeTaskQueue: () => { void commandNativeQueue('resume'); },
       stopTaskQueue: () => {
          cloudConsent.settle(false);
         const key=state.taskQueue.preparationKey;
         if(key&&state.taskQueue.preparationState==='preparing'&&state.taskQueue.pendingPreparation?.config.queueMode===(state.settings.processingLimits.manualQueue?'manual':'direct')){
           processingGenerationRef.current++;
           dispatch({type:'taskQueue/preparationCancelled',preparationKey:key});void nativeQueueService.cancelPreparation(key);return;
         }
         void commandNativeQueue('stop');
       },
      clearFinishedTasks: () => {try{saveUiPreferences({queueHideFinished:!state.taskQueue.hideFinished});dispatch({type:'taskQueue/toggleFinished'});}catch(error){dispatch({type:'app/setStatusMessage',message:localeText("main", "ui.AppStateContext.883649d476a340c1", {"value1": (String(error))}),severity:'error'});}},
      retryFailedTasks: () => { if(state.taskQueue.nativeJobId)void commandNativeQueue('retry',state.taskQueue.nativeJobId); },
      rescanIndex: async () => {
        const rootPath = state.currentPath;
        dispatch({ type: 'index/rescanStarted', rootPath });
        await requestIndexRescanJob({ rootPath, includeSubfolders: state.includeSubfolders });
      },
      loadFolder,
      chooseFolder: async () => {
        dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.6ddcbf2542088c13") });
        try {
          const selectedPath = await fileSystemService.chooseFolder(state.currentPath);
          if (!selectedPath) {
            dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.ef25e6409260b2e1") });
            return;
          }
          await loadFolder(selectedPath, 'push');
        } catch (error) {
          const issue = reportAppError(error, { domain: 'filesystem', operation: localeText("main", "ui.AppStateContext.ceee6ba8c106a2ce"), relatedPath: state.currentPath });
          dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.264ef2a6470d03cc", {"value1": (issue.userMessage)}) ,severity:'error'});
        }
      },
      reloadFolder: async () => {
        await loadFolder(state.currentPath, 'replace');
      },
      applyBatchFileRenames: mappings => dispatch({type:'filesystem/batchRenamed',mappings}),
      goUp: async () => {
        if (!state.parentPath) {
          dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.05898d125e8e9938") });
          return;
        }

        await loadFolder(state.parentPath, 'push');
      },
      goBack: async () => {
        if (state.navigationIndex <= 0) {
          dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.ff74c41602946f8c") });
          return;
        }

        await openHistoryIndex(state.navigationIndex - 1);
      },
      goForward: async () => {
        if (state.navigationIndex < 0 || state.navigationIndex >= state.navigationHistory.length - 1) {
          dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AppStateContext.e977f69ea9a53253") });
          return;
        }

        await openHistoryIndex(state.navigationIndex + 1);
      },
      setIncludeSubfolders: (value) => dispatch({ type: 'toolbar/setIncludeSubfolders', value }),
      setShowImagesOnly: (value) => dispatch({ type: 'toolbar/setShowImagesOnly', value }),
      setSmartRename: (value) => dispatch({ type: 'toolbar/setSmartRename', value }),
      setViewSize: (value) => dispatch({ type: 'toolbar/setViewSize', value }),
      setFolderGrouping:(value)=>dispatch({type:'toolbar/setFolderGrouping',value}),
      setSortMode: (value) => dispatch({ type: 'toolbar/setSortMode', value }),
      setAiConsent: value=>{void patchProfilePreferences({cloudConsent:value}).catch(error=>reportAppError(error,{domain:'settings',operation:localeText("main", "ui.AppStateContext.d1e3ec8f505b484c")}));},
      toggleSettingsPanel: () => dispatch({ type: 'settings/togglePanel' }),
      setSettingsPanelOpen: (value) => dispatch({ type: 'settings/setPanelOpen', value }),
      selectTaskImage: async(task,openEditor=false)=>{
        const request=++selectionRequestRef.current,owner=latestState.current;
        try{
          if(task.pendingNativeIdentity)throw Error(localeText("main", "ui.AppStateContext.c3c4064c4218db65"));
          const item=await invoke<IndexSearchResultItem>('get_processing_task_image',{taskId:task.id});
          if(request!==selectionRequestRef.current||latestState.current.activeWorkspace!==owner.activeWorkspace||latestState.current.currentPath!==owner.currentPath)return;
          const image=toSearchImageItem(item);
          dispatch({type:'explorer/inspectImage',image});
          if(openEditor)await editorService.open(image.path);
        }catch(error){if(request===selectionRequestRef.current)dispatch({type:'app/setStatusMessage',message:localeText("main", "ui.AppStateContext.abc2cdb134c6d155")+String(error),severity:'error'});}
      },
      openSelectedPhotoEditor: async()=>{
        const snapshot=state.search.selectionSnapshot;
        const count=isQueryView(state)&&snapshot?snapshot.totalCount-snapshot.excludedPaths.length:selectSelectedImageItems(state).length;
        if(count!==1){dispatch({type:'app/setStatusMessage',message:localeText("main", "ui.AppStateContext.012200674d2ac898")});return;}
        try{
          const images=isQueryView(state)&&snapshot?await resolveSelectedImages(2):selectSelectedImageItems(state);
          const current=latestState.current;
          if(current.activeWorkspace!==state.activeWorkspace||current.currentPath!==state.currentPath||current.search.selectionSnapshot!==snapshot||current.selectedItemIds.join('\0')!==state.selectedItemIds.join('\0'))return;
          if(images.length!==1)throw new Error(localeText("main", "ui.AppStateContext.1eece96530fdb0b3"));
          await editorService.open(images[0].path);
        }catch(error){dispatch({type:'app/setStatusMessage',message:localeText("main", "ui.AppStateContext.eb987c20c47c9eb7", {"value1": (String(error))}),severity:'error'});}
      },
      setAiProviderMode: (value) => dispatch({ type: 'settings/setAiProviderMode', value }),
      setFalPreset: (presetId) => dispatch({ type: 'settings/setFalPreset', presetId }),
      setOpenRouterApiKey: (value) => dispatch({ type: 'settings/setOpenRouterApiKey', value }),
      setOpenRouterPreset: (presetId) => dispatch({ type: 'settings/setOpenRouterPreset', presetId }),
      setOpenRouterCustomModel: (value) => dispatch({ type: 'settings/setOpenRouterCustomModel', value }),
      setLocalAiPreset: (presetId) => dispatch({ type: 'settings/setLocalAiPreset', presetId }),
      setLocalAiCustomModel: (value) => dispatch({ type: 'settings/setLocalAiCustomModel', value }),
      setLocalAiEndpoint: (value) => dispatch({ type: 'settings/setLocalAiEndpoint', value }),
      setShowApiKey: (value) => dispatch({ type: 'settings/setShowApiKey', value }),
      setMaxImagesPerRun: (value) => dispatch({ type: 'settings/setMaxImagesPerRun', value }),
      setRequestDelayMs: (value) => dispatch({ type: 'settings/setRequestDelayMs', value }),
      setMaxCostUsd: (value) => dispatch({ type: 'settings/setMaxCostUsd', value }),
      setStopOnFirstAiError: (value) => dispatch({ type: 'settings/setStopOnFirstAiError', value }),
      setMaxRetriesPerTask: (value) => dispatch({ type: 'settings/setMaxRetriesPerTask', value }),
      setRenameEnabled: (value) => dispatch({ type: 'settings/setRenameEnabled', value }),
      setRenameStopOnError: (value) => dispatch({ type: 'settings/setRenameStopOnError', value }),
      checkOpenRouterKey: () => dispatch({ type: 'settings/checkOpenRouterKey' }),
      setLocalAiStatus: (status) => dispatch({ type: 'settings/setLocalAiStatus', status }),
      setOpenRouterStatus: (status) => dispatch({ type: 'settings/setOpenRouterStatus', status }),
      updateImageMetadata,
      dismissAppIssue: (issueId) => dispatch({ type: 'appError/dismiss', issueId }),
      clearAppIssues: () => dispatch({ type: 'appError/clearAll' }),
      setStatusMessage: (message,severity='info') => dispatch({ type: 'app/setStatusMessage', message,severity }),
    };
  }, [state]);
  const liveActionsRef = useRef(actions);
  liveActionsRef.current = actions;
  const stableActions = useMemo(() => Object.fromEntries(Object.keys(actions).map(name => [name,
    (...args: unknown[]) => Reflect.apply(Reflect.get(liveActionsRef.current, name), liveActionsRef.current, args),
  ])) as unknown as AppActions, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let unlisten: (() => void) | null = null;
    let unlistenDeleted: (() => void) | null = null;

    void listenForImageFavoriteUpdates((result) => {
      dispatch({ type: 'collections/favoriteUpdated', result });
    }).then((cleanup) => {
      unlisten = cleanup;
    });

    void listenForViewerImageDeleted((payload) => {
      dispatch({ type: 'explorer/removeImageByPath', itemId: payload.itemId, path: payload.path });
    }).then((cleanup) => {
      unlistenDeleted = cleanup;
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
      if (unlistenDeleted) {
        unlistenDeleted();
      }
    };
  }, []);

  useEffect(() => {
    let isCancelled = false;

    async function initializeIndex() {
      dispatch({ type: 'index/initializeStarted' });

      try {
        const info = await indexService.initializeDatabase();
        if (!isCancelled) {
          dispatch({ type: 'index/initializeSucceeded', info });
        }
      } catch (error) {
        if (!isCancelled) {
          const issue = reportAppError(error, { domain: 'sqlite', operation: localeText("main", "ui.AppStateContext.a3e533996395e638") });
          dispatch({ type: 'index/initializeFailed', errorMessage: issue.userMessage });
        }
      }
    }

    void initializeIndex();

    return () => {
      isCancelled = true;
    };
  }, []);

  useEffect(() => {
    let isCancelled = false;

    async function loadInitialCollections() {
      dispatch({ type: 'collections/loadStarted' });
      try {
        const result = await collectionService.getCollections();
        if (!isCancelled) {
          dispatch({ type: 'collections/loadSucceeded', result });
        }
      } catch (error) {
        if (!isCancelled) {
          const issue = reportAppError(error, { domain: 'collections', operation: localeText("main", "ui.AppStateContext.4ccce4aafe425578") });
          dispatch({ type: 'collections/loadFailed', errorMessage: issue.userMessage });
        }
      }
    }

    void loadInitialCollections();

    return () => {
      isCancelled = true;
    };
  }, []);

  useEffect(() => {
    const timer=window.setTimeout(()=>saveWorkspacePreferences({lastPath:state.currentPath,viewSize:state.viewSize,sortMode:state.sortMode,folderGrouping:state.folderGrouping,includeSubfolders:state.includeSubfolders,showImagesOnly:state.showImagesOnly}),300);
    return ()=>window.clearTimeout(timer);
  },[state.currentPath,state.viewSize,state.sortMode,state.folderGrouping,state.includeSubfolders,state.showImagesOnly]);

  useEffect(()=>{
    const q=baseState.quickSearch;if(!q?.query.trim()||q.composing||q.search.errorMessage)return;
    quickTimerRef.current=setTimeout(()=>{quickTimerRef.current=null;liveActionsRef.current.submitToolbarSearch();},300);
    return()=>{if(quickTimerRef.current!==null)clearTimeout(quickTimerRef.current);quickTimerRef.current=null;};
  },[baseState.quickSearch?.query,baseState.quickSearch?.composing,baseState.quickSearch?.search.rootPath,baseState.quickSearch?.search.fields,baseState.quickSearch?.search.sortOrder,baseState.quickSearch?.dataRevision,baseState.quickSearch?.search.needsRefresh,baseState.quickSearch?.search.isSearching,baseState.quickSearch?.search.selectionSnapshot]);
  useEffect(()=>{
    const s=baseState.search;
    if(baseState.quickSearch?.active||baseState.activeWorkspace==='explorer'||!s.needsRefresh||s.isSearching||s.errorMessage||!s.lastInput)return;
    // Refresh only the already applied criteria; unfinished advanced-search edits stay drafts.
    if(searchInputSignature(s.lastInput)!==searchInputSignature(searchDraftInput(baseState.searchQuery,s)))return;
    const timer=setTimeout(()=>liveActionsRef.current.submitSearch(),300);return()=>clearTimeout(timer);
  },[baseState.activeWorkspace,baseState.quickSearch?.active,baseState.searchQuery,baseState.search]);
  useEffect(()=>()=>{quickRequestRef.current++;if(quickTimerRef.current!==null)clearTimeout(quickTimerRef.current);void indexService.closeSearchSnapshot(quickInputRef.current?.queryId).catch(()=>{});},[]);
  const snapshotHandlesRef=useRef(new Set<string>());
  useEffect(()=>{
    const ids=new Set<string>();if(queueSourceQueryRef.current)ids.add(queueSourceQueryRef.current);if(state.taskQueue.pendingPreparation?.config.sourceQueryId)ids.add(state.taskQueue.pendingPreparation.config.sourceQueryId);if(baseState.search.queryId)ids.add(baseState.search.queryId);if(baseState.search.selectionSnapshot?.input.queryId)ids.add(baseState.search.selectionSnapshot.input.queryId);
    if(state.search.queryId)ids.add(state.search.queryId);
    if(state.search.selectionSnapshot?.input.queryId)ids.add(state.search.selectionSnapshot.input.queryId);
    for(const [mode,context] of Object.entries(state.workspaceContexts??{})){
      if(mode===state.activeWorkspace||!context)continue;
      if(context.search.queryId)ids.add(context.search.queryId);
      if(context.search.selectionSnapshot?.input.queryId)ids.add(context.search.selectionSnapshot.input.queryId);
    }
    for(const id of snapshotHandlesRef.current)if(!ids.has(id))void indexService.closeSearchSnapshot(id).catch(()=>{});
    snapshotHandlesRef.current=ids;
  },[state.search.queryId,state.search.selectionSnapshot?.input.queryId,state.workspaceContexts,state.activeWorkspace,baseState.search.queryId,baseState.search.selectionSnapshot?.input.queryId,state.taskQueue.pendingPreparation]);
  useEffect(()=>()=>{for(const id of snapshotHandlesRef.current)void indexService.closeSearchSnapshot(id).catch(()=>{});},[]);

  useEffect(() => {
    let isCancelled = false;
    const workspace=loadWorkspacePreferences();
    if(isTauriRuntime() && !workspace.lastPath)return;

    async function scanInitialFolder() {
      const request = ++folderRequestRef.current;
      folderScanAbortRef.current?.abort();
      const abort = new AbortController(); folderScanAbortRef.current = abort;
      let firstPage = true;
      const syncQueue=createFolderSyncQueue(items=>syncIndexForItems(items,()=>!isCancelled && request===folderRequestRef.current && !abort.signal.aborted),abort.signal);
      dispatch({ type: 'filesystem/scanStarted', requestedPath: workspace.lastPath });

      try {
        await fileSystemService.scanDirectory({ path: workspace.lastPath, includeSubfolders: workspace.includeSubfolders, imagesOnly: workspace.showImagesOnly, readMetadata:true, signal: abort.signal,
          onPage: async result => {
            if (isCancelled || request !== folderRequestRef.current) return;
            if (firstPage) { firstPage = false; dispatch({ type: 'filesystem/scanSucceeded', result, navigationMode: 'replace' }); }
            else dispatch({ type: 'filesystem/pageSucceeded', result });
            await syncQueue.enqueue(result.items);
          } });
        await syncQueue.drain();
        if(!isCancelled && request===folderRequestRef.current)dispatch({type:'filesystem/scanFinished',requestedPath:workspace.lastPath});
      } catch (error) {
        if (!isCancelled && request === folderRequestRef.current && !abort.signal.aborted) {
          const issue = reportAppError(error, { domain: 'filesystem', operation: localeText("main", "ui.AppStateContext.2cb3695af86b6254") });
          dispatch({ type: 'filesystem/scanFailed', errorMessage: issue.userMessage });
        }
      }
    }

    void scanInitialFolder();

    return () => {
      isCancelled = true;
    };
  }, []);

  useLayoutEffect(() => {
    appStateStoreRef.current?.setState(state);
  }, [state]);

  return (
    <AppDispatchContext.Provider value={dispatch}>
      <AppStateStoreContext.Provider value={appStateStoreRef.current}>
        <AppStateContext.Provider value={state}>
        <AppActionsContext.Provider value={stableActions}>
            <IndexBackgroundJobController dispatch={dispatch} reportAppError={reportAppError} />
            <TaskQueueRuntimeController state={state} dispatch={dispatch} reportAppError={reportAppError} />
            {children}
             {cloudConsent.pending?<CloudPhotoConsentModal key={cloudConsent.pending.id} request={cloudConsent.pending} onApprove={()=>cloudConsent.settle(true)} onCancel={()=>cloudConsent.settle(false)}/>:null}
          </AppActionsContext.Provider>
        </AppStateContext.Provider>
      </AppStateStoreContext.Provider>
    </AppDispatchContext.Provider>
  );
}

export function useAppState(): AppState {
  useCurrentLocale();
  const state = useContext(AppStateContext);
  if (!state) {
    throw new Error(localeText("main", "ui.AppStateContext.2bd879061250f5e8"));
  }

  return state;
}

export function useAppActions(): AppActions {
  useCurrentLocale();
  const actions = useContext(AppActionsContext);
  if (!actions) {
    throw new Error(localeText("main", "ui.AppStateContext.506d2f8e3cd35891"));
  }

  return actions;
}

export function useAppSelector<T>(selector: (state: AppState) => T, isEqual: (left: T, right: T) => boolean = Object.is): T {
  useCurrentLocale();
  const store = useContext(AppStateStoreContext);
  if (!store) {
    throw new Error(localeText("main", "ui.AppStateContext.d743592b54d8dc3b"));
  }

  const selectedRef = useRef<T | null>(null);
  const hasSelectedRef = useRef(false);

  const getSelectedSnapshot = () => {
    const nextSelected = selector(store.getState());
    if (hasSelectedRef.current && isEqual(selectedRef.current as T, nextSelected)) {
      return selectedRef.current as T;
    }

    hasSelectedRef.current = true;
    selectedRef.current = nextSelected;
    return nextSelected;
  };

  return useSyncExternalStore(store.subscribe, getSelectedSnapshot, getSelectedSnapshot);
}

export function useAppDispatch(): Dispatch<AppAction> {
  useCurrentLocale();
  const dispatch = useContext(AppDispatchContext);
  if (!dispatch) {
    throw new Error(localeText("main", "ui.AppStateContext.4256ab0e0494d065"));
  }

  return dispatch;
}
