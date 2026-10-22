import {text as localeText} from "../../shared/i18n/Runtime";
import type { ImageItem } from '../explorer/types';

export type SearchExportFormat = 'csv' | 'json';
export type SearchExportScope = 'selected' | 'all';

export interface SearchExportPayload {
  appName: string;
  exportedAtIso: string;
  query: string;
  resultCount: number;
  exportedCount: number;
  items: SearchExportItem[];
}

export interface SearchExportItem {
  imageId: string | null;
  fileName: string;
  path: string;
  status: string;
  extension: string | null;
  sizeBytes: number | null;
  modifiedAtUnix: number | null;
  dateTaken: string;
  dimensions: string;
  tags: string[];
  description: string;
  firstSeenAtUnix: number | null;
  lastSeenAtUnix: number | null;
}

export interface SearchExportResult {
  fileName: string;
  exportedCount: number;
  format: SearchExportFormat;
}

export interface CopyPathsResult {
  copiedCount: number;
}

export class SearchExportService {
  async copyPathsText(text: string, count: number): Promise<CopyPathsResult> {
    if(text)await writeClipboardText(text);
    return {copiedCount:text ? count : 0};
  }
  exportResults(params: {
    appName: string;
    query: string;
    results: ImageItem[];
    selectedIds: string[];
    scope: SearchExportScope;
    format: SearchExportFormat;
  }): SearchExportResult {
    const items = pickItems(params.results, params.selectedIds, params.scope);
    const exportedAtIso = new Date().toISOString();
    const payload: SearchExportPayload = {
      appName: params.appName,
      exportedAtIso,
      query: params.query.trim(),
      resultCount: params.results.length,
      exportedCount: items.length,
      items: items.map(toExportItem),
    };
    const safeTimestamp = exportedAtIso.replace(/[:.]/g, '-');
    const safeQuery = slugify(params.query.trim() || 'search-results');
    const fileName = `smart-photo-indexer_${safeQuery}_${params.scope}_${safeTimestamp}.${params.format}`;
    const contents = params.format === 'json' ? toJson(payload) : toCsv(payload.items);
    const mimeType = params.format === 'json' ? 'application/json;charset=utf-8' : 'text/csv;charset=utf-8';

    downloadTextFile(fileName, contents, mimeType);

    return {
      fileName,
      exportedCount: payload.exportedCount,
      format: params.format,
    };
  }

  async copyPaths(params: {
    results: ImageItem[];
    selectedIds: string[];
    scope: SearchExportScope;
  }): Promise<CopyPathsResult> {
    const items = pickItems(params.results, params.selectedIds, params.scope);
    const text = items.map((item) => item.path).join('\n');
    if (!text) {
      return { copiedCount: 0 };
    }

    await writeClipboardText(text);
    return { copiedCount: items.length };
  }

  pickItems(results: ImageItem[], selectedIds: string[], scope: SearchExportScope): ImageItem[] {
    return pickItems(results, selectedIds, scope);
  }
}

function pickItems(results: ImageItem[], selectedIds: string[], scope: SearchExportScope): ImageItem[] {
  if (scope === 'all') return results;
  const selected = new Set(selectedIds);
  return results.filter((item) => selected.has(item.id));
}

function toExportItem(item: ImageItem): SearchExportItem {
  return {
    imageId: item.indexedImageId ?? null,
    fileName: item.name,
    path: item.path,
    status: item.status,
    extension: item.extension ?? null,
    sizeBytes: item.sizeBytes ?? null,
    modifiedAtUnix: item.modifiedAtUnix ?? null,
    dateTaken: item.dateTaken,
    dimensions: item.dimensions,
    tags: item.tags,
    description: item.description,
    firstSeenAtUnix: item.firstSeenAtUnix ?? null,
    lastSeenAtUnix: item.lastSeenAtUnix ?? null,
  };
}

function toJson(payload: SearchExportPayload): string {
  return `${JSON.stringify(payload, null, 2)}\n`;
}

function toCsv(items: SearchExportItem[]): string {
  const rows = [
    ['image_id', 'file_name', 'path', 'status', 'extension', 'size_bytes', 'modified_at_unix', 'date_taken', 'dimensions', 'tags', 'description', 'first_seen_at_unix', 'last_seen_at_unix'],
    ...items.map((item) => [
      item.imageId ?? '',
      item.fileName,
      item.path,
      item.status,
      item.extension ?? '',
      item.sizeBytes?.toString() ?? '',
      item.modifiedAtUnix?.toString() ?? '',
      item.dateTaken,
      item.dimensions,
      item.tags.join('; '),
      item.description,
      item.firstSeenAtUnix?.toString() ?? '',
      item.lastSeenAtUnix?.toString() ?? '',
    ]),
  ];

  return '\uFEFF' + rows.map((row) => row.map(escapeCsvCell).join(',')).join('\n') + '\n';
}

function escapeCsvCell(value: string): string {
  // JSON preserves original strings; CSV protects text opened by spreadsheet software.
  if (/^[\s\u0000-\u001f]*[=+@-]/u.test(value)) value = `'${value}`;
  if (/[,"\n\r]/.test(value)) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

function downloadTextFile(fileName: string, contents: string, mimeType: string): void {
  const blob = new Blob([contents], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = fileName;
  anchor.rel = 'noopener';
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

async function writeClipboardText(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const textarea = document.createElement('textarea');
  textarea.value = text;
  textarea.setAttribute('readonly', 'true');
  textarea.style.position = 'fixed';
  textarea.style.left = '-9999px';
  document.body.append(textarea);
  textarea.select();
  const copied = document.execCommand('copy');
  textarea.remove();
  if (!copied) {
    throw new Error(localeText("search", "ui.SearchExportService.e92c963e68c2f34f"));
  }
}

function slugify(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9а-яё_-]+/giu, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48) || 'search-results';
}

export const searchExportService = new SearchExportService();
