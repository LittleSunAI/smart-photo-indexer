import {text as localeText} from "../../shared/i18n/Runtime";
import {invoke} from "../../shared/tauri/invoke";
import { captureDateLabel, imageDimensions } from '../filesystem/ImageMetadata';
import { isTauriRuntime } from '../../shared/tauri/runtime';
import type { ImageItem } from '../explorer/types';
import type { NativeSearchExportResult } from './types';
import { appLogService } from '../logs/AppLogService';
import type { CleanupIndexResult, IndexDatabaseInfo, IndexImageInputDto, IndexMaintenanceInput, IndexMaintenanceResult, IndexMetadataUpdateInput, IndexMetadataUpdateResult, IndexRescanInput, IndexRescanResult, IndexSearchInput, IndexSearchResult, IndexSearchResultItem, IndexSyncResult, RemoveIndexImagesInput, RemoveIndexImagesResult } from './types';

export interface IndexedImageRecord {
  id: string;
  currentPath: string;
  fileName: string;
  fileSize: number;
  modifiedAt: string;
  sha256?: string;
  perceptualHash?: string;
  description?: string;
  tags: string[];
  status: 'processed' | 'queued' | 'notProcessed' | 'error' | 'relocated' | 'missing';
}

export interface IndexService {
  initializeDatabase(): Promise<IndexDatabaseInfo>;
  getDatabaseInfo(): Promise<IndexDatabaseInfo>;
  syncScannedImages(images: ImageItem[], reuseObservedIdentity?: boolean): Promise<IndexSyncResult>;
  updateImageMetadata(input: IndexMetadataUpdateInput): Promise<IndexMetadataUpdateResult>;
  searchIndexedImages(input: IndexSearchInput): Promise<IndexSearchResult>;
  closeSearchSnapshot(queryId?: string | null): Promise<void>;
  forkSearchSnapshot(queryId?: string | null): Promise<string | null>;
  exportSearchSnapshot(input: IndexSearchInput, format: 'csv' | 'json', selectedPaths?: string[], excludedPaths?: string[]): Promise<NativeSearchExportResult>;
  rescanIndexPaths(input: IndexRescanInput): Promise<IndexRescanResult>;
  getIndexMaintenance(input: IndexMaintenanceInput): Promise<IndexMaintenanceResult>;
  removeIndexImages(input: RemoveIndexImagesInput): Promise<RemoveIndexImagesResult>;
  cleanupIndexGarbage(): Promise<CleanupIndexResult>;
  findByPath(path: string): Promise<IndexedImageRecord | null>;
  upsertImage(record: IndexedImageRecord): Promise<void>;
  search(query: string): Promise<IndexedImageRecord[]>;
}

function browserIndexInfo(): IndexDatabaseInfo {
  return {
    databasePath: 'browser-preview://smart_photo_indexer.sqlite',
    schemaVersion: 1,
    migrationsApplied: 0,
    tables: ['images', 'image_paths', 'tags', 'image_tags', 'tasks', 'action_logs', 'search_history', 'settings_kv'],
    imageCount: 0,
    pathCount: 0,
    tagCount: 0,
    createdAtUnix: Math.floor(Date.now() / 1000),
  };
}

function toIndexImageInput(image: ImageItem): IndexImageInputDto {
  return {
    id: image.id,
    path: image.path,
    fileName: image.name,
    extension: image.extension,
    sizeBytes: image.sizeBytes,
    modifiedAtUnix: image.modifiedAtUnix,
    status: 'notProcessed',
  };
}

function createMinimalIndexImageInput(input: IndexMetadataUpdateInput): IndexImageInputDto {
  const fileName = input.path.split(/[\\/]/).pop() || 'image';
  const extension = fileName.includes('.') ? fileName.split('.').pop()?.toLowerCase() : undefined;
  return {
    id: `metadata-path:${input.path}`,
    path: input.path,
    fileName,
    extension,
    sizeBytes: undefined,
    modifiedAtUnix: undefined,
    status: 'notProcessed',
  };
}

function isPendingImageId(value: string | null | undefined): boolean {
  return Boolean(value?.trim().startsWith('pending-index:'));
}

export function toSearchImageItem(item: IndexSearchResultItem): ImageItem {
  const extension = item.extension ?? item.fileName.split('.').pop()?.toLowerCase() ?? 'image';
  const modifiedAtUnix = item.modifiedAtUnix ?? undefined;
  return {
    id: `search:${item.imageId}:${item.path.toLowerCase()}`,
    kind: 'image',
    name: item.fileName,
    path: item.path,
    status: item.status,
    dateTaken: captureDateLabel(item.fileMetadata),
    fileMetadata: item.fileMetadata,
    sizeLabel: typeof item.sizeBytes === 'number' ? `${(item.sizeBytes / 1024 / 1024).toFixed(1)} MB` : localeText("main", "ui.IndexService.3643d938d4e5506a"),
    sizeBytes: item.sizeBytes ?? undefined,
    modifiedAtUnix,
    dimensions: imageDimensions(item.fileMetadata),
    extension,
    tags: item.tags,
    collections: item.collections ?? [],
    isFavorite: item.isFavorite ?? false,
    description: item.description ?? '',
    indexState: 'known',
    indexedImageId: item.imageId,
    aiProcessedAtUnix: item.aiProcessedAtUnix,
    descriptionVersion: item.descriptionVersion,
    descriptionFormat:item.descriptionFormat,
    tagsVersion: item.tagsVersion,
    firstSeenAtUnix: item.firstSeenAtUnix ?? null,
    lastSeenAtUnix: item.lastSeenAtUnix ?? null,
  };
}

function optimisticSyncResult(images: ImageItem[]): IndexSyncResult {
  const syncedAtUnix = Math.floor(Date.now() / 1000);
  const indexedImages = images.map((image) => ({
    imageId: image.indexedImageId ?? `pending-index:${image.id}`,
    path: image.path,
    fileName: image.name,
    status: image.status,
    description: image.description,
    tags: image.tags,
    isFavorite: image.isFavorite ?? false,
    collections: image.collections ?? [],
    indexState: image.indexState ?? 'known' as const,
    previousPath: image.previousPath ?? null,
    firstSeenAtUnix: image.firstSeenAtUnix ?? syncedAtUnix,
    lastSeenAtUnix: image.lastSeenAtUnix ?? syncedAtUnix,
  }));

  return {
    databasePath: 'background-index://pending',
    receivedImages: images.length,
    insertedOrUpdatedImages: images.length,
    currentPathsUpdated: images.length,
    tagsLinked: images.reduce((total, image) => total + image.tags.length, 0),
    newImages: 0,
    knownImages: images.length,
    relocatedImages: 0,
    indexedImages,
    schemaVersion: 3,
    imageCount: images.length,
    pathCount: images.length,
    syncedAtUnix,
  };
}


function browserRescanResult(input: IndexRescanInput): IndexRescanResult {
  return { databasePath: 'browser-preview://smart_photo_indexer.sqlite', rootPath: input.rootPath ?? null, checkedPaths: 0, existingPaths: 0, missingPaths: 0, revivedPaths: 0, actionLogEntries: 0, imageCount: 0, pathCount: 0, rescannedAtUnix: Math.floor(Date.now() / 1000) };
}

function browserMaintenanceResult(input: IndexMaintenanceInput): IndexMaintenanceResult {
  return { databasePath: 'browser-preview://smart_photo_indexer.sqlite', status: input.status ?? 'missing', totalMatchingImages: 0, returnedImages: 0, missingImageCount: 0, orphanTagCount: 0, nonCurrentPathCount: 0, oldSearchHistoryCount: 0, images: [], loadedAtUnix: Math.floor(Date.now() / 1000) };
}

function browserRemoveResult(input: RemoveIndexImagesInput): RemoveIndexImagesResult {
  return { databasePath: 'browser-preview://smart_photo_indexer.sqlite', requestedImages: input.imageIds.length, removedImages: input.imageIds.length, missingRequestedImages: 0, actionLogEntries: input.imageIds.length, imageCount: 0, pathCount: 0, tagCount: 0, removedAtUnix: Math.floor(Date.now() / 1000) };
}

function browserCleanupResult(): CleanupIndexResult {
  return { databasePath: 'browser-preview://smart_photo_indexer.sqlite', orphanTagsRemoved: 0, oldSearchHistoryRemoved: 0, staleFinishedTasksRemoved: 0, imageCount: 0, pathCount: 0, tagCount: 0, cleanedAtUnix: Math.floor(Date.now() / 1000) };
}

function browserSearchResult(input: IndexSearchInput): IndexSearchResult {
  const searchedAtUnix = Math.floor(Date.now() / 1000);
  const query = input.query.trim();
  return { query, totalCount: 0, returnedCount: 0, items: [], recentSearches: query ? [query] : [], appliedStatuses: input.statuses ?? [], appliedTags: input.tags ?? [], appliedTagMode: input.tagMode ?? 'any', appliedDateFromUnix: input.dateFromUnix ?? null, appliedDateToUnix: input.dateToUnix ?? null, appliedFavoriteOnly: Boolean(input.favoriteOnly), searchedAtUnix };
}

class TauriIndexService implements IndexService {
  async exportSearchSnapshot(input: IndexSearchInput, format: 'csv' | 'json', selectedPaths?: string[], excludedPaths?: string[]): Promise<NativeSearchExportResult> {
    return invoke<NativeSearchExportResult>('export_search_snapshot', { input, format, selectedPaths, excludedPaths });
  }
  async closeSearchSnapshot(queryId?: string | null): Promise<void> {
    if (queryId && isTauriRuntime()) await invoke('close_search_snapshot', { queryId });
  }
  async forkSearchSnapshot(queryId?: string | null): Promise<string | null> {
    return queryId && isTauriRuntime() ? invoke<string>('fork_search_snapshot', { queryId }) : null;
  }
  async initializeDatabase(): Promise<IndexDatabaseInfo> {
    if (!isTauriRuntime()) return browserIndexInfo();
    return invoke<IndexDatabaseInfo>('initialize_index_database');
  }

  async getDatabaseInfo(): Promise<IndexDatabaseInfo> {
    if (!isTauriRuntime()) return browserIndexInfo();
    return invoke<IndexDatabaseInfo>('get_index_database_info');
  }

  async syncScannedImages(images: ImageItem[], reuseObservedIdentity = false): Promise<IndexSyncResult> {
    if (!isTauriRuntime()) return optimisticSyncResult(images);
    return invoke<IndexSyncResult>('sync_scanned_images_to_index', { images: images.map(toIndexImageInput), reuseObservedIdentity });
  }

  async updateImageMetadata(input: IndexMetadataUpdateInput): Promise<IndexMetadataUpdateResult> {
    const normalizedTags = normalizeTags(input.tags);
    const cleanInput: IndexMetadataUpdateInput = {
      ...input,
      imageId: isPendingImageId(input.imageId) ? null : input.imageId,
      tags: normalizedTags,
    };

    appLogService.info('task', localeText("main", "ui.IndexService.fc659e6d27aba330"), {
      imageId: input.imageId ?? null,
      effectiveImageId: cleanInput.imageId ?? null,
      path: input.path,
      descriptionLength: input.description.trim().length,
      tags: normalizedTags,
    });

    if (!isTauriRuntime()) {
      const result = { imageId: cleanInput.imageId || `browser-index:${cleanInput.path}`, path: cleanInput.path, fileName: cleanInput.path.split(/[\\/]/).pop() || 'image', status: cleanInput.description.trim() || normalizedTags.length > 0 ? 'processed' as const : 'notProcessed' as const, description: cleanInput.description.trim() || null, tags: normalizedTags, updatedAtUnix: Math.floor(Date.now() / 1000), actionLogId: Date.now(), imageCount: 0, pathCount: 0, tagCount: normalizedTags.length };
      appLogService.info('task', localeText("main", "ui.IndexService.5df200d215f2ce01"), result);
      return result;
    }

    const syncResult = cleanInput.imageId ? null : await invoke<IndexSyncResult>('sync_scanned_images_to_index', { images: [createMinimalIndexImageInput(cleanInput)] });
    const syncedImage = syncResult?.indexedImages.find((image) => image.path === cleanInput.path) ?? syncResult?.indexedImages[0] ?? null;
    const inputForSave: IndexMetadataUpdateInput = {
      ...cleanInput,
      imageId: syncedImage?.imageId ?? cleanInput.imageId ?? null,
      tags: normalizedTags,
    };

    appLogService.info('task', localeText("main", "ui.IndexService.eac0610357d2b747"), {
      path: cleanInput.path,
      syncedImageId: syncedImage?.imageId ?? null,
      insertedOrUpdatedImages: syncResult?.insertedOrUpdatedImages ?? 0,
      currentPathsUpdated: syncResult?.currentPathsUpdated ?? 0,
    });

    try {
      const result = await invoke<IndexMetadataUpdateResult>('update_image_metadata', { input: inputForSave });
      appLogService.info('task', localeText("main", "ui.IndexService.b2b419f39b7049de"), {
        imageId: result.imageId,
        path: result.path,
        fileName: result.fileName,
        status: result.status,
        descriptionLength: result.description?.length ?? 0,
        tags: result.tags,
        actionLogId: result.actionLogId,
      });
      return result;
    } catch (error) {
      appLogService.error('task', localeText("main", "ui.IndexService.28c4f2e00e5dbb27"), {
        input: inputForSave,
        errorName: error instanceof Error ? error.name : null,
        errorMessage: error instanceof Error ? error.message : String(error),
        error,
      });
      throw error;
    }
  }

  async searchIndexedImages(input: IndexSearchInput): Promise<IndexSearchResult> {
    if (!isTauriRuntime()) return browserSearchResult(input);
    return invoke<IndexSearchResult>('search_indexed_images', { input });
  }

  async rescanIndexPaths(input: IndexRescanInput): Promise<IndexRescanResult> {
    if (!isTauriRuntime()) return browserRescanResult(input);
    let afterPath=input.afterPath;let total:IndexRescanResult|undefined;
    do {
      input.signal?.throwIfAborted();
      const page=await invoke<IndexRescanResult>('rescan_index_paths', { input: {rootPath:input.rootPath,limit:Math.min(input.limit??256,1024),afterPath,excludedPaths:input.excludedPaths} });
      total=total ? {...page,checkedPaths:total.checkedPaths+page.checkedPaths,existingPaths:total.existingPaths+page.existingPaths,missingPaths:total.missingPaths+page.missingPaths,revivedPaths:total.revivedPaths+page.revivedPaths,actionLogEntries:total.actionLogEntries+page.actionLogEntries,skippedPaths:(total.skippedPaths??0)+(page.skippedPaths??0)} : page;
      afterPath=page.nextAfterPath??undefined;
    } while(afterPath);
    return total!;
  }

  async getIndexMaintenance(input: IndexMaintenanceInput): Promise<IndexMaintenanceResult> {
    if (!isTauriRuntime()) return browserMaintenanceResult(input);
    return invoke<IndexMaintenanceResult>('get_index_maintenance', { input });
  }

  async removeIndexImages(input: RemoveIndexImagesInput): Promise<RemoveIndexImagesResult> {
    const imageIds = Array.from(new Set(input.imageIds.map((id) => id.trim()).filter(Boolean)));
    if (!isTauriRuntime()) return browserRemoveResult({ imageIds });
    return invoke<RemoveIndexImagesResult>('remove_index_images', { input: { imageIds } });
  }

  async cleanupIndexGarbage(): Promise<CleanupIndexResult> {
    if (!isTauriRuntime()) return browserCleanupResult();
    return invoke<CleanupIndexResult>('cleanup_index_garbage');
  }

  async findByPath(_path: string): Promise<IndexedImageRecord | null> { return null; }
  async upsertImage(_record: IndexedImageRecord): Promise<void> {}
  async search(_query: string): Promise<IndexedImageRecord[]> { return []; }
}

export const indexService: IndexService = new TauriIndexService();

function normalizeTags(tags: string[]): string[] {
  const seen = new Set<string>();
  const normalized: string[] = [];
  for (const rawTag of tags) {
    const tag = rawTag.trim().replace(/^#+/, '').toLowerCase();
    if (!tag || seen.has(tag)) continue;
    seen.add(tag);
    normalized.push(tag);
  }
  return normalized;
}

export function mapIndexSearchResultsToImages(result: IndexSearchResult): ImageItem[] {
  return result.items.map(toSearchImageItem);
}
