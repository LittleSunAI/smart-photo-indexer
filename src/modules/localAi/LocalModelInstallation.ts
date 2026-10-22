import{text as localeText,useCurrentLocale}from'../../shared/i18n/Runtime';
import{nativeDisplayText}from'../../shared/i18n/NativeMessages';
import type{ResourceKeys}from'../../shared/i18n/generated/ResourceKeys';
import { useEffect, useSyncExternalStore } from 'react';
import {invoke} from "../../shared/tauri/invoke";
import { listen } from '@tauri-apps/api/event';
import { isTauriRuntime } from '../../shared/tauri/runtime';

interface SetupState {
  state: 'unknown' | 'checking' | 'missing' | 'installing' | 'ready' | 'error';
  message: string;
  messageKey?:ResourceKeys['settings'];
  percent?: number;
  cancellable?: boolean;
}
interface Progress { model: string; endpoint?: string; message?: string; status?: string; completed?: number; total?: number; cancellable?: boolean }
const states = new Map<string, SetupState>();
const checks = new Map<string, Promise<void>>();
const listeners = new Set<() => void>();
const key = (model: string, endpoint: string) => `${endpoint.replace(/\/+$/, '')}\n${model}`;
const get = (model: string, endpoint: string) => {
  const id = key(model, endpoint);
  if (!states.has(id)) states.set(id, { state: 'unknown', message: '', messageKey: "ui.LocalModelInstallation.c165d6a4135caa66" });
  return states.get(id)!;
};
const set = (model: string, endpoint: string, value: SetupState) => {
  states.set(key(model, endpoint), value);
  listeners.forEach(listener => listener());
};
const subscribe = (listener: () => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; };

export function checkLocalInstallation(model: string, endpoint: string): Promise<void> {
  const id = key(model, endpoint);
  if (checks.has(id)) return checks.get(id)!;
  if (get(model, endpoint).state === 'installing') return Promise.resolve();
  set(model, endpoint, { state: 'checking', message: '', messageKey: "ui.LocalModelInstallation.97f6d0ab9e2e7317" });
  const operation = (async () => {
    try {
      const result = await invoke<{ ready: boolean; message: string }>('get_ai_provider_status', { provider: 'local', model, endpoint });
      set(model, endpoint, { state: result.ready ? 'ready' : 'missing', message: result.message });
    } catch (error) { set(model, endpoint, { state: 'missing', message: String(error) }); }
    finally { checks.delete(id); }
  })();
  checks.set(id, operation);
  return operation;
}

export async function installLocalModel(model: string, endpoint: string) {
  if (get(model, endpoint).state === 'installing') return;
  await checks.get(key(model, endpoint));
  set(model, endpoint, { state: 'installing', message: '', messageKey: "ui.LocalModelInstallation.a931200c022762fd", cancellable: true });
  const cleanups: Array<() => void> = [];
  try {
    cleanups.push(await listen<Progress>('local-install-progress', event => {
      const p = event.payload;
      if (p.model !== model || (p.endpoint && key(model, p.endpoint) !== key(model, endpoint))) return;
      set(model, endpoint, { state: 'installing', message:p.message??'',messageKey:p.message?undefined:"ui.LocalModelInstallation.65c998fd59ab1739", percent: p.total ? Math.min(100, Math.floor(100 * (p.completed ?? 0) / p.total)) : undefined, cancellable: p.cancellable !== false });
    }));
    cleanups.push(await listen<Progress>('local-model-download', event => {
      const p = event.payload;
      if (p.model !== model) return;
      set(model, endpoint, { state: 'installing', message:'',messageKey:p.status==='success'?"ui.LocalModelInstallation.69ba9aac88412e90":"ui.LocalModelInstallation.3505bb60ce31d808", percent: p.total ? Math.min(100, Math.floor(100 * (p.completed ?? 0) / p.total)) : undefined, cancellable: true });
    }));
    const result = await invoke<{ ready: boolean; message: string }>('install_local_model', { model, endpoint });
    if (!result.ready) throw Error(result.message);
    set(model, endpoint, { state: 'ready', message: '', messageKey: "ui.LocalModelInstallation.881620ba38849cf3" });
  } catch (error) { set(model, endpoint, { state: 'error', message: String(error) }); }
  finally { cleanups.forEach(cleanup => cleanup()); }
}

export async function cancelLocalInstallation() {
  await invoke('cancel_local_model_install');
}

export function useLocalInstallation(model: string, endpoint: string, enabled: boolean) {
  useCurrentLocale();
  const snapshot = useSyncExternalStore(subscribe, () => get(model, endpoint));
  useEffect(() => {
    if (enabled && isTauriRuntime() && get(model, endpoint).state === 'unknown') void checkLocalInstallation(model, endpoint);
  }, [model, endpoint, enabled]);
  return {...snapshot,message:snapshot.messageKey?localeText('settings',snapshot.messageKey):nativeDisplayText(snapshot.message)};
}
