import fs from 'node:fs';
import path from 'node:path';
import {verifyVersions} from './version-contract.mjs';

const root = process.cwd();
const expectedVersion = verifyVersions(root);

const requiredFiles = [
  'package.json',
  'index.html',
  'tsconfig.json',
  'vite.config.ts',
  'src/main.tsx',
  'src/app/App.tsx',
  'src/app/AppShell.tsx',
  'src/app/AppTitleBar.tsx',
  'src/app/CommandBar.tsx',
  'src/app/Workspace.tsx',
  'src/app/StatusBar.tsx',
  'src/app/layout/TitleBar.tsx',
  'src/app/layout/MainToolbar.tsx',
  'src/app/layout/WorkspaceLayout.tsx',
  'src/app/layout/AppStatusBar.tsx',
  'src/app/state/AppStateContext.tsx',
  'src/app/state/appActions.ts',
  'src/app/state/appReducer.ts',
  'src/app/state/appSelectors.ts',
  'src/app/state/initialAppState.ts',
  'src/styles/global.css',
  'src/styles/theme.css',
  'src/styles/layout.css',
  'src/modules/explorer/ExplorerGrid.tsx',
  'src/modules/explorer/FileCard.tsx',
  'src/modules/explorer/ExplorerContextMenu.tsx',
  'src/modules/explorer/types.ts',
  'src/modules/filesystem/FileSystemService.ts',
  'src/modules/filesystem/FileOpenService.ts',
  'src/modules/filesystem/types.ts',
  'src/shared/utils/fileFormatting.ts',
  'src/modules/details/DetailsPanel.tsx',
  'src/modules/ai/AiProvider.ts',
  'src/modules/ai/AiProviderError.ts',
  'src/modules/ai/AiImageEncodingService.ts',
  'src/modules/ai/MockProvider.ts',
  'src/modules/ai/OpenRouterProvider.ts',
  'src/modules/ai/LocalAiProvider.ts',
  'src/modules/ai/AiProcessingService.ts',
  'src/modules/index/IndexService.ts',
  'src/modules/index/types.ts',
  'src/modules/search/SearchControls.tsx',
  'src/modules/search/SearchPresetService.ts',
  'src/modules/search/SearchExportService.ts',
  'src/modules/collections/types.ts',
  'src/modules/collections/CollectionService.ts',
  'src/modules/dragDrop/DragDropLayer.tsx',
  'src/modules/errors/AppError.ts',
  'src/modules/errors/ErrorCenter.tsx',
  'src/modules/search/SearchModule.ts',
  'src/modules/thumbnail/ThumbnailService.ts',
  'src/modules/thumbnail/CachedImage.tsx',
  'src/modules/performance/useProgressiveGridItems.ts',
  'src/shared/tauri/runtime.ts',
  'src/modules/rename/RenameService.ts',
  'src/modules/rename/RenamePreviewPanel.tsx',
  'src/modules/taskQueue/TaskQueueService.ts',
  'src/modules/actionLog/ActionLog.types.ts',
  'src/modules/actionLog/ActionLogService.ts',
  'src/modules/settings/Settings.types.ts',
  'src/modules/settings/SettingsService.ts',
  'src/modules/settings/SettingsPanel.tsx',
  'src/modules/settings/modelPresets.ts',
  'src/core/moduleRegistry.ts',
  'src-tauri/Cargo.toml',
  'src-tauri/tauri.conf.json',
  'src-tauri/src/main.rs',
  'src-tauri/src/lib.rs',
  'src/modules/photoEditor/EditorBootstrap.tsx',
  'src/modules/photoEditor/EditorWindow.tsx',
  'src/modules/photoEditor/EditorContracts.ts',
  'src/modules/photoEditor/EditorAiPanel.tsx',
  'src/modules/photoEditor/EditorVirtualList.tsx',
  'src-tauri/src/photo_editor/mod.rs',
  'src-tauri/src/photo_editor/store.rs',
  'src-tauri/src/photo_editor/jobs.rs',
  'src-tauri/src/photo_editor/catalog.rs',
  'src-tauri/src/photo_editor/backup.rs',
  'src-tauri/src/photo_editor/publication.rs',
];

const missing = requiredFiles.filter((file) => !fs.existsSync(path.join(root, file)));

if (missing.length > 0) {
  console.error('Missing required files:');
  for (const file of missing) {
    console.error(`- ${file}`);
  }
  process.exit(1);
}

for (const jsonFile of ['package.json', 'src-tauri/tauri.conf.json']) {
  const fullPath = path.join(root, jsonFile);
  JSON.parse(fs.readFileSync(fullPath, 'utf8'));
}

const packageJson = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
if (packageJson.version !== expectedVersion) {
  console.error(`Expected package version ${expectedVersion}, got ${packageJson.version}`);
  process.exit(1);
}

const tauriConfig = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
if (tauriConfig.version !== expectedVersion) {
  console.error(`Expected Tauri config version ${expectedVersion}, got ${tauriConfig.version}`);
  process.exit(1);
}

const cargoToml = fs.readFileSync(path.join(root, 'src-tauri/Cargo.toml'), 'utf8');
if (!cargoToml.includes(`version = "${expectedVersion}"`)) {
  console.error(`Expected Cargo.toml version ${expectedVersion}`);
  process.exit(1);
}

const layoutCss = fs.readFileSync(path.join(root, 'src/styles/layout.css'), 'utf8');
const requiredCssMarkers = [
  '.title-bar',
  'grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr)',
  '.main-toolbar__path-form',
  '.real-photo-thumbnail',
  '.generic-file-card',
  '.workspace-layout',
  '--right-panel-target',
  '.workspace-layout__title',
  '.app-status-bar__message',
  '.selection-check',
  '.explorer-context-menu',
  '.cached-image',
  '.thumbnail-loading-badge',
  '.explorer-grid__progressive-loader',
  '.cached-image--waiting',
  '.index-database-card',
  '.editable-metadata-section',
  '.metadata-textarea',
  '.settings-panel',
  '.settings-card',
  '.main-toolbar__settings-button',
  '.search-controls',
  '.filter-chip',
  '.recent-search-chip',
  '.journal-entry-list',
  '.journal-entry__type',
  '.rename-preview-card',
  '.rename-preview-row',
  '.index-maintenance-card',
  '.maintenance-row',
  '.search-presets-panel',
  '.search-preset-card',
  '.search-export-panel',
  '.favorite-toggle',
  '.search-collections-panel',
  '.collection-chip',
  '.drag-drop-overlay',
  '.drag-drop-panel',
  '.error-center',
  '.error-center__summary',
  '.thumbnail-error-badge',
];

for (const marker of requiredCssMarkers) {
  if (!layoutCss.includes(marker)) {
    console.error(`Missing layout marker: ${marker}`);
    process.exit(1);
  }
}


const appErrorService = fs.readFileSync(path.join(root, 'src/modules/errors/AppError.ts'), 'utf8');
const requiredAppErrorMarkers = [
  'AppErrorService',
  'AppErrorIssue',
  'filesystem',
  'sqlite',
  'thumbnail',
  'retryable',
  'suggestions',
];

for (const marker of requiredAppErrorMarkers) {
  if (!appErrorService.includes(marker)) {
    console.error(`Missing app error marker: ${marker}`);
    process.exit(1);
  }
}

const rustLib = ['lib.rs', 'file_operations.rs', 'file_safety.rs', 'search.rs', 'search-schema.sql', 'processing.rs', 'scanning.rs']
  .map(file => fs.readFileSync(path.join(root, 'src-tauri/src', file), 'utf8')).join('\n');
const requiredRustMarkers = [
  '#[tauri::command]',
  'fn scan_directory',
  'IMAGE_EXTENSIONS',
  'MAX_SCAN_PAGE_ITEMS',
  'tauri::generate_handler![',
  'get_or_create_thumbnail,',
  'thumbnail_cache::get_thumbnail_cache_statistics,',
  'thumbnail_cache::clear_thumbnail_cache,',
  'get_ai_image_file_fingerprint,',
  'encode_image_for_ai,',
  'fn inspect_dropped_paths',
  'DroppedPathsInspectionResultDto',
  'fn get_or_create_thumbnail',
  'fn get_ai_image_file_fingerprint',
  'fn encode_image_for_ai',
  'AiImageDataUrlResultDto',
  'AiImageFileFingerprintDto',
  'thumbnail_cache::directory(&app_handle)',
  'fn initialize_index_database',
  'fn sync_scanned_images_to_index',
  'CREATE TABLE IF NOT EXISTS images',
  'CREATE TABLE IF NOT EXISTS image_paths',
  'CREATE TABLE IF NOT EXISTS action_logs',
  'fn update_image_metadata',
  "'metadata_updated'",
  'fn search_indexed_images',
  'search_history',
  'CREATE VIRTUAL TABLE image_search USING fts5',
  'fn get_action_journal',
  'fn create_action_log_entry',
  'fn smart_rename_image',
  'fn preview_smart_rename_images',
  'SmartRenameResultDto',
  'SmartRenamePreviewResultDto',
  "'file_renamed'",
  'CREATE TABLE IF NOT EXISTS collections',
  'fn get_collections',
  'fn create_collection',
  'fn add_images_to_collection',
  'fn set_image_favorite',
  'prepare_ollama_runtime',
];

for (const marker of requiredRustMarkers) {
  if (!rustLib.includes(marker)) {
    console.error(`Missing Rust filesystem marker: ${marker}`);
    process.exit(1);
  }
}


const nativeAi = ['src-tauri/src/processing.rs','src-tauri/src/provider_transport.rs','src/modules/taskQueue/NativeQueueService.ts'].map(file => fs.readFileSync(path.join(root,file),'utf8')).join('\n');
for (const marker of ['prepare_processing_job','send_with_admission','persist_remote','get_processing_snapshot','max_tokens','num_gpu','json_schema','image_urls','validate_result']) {
  if (!nativeAi.includes(marker)) { console.error(`Missing native AI marker: ${marker}`); process.exit(1); }
}
const observer=fs.readFileSync(path.join(root,'src/app/state/TaskQueueRuntimeController.tsx'),'utf8');
if(observer.includes('analyzeQueueTask') || observer.includes('aiProcessingService')) { console.error('React must not execute inference.');process.exit(1); }

const searchPresetService = fs.readFileSync(path.join(root, 'src/modules/search/SearchPresetService.ts'), 'utf8');
const requiredSearchPresetMarkers = [
  'searchPresetService',
  'savePreset',
  'toggleFavorite',
  'markPresetUsed',
  'smart-photo-indexer.search-presets.v1',
];

for (const marker of requiredSearchPresetMarkers) {
  if (!searchPresetService.includes(marker)) {
    console.error(`Missing search preset service marker: ${marker}`);
    process.exit(1);
  }
}

const indexService = fs.readFileSync(path.join(root, 'src/modules/index/IndexService.ts'), 'utf8');
const requiredIndexServiceMarkers = [
  'getIndexMaintenance',
  'removeIndexImages',
  'cleanupIndexGarbage',
  'get_index_maintenance',
  'remove_index_images',
  'cleanup_index_garbage',
];

for (const marker of requiredIndexServiceMarkers) {
  if (!indexService.includes(marker)) {
    console.error(`Missing index maintenance service marker: ${marker}`);
    process.exit(1);
  }
}

const actionLogService = fs.readFileSync(path.join(root, 'src/modules/actionLog/ActionLogService.ts'), 'utf8');
const requiredActionLogMarkers = [
  'getActionJournal',
  'createActionLogEntry',
  'get_action_journal',
  'create_action_log_entry',
];

for (const marker of requiredActionLogMarkers) {
  if (!actionLogService.includes(marker)) {
    console.error(`Missing action log service marker: ${marker}`);
    process.exit(1);
  }
}


const renameService = fs.readFileSync(path.join(root, 'src/modules/rename/RenameService.ts'), 'utf8');
const requiredRenameMarkers = [
  'smartRenameImage',
  'smart_rename_image',
  'SmartRenameResult',
  'ui.RenameService.13bf614168f52b72',
  'previewSmartRenameImages',
  'RenamePreviewResult',
];

for (const marker of requiredRenameMarkers) {
  if (!renameService.includes(marker)) {
    console.error(`Missing rename service marker: ${marker}`);
    process.exit(1);
  }
}

console.log('Structure verification passed.');

const renameMessages = JSON.parse(fs.readFileSync(path.join(root, "src/shared/i18n/locales/ru/rename.json"), "utf8"));
if(!renameMessages["ui.RenameService.13bf614168f52b72"].includes("YYYY-MM-DD_UNIQUE8.ext")) throw Error("Missing rename format resource");
