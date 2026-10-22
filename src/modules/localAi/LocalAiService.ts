import {text as localeText} from "../../shared/i18n/Runtime";
import {invoke} from "../../shared/tauri/invoke";
import { isTauriRuntime } from '../../shared/tauri/runtime';
import { appLogService } from '../logs/AppLogService';
import type { LocalAiStatus, LocalModelStatus, LocalRuntimeStatus } from './LocalAiStatus';

interface OllamaVersionResponse {
  version?: unknown;
}

interface OllamaTagEntry {
  name?: unknown;
  model?: unknown;
}

interface OllamaTagsResponse {
  models?: unknown;
}

interface OllamaPullProgressChunk {
  status?: unknown;
  error?: unknown;
  total?: unknown;
  completed?: unknown;
}

interface OllamaChatResponse {
  message?: {
    content?: unknown;
  };
  error?: unknown;
}

interface LocalRuntimePrepareResultDto {
  runtimeStatus: 'running' | 'notRunning' | 'notFound' | 'error';
  statusText: string;
  errorMessage: string | null;
  executablePath: string | null;
  installWasAttempted: boolean;
  startWasAttempted: boolean;
}

const DEFAULT_LOCAL_ENDPOINT = 'http://127.0.0.1:11434';
const REQUEST_TIMEOUT_MS = 7_000;

function asNonEmptyString(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function getNowIso(): string {
  return new Date().toISOString();
}

function createStatus(input: Partial<LocalAiStatus> & Pick<LocalAiStatus, 'endpoint' | 'selectedModel' | 'runtimeStatus' | 'modelStatus' | 'statusText'>): LocalAiStatus {
  return {
    availableModels: [],
    ollamaVersion: null,
    progressPercent: null,
    errorMessage: null,
    checkedAt: getNowIso(),
    ...input,
  };
}

export function normalizeEndpoint(endpoint: string): string {
  const trimmed = endpoint.trim() || DEFAULT_LOCAL_ENDPOINT;

  try {
    const url = new URL(trimmed.includes('://') ? trimmed : `http://${trimmed}`);
    const isLocalHost = url.hostname === '127.0.0.1' || url.hostname === 'localhost';

    if (isLocalHost && !url.port) {
      url.port = '11434';
    }

    return url.toString().replace(/\/+$/, '');
  } catch {
    return DEFAULT_LOCAL_ENDPOINT;
  }
}

export function createLocalCheckingStatus(model: string, endpoint: string, previous: LocalAiStatus | null): LocalAiStatus {
  const cleanEndpoint = normalizeEndpoint(endpoint);
  const cleanModel = model.trim();
  const canReusePrevious = previous !== null
    && normalizeEndpoint(previous.endpoint) === cleanEndpoint
    && previous.selectedModel.trim() === cleanModel;
  const reusablePrevious = canReusePrevious ? previous : null;

  return createStatus({
    runtimeStatus: reusablePrevious?.runtimeStatus ?? 'unknown',
    modelStatus: 'verifying',
    endpoint: cleanEndpoint,
    selectedModel: cleanModel,
    availableModels: reusablePrevious?.availableModels ?? [],
    ollamaVersion: reusablePrevious?.ollamaVersion ?? null,
    progressPercent: null,
    statusText: localeText("settings", "ui.LocalAiService.a5997a113a440754", {"value1": (cleanModel)}),
    errorMessage: null,
  });
}

function createBlockedEndpointStatus(endpoint: string, selectedModel: string): LocalAiStatus {
  appLogService.warning('local-ai', localeText("settings", "ui.LocalAiService.c8daf575f1e93262"), { endpoint, selectedModel });
  return createStatus({
    runtimeStatus: 'error',
    modelStatus: 'error',
    endpoint,
    selectedModel,
    statusText: localeText("settings", "ui.LocalAiService.b0f47a7ab51abe2b"),
    errorMessage: localeText("settings", "ui.LocalAiService.73436647c46fac80"),
  });
}

function buildStatusText(runtimeStatus: LocalRuntimeStatus, modelStatus: LocalModelStatus, selectedModel: string): string {
  if (runtimeStatus === 'running' && modelStatus === 'ready') {
    return selectedModel ? localeText("settings", "ui.LocalAiService.4edcf84de913e01b", {"value1": (selectedModel)}) : localeText("settings", "ui.LocalAiService.4f0aad27d17a0356");
  }

  if (runtimeStatus === 'running') {
    return selectedModel ? localeText("settings", "ui.LocalAiService.a0386205fdfcc32d", {"value1": (selectedModel)}) : localeText("settings", "ui.LocalAiService.f140c6624d3c44ae");
  }

  if (runtimeStatus === 'notFound') return localeText("settings", "ui.LocalAiService.13b5e9ef27d4fdc2");
  if (runtimeStatus === 'notRunning') return localeText("settings", "ui.LocalAiService.5e1d1582d3978aa0");
  if (runtimeStatus === 'installing') return localeText("settings", "ui.LocalAiService.72f654d38b72fbea");
  if (runtimeStatus === 'starting') return localeText("settings", "ui.LocalAiService.614fa787864f5075");
  if (runtimeStatus === 'error') return localeText("settings", "ui.LocalAiService.3975db28809ae34d");
  return localeText("settings", "ui.LocalAiService.c9f8a2867e3c74b0");
}

async function fetchJson(path: string, endpoint: string): Promise<unknown> {
  const cleanEndpoint = normalizeEndpoint(endpoint);
  if (!isAllowedLocalEndpoint(cleanEndpoint)) {
    throw new Error(localeText("settings", "ui.LocalAiService.5526d65eccb22f32"));
  }

  const controller = new AbortController();
  const timeoutId = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

  try {
    appLogService.debug('network', `GET ${path}`, { endpoint: cleanEndpoint });
    const response = await fetch(`${cleanEndpoint}${path}`, {
      method: 'GET',
      signal: controller.signal,
    });

    if (!response.ok) {
      throw new Error(localeText("settings", "ui.LocalAiService.2059e797b8af4331", {"value1": (response.status)}));
    }

    return response.json();
  } catch (error) {
    appLogService.error('network', localeText("settings", "ui.LocalAiService.094bd6496b702d03", {"value1": (path)}), error);
    throw error;
  } finally {
    window.clearTimeout(timeoutId);
  }
}

function parseAvailableModels(payload: unknown): string[] {
  if (!payload || typeof payload !== 'object') return [];

  const models = (payload as OllamaTagsResponse).models;
  if (!Array.isArray(models)) return [];

  const names = models
    .map((entry: unknown) => {
      if (!entry || typeof entry !== 'object') return null;
      return asNonEmptyString((entry as OllamaTagEntry).name) ?? asNonEmptyString((entry as OllamaTagEntry).model);
    })
    .filter((value: string | null): value is string => Boolean(value));

  return Array.from(new Set(names));
}

function createPullChunkProcessor(
  endpoint: string,
  selectedModel: string,
  onProgress: (status: LocalAiStatus) => void,
): (line: string) => void {
  let lastProgress = 0;
  let lastStatusText = localeText("settings", "ui.LocalAiService.f58d61dde1d6a56e", {"value1": (selectedModel)});

  return (line: string) => {
    const trimmed = line.trim();
    if (!trimmed) return;

    const chunk = JSON.parse(trimmed) as OllamaPullProgressChunk;
    const chunkError = asNonEmptyString(chunk.error);
    if (chunkError) {
      appLogService.error('local-ai', localeText("settings", "ui.LocalAiService.5ba1989572fd1ca3", {"value1": (selectedModel)}), chunkError);
      throw new Error(chunkError);
    }

    const total = typeof chunk.total === 'number' ? chunk.total : 0;
    const completed = typeof chunk.completed === 'number' ? chunk.completed : 0;
    const statusText = asNonEmptyString(chunk.status) ?? lastStatusText;
    const progressPercent = total > 0 ? Math.max(0, Math.min(100, Math.round((completed / total) * 100))) : lastProgress;

    lastProgress = progressPercent;
    lastStatusText = statusText;

    const status = createStatus({
      runtimeStatus: 'running',
      modelStatus: 'pulling',
      endpoint,
      selectedModel,
      progressPercent,
      statusText,
    });
    appLogService.debug('local-ai', statusText, { selectedModel, progressPercent });
    onProgress(status);
  };
}

export function isAllowedLocalEndpoint(endpoint: string): boolean {
  const cleanEndpoint = normalizeEndpoint(endpoint);

  try {
    const url = new URL(cleanEndpoint);
    const isLocalHost = url.hostname === '127.0.0.1' || url.hostname === 'localhost';
    return url.protocol === 'http:' && isLocalHost && url.port === '11434';
  } catch {
    return false;
  }
}

export async function fetchLocalAiStatus(selectedModel: string, endpoint: string): Promise<LocalAiStatus> {
  const cleanEndpoint = normalizeEndpoint(endpoint);
  const cleanModel = selectedModel.trim();

  if (!isAllowedLocalEndpoint(cleanEndpoint)) {
    return createBlockedEndpointStatus(cleanEndpoint, cleanModel);
  }

  try {
    appLogService.info('local-ai', localeText("settings", "ui.LocalAiService.efb1654cde4c5f13"), { endpoint: cleanEndpoint, selectedModel: cleanModel });
    const versionPayload = await fetchJson('/api/version', cleanEndpoint) as OllamaVersionResponse;
    const tagsPayload = await fetchJson('/api/tags', cleanEndpoint);
    const availableModels = parseAvailableModels(tagsPayload);
    const modelStatus: LocalModelStatus = cleanModel ? (availableModels.includes(cleanModel) ? 'ready' : 'missing') : 'unknown';

    const status = createStatus({
      runtimeStatus: 'running',
      modelStatus,
      endpoint: cleanEndpoint,
      selectedModel: cleanModel,
      availableModels,
      ollamaVersion: asNonEmptyString(versionPayload?.version),
      statusText: buildStatusText('running', modelStatus, cleanModel),
    });
    appLogService.info('local-ai', status.statusText, status);
    return status;
  } catch (error) {
    const status = createStatus({
      runtimeStatus: 'notRunning',
      modelStatus: cleanModel ? 'missing' : 'unknown',
      endpoint: cleanEndpoint,
      selectedModel: cleanModel,
      statusText: buildStatusText('notRunning', cleanModel ? 'missing' : 'unknown', cleanModel),
      errorMessage: error instanceof Error ? error.message : String(error),
    });
    appLogService.warning('local-ai', status.statusText, status);
    return status;
  }
}

export async function pullLocalAiModel(
  selectedModel: string,
  endpoint: string,
  onProgress: (status: LocalAiStatus) => void,
): Promise<LocalAiStatus> {
  const cleanEndpoint = normalizeEndpoint(endpoint);
  const cleanModel = selectedModel.trim();

  if (!isAllowedLocalEndpoint(cleanEndpoint)) {
    return createBlockedEndpointStatus(cleanEndpoint, cleanModel);
  }

  if (!cleanModel) {
    return createStatus({
      runtimeStatus: 'running',
      modelStatus: 'error',
      endpoint: cleanEndpoint,
      selectedModel: cleanModel,
      statusText: localeText("settings", "ui.LocalAiService.8f70e1b69aeaeac2"),
      errorMessage: localeText("settings", "ui.LocalAiService.dd5527ce888c4c46"),
    });
  }

  appLogService.info('local-ai', localeText("settings", "ui.LocalAiService.a67fa38fad723e31", {"value1": (cleanModel)}), { endpoint: cleanEndpoint });
  const response = await fetch(`${cleanEndpoint}/api/pull`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ model: cleanModel, stream: true }),
  });

  if (!response.ok) {
    appLogService.error('local-ai', localeText("settings", "ui.LocalAiService.358fb066d1f89cf8", {"value1": (cleanModel)}), { status: response.status });
    throw new Error(localeText("settings", "ui.LocalAiService.c51c5e88161ee31b", {"value1": (response.status)}));
  }

  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error(localeText("settings", "ui.LocalAiService.1f44a9d438ef0b43"));
  }

  const processLine = createPullChunkProcessor(cleanEndpoint, cleanModel, onProgress);
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop() ?? '';

    for (const line of lines) {
      processLine(line);
    }
  }

  if (buffer.trim()) processLine(buffer);

  onProgress(createStatus({
    runtimeStatus: 'running',
    modelStatus: 'verifying',
    endpoint: cleanEndpoint,
    selectedModel: cleanModel,
    statusText: localeText("settings", "ui.LocalAiService.d6699e9792aa3f39", {"value1": (cleanModel)}),
  }));

  const after = await fetchLocalAiStatus(cleanModel, cleanEndpoint);
  if (after.modelStatus === 'ready') {
    const status = createStatus({ ...after, statusText: localeText("settings", "ui.LocalAiService.f0c12b8b49e3af13", {"value1": (cleanModel)}) });
    appLogService.info('local-ai', status.statusText, status);
    return status;
  }

  const errorStatus = createStatus({
    ...after,
    runtimeStatus: 'running',
    modelStatus: 'error',
    statusText: localeText("settings", "ui.LocalAiService.e6065f9fca8ff8b0", {"value1": (cleanModel)}),
    errorMessage: after.errorMessage ?? localeText("settings", "ui.LocalAiService.fd5619c381544f0f"),
  });
  appLogService.error('local-ai', errorStatus.statusText, errorStatus);
  return errorStatus;
}

export async function deleteLocalAiModel(selectedModel: string, endpoint: string): Promise<LocalAiStatus> {
  const cleanEndpoint = normalizeEndpoint(endpoint);
  const cleanModel = selectedModel.trim();

  if (!isAllowedLocalEndpoint(cleanEndpoint)) {
    return createBlockedEndpointStatus(cleanEndpoint, cleanModel);
  }

  if (!cleanModel) {
    return createStatus({
      runtimeStatus: 'running',
      modelStatus: 'error',
      endpoint: cleanEndpoint,
      selectedModel: cleanModel,
      statusText: localeText("settings", "ui.LocalAiService.48c8160e6a82c2b7"),
      errorMessage: localeText("settings", "ui.LocalAiService.578467307aa9d36a"),
    });
  }

  appLogService.info('local-ai', localeText("settings", "ui.LocalAiService.a2979afd8afa256a", {"value1": (cleanModel)}), { endpoint: cleanEndpoint });
  const response = await fetch(`${cleanEndpoint}/api/delete`, {
    method: 'DELETE',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ model: cleanModel }),
  });
  const rawBody = await response.text();

  if (response.status === 404) {
    const after = await fetchLocalAiStatus(cleanModel, cleanEndpoint);
    const status = createStatus({
      ...after,
      runtimeStatus: 'running',
      modelStatus: 'missing',
      selectedModel: cleanModel,
      statusText: localeText("settings", "ui.LocalAiService.12257ad7a6d86360", {"value1": (cleanModel)}),
      errorMessage: null,
    });
    appLogService.warning('local-ai', status.statusText, { endpoint: cleanEndpoint, rawBody });
    return status;
  }

  if (!response.ok) {
    appLogService.error('local-ai', localeText("settings", "ui.LocalAiService.c34d74224fbce9f8", {"value1": (cleanModel)}), {
      endpoint: cleanEndpoint,
      status: response.status,
      rawBody: rawBody.slice(0, 3000),
    });
    throw new Error(localeText("settings", "ui.LocalAiService.88567b074eb2a782", {"value1": (cleanModel), "value2": (response.status), "value3": (rawBody)}).trim());
  }

  const after = await fetchLocalAiStatus(cleanModel, cleanEndpoint);
  const status = createStatus({
    ...after,
    runtimeStatus: 'running',
    modelStatus: 'missing',
    selectedModel: cleanModel,
    statusText: localeText("settings", "ui.LocalAiService.40851fec5a593625", {"value1": (cleanModel)}),
    errorMessage: null,
  });
  appLogService.info('local-ai', status.statusText, status);
  return status;
}

export class LocalAiService {
  async checkStatus(selectedModel: string, endpoint: string): Promise<LocalAiStatus> {
    return fetchLocalAiStatus(selectedModel, endpoint);
  }

  async prepareRuntime(endpoint: string, model: string, onProgress: (status: LocalAiStatus) => void): Promise<LocalAiStatus> {
    const cleanEndpoint = normalizeEndpoint(endpoint);
    const selectedModel = model.trim();
    const before = await this.checkStatus(selectedModel, cleanEndpoint);

    if (before.runtimeStatus === 'running') return before;
    if (!isAllowedLocalEndpoint(cleanEndpoint)) return createBlockedEndpointStatus(cleanEndpoint, selectedModel);

    if (!isTauriRuntime()) {
      return createStatus({
        runtimeStatus: 'notFound',
        modelStatus: selectedModel ? 'missing' : 'unknown',
        endpoint: cleanEndpoint,
        selectedModel,
        statusText: localeText("settings", "ui.LocalAiService.f1e8dd97047065a2"),
        errorMessage: localeText("settings", "ui.LocalAiService.5c9c1e2c82204acf"),
      });
    }

    const preparingStatus = createStatus({
      runtimeStatus: 'installing',
      modelStatus: selectedModel ? 'missing' : 'unknown',
      endpoint: cleanEndpoint,
      selectedModel,
      statusText: buildStatusText('installing', selectedModel ? 'missing' : 'unknown', selectedModel),
    });
    appLogService.info('runtime', localeText("settings", "ui.LocalAiService.d03299ffd3fb8c8d"), preparingStatus);
    onProgress(preparingStatus);

    const result = await invoke<LocalRuntimePrepareResultDto>('prepare_ollama_runtime');
    appLogService.info('runtime', result.statusText, result);
    const afterStatus = await this.checkStatus(selectedModel, cleanEndpoint);

    if (afterStatus.runtimeStatus === 'running') {
      return createStatus({ ...afterStatus, statusText: result.statusText || afterStatus.statusText });
    }

    return createStatus({
      ...afterStatus,
      runtimeStatus: result.runtimeStatus === 'notFound' ? 'notFound' : 'notRunning',
      modelStatus: selectedModel ? 'missing' : 'unknown',
      statusText: result.statusText || afterStatus.statusText,
      errorMessage: result.errorMessage || afterStatus.errorMessage,
    });
  }

  async prepareStack(model: string, endpoint: string, onProgress: (status: LocalAiStatus) => void): Promise<LocalAiStatus> {
    const cleanEndpoint = normalizeEndpoint(endpoint);
    const selectedModel = model.trim();
    appLogService.info('local-ai', localeText("settings", "ui.LocalAiService.1dc48d78a30dd215", {"value1": (selectedModel)}), { endpoint: cleanEndpoint });
    const runtimeStatus = await this.prepareRuntime(cleanEndpoint, selectedModel, onProgress);
    onProgress(runtimeStatus);

    if (runtimeStatus.runtimeStatus !== 'running') return runtimeStatus;
    if (runtimeStatus.modelStatus === 'ready' || !selectedModel) return runtimeStatus;

    return pullLocalAiModel(selectedModel, cleanEndpoint, onProgress);
  }

  async deleteModel(model: string, endpoint: string): Promise<LocalAiStatus> {
    return deleteLocalAiModel(model, endpoint);
  }
}

export const localAiService = new LocalAiService();

export function extractBase64ImagePayload(dataUrl: string): string {
  const marker = 'base64,';
  const markerIndex = dataUrl.indexOf(marker);
  return markerIndex >= 0 ? dataUrl.slice(markerIndex + marker.length) : dataUrl;
}

export function parseLocalAiResponse(payload: unknown): string {
  if (!payload || typeof payload !== 'object') {
    throw new Error(localeText("settings", "ui.LocalAiService.6e260ed8d9644332"));
  }

  const content = asNonEmptyString((payload as OllamaChatResponse).message?.content);
  if (!content) {
    const providerError = asNonEmptyString((payload as OllamaChatResponse).error);
    throw new Error(providerError || localeText("settings", "ui.LocalAiService.f6c4ff80490aee7a"));
  }

  return content;
}
