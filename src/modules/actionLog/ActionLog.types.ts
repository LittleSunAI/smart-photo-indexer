export type ActionLogType =
  | 'image_indexed'
  | 'metadata_updated'
  | 'description_edited'
  | 'tags_edited'
  | 'file_renamed'
  | 'path_updated'
  | 'queue_error'
  | 'ai_error'
  | 'search_performed'
  | 'error'
  | string;

export type ActionJournalSource = 'action_log' | 'search_history' | 'browser_preview';

export interface ActionJournalEntry {
  id: string;
  source: ActionJournalSource | string;
  actionType: ActionLogType;
  imageId?: string | null;
  imageFileName?: string | null;
  imagePath?: string | null;
  oldValue?: string | null;
  newValue?: string | null;
  message: string;
  createdAtUnix: number;
  relatedRenames?: Array<{imageId?:string|null;oldPath:string;newPath:string}>;
}

export interface ActionJournalInput {
  imageId?: string | null;
  limit?: number;
  offset?: number;
  includeSearchHistory?: boolean;
}

export interface ActionJournalResult {
  totalCount: number;
  returnedCount: number;
  entries: ActionJournalEntry[];
  loadedAtUnix: number;
}

export interface CreateActionLogEntryInput {
  actionType: ActionLogType;
  imageId?: string | null;
  oldValue?: string | null;
  newValue?: string | null;
  message: string;
}

export interface CreateActionLogEntryResult {
  actionLogId: number;
  createdAtUnix: number;
}
