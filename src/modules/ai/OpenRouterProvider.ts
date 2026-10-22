import {isProviderAvailable,providerUnavailableMessage} from '../settings/ProviderAvailability';
import {text as localeText} from "../../shared/i18n/Runtime";
import { aiImageEncodingService } from './AiImageEncodingService';
import type { AiProvider, ImageAnalysisRequest, ImageAnalysisResult } from './AiProvider';
import { normalizeAiTags } from './AiProvider';
import { AiProviderError, type AiProviderErrorCode } from './AiProviderError';
import { appLogService } from '../logs/AppLogService';
import { messages } from '../../shared/i18n';

const OPENROUTER_CHAT_COMPLETIONS_URL = 'https://openrouter.ai/api/v1/chat/completions';
const OPENROUTER_TIMEOUT_MS = 120_000;
const OPENROUTER_MAX_TOKENS = 650;
const OPENROUTER_TEMPERATURE = 0.05;
const OPENROUTER_MIN_RATE_LIMIT_RETRY_MS = 60_000;
const PROMPT_VERSION = 'photo-archive-analysis-v3-strict-visible';

const MODELS_WITHOUT_JSON_SCHEMA = new Set([
  'openrouter/free',
  'google/gemma-4-31b-it:free',
  'nvidia/nemotron-nano-12b-v2-vl:free',
]);

const MODELS_WITHOUT_TEMPERATURE = new Set([
  'openai/gpt-5.2',
]);

export interface OpenRouterProviderConfig {
  apiKey: string;
  model: string;
  appTitle: string;
}

interface OpenRouterErrorMetadata {
  raw?: unknown;
  provider_name?: unknown;
  is_byok?: unknown;
}

interface OpenRouterChatResponse {
  id?: string;
  choices?: Array<{
    message?: {
      content?: unknown;
    };
    finish_reason?: unknown;
  }>;
  error?: {
    message?: string;
    code?: string | number;
    type?: string;
    metadata?: OpenRouterErrorMetadata;
  };
  model?: unknown;
  provider?: unknown;
  usage?: unknown;
}

interface StructuredImageAnalysisJson {
  description?: unknown;
  tags?: unknown;
  objects?: unknown;
  scene?: unknown;
  confidence?: unknown;
}

interface ClassifiedOpenRouterError {
  code: AiProviderErrorCode;
  retryable: boolean;
  userMessage: string;
  retryAfterMs?: number;
}

export class OpenRouterProvider implements AiProvider {
  readonly id = 'openrouter' as const;
  readonly displayName = 'OpenRouter';

  constructor(private readonly config: OpenRouterProviderConfig) {}

  async analyzeImage(request: ImageAnalysisRequest): Promise<ImageAnalysisResult> {
    if(!isProviderAvailable(this.id))throw new AiProviderError({code:'invalid_request',message:providerUnavailableMessage(),userMessage:providerUnavailableMessage(),retryable:false,provider:this.id});
    const apiKey = this.config.apiKey.trim();
    const model = (request.model || this.config.model || 'openrouter/free').trim();
    const requestStartedAt = performance.now();

    appLogService.info('task', localeText("main", "ui.OpenRouterProvider.968bc4ff606a0ec4"), {
      model,
      fileName: request.fileName,
      imagePath: request.imagePath,
      providerMode: request.providerMode,
      language: request.language,
      timeoutMs: OPENROUTER_TIMEOUT_MS,
      promptVersion: PROMPT_VERSION,
      apiKeyPresent: Boolean(apiKey),
      apiKeyPrefix: apiKey ? `${apiKey.slice(0, 7)}…` : null,
    });

    if (!apiKey) {
      throw new AiProviderError({ code: 'missing_api_key', message: messages().settings.openRouterErrors.missingApiKey, userMessage: messages().settings.openRouterErrors.missingApiKey, retryable: false, provider: 'openrouter' });
    }

    if (!apiKey.startsWith('sk-or-')) {
      throw new AiProviderError({ code: 'invalid_api_key', message: messages().settings.openRouterErrors.invalidApiKey, userMessage: messages().settings.openRouterErrors.invalidApiKey, retryable: false, provider: 'openrouter' });
    }

    if (!model) {
      throw new AiProviderError({ code: 'invalid_request', message: messages().settings.openRouterErrors.emptyModel, userMessage: messages().settings.openRouterErrors.emptyModel, retryable: false, provider: 'openrouter' });
    }

    appLogService.info('task', localeText("main", "ui.OpenRouterProvider.71722418e2545b47"), { model, fileName: request.fileName, imagePath: request.imagePath });
    const encodeStartedAt = performance.now();
    const encodedImage = await aiImageEncodingService.encodeImageForAi(request.imagePath);
    const encodeMs = Math.round(performance.now() - encodeStartedAt);
    appLogService.info('task', localeText("main", "ui.OpenRouterProvider.667efafa1025d0ad"), {
      model,
      fileName: request.fileName,
      dataUrlLength: encodedImage.dataUrl.length,
      approxBase64Kb: Math.round(encodedImage.dataUrl.length / 1024),
      encodeMs,
      cacheHit: encodedImage.cacheHit === true,
      cacheKey: encodedImage.cacheKey ?? null,
    });

    const prompt = buildImageAnalysisPrompt(request.language, request.fileName);
    const requestBody = buildOpenRouterRequestBody(model, request.language, prompt, encodedImage.dataUrl);
    const controller = new AbortController();
    const timeoutId = window.setTimeout(() => controller.abort(), OPENROUTER_TIMEOUT_MS);

    appLogService.info('network', localeText("main", "ui.OpenRouterProvider.e597d277de59a940"), {
      url: OPENROUTER_CHAT_COMPLETIONS_URL,
      model,
      fallbackModels: requestBody.models ?? null,
      fileName: request.fileName,
      timeoutMs: OPENROUTER_TIMEOUT_MS,
      note: localeText("main", "ui.OpenRouterProvider.4b642b55a6c800dd"),
      requestBody: sanitizeOpenRouterRequestBodyForLog(requestBody, encodedImage.dataUrl.length),
    });

    try {
      const networkStartedAt = performance.now();
      const response = await fetch(OPENROUTER_CHAT_COMPLETIONS_URL, {
        method: 'POST',
        mode: 'cors',
        signal: controller.signal,
        headers: {
          'Authorization': `Bearer ${apiKey}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(requestBody),
      });

      const requestMs = Math.round(performance.now() - networkStartedAt);
      const totalMs = Math.round(performance.now() - requestStartedAt);
      appLogService.info('network', localeText("main", "ui.OpenRouterProvider.9e3da541dc7c6a84"), { model, fileName: request.fileName, status: response.status, statusText: response.statusText, ok: response.ok, elapsedMs: totalMs, requestMs, totalMs, headers: pickOpenRouterResponseHeaders(response.headers) });
      const rawText = await response.text();
      appLogService.info('network', localeText("main", "ui.OpenRouterProvider.10f8e9ca5efb8d64"), { model, fileName: request.fileName, bodyLength: rawText.length, bodyPreview: rawText.slice(0, 1500) });
      const decoded = parseOpenRouterResponse(rawText);

      if (!response.ok) {
        const providerMessage = extractOpenRouterProviderMessage(decoded, rawText, localeText("main", "ui.OpenRouterProvider.e9a6563a49630f62", {"value1": (response.status)}));
        const classified = classifyOpenRouterError(response.status, providerMessage, response.headers.get('retry-after'));
        appLogService.error('network', localeText("main", "ui.OpenRouterProvider.c4bdf4f25cf52768"), { model, fileName: request.fileName, status: response.status, providerMessage, classified, rawText: rawText.slice(0, 3000) });
        throw new AiProviderError({ code: classified.code, message: localeText("main", "ui.OpenRouterProvider.5fe7bbe41cd8f7ac", {"value1": (response.status), "value2": (providerMessage)}), userMessage: classified.userMessage, retryable: classified.retryable, retryAfterMs: classified.retryAfterMs, statusCode: response.status, provider: 'openrouter', rawMessage: providerMessage });
      }

      if (decoded.error?.message) {
        const providerMessage = extractOpenRouterProviderMessage(decoded, rawText, decoded.error.message);
        const classified = classifyOpenRouterError(undefined, providerMessage, undefined);
        appLogService.error('network', localeText("main", "ui.OpenRouterProvider.7c6852d5d4cb318d"), { model, fileName: request.fileName, providerError: decoded.error, classified });
        throw new AiProviderError({ code: classified.code, message: localeText("main", "ui.OpenRouterProvider.4ded236f9f6d09b6", {"value1": (providerMessage)}), userMessage: classified.userMessage, retryable: classified.retryable, retryAfterMs: classified.retryAfterMs, provider: 'openrouter', rawMessage: providerMessage });
      }

      appLogService.info('task', localeText("main", "ui.OpenRouterProvider.de0a8cd5824493c6"), { model, provider: decoded.provider ?? null, responseModel: decoded.model ?? null, finishReason: decoded.choices?.[0]?.finish_reason ?? null, usage: decoded.usage ?? null });
      const content = extractAssistantContent(decoded);
      appLogService.info('task', localeText("main", "ui.OpenRouterProvider.7279cbe79e0fa098"), { model, fileName: request.fileName, contentLength: content.length, contentPreview: content.slice(0, 1500) });
      const analysisJson = parseAnalysisJson(content);
      const tags = normalizeAiTags(toStringArray(analysisJson.tags));
      const objects = normalizeAiTags(toObjectNameArray(analysisJson.objects));
      const description = toCleanString(analysisJson.description) || fallbackDescription(request.fileName, request.language);
      const scene = toCleanString(analysisJson.scene) || (request.language === 'ru' ? localeText("main", "ui.OpenRouterProvider.b8c45978cdbd329f") : 'photo');
      const confidence = clampConfidence(analysisJson.confidence);
      const actualModel = typeof decoded.model === 'string' && decoded.model.trim() ? decoded.model.trim() : model;
      appLogService.info('task', localeText("main", "ui.OpenRouterProvider.e561f2d9719f1c04"), {
        model,
        responseModel: actualModel,
        fileName: request.fileName,
        elapsedMs: Math.round(performance.now() - requestStartedAt),
        encodeMs,
        requestMs,
        totalMs: Math.round(performance.now() - requestStartedAt),
        cacheHit: encodedImage.cacheHit === true,
        descriptionLength: description.length,
        tags,
        objectsCount: objects.length,
        scene,
        confidence,
      });
      return { description, tags: tags.length > 0 ? tags : [scene || (request.language === 'ru' ? localeText("main", "ui.OpenRouterProvider.4b200ccccff83bb2") : 'photo')], objects, scene, confidence, source: 'openrouter', model: actualModel, analyzedAt: new Date().toISOString(), rawText: content };
    } catch (error) {
      if (error instanceof AiProviderError) throw error;
      if (error instanceof DOMException && error.name === 'AbortError') {
        appLogService.error('network', localeText("main", "ui.OpenRouterProvider.64164c2fa50f3f22"), { model, fileName: request.fileName, elapsedMs: Math.round(performance.now() - requestStartedAt), timeoutMs: OPENROUTER_TIMEOUT_MS, errorName: error.name, errorMessage: error.message });
        throw new AiProviderError({ code: 'request_timeout', message: localeText("main", "ui.OpenRouterProvider.675f4aa0264a4043", {"value1": (Math.round(OPENROUTER_TIMEOUT_MS / 1000))}), userMessage: localeText("main", "ui.OpenRouterProvider.8374182a667f7080"), retryable: true, provider: 'openrouter' });
      }
      if (error instanceof TypeError) {
        appLogService.error('network', localeText("main", "ui.OpenRouterProvider.1f22f85041901d81"), { model, fileName: request.fileName, elapsedMs: Math.round(performance.now() - requestStartedAt), errorName: error.name, errorMessage: error.message, errorStack: error.stack ?? null });
        throw new AiProviderError({ code: 'network_error', message: localeText("main", "ui.OpenRouterProvider.6a15b44640443eec", {"value1": (error.message)}), userMessage: localeText("main", "ui.OpenRouterProvider.b83a902bc768b757"), retryable: true, provider: 'openrouter' });
      }
      appLogService.error('task', localeText("main", "ui.OpenRouterProvider.8537ed0ce86dda11"), { model, fileName: request.fileName, elapsedMs: Math.round(performance.now() - requestStartedAt), error });
      throw error;
    } finally {
      window.clearTimeout(timeoutId);
    }
  }
}

function buildOpenRouterRequestBody(model: string, language: ImageAnalysisRequest['language'], prompt: string, imageDataUrl: string): Record<string, unknown> {
  const useStructuredOutput = supportsOpenRouterJsonSchema(model);
  const fallbackModels = getOpenRouterFallbackModels(model);
  const body: Record<string, unknown> = {
    messages: [
      { role: 'system', content: buildImageAnalysisSystemPrompt(language) },
      { role: 'user', content: [{ type: 'text', text: prompt }, { type: 'image_url', image_url: { url: imageDataUrl } }] },
    ],
    max_tokens: OPENROUTER_MAX_TOKENS,
    metadata: { prompt_version: PROMPT_VERSION, task: 'photo_archive_image_analysis' },
  };

  if (fallbackModels.length > 1) {
    body.models = fallbackModels;
  } else {
    body.model = model;
  }

  if (!MODELS_WITHOUT_TEMPERATURE.has(model)) {
    body.temperature = OPENROUTER_TEMPERATURE;
  }

  if (useStructuredOutput) {
    body.provider = { require_parameters: true };
    body.response_format = createImageAnalysisResponseFormat(language);
  }

  return body;
}

function getOpenRouterFallbackModels(model: string): string[] {
  if (model === 'google/gemma-4-31b-it:free' || model === 'nvidia/nemotron-nano-12b-v2-vl:free') {
    return [model, 'openrouter/free'];
  }

  if (model === 'qwen/qwen2.5-vl-72b-instruct') {
    return [model, 'google/gemma-4-31b-it', 'mistralai/ministral-8b-2512'];
  }

  return [model];
}

function supportsOpenRouterJsonSchema(model: string): boolean {
  return !MODELS_WITHOUT_JSON_SCHEMA.has(model) && !model.endsWith(':free');
}

function sanitizeOpenRouterRequestBodyForLog(body: Record<string, unknown>, imageDataUrlLength: number): Record<string, unknown> {
  return {
    ...body,
    messages: Array.isArray(body.messages) ? body.messages.map((message) => {
      if (!message || typeof message !== 'object') return message;
      const content = (message as { content?: unknown }).content;
      if (!Array.isArray(content)) return message;
      return {
        ...message,
        content: content.map((part) => {
          if (!part || typeof part !== 'object') return part;
          if ((part as { type?: unknown }).type === 'image_url') return { type: 'image_url', image_url: { url: localeText("main", "ui.OpenRouterProvider.d29d6927ad2f8eda", {"value1": (imageDataUrlLength)}) } };
          return part;
        }),
      };
    }) : body.messages,
  };
}

function pickOpenRouterResponseHeaders(headers: Headers): Record<string, string | null> {
  return {
    contentType: headers.get('content-type'),
    retryAfter: headers.get('retry-after'),
    xRateLimitLimit: headers.get('x-ratelimit-limit'),
    xRateLimitRemaining: headers.get('x-ratelimit-remaining'),
    xRateLimitReset: headers.get('x-ratelimit-reset'),
    xRequestId: headers.get('x-request-id'),
  };
}

function createImageAnalysisResponseFormat(language: ImageAnalysisRequest['language']): Record<string, unknown> {
  const isRu = language === 'ru';
  return {
    type: 'json_schema',
    json_schema: {
      name: 'smart_photo_indexer_image_analysis',
      strict: true,
      schema: {
        type: 'object',
        additionalProperties: false,
        properties: {
          description: {
            type: 'string',
            description: isRu
              ? localeText("main", "ui.OpenRouterProvider.b83a21a03c080731")
              : localeText("main", "ui.OpenRouterProvider.d7e1132bd5b33e86"),
          },
          tags: {
            type: 'array',
            minItems: 5,
            maxItems: 10,
            items: { type: 'string' },
            description: isRu
              ? localeText("main", "ui.OpenRouterProvider.a106b7a2a1d3156f")
              : localeText("main", "ui.OpenRouterProvider.453efac19f954aae"),
          },
          objects: {
            type: 'array',
            minItems: 0,
            maxItems: 16,
            items: { type: 'string' },
            description: isRu
              ? localeText("main", "ui.OpenRouterProvider.e1dc952c13e0eb2f")
              : localeText("main", "ui.OpenRouterProvider.050b3bfef9dd292a"),
          },
          scene: {
            type: 'string',
            description: isRu ? localeText("main", "ui.OpenRouterProvider.8a41646e49fc1d9f") : localeText("main", "ui.OpenRouterProvider.59711fc43d9333e3"),
          },
          confidence: {
            type: 'number',
            minimum: 0,
            maximum: 1,
            description: localeText("main", "ui.OpenRouterProvider.76a8c20006cc0623"),
          },
        },
        required: ['description', 'tags', 'objects', 'scene', 'confidence'],
      },
    },
  };
}

function buildImageAnalysisSystemPrompt(language: ImageAnalysisRequest['language']): string {
  if (language === 'ru') {
    return [
      'Ты модуль строгого анализа изображений для личного фотоархива.',
      'Описывай только то, что явно видно на изображении.',
      'Если деталь видна частично или неясно, пиши нейтрально или не упоминай ее.',
      'Не угадывай возраст, породу, назначение места, событие, погоду, время суток, настроение, отношения между объектами и причину происходящего.',
      'Не называй фон лесом, парком, загородом, двором или прогулкой, если это не очевидно. Если видны дорога, здание, окно, дверь, газон или деревья — так и пиши конкретно.',
      'Не добавляй объекты в tags/objects, если они не видны уверенно.',
      'Ответ должен быть коротким, полезным для поиска и без Markdown.',
    ].join(' ');
  }

  return [
    'You are a strict image analysis module for a personal photo archive.',
    'Describe only what is clearly visible in the image.',
    'If a detail is partial or unclear, use neutral wording or omit it.',
    'Do not guess age, breed, place purpose, event, weather, time of day, mood, relationships between objects, or reasons for what is happening.',
    'Do not call the background a forest, park, rural area, yard, or walk unless it is clearly visible. If road, building, window, door, grass, or trees are visible, name only those concrete elements.',
    'Do not add objects to tags/objects unless they are clearly visible.',
    'Return a short search-friendly answer without Markdown.',
  ].join(' ');
}

function classifyOpenRouterError(statusCode: number | undefined, providerMessage: string, retryAfterHeader: string | null | undefined): ClassifiedOpenRouterError {
  const message = providerMessage.toLowerCase();
  const retryAfterMs = parseRetryAfterMs(retryAfterHeader);

  if (statusCode === 401 || statusCode === 403 || message.includes('invalid api key') || message.includes('unauthorized')) return { code: 'invalid_api_key', retryable: false, userMessage: localeText("main", "ui.OpenRouterProvider.1638285b30808fff") };
  if (statusCode === 402 || message.includes('insufficient') || message.includes('credit') || message.includes('quota') || message.includes('billing')) return { code: 'insufficient_credits', retryable: false, userMessage: localeText("main", "ui.OpenRouterProvider.9b395ab4874b0be9") };
  if (statusCode === 429 || message.includes('rate limit') || message.includes('rate-limited') || message.includes('too many requests')) {
    const retryMs = retryAfterMs ?? OPENROUTER_MIN_RATE_LIMIT_RETRY_MS;
    return { code: 'rate_limited', retryable: true, retryAfterMs: retryMs, userMessage: localeText("main", "ui.OpenRouterProvider.41e95d8f08b1736a", {"value1": (Math.ceil(retryMs / 1000))}) };
  }
  if (message.includes('no endpoints found') || message.includes('requested parameters')) return { code: 'invalid_request', retryable: false, userMessage: localeText("main", "ui.OpenRouterProvider.733126f51a7304f8") };
  if (statusCode && [408, 409, 425, 500, 502, 503, 504].includes(statusCode)) return { code: 'provider_overloaded', retryable: true, retryAfterMs, userMessage: localeText("main", "ui.OpenRouterProvider.0fcf69405cb18bc2", {"value1": (statusCode)}) };
  if (statusCode && [400, 404, 422].includes(statusCode)) return { code: 'invalid_request', retryable: false, userMessage: localeText("main", "ui.OpenRouterProvider.0d7a62ebda95b769", {"value1": (statusCode)}) };
  if (message.includes('temporarily') || message.includes('overloaded') || message.includes('unavailable') || message.includes('timeout')) return { code: 'provider_overloaded', retryable: true, retryAfterMs, userMessage: localeText("main", "ui.OpenRouterProvider.70d0f226add13959") };
  return { code: 'provider_error', retryable: false, userMessage: providerMessage ? localeText("main", "ui.OpenRouterProvider.930e50a5248244e6", {"value1": (providerMessage)}) : localeText("main", "ui.OpenRouterProvider.962e9d304e0ae179") };
}

function parseRetryAfterMs(value: string | null | undefined): number | undefined {
  if (!value) return undefined;
  const asSeconds = Number(value);
  if (Number.isFinite(asSeconds) && asSeconds >= 0) return Math.round(asSeconds * 1000);
  const asDate = Date.parse(value);
  if (Number.isFinite(asDate)) {
    const delta = asDate - Date.now();
    return delta > 0 ? delta : undefined;
  }
  return undefined;
}

function buildImageAnalysisPrompt(language: ImageAnalysisRequest['language'], fileName: string): string {
  if (language === 'ru') {
    return [
      'Проанализируй изображение для поиска по личному фотоархиву.',
      `Имя файла: ${fileName}`,
      'Верни только валидный JSON по заданной схеме.',
      'description: 1-3 коротких предложения по-русски. Только явно видимое: главный объект, привязанные/рядом видимые предметы, фон. Без художественных догадок.',
      'tags: 5-10 коротких русских тегов в нижнем регистре. Лучше меньше тегов, но точнее. Без #, дублей и длинных фраз.',
      'objects: конкретные явно видимые объекты и детали. Если объект сомнительный — не добавляй.',
      'scene: нейтральный тип сцены, например “собака у дороги” или “уличная сцена”.',
      'confidence: число от 0 до 1. Не ставь 1.0, если фон или мелкие детали видны частично.',
      'Запрещено додумывать: лес, лесная тропинка, парк, загородный дом, солнечный день, прогулка, возраст животного, порода, настроение, контейнер, забор, если это не видно ясно.',
      'Если на фоне видны только часть здания, окно, дверь, дорога, газон или деревья — называй именно эти элементы, без вывода о месте.',
      'Не идентифицируй людей по имени. Не называй бренды/текст, если они не читаются ясно.',
    ].join('\n');
  }

  return [
    'Analyze this image for personal photo archive search.',
    `File name: ${fileName}`,
    'Return only valid JSON that matches the required schema.',
    'description: 1-3 concise English sentences. Only clearly visible evidence: main subject, visible attached/nearby objects, and background. No artistic guesses.',
    'tags: 5-10 short lowercase search tags. Fewer accurate tags are better than many vague tags.',
    'objects: concrete clearly visible objects and details. If uncertain, omit it.',
    'scene: neutral scene type.',
    'confidence: number from 0 to 1. Do not use 1.0 if background or small details are partially visible.',
    'Do not infer forest, trail, park, rural house, sunny day, walk, animal age, breed, mood, container, or fence unless clearly visible.',
    'If the background only shows parts of a building, window, door, road, grass, or trees, name only those elements.',
    'Do not identify people by name. Do not name brands/text unless clearly readable.',
  ].join('\n');
}

function parseOpenRouterResponse(rawText: string): OpenRouterChatResponse {
  try {
    return JSON.parse(rawText) as OpenRouterChatResponse;
  } catch {
    return { error: { message: rawText || localeText("main", "ui.OpenRouterProvider.8d632d9f2a11305e") } };
  }
}

function extractOpenRouterProviderMessage(response: OpenRouterChatResponse, rawText: string, fallback: string): string {
  const raw = response.error?.metadata?.raw;
  if (typeof raw === 'string' && raw.trim()) return raw.trim();
  if (response.error?.message?.trim()) return response.error.message.trim();
  return rawText.trim() || fallback;
}

function extractAssistantContent(response: OpenRouterChatResponse): string {
  const content = response.choices?.[0]?.message?.content;
  if (typeof content === 'string') return content;
  if (Array.isArray(content)) {
    return content.map((part) => {
      if (typeof part === 'string') return part;
      if (part && typeof part === 'object' && 'text' in part && typeof (part as { text?: unknown }).text === 'string') return (part as { text: string }).text;
      return '';
    }).join('\n').trim();
  }

  throw new AiProviderError({ code: 'invalid_response', message: localeText("main", "ui.OpenRouterProvider.f0f964b801f583df"), userMessage: localeText("main", "ui.OpenRouterProvider.7475b46316c910aa"), retryable: false, provider: 'openrouter' });
}

function parseAnalysisJson(content: string): StructuredImageAnalysisJson {
  const trimmed = content.trim();
  const jsonCandidate = extractJsonObject(trimmed);
  try {
    return JSON.parse(jsonCandidate) as StructuredImageAnalysisJson;
  } catch (error) {
    appLogService.error('task', localeText("main", "ui.OpenRouterProvider.0f4a0dae14fae8e2"), { parseError: error instanceof Error ? error.message : String(error), contentPreview: content.slice(0, 3000), extractedCandidate: jsonCandidate.slice(0, 3000) });
    throw new AiProviderError({ code: 'invalid_response', message: localeText("main", "ui.OpenRouterProvider.76a15f23adba268e", {"value1": (error instanceof Error ? error.message : localeText("main", "ui.OpenRouterProvider.68e6d0a9da3af10d"))}), userMessage: localeText("main", "ui.OpenRouterProvider.0019cfa184917784"), retryable: false, provider: 'openrouter', rawMessage: content });
  }
}

function extractJsonObject(text: string): string {
  if (text.startsWith('{') && text.endsWith('}')) return text;
  const fencedMatch = text.match(/```(?:json)?\s*([\s\S]*?)\s*```/i);
  if (fencedMatch?.[1]) {
    const fenced = fencedMatch[1].trim();
    if (fenced.startsWith('{') && fenced.endsWith('}')) return fenced;
  }
  const firstBrace = text.indexOf('{');
  const lastBrace = text.lastIndexOf('}');
  if (firstBrace >= 0 && lastBrace > firstBrace) return text.slice(firstBrace, lastBrace + 1);
  return text;
}

function toStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.map((item) => (typeof item === 'string' ? item.trim() : String(item ?? '').trim())).filter(Boolean);
}

function toObjectNameArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.map((item) => {
    if (typeof item === 'string') return item.trim();
    if (item && typeof item === 'object' && 'name' in item && typeof (item as { name?: unknown }).name === 'string') return (item as { name: string }).name.trim();
    return '';
  }).filter(Boolean);
}

function toCleanString(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

function clampConfidence(value: unknown): number {
  const numeric = typeof value === 'number' ? value : typeof value === 'string' ? Number(value) : NaN;
  return Number.isFinite(numeric) ? Math.max(0, Math.min(1, numeric)) : 0.75;
}

function fallbackDescription(fileName: string, language: ImageAnalysisRequest['language']): string {
  return language === 'ru'
    ? localeText("main", "ui.OpenRouterProvider.d06427cca9ac2f87", {"value1": (fileName)})
    : localeText("main", "ui.OpenRouterProvider.dcfc0dd3bd9456f3", {"value1": (fileName)});
}
