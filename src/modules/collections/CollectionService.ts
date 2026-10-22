import {text as localeText} from "../../shared/i18n/Runtime";
import {invoke} from "../../shared/tauri/invoke";
import { isTauriRuntime } from '../../shared/tauri/runtime';
import type { AddImagesToCollectionInput, CollectionMutationResult, CollectionsResult, CreateCollectionInput, PhotoCollection, SetImageFavoriteInput, SetImageFavoriteResult } from './types';

export interface CollectionService {
  getCollections(): Promise<CollectionsResult>;
  createCollection(input: CreateCollectionInput): Promise<CollectionMutationResult>;
  addImagesToCollection(input: AddImagesToCollectionInput): Promise<CollectionMutationResult>;
  renameCollection(collectionId: string, name: string): Promise<CollectionMutationResult>;
  deleteCollection(collectionId: string): Promise<CollectionMutationResult>;
  removeImagesFromCollection(input: AddImagesToCollectionInput): Promise<CollectionMutationResult>;
  setImageFavorite(input: SetImageFavoriteInput): Promise<SetImageFavoriteResult>;
}

const BROWSER_COLLECTIONS_KEY = 'smart-photo-indexer.browser.collections.v1';
const BROWSER_FAVORITES_KEY = 'smart-photo-indexer.browser.favorites.v1';

function nowUnix(): number {
  return Math.floor(Date.now() / 1000);
}

function stableBrowserId(prefix: string, value: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return `${prefix}:${(hash >>> 0).toString(16).padStart(8, '0')}`;
}

function loadBrowserCollections(): PhotoCollection[] {
  try {
    const raw = window.localStorage.getItem(BROWSER_COLLECTIONS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as PhotoCollection[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function saveBrowserCollections(collections: PhotoCollection[]): void {
  window.localStorage.setItem(BROWSER_COLLECTIONS_KEY, JSON.stringify(collections));
}

class TauriCollectionService implements CollectionService {
  async renameCollection(collectionId: string, name: string): Promise<CollectionMutationResult> {
    if (!isTauriRuntime()) throw new Error(localeText("main", "ui.CollectionService.f7da20d076156380"));
    return invoke('rename_collection', { collectionId, name });
  }
  async deleteCollection(collectionId: string): Promise<CollectionMutationResult> {
    if (!isTauriRuntime()) throw new Error(localeText("main", "ui.CollectionService.f7da20d076156380"));
    return invoke('delete_collection', { collectionId });
  }
  async removeImagesFromCollection(input: AddImagesToCollectionInput): Promise<CollectionMutationResult> {
    if (!isTauriRuntime()) throw new Error(localeText("main", "ui.CollectionService.f7da20d076156380"));
    return invoke('remove_images_from_collection', { input });
  }
  async getCollections(): Promise<CollectionsResult> {
    if (!isTauriRuntime()) {
      const collections = loadBrowserCollections().sort((left, right) => Number(right.isFavorite) - Number(left.isFavorite) || right.updatedAtUnix - left.updatedAtUnix || left.name.localeCompare(right.name));
      return { collections, totalCount: collections.length, loadedAtUnix: nowUnix() };
    }

    return invoke<CollectionsResult>('get_collections');
  }

  async createCollection(input: CreateCollectionInput): Promise<CollectionMutationResult> {
    const name = input.name.trim();
    if (!name) throw new Error(localeText("main", "ui.CollectionService.cbe27858d17f0747"));

    if (!isTauriRuntime()) {
      const collections = loadBrowserCollections();
      const existing = collections.find((collection) => collection.name.toLowerCase() === name.toLowerCase());
      const now = nowUnix();
      const collection: PhotoCollection = existing
        ? { ...existing, description: input.description ?? existing.description, color: input.color ?? existing.color, updatedAtUnix: now }
        : { id: stableBrowserId('collection', name), name, description: input.description ?? null, color: input.color ?? null, isFavorite: false, imageCount: 0, createdAtUnix: now, updatedAtUnix: now };
      const next = existing ? collections.map((item) => item.id === collection.id ? collection : item) : [collection, ...collections];
      saveBrowserCollections(next);
      return { collection, requestedImages: 0, changedImages: 0, skippedImages: 0, actionLogEntries: 0, updatedAtUnix: now };
    }

    return invoke<CollectionMutationResult>('create_collection', { input });
  }

  async addImagesToCollection(input: AddImagesToCollectionInput): Promise<CollectionMutationResult> {
    const uniqueIds = Array.from(new Set(input.imageIds.map((id) => id.trim()).filter(Boolean)));
    if (!input.collectionId.trim()) throw new Error('Collection id is empty.');
    if (uniqueIds.length === 0) throw new Error(localeText("main", "ui.CollectionService.f486f524168ec940"));

    if (!isTauriRuntime()) {
      const collections = loadBrowserCollections();
      const now = nowUnix();
      const collection = collections.find((item) => item.id === input.collectionId) ?? null;
      if (!collection) throw new Error(localeText("main", "ui.CollectionService.8518aa02091aa7ec"));
      const updated = { ...collection, imageCount: collection.imageCount + uniqueIds.length, updatedAtUnix: now };
      saveBrowserCollections(collections.map((item) => item.id === updated.id ? updated : item));
      return { collection: updated, requestedImages: uniqueIds.length, changedImages: uniqueIds.length, skippedImages: 0, actionLogEntries: 0, updatedAtUnix: now };
    }

    return invoke<CollectionMutationResult>('add_images_to_collection', { input: { collectionId: input.collectionId, imageIds: uniqueIds } });
  }

  async setImageFavorite(input: SetImageFavoriteInput): Promise<SetImageFavoriteResult> {
    if (!input.imageId?.trim() && !input.path?.trim()) throw new Error('Image id or path is required.');

    if (!isTauriRuntime()) {
      const rawFavorites = window.localStorage.getItem(BROWSER_FAVORITES_KEY);
      const favorites = new Set<string>(rawFavorites ? JSON.parse(rawFavorites) as string[] : []);
      const key = input.imageId || input.path || 'browser-image';
      if (input.isFavorite) favorites.add(key);
      else favorites.delete(key);
      window.localStorage.setItem(BROWSER_FAVORITES_KEY, JSON.stringify(Array.from(favorites)));
      return {
        imageId: input.imageId || `browser-index:${input.path}`,
        path: input.path || '',
        fileName: (input.path || '').split(/[\\/]/).pop() || 'image',
        isFavorite: input.isFavorite,
        updatedAtUnix: nowUnix(),
        actionLogId: 0,
      };
    }

    return invoke<SetImageFavoriteResult>('set_image_favorite', { input });
  }
}

export const collectionService: CollectionService = new TauriCollectionService();
