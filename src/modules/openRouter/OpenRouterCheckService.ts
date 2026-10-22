import {text as localeText} from "../../shared/i18n/Runtime";
import {isProviderAvailable,providerUnavailableMessage} from '../settings/ProviderAvailability';
import { appLogService } from '../logs/AppLogService';

export type OpenRouterCheckStatus = 'unknown' | 'checking' | 'ready' | 'missingKey' | 'invalidKey' | 'error';

export interface OpenRouterCheckResult {
  status: OpenRouterCheckStatus;
  ok: boolean;
  message: string;
  model: string;
  keyLabel: string | null;
  limitRemaining: number | null;
  limit: number | null;
  usage: number | null;
  checkedAt: string;
  rawDetails: string | null;
}

interface OpenRouterKeyResponse {
  data?: {
    label?: unknown;
    limit?: unknown;
    limit_remaining?: unknown;
    usage?: unknown;
    is_free_tier?: unknown;
  };
  error?: {
    message?: unknown;
  };
}

interface OpenRouterModelResponse {
  data?: {
    id?: unknown;
    name?: unknown;
    architecture?: {
      input_modalities?: unknown;
      output_modalities?: unknown;
      modality?: unknown;
    };
  };
  error?: {
    message?: unknown;
  };
}

const OPENROUTER_BASE_URL = 'https://openrouter.ai/api/v1';
const CHECK_TIMEOUT_MS = 15_000;

function nowIso(): string {
  return new Date().toISOString();
}

function toNumberOrNull(value: unknown): number | null {
  const numeric = typeof value === 'number' ? value : typeof value === 'string' ? Number(value) : NaN;
  return Number.isFinite(numeric) ? numeric : null;
}

function toStringOrNull(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function createResult(input: Partial<OpenRouterCheckResult> & Pick<OpenRouterCheckResult, 'status' | 'ok' | 'message' | 'model'>): OpenRouterCheckResult {
  return {
    keyLabel: null,
    limitRemaining: null,
    limit: null,
    usage: null,
    rawDetails: null,
    checkedAt: nowIso(),
    ...input,
  };
}

async function fetchWithTimeout(url: string, apiKey: string): Promise<Response> {
  const controller = new AbortController();
  const timeoutId = window.setTimeout(() => controller.abort(), CHECK_TIMEOUT_MS);

  try {
    return await fetch(url, {
      method: 'GET',
      signal: controller.signal,
      headers: {
        'Authorization': `Bearer ${apiKey}`,
        'Content-Type': 'application/json',
      },
    });
  } finally {
    window.clearTimeout(timeoutId);
  }
}

function shouldCheckModel(model: string): boolean {
  return Boolean(model.trim()) && model !== 'openrouter/free';
}

function buildModelCheckUrl(model: string): string {
  return `${OPENROUTER_BASE_URL}/model/${model.trim()}`;
}

function modelLooksVisionCapable(payload: OpenRouterModelResponse): boolean {
  const inputModalities = payload.data?.architecture?.input_modalities;
  if (Array.isArray(inputModalities)) {
    return inputModalities.includes('image');
  }

  const modality = toStringOrNull(payload.data?.architecture?.modality)?.toLowerCase() ?? '';
  return modality.includes('image');
}

class OpenRouterCheckService {
  createUnknown(model: string): OpenRouterCheckResult {
    return createResult({
      status: 'unknown',
      ok: false,
      model,
      message: localeText("main", "ui.OpenRouterCheckService.2a27fe1217ef76f9"),
    });
  }

  createChecking(model: string): OpenRouterCheckResult {
    return createResult({
      status: 'checking',
      ok: false,
      model,
      message: localeText("main", "ui.OpenRouterCheckService.89e154ec85562d73"),
    });
  }

  async check(apiKey: string, model: string): Promise<OpenRouterCheckResult> {
    if(!isProviderAvailable('openrouter'))return createResult({status:'error',ok:false,model,message:providerUnavailableMessage()});
    const cleanKey = apiKey.trim();
    const cleanModel = model.trim() || 'openrouter/free';

    if (!cleanKey) {
      const result = createResult({
        status: 'missingKey',
        ok: false,
        model: cleanModel,
        message: localeText("main", "ui.OpenRouterCheckService.06a7259737796f8f"),
      });
      appLogService.warning('network', result.message, { model: cleanModel });
      return result;
    }

    if (!cleanKey.startsWith('sk-or-')) {
      const result = createResult({
        status: 'invalidKey',
        ok: false,
        model: cleanModel,
        message: localeText("main", "ui.OpenRouterCheckService.cbccc279459fc1d0"),
      });
      appLogService.warning('network', result.message, { model: cleanModel });
      return result;
    }

    try {
      appLogService.info('network', localeText("main", "ui.OpenRouterCheckService.e7279c191f074d04"), { model: cleanModel });
      const keyResponse = await fetchWithTimeout(`${OPENROUTER_BASE_URL}/key`, cleanKey);
      const keyText = await keyResponse.text();
      const keyPayload = JSON.parse(keyText || '{}') as OpenRouterKeyResponse;

      if (!keyResponse.ok) {
        const providerMessage = toStringOrNull(keyPayload.error?.message) ?? keyText ?? `HTTP ${keyResponse.status}`;
        const result = createResult({
          status: keyResponse.status === 401 || keyResponse.status === 403 ? 'invalidKey' : 'error',
          ok: false,
          model: cleanModel,
          message: keyResponse.status === 401 || keyResponse.status === 403
            ? localeText("main", "ui.OpenRouterCheckService.5d554e236c6e1359")
            : localeText("main", "ui.OpenRouterCheckService.1653918ffec8f177", {"value1": (keyResponse.status)}),
          rawDetails: providerMessage,
        });
        appLogService.error('network', result.message, result);
        return result;
      }

      let modelDetails: OpenRouterModelResponse | null = null;
      if (shouldCheckModel(cleanModel)) {
        const modelResponse = await fetchWithTimeout(buildModelCheckUrl(cleanModel), cleanKey);
        const modelText = await modelResponse.text();
        modelDetails = JSON.parse(modelText || '{}') as OpenRouterModelResponse;

        if (!modelResponse.ok) {
          const result = createResult({
            status: 'error',
            ok: false,
            model: cleanModel,
            keyLabel: toStringOrNull(keyPayload.data?.label),
            limitRemaining: toNumberOrNull(keyPayload.data?.limit_remaining),
            limit: toNumberOrNull(keyPayload.data?.limit),
            usage: toNumberOrNull(keyPayload.data?.usage),
            message: localeText("main", "ui.OpenRouterCheckService.9a89a4215821c530", {"value1": (cleanModel)}),
            rawDetails: modelText || localeText("main", "ui.OpenRouterCheckService.7dd9a7e43c513d40", {"value1": (modelResponse.status)}),
          });
          appLogService.error('network', result.message, result);
          return result;
        }
      }

      const limitRemaining = toNumberOrNull(keyPayload.data?.limit_remaining);
      const limit = toNumberOrNull(keyPayload.data?.limit);
      const usage = toNumberOrNull(keyPayload.data?.usage);
      const visionNote = modelDetails && !modelLooksVisionCapable(modelDetails)
        ? localeText("main", "ui.OpenRouterCheckService.848d143018bad2cb", {"value1": (cleanModel)})
        : '';
      const limitText = limitRemaining !== null ? localeText("main", "ui.OpenRouterCheckService.a54ce4af589bba58", {"value1": (limitRemaining.toFixed(2))}) : '';
      const result = createResult({
        status: 'ready',
        ok: true,
        model: cleanModel,
        keyLabel: toStringOrNull(keyPayload.data?.label),
        limitRemaining,
        limit,
        usage,
        message: localeText("main", "ui.OpenRouterCheckService.fb2c4cefb6bade45", {"value1": (cleanModel), "value2": (limitText), "value3": (visionNote)}),
        rawDetails: modelDetails ? JSON.stringify(modelDetails.data ?? modelDetails, null, 2) : null,
      });
      appLogService.info('network', result.message, result);
      return result;
    } catch (error) {
      const result = createResult({
        status: 'error',
        ok: false,
        model: cleanModel,
        message: error instanceof DOMException && error.name === 'AbortError'
          ? localeText("main", "ui.OpenRouterCheckService.40afa3a862c4985d")
          : localeText("main", "ui.OpenRouterCheckService.ed7583ef67a7a8e9"),
        rawDetails: error instanceof Error ? `${error.name}: ${error.message}\n${error.stack ?? ''}` : String(error),
      });
      appLogService.error('network', result.message, result);
      return result;
    }
  }
}

export const openRouterCheckService = new OpenRouterCheckService();
