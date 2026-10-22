import {text as localeText} from "../../shared/i18n/Runtime";
import {convertFileSrc} from "@tauri-apps/api/core";
import {invoke} from "../../shared/tauri/invoke";
import { createMockExplorerItems } from '../explorer/mockExplorerItems';
import { isTauriRuntime } from '../../shared/tauri/runtime';
import type { ExplorerItem, FileItem, FolderItem, ImageItem } from '../explorer/types';
import { extensionToFileType, formatBytes, formatUnixTime } from '../../shared/utils/fileFormatting';
import type { DirectoryScanEntryDto, DirectoryScanOptions, DirectoryScanResult, DirectoryScanResultDto, DroppedPathsInspectionResult, DroppedPathsInspectionResultDto, FileSystemService, FolderDialogResultDto, PathBreadcrumb } from './types';

function toPreviewUrl(path: string): string | undefined {
  if (!isTauriRuntime()) return undefined;
  const slash = String.fromCharCode(92);
  const prefix = slash + slash + '?' + slash;
  let cleanPath = path.trim();
  if (cleanPath.toLowerCase().startsWith(prefix.toLowerCase())) {
    cleanPath = cleanPath.slice(prefix.length);
  }
  try {
    return convertFileSrc(cleanPath);
  } catch {
    return undefined;
  }
}

function createBrowserBreadcrumbs(currentPath: string): PathBreadcrumb[] {
  const normalized = currentPath.replace(/\\/g, '/');
  const parts = normalized.split('/').filter(Boolean);
  const breadcrumbs: PathBreadcrumb[] = [];
  for (let index = 0; index < parts.length; index += 1) {
    const label = parts[index];
    breadcrumbs.push({ label, path: parts.slice(0, index + 1).join('/') });
  }
  return breadcrumbs.length > 0 ? breadcrumbs : [{ label: currentPath, path: currentPath }];
}

function createBrowserPreviewResult(options: DirectoryScanOptions): DirectoryScanResult {
  const currentPath = options.path?.trim() || 'Demo / Photos / 2024 / Birthday';
  const breadcrumbs = createBrowserBreadcrumbs(currentPath);
  return {
    currentPath,
    parentPath: breadcrumbs.length > 1 ? breadcrumbs[breadcrumbs.length - 2].path : null,
    pathSegments: breadcrumbs.map((item) => item.label),
    breadcrumbs,
    items: createMockExplorerItems(),
    scannedDirectories: 1,
    skippedEntries: 0,
    truncated: false,
  };
}

function toExplorerItem(entry: DirectoryScanEntryDto): ExplorerItem {
  if (entry.kind === 'folder') {
    const folder: FolderItem = { id: entry.id, kind: 'folder', name: entry.name, path: entry.path, childCount: entry.childCount ?? 0 };
    return folder;
  }

  if (entry.kind === 'image') {
    const image: ImageItem = {
      id: entry.id,
      kind: 'image',
      name: entry.name,
      path: entry.path,
      sourceRevision: entry.sourceRevision ?? undefined,
      status: 'notProcessed',
      dateTaken: entry.fileMetadata?.dateTakenLocal ?? '',
      fileMetadata: entry.fileMetadata,
      sizeLabel: formatBytes(entry.sizeBytes),
      sizeBytes: entry.sizeBytes ?? undefined,
      modifiedAtUnix: entry.modifiedAtUnix ?? undefined,
      dimensions: localeText("main", "ui.FileSystemService.4ec7498a2455f625"),
      extension: entry.extension ?? undefined,
      previewUrl: toPreviewUrl(entry.path),
      tags: [],
      description: localeText("main", "ui.FileSystemService.8ef93355ab1454c9"),
      theme: 'birthday',
    };
    return image;
  }

  const file: FileItem = {
    id: entry.id,
    kind: 'file',
    name: entry.name,
    path: entry.path,
    extension: entry.extension ?? undefined,
    fileType: extensionToFileType(entry.extension),
    sizeLabel: formatBytes(entry.sizeBytes),
    sizeBytes: entry.sizeBytes ?? undefined,
    modifiedAt: formatUnixTime(entry.modifiedAtUnix),
    modifiedAtUnix: entry.modifiedAtUnix ?? undefined,
  };
  return file;
}

export class TauriFileSystemService implements FileSystemService {
  async chooseFolder(initialPath?: string | null): Promise<string | null> {
    if (!isTauriRuntime()) return null;
    const result = await invoke<FolderDialogResultDto>('choose_folder_dialog', {
      initialPath: initialPath && initialPath.trim().length > 0 ? initialPath.trim() : null,
    });
    return result.selectedPath;
  }

  async inspectDroppedPaths(paths: string[]): Promise<DroppedPathsInspectionResult> {
    const cleanPaths = Array.from(new Set(paths.map((path) => path.trim()).filter(Boolean)));
    if (!isTauriRuntime()) {
      const mockItems = createMockExplorerItems().filter((item) => item.kind === 'image').slice(0, Math.max(1, Math.min(6, cleanPaths.length || 3)));
      return {
        paths: cleanPaths.length > 0 ? cleanPaths : mockItems.map((item) => item.path),
        items: mockItems,
        imageCount: mockItems.filter((item) => item.kind === 'image').length,
        folderCount: 0,
        fileCount: 0,
        unsupportedCount: 0,
        inspectedAtUnix: Math.floor(Date.now() / 1000),
      };
    }
    const result = await invoke<DroppedPathsInspectionResultDto>('inspect_dropped_paths', { paths: cleanPaths });
    return {
      paths: result.paths,
      items: result.items.map(toExplorerItem),
      imageCount: result.imageCount,
      folderCount: result.folderCount,
      fileCount: result.fileCount,
      unsupportedCount: result.unsupportedCount,
      inspectedAtUnix: result.inspectedAtUnix,
    };
  }

  async scanDirectory(options: DirectoryScanOptions): Promise<DirectoryScanResult> {
    if (!isTauriRuntime()) {
      const result = createBrowserPreviewResult(options); await options.onPage?.(result); return result;
    }
    let cursor: string | null | undefined;
    try {
    options.signal?.throwIfAborted();
    let result = await invoke<DirectoryScanResultDto>('scan_directory', {
      path: options.path && options.path.trim().length > 0 ? options.path.trim() : null,
      includeSubfolders: options.includeSubfolders,
      imagesOnly: options.imagesOnly,
      readMetadata: options.readMetadata ?? false,
    });
    let combined: DirectoryScanResult | undefined;
    do {
    cursor = result.nextCursor;
    options.signal?.throwIfAborted();
    const page: DirectoryScanResult = {
      currentPath: result.currentPath,
      parentPath: result.parentPath,
      pathSegments: result.pathSegments.length > 0 ? result.pathSegments : [result.currentPath],
      breadcrumbs: result.breadcrumbs && result.breadcrumbs.length > 0 ? result.breadcrumbs : [{ label: result.currentPath, path: result.currentPath }],
      items: result.items.map(toExplorerItem),
      scannedDirectories: result.scannedDirectories,
      skippedEntries: result.skippedEntries,
      truncated: result.truncated,
    };
    await options.onPage?.(page);
    if (!combined) combined = { ...page, items: options.collectItems===false ? [] : [...page.items] };
    else { if(options.collectItems!==false)combined.items.push(...page.items); combined.scannedDirectories = page.scannedDirectories; combined.skippedEntries = page.skippedEntries; }
    options.signal?.throwIfAborted();
    if (cursor) result = await invoke<DirectoryScanResultDto>('next_directory_scan_page', { cursor });
    } while (cursor);
    return combined!;
    } finally {
      if (cursor) await invoke('close_directory_scan', { cursor }).catch(() => {});
    }
  }
}

export const fileSystemService: FileSystemService = new TauriFileSystemService();
