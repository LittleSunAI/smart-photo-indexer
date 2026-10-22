export type LocalRuntimeStatus =
  | 'unknown'
  | 'notFound'
  | 'notRunning'
  | 'installing'
  | 'starting'
  | 'running'
  | 'error';

export type LocalModelStatus =
  | 'unknown'
  | 'missing'
  | 'ready'
  | 'pulling'
  | 'verifying'
  | 'error';

export interface LocalAiStatus {
  runtimeStatus: LocalRuntimeStatus;
  modelStatus: LocalModelStatus;
  endpoint: string;
  selectedModel: string;
  availableModels: string[];
  ollamaVersion: string | null;
  progressPercent: number | null;
  statusText: string;
  errorMessage: string | null;
  checkedAt: string;
}
