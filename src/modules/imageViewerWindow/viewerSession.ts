import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { SmartImageViewerItem } from '../imageViewer/ImageViewer.types';
import { isTauriRuntime } from '../../shared/tauri/runtime';

export interface ViewerSessionPayload {
  activeImageId: string;
  images: SmartImageViewerItem[];
  openedAtUnix: number;
  totalImages?: number;
  activePosition?: number;
  pageLimit?: number;
  navigationRevision?: number;
}

export interface ViewerImageDeletedPayload {
  itemId: string;
  path: string;
  sourceWindowLabel: string;
}

export async function listenForViewerImageDeleted(handler: (payload: ViewerImageDeletedPayload) => void): Promise<UnlistenFn | null> {
  if (!isTauriRuntime()) return null;
  const currentWindowLabel = getCurrentWindow().label;
  return listen<ViewerImageDeletedPayload>('smart-photo-indexer:viewer-image-deleted', event => {
    if (event.payload.sourceWindowLabel === currentWindowLabel || !event.payload.path.trim()) return;
    handler(event.payload);
  });
}
