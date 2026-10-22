export type AiAnalysisLanguage = 'ru' | 'en';
export type AiAnalysisSource = 'mock' | 'openrouter' | 'local' | 'fal';

export interface ImageAnalysisRequest {
  imagePath: string;
  fileName: string;
  model: string;
  language: AiAnalysisLanguage;
  providerMode: AiAnalysisSource;
}

export interface ImageAnalysisResult {
  description: string;
  tags: string[];
  objects: string[];
  scene: string;
  confidence: number;
  source: AiAnalysisSource;
  model: string;
  analyzedAt: string;
  rawText?: string;
}

export interface AiProvider {
  readonly id: AiAnalysisSource;
  readonly displayName: string;
  analyzeImage(request: ImageAnalysisRequest): Promise<ImageAnalysisResult>;
}

export function normalizeAiTags(tags: string[]): string[] {
  const seen = new Set<string>();
  const normalized: string[] = [];

  for (const rawTag of tags) {
    const tag = rawTag.trim().replace(/^#+/, '').toLowerCase();
    if (!tag || seen.has(tag)) continue;
    seen.add(tag);
    normalized.push(tag);
  }

  return normalized;
}
