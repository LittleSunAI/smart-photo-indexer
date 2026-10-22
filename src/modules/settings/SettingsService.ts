import {text as localeText} from "../../shared/i18n/Runtime";
import {loadUiPreferences} from '../../shared/UiPreferences';
import type { AppSettings, ApiKeyCheckResult, LocalAiModelPresetId, OpenRouterModelPresetId } from './Settings.types';
import { defaultLocalAiModelPresetId, localAiModelPresets, openRouterModelPresets, falModelPresets, resolveLocalAiModel, resolvePresetModel } from './modelPresets';
import { messages } from '../../shared/i18n';

const STORAGE_KEY = 'smart-photo-indexer.settings.v1';

export const defaultAppSettings: AppSettings = {
  aiProviderMode: 'fal',
  privacy: {
    aiConsentAccepted: false,
    consentAcceptedAt: null,
    externalAiProviderNoticeSeen: false,
  },
  openRouter: {
    credentialRef: null,
    apiKey: '',
    selectedModel: 'openai/gpt-6-luna',
    selectedPresetId: 'openai/gpt-6-luna',
    customModel: '',
    useFreeRouter: false,
    requestDelayMs: 700,
    maxImagesPerRun: 50,
    maxEstimatedCostPerRunUsd: 0,
    showApiKey: false,
  },
  fal: {
    credentialRef: null, apiKey: '', selectedModel: 'openai/gpt-6-luna', selectedPresetId: 'openai/gpt-6-luna', customModel: '', useFreeRouter: false,
    requestDelayMs: 700, maxImagesPerRun: 50, maxEstimatedCostPerRunUsd: 0, showApiKey: false,
  },
  localAi: {
    endpoint: 'http://127.0.0.1:11434',
    selectedModel: defaultLocalAiModelPresetId,
    selectedPresetId: defaultLocalAiModelPresetId,
    customModel: '',
  },
  processingLimits: {
    onlyUnprocessed: true,
    outputMode: 'both',
    manualQueue: false,
    passes: 1, cloudConcurrency: 2, maxTokens: 3200, imageMaxSide: 2560,
    maxImagesPerRun: 50,
    requestDelayMs: 700,
    maxEstimatedCostPerRunUsd: 0,
    stopOnFirstAiError: false,
    maxRetriesPerTask: 2,
  },
  rename: {
    enabled: false,
    pattern: 'date_unique8',
    stopOnFirstError: true,
  },
  theme: loadUiPreferences().theme,
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function toNumber(value: unknown, fallback: number, min: number, max: number): number {
  const numeric = typeof value === 'number' ? value : typeof value === 'string' ? Number(value) : NaN;
  if (!Number.isFinite(numeric)) return fallback;
  return Math.min(max, Math.max(min, numeric));
}

function normalizeOpenRouterPresetId(value: unknown, provider: 'fal' | 'openrouter' = 'openrouter'): OpenRouterModelPresetId {
  if (value === 'custom') return 'custom';
  if (value === 'nex-agi/nex-n2.5-mini') return provider === 'fal' ? 'google/gemini-2.5-flash-lite' : value;
  const rows = provider === 'fal' ? falModelPresets : openRouterModelPresets;
  if (typeof value === 'string' && rows.some((preset) => preset.id === value)) {
    return value as OpenRouterModelPresetId;
  }

  return rows[0].id;
}

function normalizeLocalAiPresetId(value: unknown): LocalAiModelPresetId {
  if (typeof value === 'string' && localAiModelPresets.some((preset) => preset.id === value)) {
    return value as LocalAiModelPresetId;
  }

  return defaultAppSettings.localAi.selectedPresetId;
}

function mergeSettings(raw: unknown): AppSettings {
  if (!isRecord(raw)) return {...defaultAppSettings,theme:loadUiPreferences().theme};

  const privacy = isRecord(raw.privacy) ? raw.privacy : {};
  const openRouter = isRecord(raw.openRouter) ? raw.openRouter : {};
  const fal = isRecord(raw.fal) ? raw.fal : {};
  const localAi = isRecord(raw.localAi) ? raw.localAi : {};
  const limits = isRecord(raw.processingLimits) ? raw.processingLimits : {};
  const rename = isRecord(raw.rename) ? raw.rename : {};
  const selectedPresetId = normalizeOpenRouterPresetId(openRouter.selectedPresetId);
  const customModel = typeof openRouter.customModel === 'string' ? openRouter.customModel : '';
  const falCustomModel = typeof fal.customModel === 'string' ? fal.customModel : '';
  const localPresetId = normalizeLocalAiPresetId(localAi.selectedPresetId);
  const localCustomModel = typeof localAi.customModel === 'string' ? localAi.customModel : '';

  const selectedModel = resolvePresetModel(selectedPresetId, customModel) || defaultAppSettings.openRouter.selectedModel;
  const selectedLocalModel = resolveLocalAiModel(localPresetId, localCustomModel) || defaultAppSettings.localAi.selectedModel;
  const maxImagesPerRun = toNumber(limits.maxImagesPerRun ?? openRouter.maxImagesPerRun, defaultAppSettings.processingLimits.maxImagesPerRun, 1, 5000);
  const requestDelayMs = toNumber(limits.requestDelayMs ?? openRouter.requestDelayMs, defaultAppSettings.processingLimits.requestDelayMs, 0, 60000);
  const maxEstimatedCostPerRunUsd = toNumber(limits.maxEstimatedCostPerRunUsd ?? openRouter.maxEstimatedCostPerRunUsd, defaultAppSettings.processingLimits.maxEstimatedCostPerRunUsd, 0, 1000);
  const maxRetriesPerTask = toNumber(limits.maxRetriesPerTask, defaultAppSettings.processingLimits.maxRetriesPerTask, 0, 5);

  return {
    ...defaultAppSettings,
    aiProviderMode: raw.aiProviderMode === 'local' || raw.aiProviderMode === 'fal' ? raw.aiProviderMode : 'openrouter',
    privacy: {
      aiConsentAccepted: privacy.aiConsentAccepted === true,
      consentAcceptedAt: typeof privacy.consentAcceptedAt === 'string' ? privacy.consentAcceptedAt : null,
      externalAiProviderNoticeSeen: privacy.externalAiProviderNoticeSeen === true,
    },
    openRouter: {
      ...defaultAppSettings.openRouter,
      credentialRef: typeof openRouter.credentialRef === 'string' ? openRouter.credentialRef : null,
      apiKey: typeof openRouter.apiKey === 'string' ? openRouter.apiKey : '',
      selectedPresetId,
      selectedModel,
      customModel,
      useFreeRouter: false,
      requestDelayMs,
      maxImagesPerRun,
      maxEstimatedCostPerRunUsd,
      showApiKey: openRouter.showApiKey === true,
    },
    fal: {
      ...defaultAppSettings.fal,
      credentialRef: typeof fal.credentialRef === 'string' ? fal.credentialRef : null,
      selectedPresetId: normalizeOpenRouterPresetId(fal.selectedPresetId,'fal'),
      selectedModel: resolvePresetModel(normalizeOpenRouterPresetId(fal.selectedPresetId,'fal'),falCustomModel,'fal'),
      customModel: falCustomModel,
    },
    localAi: {
      ...defaultAppSettings.localAi,
      endpoint: typeof localAi.endpoint === 'string' && localAi.endpoint.trim()
        ? localAi.endpoint.trim()
        : defaultAppSettings.localAi.endpoint,
      selectedPresetId: localPresetId,
      selectedModel: selectedLocalModel,
      customModel: localCustomModel,
    },
    processingLimits: {
      onlyUnprocessed: limits.onlyUnprocessed !== false,
      outputMode: limits.outputMode==='tags'||limits.outputMode==='description'?limits.outputMode:'both',
      manualQueue: limits.manualQueue === true,
      passes: Math.round(toNumber(limits.passes, 1, 1, 5)),
      cloudConcurrency: Math.round(toNumber(limits.cloudConcurrency, 2, 1, 100)),
      maxTokens: Math.round(toNumber(limits.maxTokens, 3200, 1600, 4096)),
      imageMaxSide: Math.round(toNumber(limits.imageMaxSide, 2560, 1024, 4096)),
      maxImagesPerRun,
      requestDelayMs,
      maxEstimatedCostPerRunUsd,
      stopOnFirstAiError: limits.stopOnFirstAiError === true,
      maxRetriesPerTask,
    },
    rename: {
      enabled: rename.enabled === true,
      pattern: 'date_unique8',
      stopOnFirstError: rename.stopOnFirstError !== false,
    },
    theme: loadUiPreferences().theme,
  };
}

export class SettingsService {
  loadSettings(): AppSettings {
    if (typeof window === 'undefined') return defaultAppSettings;

    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (!raw) return {...defaultAppSettings,theme:loadUiPreferences().theme};
      return mergeSettings(JSON.parse(raw));
    } catch {
      return {...defaultAppSettings,theme:loadUiPreferences().theme};
    }
  }

  saveSettings(settings: AppSettings): void {
    if (typeof window === 'undefined') return;
    // Preserve an existing legacy key until native vault migration acknowledges success.
    let legacy = '';
    try { legacy = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? '{}').openRouter?.apiKey ?? ''; } catch { /* no previous setting */ }
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...settings,theme:loadUiPreferences().theme,
      openRouter: { ...settings.openRouter, apiKey: settings.openRouter.credentialRef ? '' : legacy },
      fal: { ...settings.fal, apiKey: '' },
    }));
  }

  checkOpenRouterKey(apiKey: string): ApiKeyCheckResult {
    const trimmed = apiKey.trim();

    if (!trimmed) {
      return { ok: false, message: messages().settings.apiKey.checks.empty, checkedAt: new Date().toISOString() };
    }

    if (!trimmed.startsWith('sk-or-')) {
      return { ok: false, message: messages().settings.apiKey.checks.unusual, checkedAt: new Date().toISOString() };
    }

    if (trimmed.length < 24) {
      return { ok: false, message: messages().settings.apiKey.checks.tooShort, checkedAt: new Date().toISOString() };
    }

    return { ok: true, message: localeText("settings", "ui.SettingsService.830340e1d93ed549"), checkedAt: new Date().toISOString() };
  }
}

export const settingsService = new SettingsService();
