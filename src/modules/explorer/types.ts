export type ExplorerItemKind = 'folder' | 'image' | 'file';
export type ProcessingStatus = 'processed' | 'queued' | 'notProcessed' | 'error' | 'relocated' | 'missing';
export type ImageIndexState = 'new' | 'known' | 'pathUpdated';
export type MockPhotoTheme = 'birthday' | 'balloons' | 'outdoorParty' | 'familyTable' | 'dog' | 'mountains' | 'sunset' | 'dinner' | 'kidsCake' | 'girlsBalloons' | 'cat' | 'forest' | 'hikers' | 'city' | 'picnic' | 'pier' | 'familyPortrait';

export interface ExplorerItemBase {
  id: string;
  kind: ExplorerItemKind;
  name: string;
  path: string;
  sourceRevision?: string;
}

export interface FolderItem extends ExplorerItemBase {
  kind: 'folder';
  childCount: number;
}

export interface FileItem extends ExplorerItemBase {
  kind: 'file';
  extension?: string;
  fileType: string;
  sizeLabel: string;
  sizeBytes?: number;
  modifiedAt: string;
  modifiedAtUnix?: number;
}

export interface ImageItem extends ExplorerItemBase {
  fileMetadata?: ImageFileMetadata | null;
  aiProcessedAtUnix?: number | null;
  descriptionVersion?: number;
  descriptionFormat?: import('../details/DescriptionFormat').DescriptionFormat | null;
  tagsVersion?: number;
  kind: 'image';
  status: ProcessingStatus;
  dateTaken: string;
  sizeLabel: string;
  sizeBytes?: number;
  modifiedAtUnix?: number;
  dimensions: string;
  extension?: string;
  previewUrl?: string;
  tags: string[];
  collections?: string[];
  isFavorite?: boolean;
  description: string;
  indexState?: ImageIndexState;
  indexedImageId?: string;
  previousPath?: string | null;
  firstSeenAtUnix?: number | null;
  lastSeenAtUnix?: number | null;
  theme?: MockPhotoTheme;
}

export type ExplorerItem = FolderItem | FileItem | ImageItem;

export interface ImageFileMetadata {
  colorProfile?: string;
  decodeSupport?: string;
  framePolicy?: string;
  dateTakenRaw?: string | null;
  dateTakenLocal?: string | null;
  dateTakenOffset?: string | null;
  dateTakenUnix?: number | null;
  orientation?: number | null;
  width?: number | null;
  height?: number | null;
  cameraMake?: string | null;
  cameraModel?: string | null;
  modifiedAtUnix?: number | null;
  sizeBytes?: number | null;
  exifState: string;
}
