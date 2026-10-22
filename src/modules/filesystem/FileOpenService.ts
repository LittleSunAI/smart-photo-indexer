import {text as localeText} from "../../shared/i18n/Runtime";
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import {invoke} from "../../shared/tauri/invoke";
import { isTauriRuntime } from '../../shared/tauri/runtime';

export interface FileOpenService {
  openExternalPath(path: string): Promise<void>;
  revealInFileManager(path: string): Promise<void>;
  canUseSystemOpener(): boolean;
}

function normalizePath(path: string): string {
  return path.trim();
}

export class TauriFileOpenService implements FileOpenService {
  canUseSystemOpener(): boolean {
    return isTauriRuntime();
  }

  async openExternalPath(path: string): Promise<void> {
    const normalizedPath = normalizePath(path);
    if (!normalizedPath) {
      throw new Error(localeText("main", "ui.FileOpenService.c249a45d67ab535f"));
    }

    if (!this.canUseSystemOpener()) {
      throw new Error(localeText("main", "ui.FileOpenService.e425dc6edb6b0290"));
    }

    await invoke('open_external_path', { path: normalizedPath });
  }

  async revealInFileManager(path: string): Promise<void> {
    const normalizedPath = normalizePath(path);
    if (!normalizedPath) {
      throw new Error(localeText("main", "ui.FileOpenService.c27ad6049c061788"));
    }

    if (!this.canUseSystemOpener()) {
      throw new Error(localeText("main", "ui.FileOpenService.7db7e47e9c10be83"));
    }

    await revealItemInDir(normalizedPath);
  }
}

export const fileOpenService: FileOpenService = new TauriFileOpenService();
