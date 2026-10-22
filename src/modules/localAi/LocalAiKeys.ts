export type LocalRuntimeKey = string;
export type LocalModelKey = string;
export type LocalAiOperationId = string;

export function normalizeLocalAiEndpoint(endpoint: string): string {
  const trimmed = endpoint.trim();
  if (!trimmed) return '';
  return trimmed.replace(/\/+$/, '');
}

export function makeLocalRuntimeKey(endpoint: string): LocalRuntimeKey {
  return normalizeLocalAiEndpoint(endpoint).toLowerCase();
}

export function makeLocalModelKey(endpoint: string, model: string): LocalModelKey {
  const endpointKey = makeLocalRuntimeKey(endpoint);
  const modelKey = model.trim();
  return `${endpointKey}::${modelKey}`;
}

export function createLocalAiOperationId(kind: string, key: string): LocalAiOperationId {
  const safeKind = kind.trim() || 'local-ai';
  const safeKey = key.trim() || 'unknown';
  return `${safeKind}:${safeKey}:${Date.now()}:${Math.random().toString(36).slice(2, 10)}`;
}
