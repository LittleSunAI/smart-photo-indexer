import {invoke} from "../../shared/tauri/invoke";
import type { SmartImageViewerItem } from '../imageViewer/ImageViewer.types';
import { notifySmartImageViewerOpen } from '../imageViewer/imageViewerBus';
import { isTauriRuntime } from '../../shared/tauri/runtime';

export async function openSmartImageViewer(item: SmartImageViewerItem, images: SmartImageViewerItem[]): Promise<void> {
  if (!item.id.trim()) return;
  if (!isTauriRuntime()) {
    notifySmartImageViewerOpen(item.id);
    return;
  }
  await invoke<string>('open_viewer_window', {
    activePath: item.path, paths: images.filter(image => image.kind === 'image').map(image => image.path),
  });
}
