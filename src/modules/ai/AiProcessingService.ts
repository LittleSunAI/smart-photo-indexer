import {text as localeText} from "../../shared/i18n/Runtime";
import type { AppSettings } from '../settings/Settings.types';
import type { QueueTask } from '../taskQueue/types';
import type { ImageItem } from '../explorer/types';
import { appLogService } from '../logs/AppLogService';
import type { AiProvider, ImageAnalysisRequest, ImageAnalysisResult } from './AiProvider';
import { normalizeAiTags } from './AiProvider';
import { OpenRouterProvider } from './OpenRouterProvider';
import { LocalAiProvider } from './LocalAiProvider';

export interface AiProcessingContext {
  settings: AppSettings;
  appName: string;
  language: 'ru' | 'en';
}

export interface AnalyzeQueueTaskInput {
  task: QueueTask;
  image: ImageItem;
  context: AiProcessingContext;
}

export interface AiProcessingService {
  analyzeQueueTask(input: AnalyzeQueueTaskInput): Promise<ImageAnalysisResult>;
}

export class DefaultAiProcessingService implements AiProcessingService {
  async analyzeQueueTask({ task, image, context }: AnalyzeQueueTaskInput): Promise<ImageAnalysisResult> {
    const provider = createProvider(context.settings, context.appName);
    const selectedModel = resolveProviderModel(context.settings, task.model);
    const request: ImageAnalysisRequest = {
      imagePath: image.path,
      fileName: image.name,
      model: selectedModel,
      language: context.language,
      providerMode: provider.id,
    };

    appLogService.info('task', localeText("main", "ui.AiProcessingService.683d9c74a79ca807"), {
      taskId: task.id,
      provider: provider.id,
      taskModel: task.model || null,
      resolvedModel: request.model,
      fileName: image.name,
      imagePath: image.path,
      indexedImageId: image.indexedImageId ?? null,
      status: image.status,
    });

    try {
      const result = enrichAnalysisForArchiveSearch(await provider.analyzeImage(request));
      appLogService.info('task', localeText("main", "ui.AiProcessingService.de4b6a9d1fb9eee1"), {
        taskId: task.id,
        provider: provider.id,
        model: result.model,
        fileName: image.name,
        descriptionLength: result.description.length,
        tags: result.tags,
        objectsCount: result.objects.length,
        scene: result.scene,
        confidence: result.confidence,
      });
      return result;
    } catch (error) {
      appLogService.error('task', localeText("main", "ui.AiProcessingService.84714a881acb759f"), {
        taskId: task.id,
        provider: provider.id,
        model: request.model,
        fileName: image.name,
        imagePath: image.path,
        error,
      });
      throw error;
    }
  }
}

function resolveProviderModel(settings: AppSettings, taskModel?: string): string {
  const cleanTaskModel = taskModel?.trim() ?? '';

  if (settings.aiProviderMode === 'local') {
    return cleanTaskModel || settings.localAi.selectedModel || 'qwen2.5vl:7b';
  }

  if (settings.aiProviderMode === 'openrouter') {
    const selectedOpenRouterModel = settings.openRouter.selectedModel.trim() || 'openrouter/free';
    if (!cleanTaskModel) return selectedOpenRouterModel;

    if (looksLikeLocalOllamaModel(cleanTaskModel)) {
      appLogService.warning('task', localeText("main", "ui.AiProcessingService.08f2b130913fc249"), {
        taskModel: cleanTaskModel,
        selectedOpenRouterModel,
      });
      return selectedOpenRouterModel;
    }

    return cleanTaskModel;
  }

  return cleanTaskModel || 'mock';
}

function looksLikeLocalOllamaModel(model: string): boolean {
  const clean = model.trim().toLowerCase();
  return Boolean(clean && clean.includes(':') && !clean.includes('/'));
}

function cleanArchiveTagCandidate(value: string): string {
  const clean = value.trim();
  if (!clean) return '';
  if (clean.toLowerCase() === '[object object]') return '';
  if (clean.length > 64) return '';
  return clean;
}

function enrichAnalysisForArchiveSearch(result: ImageAnalysisResult): ImageAnalysisResult {
  const tags = normalizeAiTags([
    ...result.tags,
    result.scene,
    ...result.objects,
  ].map(cleanArchiveTagCandidate).filter(Boolean));

  return {
    ...result,
    tags: tags.length > 0 ? tags : result.tags,
  };
}

function createProvider(settings: AppSettings, appName: string): AiProvider {

  if (settings.aiProviderMode === 'local') {
    return new LocalAiProvider({
      endpoint: settings.localAi.endpoint,
      model: settings.localAi.selectedModel,
    });
  }

  return new OpenRouterProvider({
    apiKey: settings.openRouter.apiKey,
    model: settings.openRouter.selectedModel,
    appTitle: appName,
  });
}

export const aiProcessingService: AiProcessingService = new DefaultAiProcessingService();
