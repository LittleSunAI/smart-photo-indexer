import {text as localeText} from "../../shared/i18n/Runtime";
import { aiImageEncodingService } from './AiImageEncodingService';
import type { AiProvider, ImageAnalysisRequest, ImageAnalysisResult } from './AiProvider';
import { normalizeAiTags } from './AiProvider';
import { AiProviderError } from './AiProviderError';
import { appLogService } from '../logs/AppLogService';
import { extractBase64ImagePayload, isAllowedLocalEndpoint, normalizeEndpoint, parseLocalAiResponse } from '../localAi/LocalAiService';

const LOCAL_AI_TIMEOUT_MS = 180_000;
const LOCAL_AI_TEMPERATURE = 0.05;
const LOCAL_AI_MAX_TOKENS = 700;

interface LocalAiProviderConfig {
  endpoint: string;
  model: string;
}

interface LocalAiStructuredResponse {
  description?: unknown;
  tags?: unknown;
  objects?: unknown;
  scene?: unknown;
  confidence?: unknown;
}

interface LocalChatAttemptResult {
  response: Response;
  rawBody: string;
  elapsedMs: number;
  requestMs: number;
  totalMs: number;
  usedJsonFormat: boolean;
}

function buildLocalPrompt(language: 'ru' | 'en', fileName: string): string {
  if (language === 'en') {
    return [
      'You are a strict image analysis module for a personal photo archive.',
      `File name: ${fileName}`,
      'Return ONLY one valid JSON object. No Markdown. No comments. No text outside JSON. No repeated filler characters.',
      'Required JSON schema: {"description": string, "tags": string[], "objects": string[], "scene": string, "confidence": number}.',
      'Describe only what is clearly visible in the image.',
      'If a detail is partial or unclear, use neutral wording or omit it.',
      'Do not guess breed, age, place purpose, event, weather, time of day, mood, relationships between objects, or reasons for what is happening.',
      'Do not call the background a forest, park, yard, rural area, or walk unless it is clearly visible. If road, building, window, door, grass, trees, pole, or ground are visible, name only those concrete elements.',
      'Do not call an animal a statue, toy, or mannequin unless there is clear evidence that it is not a living animal.',
      'description: 1-3 short English sentences. Only visible scene, main objects, background, and important details without guesses.',
      'tags: 5-10 short lowercase search tags. No #, no duplicates, no uncertain tags.',
      'objects: concrete clearly visible objects and details. For example: dog, leash, rope, pole, road, grass, window, door, tree. Do not add an object unless it is clearly visible.',
      'scene: short neutral scene type without guesses.',
      'confidence: number from 0 to 1. Use less than 0.9 if background or details are partially visible.',
    ].join('\n');
  }

  return [
    'Ты модуль строгого анализа изображений для личного фотоархива.',
    `Имя файла: ${fileName}`,
    'Верни ТОЛЬКО один валидный JSON-объект. Без Markdown. Без комментариев. Без пояснений вне JSON. Без повторяющихся мусорных символов.',
    'Обязательная JSON-схема: {"description": string, "tags": string[], "objects": string[], "scene": string, "confidence": number}.',
    'Описывай только то, что явно видно на изображении.',
    'Если деталь видна частично или неясно, пиши нейтрально или не упоминай её.',
    'Не угадывай породу, возраст, назначение места, событие, погоду, время суток, настроение, отношения между объектами и причину происходящего.',
    'Не называй фон лесом, парком, двором, загородом или прогулкой, если это не очевидно. Если видны дорога, здание, окно, дверь, газон, деревья, столб или земля — называй именно эти конкретные элементы.',
    'Не называй животное статуей, игрушкой или манекеном, если нет явных признаков, что это не живое животное.',
    'description: 1-3 коротких предложения по-русски. Только видимая сцена, главные объекты, фон и важные детали без догадок.',
    'tags: 5-10 коротких русских поисковых тегов в нижнем регистре. Без #, без дублей, без спорных тегов.',
    'objects: конкретные явно видимые объекты и детали. Например: собака, поводок, верёвка, столб, дорога, газон, окно, дверь, дерево. Не добавляй объект, если он не виден уверенно.',
    'scene: короткий нейтральный тип сцены без догадок.',
    'confidence: число от 0 до 1. Ставь меньше 0.9, если фон или детали видны частично.',
  ].join('\n');
}

function parseStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => {
      if (typeof item === 'string') return item.trim();
      if (item && typeof item === 'object' && 'name' in item) {
        const namedItem = item as { name?: unknown };
        return typeof namedItem.name === 'string' ? namedItem.name.trim() : String(namedItem.name ?? '').trim();
      }
      return String(item ?? '').trim();
    })
    .filter(Boolean);
}

function parseConfidence(value: unknown): number {
  const numeric = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(numeric) ? Math.max(0, Math.min(1, numeric)) : 0.74;
}

function stripMarkdownFence(value: string): string {
  return value
    .replace(/^```json\s*/i, '')
    .replace(/^```\s*/i, '')
    .replace(/```$/i, '')
    .trim();
}

function extractJsonObject(value: string): string {
  const start = value.indexOf('{');
  const end = value.lastIndexOf('}');

  if (start < 0 || end <= start) {
    return '';
  }

  return value.slice(start, end + 1);
}

function parseAnalysisJson(rawText: string): LocalAiStructuredResponse {
  const clean = stripMarkdownFence(rawText.trim());
  const candidates = [clean, extractJsonObject(clean)].filter(Boolean);

  for (const candidate of candidates) {
    try {
      const parsed = JSON.parse(candidate) as LocalAiStructuredResponse;
      if (parsed && typeof parsed === 'object') {
        return parsed;
      }
    } catch {
      // Try next candidate.
    }
  }

  appLogService.error('local-ai', localeText("main", "ui.LocalAiProvider.d82bd1f231c2f040"), {
    rawPreview: rawText.slice(0, 3000),
    extractedJsonCandidate: extractJsonObject(clean).slice(0, 3000),
  });

  throw new AiProviderError({
    code: 'invalid_response',
    message: localeText("main", "ui.LocalAiProvider.c06fef7b88933391", {"value1": (rawText.slice(0, 500))}),
    userMessage: localeText("main", "ui.LocalAiProvider.75a1ab68a2e83fda"),
    retryable: true,
    provider: 'local',
    rawMessage: rawText,
  });
}

function fallbackDescription(fileName: string, language: ImageAnalysisRequest['language']): string {
  return language === 'ru'
    ? localeText("main", "ui.LocalAiProvider.974910b6fa400988", {"value1": (fileName)})
    : localeText("main", "ui.LocalAiProvider.9cb42a27672daf4b", {"value1": (fileName)});
}

function parseLocalChatPayload(rawBody: string): unknown {
  try {
    return JSON.parse(rawBody);
  } catch (error) {
    appLogService.error('local-ai', localeText("main", "ui.LocalAiProvider.b025f8011a183119"), {
      parseError: error instanceof Error ? error.message : String(error),
      rawBodyPreview: rawBody.slice(0, 3000),
    });
    throw new AiProviderError({
      code: 'invalid_response',
      message: localeText("main", "ui.LocalAiProvider.928d134f04406cfa"),
      userMessage: localeText("main", "ui.LocalAiProvider.08f95852c12c3957"),
      retryable: true,
      provider: 'local',
      rawMessage: rawBody,
    });
  }
}

function buildLocalRequestBody(model: string, prompt: string, imageDataUrl: string, useJsonFormat: boolean): Record<string, unknown> {
  const body: Record<string, unknown> = {
    model,
    stream: false,
    messages: [
      {
        role: 'user',
        content: prompt,
        images: [extractBase64ImagePayload(imageDataUrl)],
      },
    ],
    options: {
      temperature: LOCAL_AI_TEMPERATURE,
      num_predict: LOCAL_AI_MAX_TOKENS,
    },
  };

  if (useJsonFormat) {
    body.format = 'json';
  }

  return body;
}

function sanitizeLocalRequestBodyForLog(body: Record<string, unknown>, imageDataUrlLength: number): Record<string, unknown> {
  const messages = Array.isArray(body.messages) ? body.messages.map((message) => {
    if (!message || typeof message !== 'object') return message;
    return {
      ...message,
      images: [localeText("main", "ui.LocalAiProvider.1a3d4380f1bd739f", {"value1": (imageDataUrlLength)})],
    };
  }) : body.messages;

  return {
    ...body,
    messages,
  };
}

function isOllamaJsonGrammarFailure(rawBody: string): boolean {
  const lower = rawBody.toLowerCase();
  return lower.includes('unexpected empty grammar stack') || lower.includes('grammar stack');
}

function isUnsupportedOllamaModelArchitecture(rawBody: string): boolean {
  const lower = rawBody.toLowerCase();
  return lower.includes('unknown model architecture') && lower.includes('mllama');
}

async function sendLocalChatAttempt(input: {
  endpoint: string;
  model: string;
  fileName: string;
  body: Record<string, unknown>;
  imageDataUrlLength: number;
  startedAt: number;
  signal: AbortSignal;
  usedJsonFormat: boolean;
}): Promise<LocalChatAttemptResult> {
  const { endpoint, model, fileName, body, imageDataUrlLength, startedAt, signal, usedJsonFormat } = input;
  appLogService.info('local-ai', usedJsonFormat ? localeText("main", "ui.LocalAiProvider.45202e974cc2d3f0") : localeText("main", "ui.LocalAiProvider.25a77be0d659e2a8"), {
    endpoint,
    model,
    fileName,
    timeoutMs: LOCAL_AI_TIMEOUT_MS,
    usedJsonFormat,
    requestBody: sanitizeLocalRequestBodyForLog(body, imageDataUrlLength),
  });

  const requestStartedAt = performance.now();
  const response = await fetch(`${endpoint}/api/chat`, {
    method: 'POST',
    signal,
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  });

  const requestMs = Math.round(performance.now() - requestStartedAt);
  const totalMs = Math.round(performance.now() - startedAt);
  appLogService.info('local-ai', localeText("main", "ui.LocalAiProvider.ba13905bb40f13bf"), {
    endpoint,
    model,
    fileName,
    status: response.status,
    statusText: response.statusText,
    ok: response.ok,
    elapsedMs: totalMs,
    requestMs,
    totalMs,
    usedJsonFormat,
  });

  const rawBody = await response.text();
  appLogService.info('local-ai', localeText("main", "ui.LocalAiProvider.cd076cd302521677"), {
    model,
    fileName,
    bodyLength: rawBody.length,
    bodyPreview: rawBody.slice(0, 2000),
    usedJsonFormat,
  });

  return {
    response,
    rawBody,
    elapsedMs: totalMs,
    requestMs,
    totalMs,
    usedJsonFormat,
  };
}

export class LocalAiProvider implements AiProvider {
  readonly id = 'local' as const;
  readonly displayName = 'Local AI Provider';

  constructor(private readonly config: LocalAiProviderConfig) {}

  async analyzeImage(request: ImageAnalysisRequest): Promise<ImageAnalysisResult> {
    const model = (request.model || this.config.model).trim();
    const startedAt = performance.now();

    appLogService.info('local-ai', localeText("main", "ui.LocalAiProvider.b73df5f27599fe96"), {
      model,
      fileName: request.fileName,
      imagePath: request.imagePath,
      providerMode: request.providerMode,
      timeoutMs: LOCAL_AI_TIMEOUT_MS,
    });

    if (!model) {
      appLogService.error('local-ai', localeText("main", "ui.LocalAiProvider.cb2f7ec9869a4ada"), { fileName: request.fileName });
      throw new AiProviderError({
        code: 'invalid_request',
        message: localeText("main", "ui.LocalAiProvider.1a89cf1a06a0b591"),
        userMessage: localeText("main", "ui.LocalAiProvider.2089ee497e997286"),
        retryable: false,
        provider: 'local',
      });
    }

    const endpoint = normalizeEndpoint(this.config.endpoint);

    if (!isAllowedLocalEndpoint(endpoint)) {
      appLogService.error('local-ai', localeText("main", "ui.LocalAiProvider.f74f42ef8e992a39"), { endpoint, model, fileName: request.fileName });
      throw new AiProviderError({
        code: 'invalid_request',
        message: localeText("main", "ui.LocalAiProvider.7b42a4f8e46d633c", {"value1": (endpoint)}),
        userMessage: localeText("main", "ui.LocalAiProvider.27894f983f26253c"),
        retryable: false,
        provider: 'local',
      });
    }

    appLogService.info('local-ai', localeText("main", "ui.LocalAiProvider.52fa5fa27ae53ed2"), {
      model,
      endpoint,
      fileName: request.fileName,
      imagePath: request.imagePath,
    });
    const encodeStartedAt = performance.now();
    const encodedImage = await aiImageEncodingService.encodeImageForAi(request.imagePath);
    const encodeMs = Math.round(performance.now() - encodeStartedAt);
    appLogService.info('local-ai', localeText("main", "ui.LocalAiProvider.a100b296214d2003"), {
      model,
      fileName: request.fileName,
      dataUrlLength: encodedImage.dataUrl.length,
      approxBase64Kb: Math.round(encodedImage.dataUrl.length / 1024),
      encodeMs,
      cacheHit: encodedImage.cacheHit === true,
      cacheKey: encodedImage.cacheKey ?? null,
    });

    const prompt = buildLocalPrompt(request.language, request.fileName);
    const controller = new AbortController();
    const timeoutId = window.setTimeout(() => controller.abort(), LOCAL_AI_TIMEOUT_MS);

    try {
      const jsonBody = buildLocalRequestBody(model, prompt, encodedImage.dataUrl, true);
      let attempt = await sendLocalChatAttempt({
        endpoint,
        model,
        fileName: request.fileName,
        body: jsonBody,
        imageDataUrlLength: encodedImage.dataUrl.length,
        startedAt,
        signal: controller.signal,
        usedJsonFormat: true,
      });

      if (!attempt.response.ok && isOllamaJsonGrammarFailure(attempt.rawBody)) {
        appLogService.warning('local-ai', localeText("main", "ui.LocalAiProvider.5ef9795c2004e1fd"), {
          endpoint,
          model,
          fileName: request.fileName,
          status: attempt.response.status,
          rawBody: attempt.rawBody.slice(0, 3000),
        });

        const fallbackBody = buildLocalRequestBody(model, prompt, encodedImage.dataUrl, false);
        attempt = await sendLocalChatAttempt({
          endpoint,
          model,
          fileName: request.fileName,
          body: fallbackBody,
          imageDataUrlLength: encodedImage.dataUrl.length,
          startedAt,
          signal: controller.signal,
          usedJsonFormat: false,
        });
      }

      if (!attempt.response.ok) {
        const unsupportedArchitecture = isUnsupportedOllamaModelArchitecture(attempt.rawBody);
        appLogService.error('local-ai', localeText("main", "ui.LocalAiProvider.472b1ccc58b8cf06"), {
          endpoint,
          model,
          fileName: request.fileName,
          status: attempt.response.status,
          usedJsonFormat: attempt.usedJsonFormat,
          unsupportedArchitecture,
          rawBody: attempt.rawBody.slice(0, 3000),
        });
        throw new AiProviderError({
          code: unsupportedArchitecture ? 'invalid_request' : attempt.response.status >= 500 ? 'provider_error' : 'invalid_request',
          message: unsupportedArchitecture
            ? localeText("main", "ui.LocalAiProvider.0f41c7d2c794f202", {"value1": (model)})
            : localeText("main", "ui.LocalAiProvider.8494ea8c89d0bfa9", {"value1": (attempt.response.status)}),
          userMessage: unsupportedArchitecture
            ? localeText("main", "ui.LocalAiProvider.6bbdf228aad66722")
            : attempt.response.status >= 500
              ? localeText("main", "ui.LocalAiProvider.efb322be8241bf20")
              : localeText("main", "ui.LocalAiProvider.897065a1e62440f0"),
          retryable: unsupportedArchitecture ? false : attempt.response.status >= 500,
          statusCode: attempt.response.status,
          provider: 'local',
          rawMessage: attempt.rawBody,
        });
      }

      const payload = parseLocalChatPayload(attempt.rawBody);
      const rawText = parseLocalAiResponse(payload);
      appLogService.info('local-ai', localeText("main", "ui.LocalAiProvider.5a717fcc0d52e7d9"), {
        model,
        fileName: request.fileName,
        contentLength: rawText.length,
        contentPreview: rawText.slice(0, 2000),
        usedJsonFormat: attempt.usedJsonFormat,
      });
      const parsed = parseAnalysisJson(rawText);

      const description = typeof parsed.description === 'string' && parsed.description.trim()
        ? parsed.description.trim()
        : fallbackDescription(request.fileName, request.language);
      const tags = normalizeAiTags(parseStringArray(parsed.tags));
      const objects = normalizeAiTags(parseStringArray(parsed.objects));
      const scene = typeof parsed.scene === 'string' && parsed.scene.trim()
        ? parsed.scene.trim()
        : request.language === 'ru'
          ? localeText("main", "ui.LocalAiProvider.ea46f296b909e86b")
          : localeText("main", "ui.LocalAiProvider.c429878ddbadba3f");
      const confidence = parseConfidence(parsed.confidence);

      appLogService.info('local-ai', localeText("main", "ui.LocalAiProvider.0304d8fa265bca94"), {
        model,
        fileName: request.fileName,
        elapsedMs: Math.round(performance.now() - startedAt),
        encodeMs,
        requestMs: attempt.requestMs,
        totalMs: Math.round(performance.now() - startedAt),
        cacheHit: encodedImage.cacheHit === true,
        usedJsonFormat: attempt.usedJsonFormat,
        descriptionLength: description.length,
        tags,
        objectsCount: objects.length,
        scene,
        confidence,
      });

      return {
        description,
        tags: tags.length > 0 ? tags : [request.language === 'ru' ? localeText("main", "ui.LocalAiProvider.bf91f52b1ae23c38") : 'photo'],
        objects,
        scene,
        confidence,
        source: 'local',
        model,
        analyzedAt: new Date().toISOString(),
        rawText,
      };
    } catch (error) {
      if (error instanceof AiProviderError) {
        throw error;
      }

      if (error instanceof DOMException && error.name === 'AbortError') {
        appLogService.error('local-ai', localeText("main", "ui.LocalAiProvider.43c1648db6aabeee"), {
          endpoint,
          model,
          fileName: request.fileName,
          elapsedMs: Math.round(performance.now() - startedAt),
          timeoutMs: LOCAL_AI_TIMEOUT_MS,
        });
        throw new AiProviderError({
          code: 'request_timeout',
          message: localeText("main", "ui.LocalAiProvider.01aaa68201501a04", {"value1": (Math.round(LOCAL_AI_TIMEOUT_MS / 1000))}),
          userMessage: localeText("main", "ui.LocalAiProvider.e43b57986c4889f5"),
          retryable: true,
          provider: 'local',
        });
      }

      if (error instanceof TypeError) {
        appLogService.error('local-ai', localeText("main", "ui.LocalAiProvider.0a145d772649cc91"), {
          endpoint,
          model,
          fileName: request.fileName,
          elapsedMs: Math.round(performance.now() - startedAt),
          errorName: error.name,
          errorMessage: error.message,
          errorStack: error.stack ?? null,
        });
        throw new AiProviderError({
          code: 'network_error',
          message: localeText("main", "ui.LocalAiProvider.0b4657b49779beee", {"value1": (error.message)}),
          userMessage: localeText("main", "ui.LocalAiProvider.9e6e940a81ea543b"),
          retryable: true,
          provider: 'local',
        });
      }

      appLogService.error('local-ai', localeText("main", "ui.LocalAiProvider.935e92e914e52977"), {
        endpoint,
        model,
        fileName: request.fileName,
        elapsedMs: Math.round(performance.now() - startedAt),
        error,
      });
      throw error;
    } finally {
      window.clearTimeout(timeoutId);
    }
  }
}
