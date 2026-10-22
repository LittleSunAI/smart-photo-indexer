import type { AiProviderErrorCode } from '../ai/AiProviderError';
import type { ImageItem } from '../explorer/types';

export interface TaskSummary {
  totalItems: number;
  processed: number;
  inQueue: number;
  notProcessed: number;
  errors: number;
  currentTask: string;
  overallProgress: number;
}

export type QueueRunState = 'idle' | 'running' | 'paused' | 'stopped' | 'completed';
export type QueueTaskStatus = 'pending' | 'running' | 'done' | 'failed' | 'cancelled' | 'skipped';
export type QueueTaskType = 'scan' | 'thumbnail' | 'aiAnalysis' | 'rename' | 'relocateCheck';

export interface QueueTask {
  indexedImageId?:string;
  pendingNativeIdentity?: boolean;
  nativeStage?: string;
  nativeOrdinal?: number;
  currentStartedAtUnix?:number|null;
  completedPasses?: number;
  sourceSnapshot?: ImageItem;
  renamePlanId?: string | null;
  id: string;
  type: QueueTaskType;
  sourceItemId: string | null;
  targetPath: string;
  fileName: string;
  status: QueueTaskStatus;
  progress: number;
  createdAt: string;
  startedAt?: string;
  finishedAt?: string;
  errorMessage?: string;
  errorCode?: AiProviderErrorCode | string;
  retryable?: boolean;
  attemptCount: number;
  maxAttempts: number;
  nextRetryAt?: string;
  analysisDescription?: string;
  analysisTags?: string[];
  analysisConfidence?: number;
  analysisSource?: string;
  renameWasAttempted?: boolean;
  renameWasRenamed?: boolean;
  renameOldPath?: string;
  renameNewPath?: string;
  renameOldFileName?: string;
  renameNewFileName?: string;
  providerMode?: string;
  model?: string;
  estimatedCostUsd: number;
  smartRenameEnabled?: boolean;
}

export interface TaskQueueRunConfig {
  queueMode?:'direct'|'manual';
  queueContractVersion?:number;
  onlyUnprocessed?:boolean;
  sourceRoots?:string[];
  sourceQueryId?:string;
  excludedSourcePaths?:string[];
  recursiveProcessing?:boolean;
  analysisCopy?:{enabled:boolean;destination:"beside"|"folder";folder:string};
  outputMode?: import('../settings/Settings.types').AnalysisOutputMode;
  autoStart?: boolean;
  credentialRef?: string | null;
  endpoint?: string;
  passes?: number;
  concurrency?: number;
  effectiveConcurrency?: number;
  occupiedRequests?: number;
  concurrencyPolicy?: string;
  requestedConcurrency?: number;
  maxTokens?: number;
  imageMaxSide?: number;
  providerMode: string;
  model: string;
  maxImagesPerRun: number;
  requestDelayMs: number;
  maxEstimatedCostUsd: number;
  estimatedCostPerImageUsd: number;
  maxRetriesPerTask: number;
  stopOnFirstError: boolean;
  smartRenameEnabled: boolean;
  renameStopOnFirstError: boolean;
}

export interface TaskQueueState {
  pendingPreparation?:{key:string;tasks:QueueTask[];config:TaskQueueRunConfig;targetJobId?:string|null;targetVersion?:number};
  preparationResult?:import('./NativeQueueService').PreparedQueue;
  estimatedPendingUsd?:number;
  preparationKey?: string;
  preparationState?: 'preparing' | 'starting' | 'cancelled' | 'failed';
  commandPending?: boolean;
  hideFinished?: boolean;
  priorStatuses?:Record<string,import('../explorer/types').ProcessingStatus>;
  nativeJobId?: string;
  nativeStatus?: string;
  nativeRevision?: number;
  nativeSnapshotPending?: boolean;
  nativeCounters?: TaskQueueCounters;
  nativeWorkingJobs?:import('./NativeQueueService').NativeJob[];
  nativeAggregate?:import('./NativeQueueService').QueueAggregate;
  nativeJobs?: import('./NativeQueueService').NativeJob[];
  reservedCostUsd?: number;
  unknownCosts?: number;
  unresolvedRequests?: number;
  tasks: QueueTask[];
  runState: QueueRunState;
  activeTaskId: string | null;
  preparedAt: string | null;
  startedAt: string | null;
  finishedAt: string | null;
  lastMessage: string;
  lastErrorMessage: string | null;
  lastErrorCode: string | null;
  consecutiveErrorCount: number;
  estimatedRunCostUsd: number;
  skippedByLimitCount: number;
  skippedByCostCount: number;
  skippedNonImageCount: number;
  config: TaskQueueRunConfig;
}

export interface TaskQueueCounters {
  total: number;
  pending: number;
  running: number;
  done: number;
  failed: number;
  cancelled: number;
  skipped: number;
  finished: number;
  retryWaiting: number;
  progressPercent: number;
}
