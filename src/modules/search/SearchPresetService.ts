import {text as localeText} from "../../shared/i18n/Runtime";
import {normalizeSearchFields,legacySearchFields,defaultSearchFields,type SearchField} from './SearchFieldsContract';
import {normalizeSearchOrder} from './SearchOrder';
import type { IndexSearchStatusFilter } from '../index/types';

export type SearchPresetTagMode = 'any' | 'all';

export interface SearchPresetDefinition {
  fields?:SearchField[];
  untaggedOnly?: boolean;
  expandTerms?: boolean;
  collectionId?: string;
  rootPath?: string;
  sortOrder?: string;
  dateField?: 'modified' | 'taken' | 'indexed' | 'analyzed';
  id: string;
  name: string;
  query: string;
  statusFilter: IndexSearchStatusFilter;
  tagFilterText: string;
  tagMode: SearchPresetTagMode;
  dateFrom: string;
  dateTo: string;
  favoriteOnly: boolean;
  isFavorite: boolean;
  createdAtUnix: number;
  updatedAtUnix: number;
  lastUsedAtUnix: number | null;
}

export interface SearchPresetDraft {
  fields?:SearchField[];
  untaggedOnly?: boolean;
  expandTerms?: boolean;
  collectionId?: string;
  rootPath?: string;
  sortOrder?: string;
  dateField?: 'modified' | 'taken' | 'indexed' | 'analyzed';
  name?: string;
  query: string;
  statusFilter: IndexSearchStatusFilter;
  tagFilterText: string;
  tagMode: SearchPresetTagMode;
  dateFrom: string;
  dateTo: string;
  favoriteOnly?: boolean;
}

const STORAGE_KEY = 'smart-photo-indexer.search-presets.v1';
const MAX_PRESETS = 40;

function nowUnix(): number {
  return Math.floor(Date.now() / 1000);
}

function createPresetId(): string {
  return `search-preset-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function normalizeTagText(value: string): string {
  const seen = new Set<string>();
  const tags: string[] = [];
  for (const raw of value.split(/[\n,;]+/)) {
    const tag = raw.trim().replace(/^#+/, '').toLowerCase();
    if (!tag || seen.has(tag)) continue;
    seen.add(tag);
    tags.push(tag);
  }
  return tags.join(', ');
}

function normalizePreset(input: Partial<SearchPresetDefinition>): SearchPresetDefinition | null {
  if (!input || typeof input !== 'object') return null;
  const timestamp = nowUnix();
  const id = typeof input.id === 'string' && input.id.trim() ? input.id : createPresetId();
  const query = typeof input.query === 'string' ? input.query : '';
  const statusFilter = isStatusFilter(input.statusFilter) ? input.statusFilter : 'all';
  const tagMode = input.tagMode === 'all' ? 'all' : 'any';
  const tagFilterText = typeof input.tagFilterText === 'string' ? normalizeTagText(input.tagFilterText) : '';
  const dateFrom = typeof input.dateFrom === 'string' ? input.dateFrom : '';
  const dateTo = typeof input.dateTo === 'string' ? input.dateTo : '';
  const favoriteOnly = Boolean(input.favoriteOnly);
  const name = typeof input.name === 'string' && input.name.trim() ? input.name.trim() : buildPresetName({ query, statusFilter, tagFilterText, tagMode, dateFrom, dateTo, favoriteOnly });

  return {
    id,
    name,
    query,
    statusFilter,
    tagFilterText,
    tagMode,
    dateFrom,
    dateTo,
    favoriteOnly,
    untaggedOnly: input.untaggedOnly===true,
    fields:normalizeSearchFields(input.fields,legacySearchFields),
    expandTerms: input.expandTerms === true,
    collectionId: typeof input.collectionId==='string' ? input.collectionId : '',
    rootPath: typeof input.rootPath==='string' ? input.rootPath : '',
    sortOrder: normalizeSearchOrder(input.sortOrder),
    dateField: input.dateField==='taken' || input.dateField==='indexed' || input.dateField==='analyzed' ? input.dateField : 'modified',
    isFavorite: Boolean(input.isFavorite),
    createdAtUnix: typeof input.createdAtUnix === 'number' ? input.createdAtUnix : timestamp,
    updatedAtUnix: typeof input.updatedAtUnix === 'number' ? input.updatedAtUnix : timestamp,
    lastUsedAtUnix: typeof input.lastUsedAtUnix === 'number' ? input.lastUsedAtUnix : null,
  };
}

function isStatusFilter(value: unknown): value is IndexSearchStatusFilter {
  return value === 'all'
    || value === 'processed'
    || value === 'queued'
    || value === 'notProcessed'
    || value === 'error'
    || value === 'relocated'
    || value === 'missing';
}

function sortPresets(presets: SearchPresetDefinition[]): SearchPresetDefinition[] {
  return [...presets].sort((a, b) => {
    if (a.isFavorite !== b.isFavorite) return a.isFavorite ? -1 : 1;
    return (b.lastUsedAtUnix ?? b.updatedAtUnix) - (a.lastUsedAtUnix ?? a.updatedAtUnix);
  });
}

function safeLocalStorage(): Storage | null {
  try {
    return typeof window !== 'undefined' ? window.localStorage : null;
  } catch {
    return null;
  }
}

function readPresetsFromStorage(): SearchPresetDefinition[] {
  const storage = safeLocalStorage();
  if (!storage) return [];
  const raw = storage.getItem(STORAGE_KEY);
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return sortPresets(parsed.map((item) => normalizePreset(item as Partial<SearchPresetDefinition>)).filter(Boolean) as SearchPresetDefinition[]).slice(0, MAX_PRESETS);
  } catch {
    return [];
  }
}

function writePresetsToStorage(presets: SearchPresetDefinition[]): void {
  const storage = safeLocalStorage();
  if (!storage) return;
  storage.setItem(STORAGE_KEY, JSON.stringify(sortPresets(presets).slice(0, MAX_PRESETS)));
}

export function buildPresetName(draft: Omit<SearchPresetDraft, 'name'>): string {
  const parts: string[] = [];
  const query = draft.query.trim();
  const tags = normalizeTagText(draft.tagFilterText);
  if (query) parts.push(query.length > 42 ? `${query.slice(0, 39)}…` : query);
  if (tags) parts.push(`#${tags.split(', ').slice(0, 3).join(' #')}`);
  if (draft.statusFilter !== 'all') parts.push(draft.statusFilter);
  if (draft.dateFrom || draft.dateTo) parts.push(`${draft.dateFrom || 'any'}–${draft.dateTo || 'any'}`);
  if (draft.favoriteOnly) parts.push('favorites');
  if (draft.untaggedOnly) parts.push(localeText("search", "ui.SearchPresetService.b8681f473d639297"));
  return parts.length > 0 ? parts.join(' · ') : localeText("search", "ui.SearchPresetService.20715e9df1af5ffb");
}

export const searchPresetService = {
  loadPresets(): SearchPresetDefinition[] {
    return readPresetsFromStorage();
  },

  savePreset(draft: SearchPresetDraft, existingPresets: SearchPresetDefinition[]): SearchPresetDefinition[] {
    const timestamp = nowUnix();
    const preset: SearchPresetDefinition = {
      id: createPresetId(),
      name: (draft.name?.trim() || buildPresetName(draft)).slice(0, 90),
      query: draft.query.trim(),
      statusFilter: draft.statusFilter,
      tagFilterText: normalizeTagText(draft.tagFilterText),
      tagMode: draft.tagMode,
      dateFrom: draft.dateFrom,
      dateTo: draft.dateTo,
      favoriteOnly: Boolean(draft.favoriteOnly),
      untaggedOnly: draft.untaggedOnly===true,
      collectionId: draft.collectionId || '',
      rootPath: draft.rootPath || '',
      sortOrder: draft.sortOrder || 'relevance',
      dateField: draft.dateField || 'modified',
      fields:normalizeSearchFields(draft.fields,defaultSearchFields),
      expandTerms: draft.expandTerms === true,
      isFavorite: false,
      createdAtUnix: timestamp,
      updatedAtUnix: timestamp,
      lastUsedAtUnix: null,
    };

    const next = sortPresets([preset, ...existingPresets.filter((item) => !isSameSearchPreset(item, preset))]).slice(0, MAX_PRESETS);
    writePresetsToStorage(next);
    return next;
  },

  deletePreset(id: string, existingPresets: SearchPresetDefinition[]): SearchPresetDefinition[] {
    const next = existingPresets.filter((preset) => preset.id !== id);
    writePresetsToStorage(next);
    return next;
  },

  toggleFavorite(id: string, existingPresets: SearchPresetDefinition[]): SearchPresetDefinition[] {
    const timestamp = nowUnix();
    const next = sortPresets(existingPresets.map((preset) => preset.id === id ? { ...preset, isFavorite: !preset.isFavorite, updatedAtUnix: timestamp } : preset));
    writePresetsToStorage(next);
    return next;
  },

  renamePreset(id: string, name: string, existingPresets: SearchPresetDefinition[]): SearchPresetDefinition[] {
    const timestamp = nowUnix();
    const cleanName = name.trim().slice(0, 90);
    if (!cleanName) return existingPresets;
    const next = sortPresets(existingPresets.map((preset) => preset.id === id ? { ...preset, name: cleanName, updatedAtUnix: timestamp } : preset));
    writePresetsToStorage(next);
    return next;
  },

  markPresetUsed(id: string, existingPresets: SearchPresetDefinition[]): SearchPresetDefinition[] {
    const timestamp = nowUnix();
    const next = sortPresets(existingPresets.map((preset) => preset.id === id ? { ...preset, lastUsedAtUnix: timestamp } : preset));
    writePresetsToStorage(next);
    return next;
  },
};

function isSameSearchPreset(a: SearchPresetDefinition, b: SearchPresetDefinition): boolean {
  return a.query.trim().toLowerCase() === b.query.trim().toLowerCase()
    && a.statusFilter === b.statusFilter
    && normalizeTagText(a.tagFilterText) === normalizeTagText(b.tagFilterText)
    && a.tagMode === b.tagMode
    && a.dateFrom === b.dateFrom
    && a.dateTo === b.dateTo
    && Boolean(a.favoriteOnly) === Boolean(b.favoriteOnly)
    && Boolean(a.untaggedOnly) === Boolean(b.untaggedOnly)
    && (a.collectionId || '') === (b.collectionId || '')
    && (a.rootPath || '') === (b.rootPath || '')
    && (a.sortOrder || 'relevance') === (b.sortOrder || 'relevance')
    && (a.dateField || 'modified') === (b.dateField || 'modified')
    && normalizeSearchFields(a.fields,legacySearchFields).join(',')===normalizeSearchFields(b.fields,legacySearchFields).join(',')
    && Boolean(a.expandTerms) === Boolean(b.expandTerms);
}
