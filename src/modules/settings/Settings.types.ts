export type AiProviderMode = 'openrouter' | 'local' | 'fal';
export type AnalysisOutputMode = 'both' | 'tags' | 'description';

export type OpenRouterModelPresetId =
  | 'google/gemini-2.5-flash-lite'
  | 'nex-agi/nex-n2.5-mini'
  | 'openai/gpt-6-luna'
  | 'google/gemini-3.8-flash'
  | 'openai/gpt-6.1-sol'
  | 'openrouter/free'
  | 'google/gemma-4-31b-it:free'
  | 'nvidia/nemotron-nano-12b-v2-vl:free'
  | 'mistralai/ministral-3b-2512'
  | 'mistralai/ministral-8b-2512'
  | 'google/gemma-4-31b-it'
  | 'openai/gpt-4o-mini'
  | 'openai/gpt-4.1-mini'
  | 'qwen/qwen2.5-vl-72b-instruct'
  | 'openai/gpt-5.2'
  | 'custom';

export type LocalAiModelPresetId =
  | 'qwen3.5:2b'
  | 'qwen3.5:4b'
  | 'gemma3:4b'
  | 'minicpm-v4.6:1b'
  | 'qwen3-vl:2b-instruct-q4_K_M'
  | 'qwen3-vl:4b-instruct-q4_K_M'
  | 'qwen2.5vl:3b'
  | 'qwen2.5vl:7b'
  | 'minicpm-v:8b'
  | 'custom';

export interface OpenRouterModelPreset {
  id: OpenRouterModelPresetId;
  label: string;
  model: string;
  priceTier: 'free' | 'ultraCheap' | 'cheap' | 'balanced' | 'premium' | 'custom';
  notes: string;
}

export interface LocalAiModelPreset {
  id: LocalAiModelPresetId;
  label: string;
  model: string;
  notes: string;
}

export interface OpenRouterSettings {
  credentialRef: string | null;
  apiKey: string;
  selectedModel: string;
  selectedPresetId: OpenRouterModelPresetId;
  customModel: string;
  useFreeRouter: boolean;
  requestDelayMs: number;
  maxImagesPerRun: number;
  maxEstimatedCostPerRunUsd: number;
  showApiKey: boolean;
}

export interface LocalAiSettings {
  endpoint: string;
  selectedModel: string;
  selectedPresetId: LocalAiModelPresetId;
  customModel: string;
}

export interface ProcessingLimitSettings {
  onlyUnprocessed?: boolean;
  outputMode: AnalysisOutputMode;
  manualQueue: boolean;
  passes: number;
  cloudConcurrency: number;
  maxTokens: number;
  imageMaxSide: number;
  maxImagesPerRun: number;
  requestDelayMs: number;
  maxEstimatedCostPerRunUsd: number;
  stopOnFirstAiError: boolean;
  maxRetriesPerTask: number;
}


export interface RenameSettings {
  enabled: boolean;
  pattern: 'date_unique8';
  stopOnFirstError: boolean;
}

export interface PrivacySettings {
  aiConsentAccepted: boolean;
  consentAcceptedAt: string | null;
  externalAiProviderNoticeSeen: boolean;
}

export interface AppSettings {
  aiProviderMode: AiProviderMode;
  privacy: PrivacySettings;
  openRouter: OpenRouterSettings;
  fal: OpenRouterSettings;
  localAi: LocalAiSettings;
  processingLimits: ProcessingLimitSettings;
  rename: RenameSettings;
  theme: 'light'|'dark';
}

export interface ApiKeyCheckResult {
  ok: boolean;
  message: string;
  checkedAt: string;
}
