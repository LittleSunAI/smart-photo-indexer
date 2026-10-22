import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import { useEffect } from 'react';
import {invoke} from "../../shared/tauri/invoke";
import { useAppDispatch, useAppState } from './AppStateContext';
import { isTauriRuntime } from '../../shared/tauri/runtime';
import {prepareSavedProvider} from '../../modules/settings/ProviderReadiness';

let migration: Promise<Array<{ provider: 'fal' | 'openrouter'; credentialRef: string; stored: boolean }>> | undefined;
export function requestSelectedLocalAiPrepare() {
  window.dispatchEvent(new CustomEvent('smart-photo-indexer:prepare-local-ai'));
}
export function AiReadinessController() {
  useCurrentLocale();
  const state = useAppState();
  const dispatch = useAppDispatch();
  useEffect(() => {
    if (!isTauriRuntime()) return;
    let cancelled = false;
    // A shared promise makes React remounts reuse the same migration acknowledgement.
    migration ??= (async () => {
      const found = await invoke<Array<{ provider: 'fal' | 'openrouter'; credentialRef: string; stored: boolean }>>('get_provider_credentials');
      const legacy = state.settings.openRouter.apiKey;
      if (legacy.trim()) {
        const saved = await invoke<{ credentialRef: string }>('save_provider_credential', { provider: 'openrouter', key: legacy, persistent: true });
        return [{ provider: 'openrouter' as const, credentialRef: saved.credentialRef, stored: true }, ...found.filter(item => item.provider !== 'openrouter')];
      }
      return found;
    })();
    void migration.then(items => {
      if (cancelled) return;
      const seen = new Set<string>();
      for (const item of items) if (item.stored && !seen.has(item.provider)) {
        seen.add(item.provider);
        dispatch({ type: 'settings/setCloudCredential', provider: item.provider, credentialRef: item.credentialRef });
        // Existing credentials are prepared once at startup, including the unselected service.
        void prepareSavedProvider(item.provider,state.settings[item.provider==='fal'?'fal':'openRouter'].selectedModel,item.credentialRef).catch(()=>{});
      }
    }).catch(error => {
      migration = undefined;
      if (!cancelled) dispatch({ type: 'app/setStatusMessage', message: localeText("main", "ui.AiReadinessController.571b822b2804d346", {"value1": (String(error))}) ,severity:'error'});
    });
    return () => { cancelled = true; };
  }, [dispatch]);
  return null;
}
