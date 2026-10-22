import type { ExplorerItem } from '../explorer/types';

export interface DirectoryScanOptions {
  path?: string | null;
  includeSubfolders: boolean;
  imagesOnly: boolean;
  signal?: AbortSignal;
  collectItems?: boolean;
  readMetadata?: boolean;
  onPage?(page: DirectoryScanResult): void | Promise<void>;
}

export interface PathBreadcrumb {
  label: string;
  path: string;
}

export interface DirectoryScanEntryDto {
  id: string;
  kind: 'folder' | 'image' | 'file';
  name: string;
  path: string;
  extension: string | null;
  sizeBytes: number | null;
  modifiedAtUnix: number | null;
  childCount: number | null;
  isImage: boolean;
  sourceRevision?: string | null;
  fileMetadata?: import('../explorer/types').ImageFileMetadata | null;
}

export interface DirectoryScanResultDto {
  nextCursor?: string | null;
  currentPath: string;
  parentPath: string | null;
  pathSegments: string[];
  breadcrumbs?: PathBreadcrumb[];
  items: DirectoryScanEntryDto[];
  scannedDirectories: number;
  skippedEntries: number;
  truncated: boolean;
}

export interface DirectoryScanResult {
  currentPath: string;
  parentPath: string | null;
  pathSegments: string[];
  breadcrumbs: PathBreadcrumb[];
  items: ExplorerItem[];
  scannedDirectories: number;
  skippedEntries: number;
  truncated: boolean;
}

export interface DroppedPathsInspectionResult {
  paths: string[];
  items: ExplorerItem[];
  imageCount: number;
  folderCount: number;
  fileCount: number;
  unsupportedCount: number;
  inspectedAtUnix: number;
}

export interface DroppedPathsInspectionResultDto {
  paths: string[];
  items: DirectoryScanEntryDto[];
  imageCount: number;
  folderCount: number;
  fileCount: number;
  unsupportedCount: number;
  inspectedAtUnix: number;
}

export interface FolderDialogResultDto {
  selectedPath: string | null;
  openedAtUnix: number;
}

export interface FileSystemService {
  chooseFolder(initialPath?: string | null): Promise<string | null>;
  scanDirectory(options: DirectoryScanOptions): Promise<DirectoryScanResult>;
  inspectDroppedPaths(paths: string[]): Promise<DroppedPathsInspectionResult>;
}
