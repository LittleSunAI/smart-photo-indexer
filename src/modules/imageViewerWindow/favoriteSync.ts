import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { SetImageFavoriteResult } from '../collections/types';
import { isTauriRuntime } from '../../shared/tauri/runtime';

const IMAGE_FAVORITE_UPDATED_EVENT = 'smart-photo-indexer:image-favorite-updated';

interface ImageFavoriteUpdatedPayload {
  result: SetImageFavoriteResult;
  sourceWindowLabel: string;
}

export async function broadcastImageFavoriteUpdated(result: SetImageFavoriteResult): Promise<void> {
  if (!isTauriRuntime()) return;

  const sourceWindowLabel = getCurrentWindow().label;
  await emit(IMAGE_FAVORITE_UPDATED_EVENT, { result, sourceWindowLabel } satisfies ImageFavoriteUpdatedPayload);
}

export async function listenForImageFavoriteUpdates(handler: (result: SetImageFavoriteResult) => void): Promise<UnlistenFn | null> {
  if (!isTauriRuntime()) return null;

  const currentWindowLabel = getCurrentWindow().label;
  return listen<ImageFavoriteUpdatedPayload>(IMAGE_FAVORITE_UPDATED_EVENT, (event) => {
    if (event.payload.sourceWindowLabel === currentWindowLabel) return;
    handler(event.payload.result);
  });
}
