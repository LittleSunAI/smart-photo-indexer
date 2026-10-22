import type { ExplorerItem } from '../modules/explorer/types';
import type { TaskQueueState, TaskSummary } from '../modules/taskQueue/types';
import type { PathBreadcrumb } from '../modules/filesystem/types';
import type { IndexDatabaseState, SearchState } from '../modules/index/types';
import type { AppSettings, ApiKeyCheckResult } from '../modules/settings/Settings.types';
import type { RenamePreviewResult } from '../modules/rename/RenameService';
import type { TaskQueueRunConfig } from '../modules/taskQueue/types';
import type { SearchPresetDefinition } from '../modules/search/SearchPresetService';
import type { CollectionsState } from '../modules/collections/types';
import type { AppErrorIssue } from '../modules/errors/AppError';
import type { LocalAiStatus, LocalModelStatus, LocalRuntimeStatus } from '../modules/localAi/LocalAiStatus';
import type { LocalAiOperationId, LocalModelKey, LocalRuntimeKey } from '../modules/localAi/LocalAiKeys';
import type { OpenRouterCheckResult } from '../modules/openRouter/OpenRouterCheckService';

export type WorkspaceMode = 'explorer' | 'search' | 'favorites' | 'ready' | 'collections';
export type ViewSize = 'table' | 'list' | 'small' | 'medium' | 'large' | 'extraLarge';
export type SortMode = 'readyFirst' | 'readyLast' | 'errorsFirst' | 'errorsLast' | 'analyzedAsc' | 'analyzedDesc' | 'dateTakenNewest' | 'dateTakenOldest' | 'nameAsc' | 'nameDesc' | 'status' | 'statusDesc' | 'type' | 'typeDesc' | 'sizeAsc' | 'sizeDesc' | 'dimensionsAsc' | 'dimensionsDesc' | 'modifiedAsc' | 'modifiedDesc' | 'cameraAsc' | 'cameraDesc' | 'pathAsc' | 'pathDesc' | 'tagsAsc' | 'tagsDesc' | 'favoriteAsc' | 'favoriteDesc';
export type DetailsTab = 'details' | 'aiDescription' | 'tags' | 'tasks' | 'index' | 'journal';

export interface ExplorerContextMenuState {
  isOpen: boolean;
  x: number;
  y: number;
  itemId: string | null;
}

export type DropActionTarget = 'openFolder' | 'queueImages' | 'selectImages' | 'addToCollection';

export interface DragDropState {
  isDraggingOver: boolean;
  isInspecting: boolean;
  paths: string[];
  items: ExplorerItem[];
  imageCount: number;
  folderCount: number;
  fileCount: number;
  unsupportedCount: number;
  errorMessage: string | null;
  droppedAt: string | null;
}


export interface RenamePreviewState {
  targetJobId?:string|null;targetVersion?:number;
  isPreparing: boolean;
  result: RenamePreviewResult | null;
  errorMessage: string | null;
  imageIds: string[];
  skippedNonImageCount: number;
  skippedByLimitCount: number;
  skippedByCostCount: number;
  config: TaskQueueRunConfig | null;
  createdAt: string | null;
  confirmedAt: string | null;
}

export interface ProcessingSelectionDraft {
  imageIds: string[];
  imageCount: number;
  skippedCount: number;
  smartRenameEnabled: boolean;
  createdAt: string | null;
}

export interface LocalRuntimeReadinessState {
  operationId: LocalAiOperationId | null;
  endpoint: string;
  runtimeStatus: LocalRuntimeStatus;
  ollamaVersion: string | null;
  errorMessage: string | null;
  checkedAt: string | null;
}

export interface LocalModelReadinessState {
  operationId: LocalAiOperationId | null;
  key: LocalModelKey;
  endpoint: string;
  model: string;
  modelStatus: LocalModelStatus;
  availableModels: string[];
  progressPercent: number | null;
  statusText: string;
  errorMessage: string | null;
  checkedAt: string | null;
}

export interface LocalAiReadinessOperation {
  operationId: LocalAiOperationId;
  kind: 'auto-runtime-check' | 'manual-prepare-stack' | 'manual-delete-model';
  key: LocalModelKey;
  endpoint: string;
  model: string;
  startedAt: string;
}

export interface LocalAiReadinessState {
  runtimeByEndpoint: Record<LocalRuntimeKey, LocalRuntimeReadinessState>;
  modelByKey: Record<LocalModelKey, LocalModelReadinessState>;
  activeOperation: LocalAiReadinessOperation | null;
}

export interface AppState {
  quickSearch?:import('./state/QuickSearch').QuickSearchState;
  underlyingSearch?:{search:SearchState;query:string};
  workspaceContexts?:Partial<Record<WorkspaceMode,{search:SearchState;query:string;selectedItemId:string|null;selectedItemIds:string[];selectionAnchorItemId:string|null}>>;
  appName: string;
  appIssues: AppErrorIssue[];
  activeWorkspace: WorkspaceMode;
  activeDetailsTab: DetailsTab;
  currentPath: string | null;
  parentPath: string | null;
  pathSegments: string[];
  breadcrumbs: PathBreadcrumb[];
  navigationHistory: string[];
  navigationIndex: number;
  searchQuery: string;
  filenameFilter?:string;
  recentSearches: string[];
  searchPresets: SearchPresetDefinition[];
  collections: CollectionsState;
  search: SearchState;
  includeSubfolders: boolean;
  showImagesOnly: boolean;
  smartRenameEnabled: boolean;
  aiConsentAccepted: boolean;
  settingsPanelOpen: boolean;
  settings: AppSettings;
  apiKeyCheckResult: ApiKeyCheckResult | null;
  localAiStatus: LocalAiStatus | null;
  localAiReadiness: LocalAiReadinessState;
  openRouterStatus: OpenRouterCheckResult | null;
  viewSize: ViewSize;
  folderGrouping:'foldersFirst'|'filesFirst'|'mixed';
  explorerSortSnapshot?:Record<string,ExplorerItem>;
  sortMode: SortMode;
  inspectedImage?: import('../modules/explorer/types').ImageItem;
  selectedItemId: string | null;
  selectedItemIds: string[];
  selectionAnchorItemId: string | null;
  contextMenu: ExplorerContextMenuState;
  dragDrop: DragDropState;
  processingSelectionDraft: ProcessingSelectionDraft;
  renamePreview: RenamePreviewState;
  explorerItems: ExplorerItem[];
  explorerDisplayOrder: string[];
  isFolderScanActive: boolean;
  isScanningFolder: boolean;
  filesystemError: string | null;
  scannedDirectories: number;
  skippedEntries: number;
  scanWasTruncated: boolean;
  statusMessage: string;
  statusSeverity?: 'info'|'warning'|'error'|'critical';
  taskSummary: TaskSummary;
  taskQueue: TaskQueueState;
  indexDatabase: IndexDatabaseState;
}
