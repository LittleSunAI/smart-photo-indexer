import {localeTag as displayLocaleTag} from "../../shared/i18n/Formatting";
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import { useEffect, useState } from 'react';
import {invoke} from "../../shared/tauri/invoke";
import { isTauriRuntime } from '../../shared/tauri/runtime';

interface CacheStatistics { files: number; bytes: number; removedFiles: number; removedBytes: number; maxBytes: number; maxFiles: number; maxAgeDays: number; maintenanceError?: string | null }
export function ThumbnailCachePanel() {
  useCurrentLocale();
  const [stats, setStats] = useState<CacheStatistics | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    if (isTauriRuntime()) void invoke<CacheStatistics>('get_thumbnail_cache_statistics').then(value => { if (active) setStats(value); }, value => { if (active) setError(String(value)); });
    return () => { active = false; };
  }, []);
  async function clear() {
    setBusy(true); setError('');
    try {
      const value = await invoke<CacheStatistics>('clear_thumbnail_cache');
      setStats(value); setMessage(localeText("details", "ui.ThumbnailCachePanel.90a755f000790601", {"value1": (value.removedFiles), "value2": ((value.removedBytes/1048576).toFixed(1))}));
      window.dispatchEvent(new Event('smart-photo-indexer:thumbnail-cache-cleared'));
    } catch (value) { setError(String(value)); }
    finally { setBusy(false); }
  }
  return <section className="details-section" aria-label={localeText("details", "ui.ThumbnailCachePanel.62302399ac5fdda8")}>
    {stats ? <p>{stats.files} {' '}{localeText("details", "ui.ThumbnailCachePanel.e3b0ac928c6319a8")}{' '}{(stats.bytes/1048576).toFixed(1)} {' '}{localeText("details", "ui.ThumbnailCachePanel.6065acaa45483ec4")}{' '}{(stats.maxBytes/1073741824).toFixed(0)} {' '}{localeText("details", "ui.ThumbnailCachePanel.5231d79dfa960cc1")}{' '}{stats.maxFiles.toLocaleString(displayLocaleTag())} {' '}{localeText("details", "ui.ThumbnailCachePanel.acdd30d90c322990")}{' '}{stats.maxAgeDays} {' '}{localeText("details", "ui.ThumbnailCachePanel.c1e689ef195e5864")}</p> : <p>{error ? localeText("details", "ui.ThumbnailCachePanel.a64a6750c9affc5e") : localeText("details", "ui.ThumbnailCachePanel.1ecbefba3c1d4e24")}</p>}
    <p>{localeText("details", "ui.ThumbnailCachePanel.f7d8e1335611c511")}</p>
    {stats?.maintenanceError ? <p role="alert">{localeText("details", "ui.ThumbnailCachePanel.ed13f6f43e7658ed")}{' '}{stats.maintenanceError}</p> : null}
    <Hint message={{id:"library.cache-clear"}}><button type="button" className="secondary-action" disabled={busy || !isTauriRuntime()} onClick={() => { void clear(); }}>{busy ? localeText("details", "ui.ThumbnailCachePanel.0714d2e33da28f5b") : localeText("details", "ui.ThumbnailCachePanel.a80fdaa40bab0833")}</button></Hint>
    {message ? <p role="status">{message}</p> : null}{error ? <p role="alert">{error}</p> : null}
  </section>;
}
