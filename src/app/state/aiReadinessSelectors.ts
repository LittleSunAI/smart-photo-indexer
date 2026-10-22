import type { AppState, LocalAiReadinessOperation } from '../types';
import type { LocalAiStatus } from '../../modules/localAi/LocalAiStatus';
import type { LocalModelKey } from '../../modules/localAi/LocalAiKeys';
import { makeLocalModelKey, makeLocalRuntimeKey, normalizeLocalAiEndpoint } from '../../modules/localAi/LocalAiKeys';
import type { OpenRouterCheckResult } from '../../modules/openRouter/OpenRouterCheckService';

export function selectCurrentLocalModelKey(state: AppState): LocalModelKey | null {
  if (state.settings.aiProviderMode !== 'local') return null;

  const endpoint = state.settings.localAi.endpoint;
  const model = state.settings.localAi.selectedModel.trim();
  if (!model) return null;

  return makeLocalModelKey(endpoint, model);
}

export function selectCurrentLocalAiOperation(state: AppState): LocalAiReadinessOperation | null {
  const key = selectCurrentLocalModelKey(state);
  if (!key) return null;

  const operation = state.localAiReadiness.activeOperation;
  return operation?.key === key ? operation : null;
}

function isLegacyStatusForSelectedLocalModel(state: AppState, status: LocalAiStatus | null): status is LocalAiStatus {
  if (!status || state.settings.aiProviderMode !== 'local') return false;

  const legacyEndpoint = normalizeLocalAiEndpoint(status.endpoint);
  const selectedEndpoint = normalizeLocalAiEndpoint(state.settings.localAi.endpoint);
  const selectedModel = state.settings.localAi.selectedModel.trim();

  return status.selectedModel === selectedModel && legacyEndpoint === selectedEndpoint;
}

export function selectCurrentLocalAiStatus(state: AppState): LocalAiStatus | null {
  const key = selectCurrentLocalModelKey(state);
  if (!key) return null;

  const runtimeKey = makeLocalRuntimeKey(state.settings.localAi.endpoint);
  const model = state.localAiReadiness.modelByKey[key];
  const runtime = state.localAiReadiness.runtimeByEndpoint[runtimeKey];
  const matchingLegacyStatus = isLegacyStatusForSelectedLocalModel(state, state.localAiStatus) ? state.localAiStatus : null;

  if (model) {
    return {
      runtimeStatus: runtime?.runtimeStatus ?? matchingLegacyStatus?.runtimeStatus ?? 'unknown',
      modelStatus: model.modelStatus,
      endpoint: model.endpoint,
      selectedModel: model.model,
      availableModels: model.availableModels,
      ollamaVersion: runtime?.ollamaVersion ?? matchingLegacyStatus?.ollamaVersion ?? null,
      progressPercent: model.progressPercent,
      statusText: model.statusText,
      errorMessage: model.errorMessage ?? runtime?.errorMessage ?? null,
      checkedAt: model.checkedAt ?? runtime?.checkedAt ?? new Date(0).toISOString(),
    };
  }

  return matchingLegacyStatus;
}

export function selectIsCurrentLocalAiBusy(state: AppState): boolean {
  const status = selectCurrentLocalAiStatus(state);
  const operation = selectCurrentLocalAiOperation(state);

  return Boolean(operation)
    || status?.runtimeStatus === 'installing'
    || status?.runtimeStatus === 'starting'
    || status?.modelStatus === 'pulling'
    || status?.modelStatus === 'verifying';
}

export function selectCurrentOpenRouterStatus(state: AppState): OpenRouterCheckResult | null {
  if (state.settings.aiProviderMode !== 'openrouter') return null;

  return state.openRouterStatus?.model === state.settings.openRouter.selectedModel
    ? state.openRouterStatus
    : null;
}

export function selectIsCurrentOpenRouterBusy(state: AppState): boolean {
  return selectCurrentOpenRouterStatus(state)?.status === 'checking';
}
