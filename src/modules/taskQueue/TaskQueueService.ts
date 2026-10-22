import {displayNumber} from '../../shared/i18n/Formatting';
import {text as localeText} from "../../shared/i18n/Runtime";
import type { QueueTask, TaskQueueCounters, TaskQueueRunConfig, TaskQueueState } from './types';
import type { ImageItem } from '../explorer/types';
import catalog from '../../shared/ai/model-catalog.json';
import {loadUiPreferences} from '../../shared/UiPreferences';

export const defaultTaskQueueConfig: TaskQueueRunConfig = {
  providerMode: 'openrouter',
  model: 'openai/gpt-6-luna',
  maxImagesPerRun: 50,
  requestDelayMs: 700,
  maxEstimatedCostUsd: 0,
  queueContractVersion: 2,
  onlyUnprocessed: true,
  estimatedCostPerImageUsd: 0,
  maxRetriesPerTask: 2,
  stopOnFirstError: false,
  smartRenameEnabled: false,
  renameStopOnFirstError: true,
};

export function createEmptyTaskQueueState(): TaskQueueState {
  return {
    tasks: [],
    hideFinished:Boolean(loadUiPreferences().queueHideFinished),
    runState: 'idle',
    activeTaskId: null,
    preparedAt: null,
    startedAt: null,
    finishedAt: null,
    lastMessage: localeText("main", "ui.TaskQueueService.58e311f478287596"),
    lastErrorMessage: null,
    lastErrorCode: null,
    consecutiveErrorCount: 0,
    estimatedRunCostUsd: 0,
    skippedByLimitCount: 0,
    skippedByCostCount: 0,
    skippedNonImageCount: 0,
    config: defaultTaskQueueConfig,
  };
}

export function createAiAnalysisTasks(images: ImageItem[], config: TaskQueueRunConfig): QueueTask[] {
  const createdAt = new Date().toISOString();
  const maxAttempts = Math.max(1, Math.round(config.maxRetriesPerTask) + 1);
  const estimatedCostUsd = Math.max(0, config.estimatedCostPerImageUsd);

  return images.map((image, index) => ({
    id: `ai-${Date.now()}-${index}-${safeTaskPart(image.id)}`,
    type: 'aiAnalysis',
    sourceItemId: image.id,
    sourceSnapshot: { ...image, tags: [...image.tags], collections: image.collections ? [...image.collections] : undefined },
    targetPath: image.path,
    fileName: image.name,
    status: 'pending',
    progress: 0,
    createdAt,
    attemptCount: 0,
    maxAttempts,
    estimatedCostUsd,
    providerMode: config.providerMode,
    model: config.model,
    smartRenameEnabled: config.smartRenameEnabled,
  }));
}

export function summarizeTaskQueue(queue: TaskQueueState): TaskQueueCounters {
  if (queue.nativeCounters) return queue.nativeCounters;
  const total = queue.tasks.length;
  const pending = queue.tasks.filter((task) => task.status === 'pending').length;
  const running = queue.tasks.filter((task) => task.status === 'running').length;
  const done = queue.tasks.filter((task) => task.status === 'done').length;
  const failed = queue.tasks.filter((task) => task.status === 'failed').length;
  const cancelled = queue.tasks.filter((task) => task.status === 'cancelled').length;
  const skipped = queue.tasks.filter((task) => task.status === 'skipped').length;
  const retryWaiting = queue.tasks.filter((task) => task.status === 'pending' && isWaitingForRetry(task)).length;
  const finished = done + failed + cancelled + skipped;
  const progressPercent = total > 0 ? Math.round((finished / total) * 100) : 0;

  return { total, pending, running, done, failed, cancelled, skipped, finished, retryWaiting, progressPercent };
}
export function visibleQueueTasks(tasks:QueueTask[],hideDone:boolean):QueueTask[]{return hideDone?tasks.filter(task=>task.status!=='done'):tasks;}

export function getNextRunnableTask(queue: TaskQueueState, nowMs: number = Date.now()): QueueTask | null {
  return queue.tasks.find((task) => task.status === 'pending' && getTaskRetryReadyAtMs(task) <= nowMs) ?? null;
}

export function getNextPendingRetryAtMs(queue: TaskQueueState, nowMs: number = Date.now()): number | null {
  const retryTimes = queue.tasks
    .filter((task) => task.status === 'pending')
    .map(getTaskRetryReadyAtMs)
    .filter((value) => value > nowMs)
    .sort((a, b) => a - b);

  return retryTimes[0] ?? null;
}

export function estimateQueueCost(tasks: QueueTask[]): number {
  return tasks.reduce((sum, task) => sum + Math.max(0, task.estimatedCostUsd), 0);
}

export function estimateOpenRouterCostPerImageUsd(model: string, providerMode: string, maxTokens=3200, passes=1): number {
  if (providerMode === 'mock' || providerMode === 'local') return 0;
  if (!model || model === 'openrouter/free' || model.endsWith(':free')) return 0;

  const models=providerMode==='fal'?catalog.fal:catalog.openrouter;
  const price=models.find(row=>row.id===model);
  if(!price)return 0;
  // Match the native reservation envelope per pass, including recheck candidates.
  const input=16000+9000*(Math.max(1,passes)-1);
  return (input*price.inputUsdPerMillion+maxTokens*price.outputUsdPerMillion)/1_000_000;
}

export function formatCostUsd(value:number):string {
  return `$${displayNumber(Math.max(0,value),{minimumFractionDigits:4,maximumFractionDigits:6})}`;
}

function isWaitingForRetry(task: QueueTask): boolean {
  return task.status === 'pending' && getTaskRetryReadyAtMs(task) > Date.now();
}

function getTaskRetryReadyAtMs(task: QueueTask): number {
  if (!task.nextRetryAt) return 0;
  const parsed = Date.parse(task.nextRetryAt);
  return Number.isFinite(parsed) ? parsed : 0;
}

function safeTaskPart(value: string): string {
  return value.replace(/[^a-z0-9_-]+/gi, '_').slice(0, 48) || 'image';
}
