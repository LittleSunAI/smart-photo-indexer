import {text as localeText} from "../../shared/i18n/Runtime";
import type { AppState } from '../types';
import { createMockExplorerItems } from '../../modules/explorer/mockExplorerItems';
import { settingsService } from '../../modules/settings/SettingsService';
import { createEmptyTaskQueueState } from '../../modules/taskQueue/TaskQueueService';
import { searchPresetService } from '../../modules/search/SearchPresetService';
import { messages } from '../../shared/i18n';
import { isTauriRuntime } from '../../shared/tauri/runtime';
import { loadWorkspacePreferences } from '../../modules/settings/WorkspacePreferences';

function createInitialTaskSummary(items: ReturnType<typeof createMockExplorerItems>): AppState['taskSummary'] {
  const images = items.filter((item) => item.kind === 'image');
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
    currentTask: localeText("main", "ui.initialAppState.3740f773a760c488"),
    overallProgress: images.length > 0 ? Math.round((done / images.length) * 100) : 0,
  };
}

export function createInitialAppState(): AppState {
  const explorerItems = isTauriRuntime() ? [] : createMockExplorerItems();
  const workspace=loadWorkspacePreferences();
  const settings = settingsService.loadSettings();
  const firstImage = explorerItems.find((item) => item.kind === 'image') ?? explorerItems[0] ?? null;
  const taskQueue = createEmptyTaskQueueState();

  return {
    appName: messages().appName,
    appIssues: [],
    activeWorkspace: 'explorer',
    activeDetailsTab: 'details',
    currentPath: null,
    parentPath: null,
    pathSegments: [messages().appName],
    breadcrumbs: [{ label: messages().appName, path: '' }],
    navigationHistory: [],
    navigationIndex: -1,
    searchQuery: '',
    recentSearches: [],
    searchPresets: searchPresetService.loadPresets(),
    collections: {
      isLoading: false,
      errorMessage: null,
      items: [],
      lastLoadedAtUnix: null,
      lastMutationMessage: null,
    },
    search: {
      expandTerms: true,
      isSearching: false,
      errorMessage: null,
      results: [],
      resultCount: 0,
      lastQuery: '',
      statusFilter: 'all',
      tagFilterText: '',
      tagMode: 'any',
      dateFrom: '',
      dateTo: '',
      collectionId: '',
      rootPath: '',
      sortOrder: 'relevance',
      dateField: 'modified',
      favoriteOnly: false,
      lastAppliedTags: [],
      lastAppliedStatuses: [],
      lastAppliedDateFromUnix: null,
      lastAppliedDateToUnix: null,
      lastAppliedFavoriteOnly: false,
      lastSearchedAtUnix: null,
    },
    includeSubfolders: workspace.includeSubfolders,
    showImagesOnly: workspace.showImagesOnly,
    smartRenameEnabled: settings.rename.enabled,
    aiConsentAccepted: settings.privacy.aiConsentAccepted,
    settingsPanelOpen: false,
    settings,
    apiKeyCheckResult: null,
    localAiStatus: null,
    localAiReadiness: {
      runtimeByEndpoint: {},
      modelByKey: {},
      activeOperation: null,
    },
    openRouterStatus: null,
    viewSize: workspace.viewSize,
    folderGrouping:workspace.folderGrouping,
    explorerSortSnapshot:{},
    sortMode: workspace.sortMode,
    explorerDisplayOrder: [],
    isFolderScanActive: false,
    selectedItemId: firstImage?.id ?? null,
    selectedItemIds: firstImage ? [firstImage.id] : [],
    selectionAnchorItemId: firstImage?.id ?? null,
    contextMenu: {
      isOpen: false,
      x: 0,
      y: 0,
      itemId: null,
    },
    dragDrop: {
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
    },
    processingSelectionDraft: {
      imageIds: [],
      imageCount: 0,
      skippedCount: 0,
      smartRenameEnabled: false,
      createdAt: null,
    },
    renamePreview: {
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
    },
    explorerItems,
    isScanningFolder: false,
    filesystemError: null,
    scannedDirectories: 0,
    skippedEntries: 0,
    scanWasTruncated: false,
    statusMessage: localeText("main", "ui.initialAppState.11eca4ebfe22cd47"),
    taskSummary: createInitialTaskSummary(explorerItems),
    taskQueue,
    indexDatabase: {
      isInitializing: false,
      isReady: false,
      errorMessage: null,
      databasePath: null,
      schemaVersion: 0,
      migrationsApplied: 0,
      tables: [],
      imageCount: 0,
      pathCount: 0,
      tagCount: 0,
      lastSyncAtUnix: null,
      lastSyncedImages: 0,
      lastKnownImages: 0,
      lastNewImages: 0,
      lastRelocatedImages: 0,
      lastRescanAtUnix: null,
      lastRescanCheckedPaths: 0,
      lastMissingImages: 0,
      lastRevivedImages: 0,
    },
  };
}
