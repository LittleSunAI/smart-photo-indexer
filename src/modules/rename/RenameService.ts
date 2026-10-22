import {text as localeText} from "../../shared/i18n/Runtime";
import {invoke} from "../../shared/tauri/invoke";
import { isTauriRuntime } from '../../shared/tauri/runtime';
import type { ImageItem } from '../explorer/types';

export type RenamePreviewStatus = 'ready' | 'unchanged' | 'conflict' | 'error';

export interface RenamePlanItem {
  planId?: string | null;
  imageId: string | null;
  oldPath: string;
  newPath: string;
  oldFileName: string;
  newFileName: string;
  reason: string;
}

export interface SmartRenameInput {
  planId?: string | null;
  imageId?: string | null;
  currentPath: string;
  fileName: string;
  modifiedAtUnix?: number | null;
  pattern: 'date_unique8';
}

export interface SmartRenamePreviewItem {
  planId?: string | null;
  imageId?: string | null;
  oldPath: string;
  newPath: string;
  oldFileName: string;
  newFileName: string;
  status: RenamePreviewStatus;
  message: string;
  sizeBytes?: number | null;
  modifiedAtUnix?: number | null;
  candidateCollisions: number;
}

export interface RenamePreviewResult {
  total: number;
  ready: number;
  unchanged: number;
  conflicts: number;
  errors: number;
  items: SmartRenamePreviewItem[];
  createdAtUnix: number;
}

export interface SmartRenameResult {
  imageId?: string | null;
  oldPath: string;
  newPath: string;
  oldFileName: string;
  newFileName: string;
  wasRenamed: boolean;
  sizeBytes?: number | null;
  modifiedAtUnix?: number | null;
  actionLogId?: number | null;
  message: string;
}

export interface RenameService {
  createPlan(images: ImageItem[]): Promise<RenamePlanItem[]>;
  previewSmartRenameImages(images: ImageItem[],analysisCopy?:import("../settings/ProfilePreferences").ProfilePreferences["analysisCopy"]): Promise<RenamePreviewResult>;
  smartRenameImage(input: SmartRenameInput): Promise<SmartRenameResult>;
  applyPlan(plan: RenamePlanItem[]): Promise<SmartRenameResult[]>;
}

function browserPreviewItem(input: SmartRenameInput): SmartRenamePreviewItem {
  const now = Math.floor(Date.now() / 1000);
  const extension = input.fileName.includes('.') ? `.${input.fileName.split('.').pop()}` : '';
  const date = formatDateFromUnix(input.modifiedAtUnix ?? now);
  const unique = quickUnique8(`${input.currentPath}|${input.fileName}|${input.modifiedAtUnix ?? 0}`);
  const newFileName = `${date}_${unique}${extension.toLowerCase()}`;
  const newPath = input.currentPath.replace(/[^\\/]+$/, newFileName);
  const unchanged = newFileName === input.fileName;

  return {
    imageId: input.imageId ?? null,
    oldPath: input.currentPath,
    newPath,
    oldFileName: input.fileName,
    newFileName,
    status: unchanged ? 'unchanged' : 'ready',
    sizeBytes: null,
    modifiedAtUnix: input.modifiedAtUnix ?? null,
    candidateCollisions: 0,
    message: unchanged
      ? localeText("rename", "ui.RenameService.999fa5ee10990b52")
      : localeText("rename", "ui.RenameService.200a69460249202a"),
  };
}

function browserRenameResult(input: SmartRenameInput): SmartRenameResult {
  const preview = browserPreviewItem(input);

  return {
    imageId: input.imageId ?? null,
    oldPath: input.currentPath,
    newPath: preview.newPath,
    oldFileName: input.fileName,
    newFileName: preview.newFileName,
    wasRenamed: false,
    modifiedAtUnix: input.modifiedAtUnix ?? null,
    actionLogId: null,
    message: localeText("rename", "ui.RenameService.c379bacdf99deeec"),
  };
}

class TauriRenameService implements RenameService {
  async createPlan(images: ImageItem[]): Promise<RenamePlanItem[]> {
    const preview = await this.previewSmartRenameImages(images);

    return preview.items.map((item) => ({
      planId: item.planId,
      imageId: item.imageId ?? null,
      oldPath: item.oldPath,
      newPath: item.newPath,
      oldFileName: item.oldFileName,
      newFileName: item.newFileName,
      reason: item.message || localeText("rename", "ui.RenameService.13bf614168f52b72"),
    }));
  }

  async previewSmartRenameImages(images: ImageItem[],analysisCopy?:import("../settings/ProfilePreferences").ProfilePreferences["analysisCopy"]): Promise<RenamePreviewResult> {
    const inputs = images.map((image) => ({
      imageId: image.indexedImageId ?? null,
      currentPath: image.path,
      fileName: image.name,
      modifiedAtUnix: image.modifiedAtUnix ?? null,
      pattern: 'date_unique8' as const,
    }));

    if (!isTauriRuntime()) {
      const items = inputs.map(browserPreviewItem);
      return summarizeBrowserPreview(items);
    }

    return invoke<RenamePreviewResult>('preview_smart_rename_images', { inputs,analysisCopy });
  }

  async smartRenameImage(input: SmartRenameInput): Promise<SmartRenameResult> {
    if (!isTauriRuntime()) {
      return browserRenameResult(input);
    }

    return invoke<SmartRenameResult>('smart_rename_image', { input });
  }

  async applyPlan(plan: RenamePlanItem[]): Promise<SmartRenameResult[]> {
    const results: SmartRenameResult[] = [];
    for (const item of plan) {
      results.push(await this.smartRenameImage({
        planId: item.planId,
        imageId: item.imageId,
        currentPath: item.oldPath,
        fileName: item.oldFileName,
        pattern: 'date_unique8',
      }));
    }
    return results;
  }
}

function summarizeBrowserPreview(items: SmartRenamePreviewItem[]): RenamePreviewResult {
  return {
    total: items.length,
    ready: items.filter((item) => item.status === 'ready').length,
    unchanged: items.filter((item) => item.status === 'unchanged').length,
    conflicts: items.filter((item) => item.status === 'conflict').length,
    errors: items.filter((item) => item.status === 'error').length,
    items,
    createdAtUnix: Math.floor(Date.now() / 1000),
  };
}

function quickUnique8(value: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(36).toUpperCase().padStart(8, '0').slice(0, 8);
}

function formatDateFromUnix(value: number): string {
  const date = new Date(value * 1000);
  if (Number.isNaN(date.getTime())) {
    return new Date().toISOString().slice(0, 10);
  }
  return date.toISOString().slice(0, 10);
}

export const renameService: RenameService = new TauriRenameService();
