import type { DetailsTab, LocalAiReadinessOperation, SortMode, ViewSize, WorkspaceMode } from '../types';
import type { DirectoryScanResult } from '../../modules/filesystem/types';
import type { IndexDatabaseInfo, IndexMetadataUpdateInput, IndexMetadataUpdateResult, IndexSearchResult, IndexSearchStatusFilter, IndexRescanResult, IndexSyncResult } from '../../modules/index/types';
import type { AiProviderMode, AppSettings, LocalAiModelPresetId, OpenRouterModelPresetId } from '../../modules/settings/Settings.types';
import type { QueueTask, TaskQueueRunConfig } from '../../modules/taskQueue/types';
import type { ImageAnalysisResult } from '../../modules/ai/AiProvider';
import type { RenamePreviewResult, SmartRenameResult } from '../../modules/rename/RenameService';
import type { SearchPresetDefinition } from '../../modules/search/SearchPresetService';
import type { SearchExportFormat, SearchExportScope } from '../../modules/search/SearchExportService';
import type { CollectionsResult, CollectionMutationResult, SetImageFavoriteResult } from '../../modules/collections/types';
import type { AppErrorIssue } from '../../modules/errors/AppError';
import type { LocalAiStatus } from '../../modules/localAi/LocalAiStatus';
import type { LocalAiOperationId, LocalModelKey } from '../../modules/localAi/LocalAiKeys';
import type { OpenRouterCheckResult } from '../../modules/openRouter/OpenRouterCheckService';

export type NavigationMode = 'push' | 'replace' | 'history';

export type AppAction =
  | {type:'filesystem/actionsApplied';outcomes:import('../../modules/filesystem/FileActions').FileOutcome[];images:import('../../modules/explorer/types').ImageItem[]}
  | {type:'quickSearch/update';query?:string;field?:'fields'|'rootPath'|'sortOrder';value?:string;composing?:boolean}
  | {type:'quickSearch/apply';action:AppAction;dataRevision?:number}
  | {type:'quickSearch/clear'}
  | {type:'explorer/inspectImage';image:import('../../modules/explorer/types').ImageItem}
  | { type: 'filesystem/batchRenamed'; mappings:Array<[string,string]> }
  | { type: 'workspace/setActive'; mode: WorkspaceMode }
  | { type: 'workspace/openCollection'; collectionId: string }
  | { type: 'settings/setAnalysisOutput'; value: import('../../modules/settings/Settings.types').AnalysisOutputMode }
  | { type: 'details/setActiveTab'; tab: DetailsTab }
  | { type: 'search/setQuery'; query: string }
  | {type:'explorer/setNameFilter';value:string}
  | { type: 'search/setStatusFilter'; value: IndexSearchStatusFilter }
  | { type: 'search/setTagFilterText'; value: string }
  | { type: 'search/setTagMode'; value: 'any' | 'all' }
  | { type: 'search/addTagFilter'; tag: string }
  | { type: 'search/removeTagFilter'; tag: string }
  | { type: 'search/setDateFrom'; value: string }
  | { type: 'search/setDateTo'; value: string }
  | { type: 'search/setScope'; field: 'fields' | 'collectionId' | 'rootPath' | 'sortOrder' | 'dateField' | 'expandTerms' | 'untaggedOnly' | 'favoriteOnly'; value: string }
  | { type: 'search/applyQuickFilter'; filter: 'processedToday' | 'missing' | 'errors' | 'untagged' | 'favorites' }
  | { type: 'search/presetSaved'; presets: SearchPresetDefinition[] }
  | { type: 'search/presetApplied'; preset: SearchPresetDefinition; presets: SearchPresetDefinition[] }
  | { type: 'search/presetDeleted'; presets: SearchPresetDefinition[] }
  | { type: 'search/presetFavoriteToggled'; presets: SearchPresetDefinition[] }
  | { type: 'search/presetRenamed'; presets: SearchPresetDefinition[] }
  | { type: 'search/exportStarted'; scope: SearchExportScope; format: SearchExportFormat }
  | { type: 'search/exportSucceeded'; exportedCount: number; fileName: string; format: SearchExportFormat }
  | { type: 'search/exportFailed'; errorMessage: string }
  | { type: 'search/exportCancelled' }
  | { type: 'search/copyPathsSucceeded'; copiedCount: number; scope: SearchExportScope }
  | { type: 'search/copyPathsFailed'; errorMessage: string }
  | { type: 'search/revealFoldersStarted'; count: number; scope: SearchExportScope }
  | { type: 'search/revealFoldersSucceeded'; openedCount: number; skippedCount: number }
  | { type: 'search/revealFoldersFailed'; errorMessage: string }
  | { type: 'collections/loadStarted' }
  | { type: 'collections/loadSucceeded'; result: CollectionsResult }
  | { type: 'collections/loadFailed'; errorMessage: string }
  | { type: 'collections/createSucceeded'; result: CollectionMutationResult }
  | { type: 'collections/createFailed'; errorMessage: string }
  | { type: 'collections/mutated'; collectionId: string; oldName: string; newName?: string; imageIds?: string[]; deleted?: boolean; result: CollectionMutationResult }
  | { type: 'collections/addImagesSucceeded'; result: CollectionMutationResult; imageIds: string[] }
  | { type: 'collections/addImagesFailed'; errorMessage: string }
  | { type: 'collections/favoriteUpdated'; result: SetImageFavoriteResult }
  | { type: 'collections/favoriteFailed'; errorMessage: string }
  | { type: 'search/submit' }
  | { type: 'search/start'; query: string; workspace?:WorkspaceMode }
  | { type: 'search/pageStarted' }
  | { type: 'search/pageSucceeded'; result: IndexSearchResult }
  | { type: 'search/pageFailed'; errorMessage: string }
  | { type: 'search/succeeded'; result: IndexSearchResult; sortOrder?: string; workspace?:WorkspaceMode; input?:import('../../modules/index/types').IndexSearchInput;dataRevision?:number }
  | { type: 'search/failed'; errorMessage: string }
  | { type: 'search/clear'; keepWorkspace?: boolean }
  | { type: 'search/selectSnapshot'; input: import('../../modules/index/types').IndexSearchInput; sourceQueryId: string; totalCount: number }
  | { type: 'explorer/selectItem'; itemId: string | null }
  | { type: 'explorer/toggleSelection'; itemId: string }
  | { type: 'explorer/selectRange'; itemId: string; visibleItemIds: string[] }
  | { type: 'explorer/selectAllVisible'; visibleItemIds: string[] }
  | { type: 'explorer/clearSelection' }
  | { type: 'explorer/openFolder'; folderId: string }
  | { type: 'explorer/openContextMenu'; itemId: string | null; x: number; y: number }
  | { type: 'explorer/closeContextMenu' }
  | { type: 'explorer/removeImageByPath'; itemId?: string | null; path: string }
  | { type: 'dragDrop/setOver'; value: boolean }
  | { type: 'dragDrop/inspectStarted'; paths: string[] }
  | { type: 'dragDrop/inspectSucceeded'; items: import('../../modules/explorer/types').ExplorerItem[]; paths: string[]; imageCount: number; folderCount: number; fileCount: number; unsupportedCount: number }
  | { type: 'dragDrop/inspectFailed'; errorMessage: string }
  | { type: 'dragDrop/clear' }
  | { type: 'processing/queueDroppedImages' }
  | { type: 'processing/queueSelectedImages' }
  | { type: 'renamePreview/start'; targetJobId?:string|null;targetVersion?:number; imageIds: string[]; skippedNonImageCount: number; skippedByLimitCount: number; skippedByCostCount: number; config: TaskQueueRunConfig }
  | { type: 'renamePreview/succeeded'; result: RenamePreviewResult }
  | { type: 'renamePreview/failed'; errorMessage: string }
  | { type: 'renamePreview/cancel' }
  | { type: 'renamePreview/accepted' }
  | { type: 'taskQueue/createFromSelection'; targetJobId?:string|null;targetVersion?:number; tasks: QueueTask[]; skippedNonImageCount: number; skippedByLimitCount: number; skippedByCostCount: number; config: TaskQueueRunConfig }
  | { type: 'taskQueue/createBlocked'; reason: string }
  | { type: 'taskQueue/consentCancelled' }
  | { type: 'taskQueue/nativePrepared'; preparationKey: string; jobId: string; result?:import('../../modules/taskQueue/NativeQueueService').PreparedQueue }
  | { type: 'taskQueue/preparationCancelled'; preparationKey: string }
  | { type: 'taskQueue/selectionPreparing'; preparationKey: string }
  | { type: 'taskQueue/toggleFinished' }
  | { type: 'taskQueue/nativeCommandPending'; jobId: string; pending: boolean; error?: string }
  | { type: 'taskQueue/selectNativeJob'; jobId: string }
  | { type: 'taskQueue/nativeObserved'; snapshot: import('../../modules/taskQueue/NativeQueueService').ProcessingSnapshot }
  | { type: 'processing/fileRenamed'; result: SmartRenameResult }
  | { type: 'taskQueue/start' }
  | { type: 'taskQueue/pause' }
  | { type: 'taskQueue/resume' }
  | { type: 'taskQueue/stop' }
  | { type: 'taskQueue/clearFinished' }
  | { type: 'taskQueue/taskStarted'; taskId: string }
  | { type: 'taskQueue/taskProgress'; taskId: string; progress: number }
  | { type: 'taskQueue/taskSucceeded'; taskId: string; result?: ImageAnalysisResult }
  | { type: 'taskQueue/taskRenameSucceeded'; taskId: string; result: SmartRenameResult }
  | { type: 'taskQueue/taskFailed'; taskId: string; errorMessage: string; errorCode?: string; retryable?: boolean; retryAfterMs?: number }
  | { type: 'taskQueue/retryFailed' }
  | { type: 'filesystem/scanStarted'; requestedPath: string | null; preserveSelection?: boolean }
  | { type: 'filesystem/scanSucceeded'; result: DirectoryScanResult; navigationMode: NavigationMode; historyTargetIndex?: number; preserveSelection?: boolean }
  | { type: 'filesystem/pageSucceeded'; result: DirectoryScanResult }
  | { type: 'filesystem/scanFinished'; requestedPath: string | null }
  | { type: 'filesystem/scanFailed'; errorMessage: string }
  | { type: 'index/initializeStarted' }
  | { type: 'index/initializeSucceeded'; info: IndexDatabaseInfo }
  | { type: 'index/initializeFailed'; errorMessage: string }
  | { type: 'index/countsUpdated'; counts: { imageCount: number; pathCount: number; tagCount: number } }
  | { type: 'index/syncStarted'; imageCount: number }
  | { type: 'index/syncSucceeded'; result: IndexSyncResult }
  | { type: 'index/syncFailed'; errorMessage: string }
  | { type: 'index/rescanStarted'; rootPath: string | null }
  | { type: 'index/rescanSucceeded'; result: IndexRescanResult }
  | { type: 'index/rescanFailed'; errorMessage: string }
  | { type: 'index/metadataUpdateStarted'; imageName: string }
  | { type: 'index/metadataUpdateSucceeded'; result: IndexMetadataUpdateResult }
    | { type: 'index/statusesUpdated'; changes:Array<{imageId:string;paths?:string[];status:import('../../modules/explorer/types').ProcessingStatus}> }
  | { type: 'index/metadataUpdateFailed'; errorMessage: string }
  | { type: 'toolbar/setIncludeSubfolders'; value: boolean }
  | { type: 'toolbar/setShowImagesOnly'; value: boolean }
  | { type: 'toolbar/setSmartRename'; value: boolean }
  | { type: 'toolbar/setViewSize'; value: ViewSize }
  | {type:'toolbar/setFolderGrouping';value:'foldersFirst'|'filesFirst'|'mixed'}
  | { type: 'toolbar/setSortMode'; value: SortMode }
  | { type: 'privacy/setAiConsent'; value: boolean }
  | { type: 'settings/togglePanel' }
  | { type: 'settings/setPanelOpen'; value: boolean }
  | { type: 'settings/setAiProviderMode'; value: AiProviderMode }
  | { type: 'settings/setCloudCredential'; provider: 'fal' | 'openrouter'; credentialRef: string }
  | { type: 'settings/setFalPreset'; presetId: OpenRouterModelPresetId }
  | { type: 'settings/setAnalysisLimit'; name: 'passes' | 'cloudConcurrency' | 'maxTokens' | 'imageMaxSide'; value: number }
  | { type: 'settings/setOpenRouterApiKey'; value: string }
  | { type: 'settings/setOpenRouterPreset'; presetId: OpenRouterModelPresetId }
  | { type: 'settings/setOpenRouterCustomModel'; value: string }
  | { type: 'settings/setLocalAiPreset'; presetId: LocalAiModelPresetId }
  | { type: 'settings/setLocalAiCustomModel'; value: string }
  | { type: 'settings/setLocalAiEndpoint'; value: string }
  | { type: 'settings/setShowApiKey'; value: boolean }
  | { type: 'settings/setManualQueue'; value: boolean }
  | { type: 'settings/setMaxImagesPerRun'; value: number }
  | { type: 'settings/setRequestDelayMs'; value: number }
  | { type: 'settings/setMaxCostUsd'; value: number }
  | { type: 'settings/setStopOnFirstAiError'; value: boolean }
  | { type: 'settings/setOnlyUnprocessed'; value: boolean }
  | { type: 'settings/setMaxRetriesPerTask'; value: number }
  | { type: 'settings/setRenameEnabled'; value: boolean }
  | { type: 'settings/setRenameStopOnError'; value: boolean }
  | { type: 'settings/applySettings'; settings: AppSettings }
  | { type: 'settings/checkOpenRouterKey' }
  | { type: 'settings/resetApiKeyCheck' }
  | { type: 'settings/setLocalAiStatus'; status: LocalAiStatus | null }
  | { type: 'settings/setOpenRouterStatus'; status: OpenRouterCheckResult | null }
  | { type: 'localAi/autoCheckStarted'; operation: LocalAiReadinessOperation; status: LocalAiStatus }
  | { type: 'localAi/manualPrepareStarted'; operation: LocalAiReadinessOperation; status: LocalAiStatus }
  | { type: 'localAi/deleteStarted'; operation: LocalAiReadinessOperation; status: LocalAiStatus }
  | { type: 'localAi/operationProgress'; operationId: LocalAiOperationId; key: LocalModelKey; status: LocalAiStatus }
  | { type: 'localAi/operationSucceeded'; operationId: LocalAiOperationId; key: LocalModelKey; status: LocalAiStatus }
  | { type: 'localAi/operationFailed'; operationId: LocalAiOperationId; key: LocalModelKey; status: LocalAiStatus }
  | { type: 'localAi/deleteSucceeded'; operationId: LocalAiOperationId; key: LocalModelKey; status: LocalAiStatus }
  | { type: 'appError/report'; issue: AppErrorIssue }
  | { type: 'appError/dismiss'; issueId: string }
  | { type: 'appError/clearAll' }
  | { type: 'app/setStatusMessage'; message: string; severity?: 'info'|'warning'|'error'|'critical' };

export interface AppActions {
  favoriteImagePath(path:string,imageId?:string):Promise<void>;
  selectNativeJob(jobId: string): void;
  setFalPreset(presetId: OpenRouterModelPresetId): void;
  setWorkspaceMode(mode: WorkspaceMode): void;
  openCollection(collectionId: string): void;
  setDetailsTab(tab: DetailsTab): void;
  setSearchQuery(query: string): void;
  setSearchStatusFilter(value: IndexSearchStatusFilter): void;
  setSearchTagFilterText(value: string): void;
  setSearchTagMode(value: 'any' | 'all'): void;
  addSearchTagFilter(tag: string): void;
  removeSearchTagFilter(tag: string): void;
  setSearchDateFrom(value: string): void;
  setSearchDateTo(value: string): void;
  setFilenameFilter(value:string):void;
  setSearchScope(field: 'fields' | 'collectionId' | 'rootPath' | 'sortOrder' | 'dateField' | 'expandTerms' | 'untaggedOnly' | 'favoriteOnly', value: string): void;
  applySearchQuickFilter(filter: 'processedToday' | 'missing' | 'errors' | 'untagged' | 'favorites'): void;
  saveCurrentSearchPreset(name?: string): void;
  applySearchPreset(presetId: string): void;
  deleteSearchPreset(presetId: string): void;
  toggleSearchPresetFavorite(presetId: string): void;
  renameSearchPreset(presetId: string, name: string): void;
  exportSearchResults(format: SearchExportFormat, scope: SearchExportScope): void;
  copySearchResultPaths(scope: SearchExportScope): Promise<void>;
  revealSearchResultFolders(scope: SearchExportScope): Promise<void>;
  loadCollections(): Promise<void>;
  createCollection(name: string): Promise<CollectionMutationResult|null>;
  renameCollection(collectionId: string, name: string): Promise<void>;
  deleteCollection(collectionId: string): Promise<void>;
  removeSelectedFromCollection(collectionId: string): Promise<void>;
  addSelectedImagesToCollection(collectionId: string): Promise<CollectionMutationResult|null>;
  toggleFavoriteForItem(itemId: string): Promise<void>;
  submitSearch(sortOrderOverride?:string): void;
  submitToolbarSearch(): void;
  loadMoreSearchResults(): void;
  clearSearch(keepWorkspace?:boolean): void;
  selectItem(itemId: string | null): void;
  toggleItemSelection(itemId: string): void;
  selectRangeToItem(itemId: string, visibleItemIds: string[]): void;
  selectAllVisible(visibleItemIds: string[]): void;
  selectAllInWorkspace(visibleItemIds: string[]): void;
  resolveSelectedImages(maximum: number): Promise<import('../../modules/explorer/types').ImageItem[]>;
  openSearchViewer(itemId: string): Promise<void>;
  clearSelection(): void;
  openFolder(folderId: string): void;
  openSelectedFile(itemId: string): void;
  revealSelectedInFileManager(itemId: string): void;
  openPath(path: string, navigationMode?: NavigationMode, historyTargetIndex?: number): Promise<void>;
  openBreadcrumb(path: string): Promise<void>;
  openContextMenu(itemId: string | null, x: number, y: number): void;
  closeContextMenu(): void;
  removeImageByPath(itemId: string | null, path: string): void;
  setDragOver(value: boolean): void;
  handleDroppedPaths(paths: string[]): Promise<void>;
  clearDroppedPaths(): void;
  openFirstDroppedFolder(): Promise<void>;
  selectDroppedImages(): void;
  queueDroppedImages(): void;
  addDroppedImagesToCollection(collectionId: string): Promise<void>;
  queueSelectedForProcessing(manualOnly?:boolean): void;
  confirmRenamePreviewQueue(): void;
  cancelRenamePreview(): void;
  commandQueueJob(jobId:string,action:'start'|'resume'):Promise<void>;
  startTaskQueue(): void;
  pauseTaskQueue(): void;
  resumeTaskQueue(): void;
  stopTaskQueue(): void;
  clearFinishedTasks(): void;
  retryFailedTasks(): void;
  rescanIndex(): Promise<void>;
  loadFolder(path?: string | null, navigationMode?: NavigationMode, historyTargetIndex?: number): Promise<void>;
  chooseFolder(): Promise<void>;
  reloadFolder(): Promise<void>;
  applyBatchFileRenames(mappings:Array<[string,string]>):void;
  goUp(): Promise<void>;
  goBack(): Promise<void>;
  goForward(): Promise<void>;
  setIncludeSubfolders(value: boolean): void;
  setShowImagesOnly(value: boolean): void;
  setSmartRename(value: boolean): void;
  setViewSize(value: ViewSize): void;
  setFolderGrouping(value:'foldersFirst'|'filesFirst'|'mixed'):void;
  setSortMode(value: SortMode): void;
  setAiConsent(value: boolean): void;
  toggleSettingsPanel(): void;
  setSettingsPanelOpen(value: boolean): void;
  setAiProviderMode(value: AiProviderMode): void;
  setOpenRouterApiKey(value: string): void;
  setOpenRouterPreset(presetId: OpenRouterModelPresetId): void;
  setOpenRouterCustomModel(value: string): void;
  setLocalAiPreset(presetId: LocalAiModelPresetId): void;
  setLocalAiCustomModel(value: string): void;
  setLocalAiEndpoint(value: string): void;
  setShowApiKey(value: boolean): void;
  setMaxImagesPerRun(value: number): void;
  setRequestDelayMs(value: number): void;
  setMaxCostUsd(value: number): void;
  setStopOnFirstAiError(value: boolean): void;
  setMaxRetriesPerTask(value: number): void;
  setRenameEnabled(value: boolean): void;
  setRenameStopOnError(value: boolean): void;
  checkOpenRouterKey(): void;
  setLocalAiStatus(status: LocalAiStatus | null): void;
  setOpenRouterStatus(status: OpenRouterCheckResult | null): void;
  updateImageMetadata(input: IndexMetadataUpdateInput): Promise<void>;
  dismissAppIssue(issueId: string): void;
  clearAppIssues(): void;
  setStatusMessage(message: string,severity?:'info'|'warning'|'error'|'critical'): void;
  setQuickSearchQuery(query:string):void;
  setQuickSearchScope(field:'fields'|'rootPath'|'sortOrder',value:string):void;
  setQuickSearchComposing(value:boolean):void;
  clearQuickSearch():void;
  openSelectedPhotoEditor(): Promise<void>;
  selectTaskImage(task:QueueTask,openEditor?:boolean):Promise<void>;
}
