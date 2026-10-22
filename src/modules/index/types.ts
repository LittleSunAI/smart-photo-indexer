export interface IndexDatabaseInfo {
  databasePath: string;
  schemaVersion: number;
  migrationsApplied: number;
  tables: string[];
  imageCount: number;
  pathCount: number;
  tagCount: number;
  createdAtUnix: number;
}

export interface IndexImageInputDto {
  id: string;
  path: string;
  fileName: string;
  extension?: string;
  sizeBytes?: number;
  modifiedAtUnix?: number;
  width?: number;
  height?: number;
  status?: string;
  description?: string;
  tags?: string[];
}

export type ImageIndexState = 'new' | 'known' | 'pathUpdated';

export interface IndexImageStateDto {
  aiProcessedAtUnix?: number | null;
  descriptionFormat?:import('../details/DescriptionFormat').DescriptionFormat|null;
  sourceRevision?: string | null;
  fileMetadata?: import('../explorer/types').ImageFileMetadata | null;
  descriptionVersion?: number;
  tagsVersion?: number;
  imageId: string;
  path: string;
  fileName: string;
  status: 'processed' | 'queued' | 'notProcessed' | 'error' | 'relocated' | 'missing';
  description?: string | null;
  tags: string[];
  isFavorite: boolean;
  collections: string[];
  indexState: ImageIndexState;
  previousPath?: string | null;
  firstSeenAtUnix?: number | null;
  lastSeenAtUnix?: number | null;
}

export interface IndexSyncResult {
  skippedImages?:Array<{path:string;reason:string}>;
  databasePath: string;
  receivedImages: number;
  insertedOrUpdatedImages: number;
  currentPathsUpdated: number;
  tagsLinked: number;
  newImages: number;
  knownImages: number;
  relocatedImages: number;
  indexedImages: IndexImageStateDto[];
  schemaVersion: number;
  imageCount: number;
  pathCount: number;
  syncedAtUnix: number;
}

export interface IndexDatabaseState {
  isInitializing: boolean;
  isReady: boolean;
  errorMessage: string | null;
  databasePath: string | null;
  schemaVersion: number;
  migrationsApplied: number;
  tables: string[];
  imageCount: number;
  pathCount: number;
  tagCount: number;
  lastSyncAtUnix: number | null;
  lastSyncedImages: number;
  lastKnownImages: number;
  lastNewImages: number;
  lastRelocatedImages: number;
  lastRescanAtUnix: number | null;
  lastRescanCheckedPaths: number;
  lastMissingImages: number;
  lastRevivedImages: number;
}


export interface IndexMetadataUpdateInput {
  descriptionFormat?:import('../details/DescriptionFormat').DescriptionFormat|null;
  imageId?: string | null;
  path: string;
  description: string;
  tags: string[];
  fields?: Array<'description' | 'tags'>;
  source?: 'manual' | 'ai';
  model?: string;
  expectedDescriptionVersion?: number;
  expectedTagsVersion?: number;
}

export interface IndexMetadataUpdateResult {
  aiProcessedAtUnix?: number | null;
  descriptionFormat?:import('../details/DescriptionFormat').DescriptionFormat|null;
  descriptionVersion?: number;
  tagsVersion?: number;
  imageId: string;
  path: string;
  fileName: string;
  status: 'processed' | 'queued' | 'notProcessed' | 'error' | 'relocated' | 'missing';
  description?: string | null;
  tags: string[];
  updatedAtUnix: number;
  actionLogId: number;
  imageCount: number;
  pathCount: number;
  tagCount: number;
}


export type IndexSearchStatusFilter = 'all' | 'processed' | 'queued' | 'notProcessed' | 'error' | 'relocated' | 'missing';

export interface IndexSearchInput {
  fields?:import('../search/SearchFieldsContract').SearchField[];
  untaggedOnly?: boolean;
  expandTerms?: boolean;
  queryId?: string;
  collectionId?: string;
  rootPath?: string;
  sortOrder?: string;
  dateField?: 'modified' | 'taken' | 'indexed' | 'analyzed';
  dateFromLocal?: string;
  dateToLocal?: string;
  dateToExclusive?: boolean;
  query: string;
  statuses: string[];
  tags?: string[];
  tagMode?: 'any' | 'all';
  dateFromUnix?: number | null;
  dateToUnix?: number | null;
  favoriteOnly?: boolean;
  limit?: number;
  offset?: number;
}

export interface IndexSearchResultItem {
  aiProcessedAtUnix?: number | null;
  descriptionFormat?:import('../details/DescriptionFormat').DescriptionFormat|null;
  fileMetadata?: import('../explorer/types').ImageFileMetadata | null;
  descriptionVersion?: number;
  tagsVersion?: number;
  imageId: string;
  path: string;
  fileName: string;
  extension?: string | null;
  sizeBytes?: number | null;
  modifiedAtUnix?: number | null;
  status: 'processed' | 'queued' | 'notProcessed' | 'error' | 'relocated' | 'missing';
  description?: string | null;
  tags: string[];
  isFavorite: boolean;
  collections: string[];
  score: number;
  matchedFields: string[];
  firstSeenAtUnix?: number | null;
  lastSeenAtUnix?: number | null;
}

export interface IndexSearchResult {
  statusCounts?:{processed:number;inQueue:number;notProcessed:number;errors:number};
  queryId?: string;
  hasMore?: boolean;
  nextCursor?: number | null;
  query: string;
  totalCount: number;
  returnedCount: number;
  items: IndexSearchResultItem[];
  recentSearches: string[];
  appliedStatuses: string[];
  appliedTags: string[];
  appliedTagMode: 'any' | 'all';
  appliedDateFromUnix?: number | null;
  appliedDateToUnix?: number | null;
  appliedFavoriteOnly: boolean;
  searchedAtUnix: number;
}

export type NativeSearchExportResult = { cancelled: true } | {
  cancelled: false;
  fileName: string;
  path: string;
  exportedCount: number;
  resultCount: number;
  format: 'csv' | 'json';
};


export interface IndexRescanInput {
  rootPath?: string | null;
  limit?: number;
  afterPath?: string;
  excludedPaths?: string[];
  signal?: AbortSignal;
}

export interface IndexRescanResult {
  nextAfterPath?: string | null;
  skippedPaths?: number;
  databasePath: string;
  rootPath?: string | null;
  checkedPaths: number;
  existingPaths: number;
  missingPaths: number;
  revivedPaths: number;
  actionLogEntries: number;
  imageCount: number;
  pathCount: number;
  rescannedAtUnix: number;
}

export interface SearchState {
  fields?:import('../search/SearchFieldsContract').SearchField[];
  operationFeedback?: {message:string;error?:boolean};
  isLoadingPage?:boolean;
  pageError?:string|null;
  lastInput?:IndexSearchInput;
  statusCounts?:{processed:number;inQueue:number;notProcessed:number;errors:number};
  needsRefresh?:boolean;
  dataRevision?:number;
  untaggedOnly?: boolean;
  queryId?: string;
  selectionSnapshot?: { input: IndexSearchInput; totalCount: number; excludedPaths: string[] };
  expandTerms: boolean;
  collectionId: string;
  rootPath: string;
  sortOrder: string;
  appliedSortOrder?: string;
  dateField: 'modified' | 'taken' | 'indexed' | 'analyzed';
  isSearching: boolean;
  errorMessage: string | null;
  results: import('../explorer/types').ImageItem[];
  resultCount: number;
  lastQuery: string;
  statusFilter: IndexSearchStatusFilter;
  tagFilterText: string;
  tagMode: 'any' | 'all';
  dateFrom: string;
  dateTo: string;
  favoriteOnly: boolean;
  lastAppliedTags: string[];
  lastAppliedStatuses: string[];
  lastAppliedDateFromUnix?: number | null;
  lastAppliedDateToUnix?: number | null;
  lastAppliedFavoriteOnly: boolean;
  lastSearchedAtUnix: number | null;
}

export interface IndexMaintenanceImage {
  imageId: string;
  fileName: string;
  path: string;
  status: 'processed' | 'queued' | 'notProcessed' | 'error' | 'relocated' | 'missing';
  description?: string | null;
  tags: string[];
  sizeBytes?: number | null;
  modifiedAtUnix?: number | null;
  missingAtUnix?: number | null;
  lastSeenAtUnix?: number | null;
}

export interface IndexMaintenanceInput {
  status?: 'all' | 'processed' | 'queued' | 'notProcessed' | 'error' | 'relocated' | 'missing';
  limit?: number;
  offset?: number;
}

export interface IndexMaintenanceResult {
  databasePath: string;
  status: string;
  totalMatchingImages: number;
  returnedImages: number;
  missingImageCount: number;
  orphanTagCount: number;
  nonCurrentPathCount: number;
  oldSearchHistoryCount: number;
  images: IndexMaintenanceImage[];
  loadedAtUnix: number;
}

export interface RemoveIndexImagesInput {
  imageIds: string[];
}

export interface RemoveIndexImagesResult {
  databasePath: string;
  requestedImages: number;
  removedImages: number;
  missingRequestedImages: number;
  actionLogEntries: number;
  imageCount: number;
  pathCount: number;
  tagCount: number;
  removedAtUnix: number;
}

export interface CleanupIndexResult {
  databasePath: string;
  orphanTagsRemoved: number;
  oldSearchHistoryRemoved: number;
  staleFinishedTasksRemoved: number;
  imageCount: number;
  pathCount: number;
  tagCount: number;
  cleanedAtUnix: number;
}
