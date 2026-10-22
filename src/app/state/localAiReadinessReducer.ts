import type { AppState, LocalAiReadinessOperation, LocalModelReadinessState, LocalRuntimeReadinessState } from '../types';
import type { LocalAiStatus } from '../../modules/localAi/LocalAiStatus';
import type { LocalAiOperationId, LocalModelKey } from '../../modules/localAi/LocalAiKeys';
import { makeLocalRuntimeKey } from '../../modules/localAi/LocalAiKeys';

function runtimeFromStatus(operationId: LocalAiOperationId, status: LocalAiStatus): LocalRuntimeReadinessState {
  return {
    operationId,
    endpoint: status.endpoint,
    runtimeStatus: status.runtimeStatus,
    ollamaVersion: status.ollamaVersion,
    errorMessage: status.runtimeStatus === 'error' ? status.errorMessage : null,
    checkedAt: status.checkedAt,
  };
}

function modelFromStatus(operationId: LocalAiOperationId, key: LocalModelKey, status: LocalAiStatus): LocalModelReadinessState {
  return {
    operationId,
    key,
    endpoint: status.endpoint,
    model: status.selectedModel,
    modelStatus: status.modelStatus,
    availableModels: status.availableModels,
    progressPercent: status.progressPercent,
    statusText: status.statusText,
    errorMessage: status.errorMessage,
    checkedAt: status.checkedAt,
  };
}

function shouldAcceptStatus(state: AppState, operationId: LocalAiOperationId, key: LocalModelKey): boolean {
  const activeOperation = state.localAiReadiness.activeOperation;
  const existingModelState = state.localAiReadiness.modelByKey[key];

  if (activeOperation?.key === key) {
    return activeOperation.operationId === operationId;
  }

  if (existingModelState?.operationId) {
    return existingModelState.operationId === operationId;
  }

  return true;
}

function applyLocalAiStatus(
  state: AppState,
  operationId: LocalAiOperationId,
  key: LocalModelKey,
  status: LocalAiStatus,
  finishOperation: boolean,
): AppState {
  if (!shouldAcceptStatus(state, operationId, key)) {
    return state;
  }

  const runtimeKey = makeLocalRuntimeKey(status.endpoint);
  const nextActiveOperation = finishOperation && state.localAiReadiness.activeOperation?.operationId === operationId
    ? null
    : state.localAiReadiness.activeOperation;
  const isVisibleOperation = state.localAiReadiness.activeOperation?.operationId === operationId;

  return {
    ...state,
    localAiStatus: isVisibleOperation ? status : state.localAiStatus,
    localAiReadiness: {
      ...state.localAiReadiness,
      activeOperation: nextActiveOperation,
      runtimeByEndpoint: {
        ...state.localAiReadiness.runtimeByEndpoint,
        [runtimeKey]: runtimeFromStatus(operationId, status),
      },
      modelByKey: {
        ...state.localAiReadiness.modelByKey,
        [key]: modelFromStatus(operationId, key, status),
      },
    },
  };
}

export function reduceLocalAiOperationStarted(
  state: AppState,
  operation: LocalAiReadinessOperation,
  status: LocalAiStatus,
): AppState {
  const runtimeKey = makeLocalRuntimeKey(status.endpoint);

  return {
    ...state,
    localAiStatus: status,
    localAiReadiness: {
      ...state.localAiReadiness,
      activeOperation: operation,
      runtimeByEndpoint: {
        ...state.localAiReadiness.runtimeByEndpoint,
        [runtimeKey]: runtimeFromStatus(operation.operationId, status),
      },
      modelByKey: {
        ...state.localAiReadiness.modelByKey,
        [operation.key]: modelFromStatus(operation.operationId, operation.key, status),
      },
    },
  };
}

export function reduceLocalAiOperationProgress(
  state: AppState,
  operationId: LocalAiOperationId,
  key: LocalModelKey,
  status: LocalAiStatus,
): AppState {
  return applyLocalAiStatus(state, operationId, key, status, false);
}

export function reduceLocalAiOperationFinished(
  state: AppState,
  operationId: LocalAiOperationId,
  key: LocalModelKey,
  status: LocalAiStatus,
): AppState {
  return applyLocalAiStatus(state, operationId, key, status, true);
}
