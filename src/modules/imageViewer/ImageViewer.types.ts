export type SmartImageViewerWindowMode = 'full' | 'compact' | 'minimized';

export type SmartImageViewerZoomPreset = 10 | 25 | 50 | 75 | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800;

export const smartImageViewerZoomPresets: SmartImageViewerZoomPreset[] = [
  10,
  25,
  50,
  75,
  100,
  200,
  300,
  400,
  500,
  600,
  700,
  800,
];

export type SmartImageResizeMode = 'pixels' | 'percent';
export type SmartImageResizeFormat = 'jpeg' | 'png' | 'tiff' | 'bmp';

export interface SmartImageViewerItem {
  sourceRevision?:string;
  id: string;
  kind?: string;
  name: string;
  path: string;
  indexedImageId?: string;
  sizeLabel?: string;
  sizeBytes?: number;
  dimensions?: string;
  extension?: string;
  dateTaken?: string;
  modifiedAtUnix?: number;
  status?: string;
  isFavorite?: boolean;
  tags?: string[];
  collections?: string[];
  description?: string;
  previewUrl?: string;
}

export interface SmartImageViewerSaveResizeInput {
  sourcePath: string;
  targetPath: string;
  width: number;
  height: number;
  quality: number;
  format: SmartImageResizeFormat;
}

export interface SmartImageViewerSaveResizeResult {
  targetPath: string;
  fileName: string;
  width: number;
  height: number;
  sizeBytes: number;
  format: SmartImageResizeFormat;
  savedAtUnix: number;
}

export interface SmartImageViewerChooseSavePathInput {
  sourcePath: string;
  defaultFileName: string;
  format: SmartImageResizeFormat;
}

export interface SmartImageViewerChooseSavePathResult {
  selectedPath: string | null;
  openedAtUnix: number;
}

export interface SmartImageViewerSaveOriginalPathInput {
  sourcePath: string;
  defaultFileName: string;
}

export interface SmartImageViewerSaveOriginalCopyInput {
  sourcePath: string;
  targetPath: string;
}

export interface SmartImageViewerSaveOriginalCopyResult {
  targetPath: string;
  fileName: string;
  sizeBytes: number;
  savedAtUnix: number;
}

export interface SmartImageViewerDeleteResult {
  deletedPath: string;
  deletedAtUnix: number;
}

export interface SmartImageViewerShareFileResult {
  fileName: string;
  mimeType: string;
  bytes: number[];
  sizeBytes: number;
}

export interface SmartImageViewerGenericActionResult {
  path: string;
  action: string;
  completedAtUnix: number;
}

export interface SmartImageViewerFileActions {
  editExternal(path: string): Promise<void>;
  revealInFileManager(path: string): Promise<void>;

  chooseSaveImagePath?(input: SmartImageViewerChooseSavePathInput): Promise<SmartImageViewerChooseSavePathResult>;
  saveResizedImage?(input: SmartImageViewerSaveResizeInput): Promise<SmartImageViewerSaveResizeResult>;

  chooseSaveOriginalImagePath?(input: SmartImageViewerSaveOriginalPathInput): Promise<SmartImageViewerChooseSavePathResult>;
  saveOriginalImageCopy?(input: SmartImageViewerSaveOriginalCopyInput): Promise<SmartImageViewerSaveOriginalCopyResult>;

  deleteImage?(path: string): Promise<SmartImageViewerDeleteResult>;
  printImage?(path: string): Promise<SmartImageViewerGenericActionResult>;
  shareImage?(path: string): Promise<SmartImageViewerGenericActionResult>;
  readImageFileForShare?(path: string): Promise<SmartImageViewerShareFileResult>;
  setDesktopWallpaper?(path: string): Promise<SmartImageViewerGenericActionResult>;
}
