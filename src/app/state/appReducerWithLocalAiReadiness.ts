import type { AppState } from '../types';
import type { AppAction } from './appActions';
import { appReducer } from './appReducer';
import {
  reduceLocalAiOperationFinished,
  reduceLocalAiOperationProgress,
  reduceLocalAiOperationStarted,
} from './localAiReadinessReducer';

export function appReducerWithLocalAiReadiness(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case 'localAi/autoCheckStarted':
    case 'localAi/manualPrepareStarted':
    case 'localAi/deleteStarted': {
      return reduceLocalAiOperationStarted(state, action.operation, action.status);
    }

    case 'localAi/operationProgress': {
      return reduceLocalAiOperationProgress(state, action.operationId, action.key, action.status);
    }

    case 'localAi/operationSucceeded':
    case 'localAi/operationFailed':
    case 'localAi/deleteSucceeded': {
      return reduceLocalAiOperationFinished(state, action.operationId, action.key, action.status);
    }

    default: {
      return appReducer(state, action);
    }
  }
}
