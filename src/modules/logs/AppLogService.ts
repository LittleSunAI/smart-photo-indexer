import {text as localeText} from "../../shared/i18n/Runtime";
export type AppLogLevel = 'debug' | 'info' | 'warning' | 'error';
export type AppLogCategory = 'app' | 'ui' | 'local-ai' | 'network' | 'runtime' | 'task';
export type AppLogWriteIntervalMs = 100 | 500 | 1000 | 5000;

export interface AppLogEntry {
  id: string;
  createdAt: string;
  level: AppLogLevel;
  category: AppLogCategory;
  message: string;
  details: string | null;
}

type LogListener = (entries: AppLogEntry[]) => void;
type WriteIntervalListener = (intervalMs: AppLogWriteIntervalMs) => void;

const STORAGE_KEY = 'smart-photo-indexer.logs.v1';
const WRITE_INTERVAL_STORAGE_KEY = 'smart-photo-indexer.logs.write-interval-ms.v1';
export const APP_LOG_MAX_ENTRIES = 800;
const APP_LOG_FALLBACK_MAX_ENTRIES = 120;
const APP_LOG_MAX_DETAILS_CHARS = 6_000;
const DEFAULT_WRITE_INTERVAL_MS: AppLogWriteIntervalMs = 500;
const allowedWriteIntervals: AppLogWriteIntervalMs[] = [100, 500, 1000, 5000];

function nowIso(): string {
  return new Date().toISOString();
}

function createId(): string {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function truncateDetails(value: string): string {
  if (value.length <= APP_LOG_MAX_DETAILS_CHARS) return value;
  return localeText("main", "ui.AppLogService.ebda6b192bdc9194", {"value1": (value.slice(0, APP_LOG_MAX_DETAILS_CHARS))});
}

function normalizeDetails(details: unknown): string | null {
  if (details === undefined || details === null) return null;
  if (details instanceof Error) return truncateDetails(`${details.name}: ${details.message}\n${details.stack ?? ''}`.trim());
  if (typeof details === 'string') return truncateDetails(details);

  try {
    return truncateDetails(JSON.stringify(details, null, 2));
  } catch {
    return truncateDetails(String(details));
  }
}

function readStoredEntries(): AppLogEntry[] {
  if (typeof window === 'undefined') return [];

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((entry): entry is AppLogEntry => Boolean(entry && typeof entry === 'object' && typeof entry.id === 'string'));
  } catch {
    return [];
  }
}

function writeStoredEntries(entries: AppLogEntry[]): void {
  if (typeof window === 'undefined') return;
  const trimmedEntries = entries.slice(-APP_LOG_MAX_ENTRIES);

  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(trimmedEntries));
  } catch {
    try {
      const fallbackEntries = trimmedEntries.slice(-APP_LOG_FALLBACK_MAX_ENTRIES).map((entry) => ({
        ...entry,
        details: entry.details ? truncateDetails(entry.details) : null,
      }));
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(fallbackEntries));
    } catch {
      // Logging must never take down the app UI.
    }
  }
}

function readWriteIntervalMs(): AppLogWriteIntervalMs {
  if (typeof window === 'undefined') return DEFAULT_WRITE_INTERVAL_MS;

  try {
    const raw = Number(window.localStorage.getItem(WRITE_INTERVAL_STORAGE_KEY));
    return allowedWriteIntervals.includes(raw as AppLogWriteIntervalMs) ? raw as AppLogWriteIntervalMs : DEFAULT_WRITE_INTERVAL_MS;
  } catch {
    return DEFAULT_WRITE_INTERVAL_MS;
  }
}

function writeIntervalMs(intervalMs: AppLogWriteIntervalMs): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(WRITE_INTERVAL_STORAGE_KEY, String(intervalMs));
  } catch {
    // Ignore storage failures: the selected interval is a convenience setting.
  }
}

export class AppLogService {
  private listeners = new Set<LogListener>();
  private intervalListeners = new Set<WriteIntervalListener>();
  private lastDebugWriteAt = 0;

  getEntries(): AppLogEntry[] {
    return readStoredEntries();
  }

  getWriteIntervalMs(): AppLogWriteIntervalMs {
    return readWriteIntervalMs();
  }

  setWriteIntervalMs(intervalMs: AppLogWriteIntervalMs): void {
    writeIntervalMs(intervalMs);
    this.intervalListeners.forEach((listener) => listener(intervalMs));
    this.add('info', 'app', localeText("main", "ui.AppLogService.9dd7fa3a6c85a5fd", {"value1": (intervalMs / 1000)}));
  }

  subscribe(listener: LogListener): () => void {
    this.listeners.add(listener);
    listener(this.getEntries());
    return () => this.listeners.delete(listener);
  }

  subscribeWriteInterval(listener: WriteIntervalListener): () => void {
    this.intervalListeners.add(listener);
    listener(this.getWriteIntervalMs());
    return () => this.intervalListeners.delete(listener);
  }

  add(level: AppLogLevel, category: AppLogCategory, message: string, details?: unknown): AppLogEntry | null {
    if (!this.shouldWrite(level)) {
      return null;
    }

    const entry: AppLogEntry = {
      id: createId(),
      createdAt: nowIso(),
      level,
      category,
      message,
      details: normalizeDetails(details),
    };

    const nextEntries = [...this.getEntries(), entry].slice(-APP_LOG_MAX_ENTRIES);
    writeStoredEntries(nextEntries);
    this.listeners.forEach((listener) => listener(nextEntries));
    return entry;
  }

  debug(category: AppLogCategory, message: string, details?: unknown): AppLogEntry | null {
    return this.add('debug', category, message, details);
  }

  info(category: AppLogCategory, message: string, details?: unknown): AppLogEntry | null {
    return this.add('info', category, message, details);
  }

  warning(category: AppLogCategory, message: string, details?: unknown): AppLogEntry | null {
    return this.add('warning', category, message, details);
  }

  error(category: AppLogCategory, message: string, details?: unknown): AppLogEntry | null {
    return this.add('error', category, message, details);
  }

  clear(): void {
    writeStoredEntries([]);
    this.listeners.forEach((listener) => listener([]));
  }

  exportText(entries: AppLogEntry[] = this.getEntries()): string {
    return entries.map((entry) => {
      const head = `[${entry.createdAt}] [${entry.level.toUpperCase()}] [${entry.category}] ${entry.message}`;
      return entry.details ? `${head}\n${entry.details}` : head;
    }).join('\n\n');
  }

  private shouldWrite(level: AppLogLevel): boolean {
    if (level !== 'debug') return true;

    const now = Date.now();
    const intervalMs = this.getWriteIntervalMs();
    if (now - this.lastDebugWriteAt < intervalMs) return false;
    this.lastDebugWriteAt = now;
    return true;
  }
}

export const appLogService = new AppLogService();
