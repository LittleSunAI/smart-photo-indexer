import {invoke} from "../../shared/tauri/invoke";
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import type {
  SmartImageViewerChooseSavePathInput,
  SmartImageViewerChooseSavePathResult,
  SmartImageViewerDeleteResult,
  SmartImageViewerFileActions,
  SmartImageViewerGenericActionResult,
  SmartImageViewerSaveOriginalCopyInput,
  SmartImageViewerSaveOriginalCopyResult,
  SmartImageViewerSaveOriginalPathInput,
  SmartImageViewerSaveResizeInput,
  SmartImageViewerSaveResizeResult,
  SmartImageViewerShareFileResult,
} from './ImageViewer.types';

export const tauriSmartImageViewerFileActions: SmartImageViewerFileActions = {
  editExternal(path: string): Promise<void> {
    return invoke<SmartImageViewerGenericActionResult>('edit_image_file', { path }).then(() => undefined);
  },

  revealInFileManager(path: string): Promise<void> {
    return revealItemInDir(path);
  },

  chooseSaveImagePath(input: SmartImageViewerChooseSavePathInput): Promise<SmartImageViewerChooseSavePathResult> {
    return invoke<SmartImageViewerChooseSavePathResult>('choose_save_image_path', { input });
  },

  saveResizedImage(input: SmartImageViewerSaveResizeInput): Promise<SmartImageViewerSaveResizeResult> {
    return invoke<SmartImageViewerSaveResizeResult>('save_resized_image', { input });
  },

  chooseSaveOriginalImagePath(input: SmartImageViewerSaveOriginalPathInput): Promise<SmartImageViewerChooseSavePathResult> {
    return invoke<SmartImageViewerChooseSavePathResult>('choose_save_original_image_path', { input });
  },

  saveOriginalImageCopy(input: SmartImageViewerSaveOriginalCopyInput): Promise<SmartImageViewerSaveOriginalCopyResult> {
    return invoke<SmartImageViewerSaveOriginalCopyResult>('save_original_image_copy', { input });
  },

  deleteImage(path: string): Promise<SmartImageViewerDeleteResult> {
    return invoke<SmartImageViewerDeleteResult>('trash_image_file', { path });
  },

  printImage(path: string): Promise<SmartImageViewerGenericActionResult> {
    return invoke<SmartImageViewerGenericActionResult>('print_image_file', { path });
  },

  readImageFileForShare(path: string): Promise<SmartImageViewerShareFileResult> {
    return invoke<SmartImageViewerShareFileResult>('read_image_file_for_share', { path });
  },
  shareImage(path: string): Promise<SmartImageViewerGenericActionResult> {
    return invoke<SmartImageViewerGenericActionResult>('share_image_file', { path });
  },

  setDesktopWallpaper(path: string): Promise<SmartImageViewerGenericActionResult> {
    return invoke<SmartImageViewerGenericActionResult>('set_desktop_wallpaper', { path });
  },
};
