import {text as localeText} from "../../shared/i18n/Runtime";
import {invoke} from "../../shared/tauri/invoke";
import { isTauriRuntime } from '../../shared/tauri/runtime';
import { appLogService } from '../logs/AppLogService';

export interface AiImageDataUrlResult {
  sourcePath: string;
  dataUrl: string;
  mimeType: string;
  originalSizeBytes: number;
  encodedSizeBytes: number;
  width: number | null;
  height: number | null;
  wasResized: boolean;
  encodedAtUnix: number;
  cacheHit?: boolean;
  cacheKey?: string;
}

interface AiImageFileFingerprintResult {
  sourcePath: string;
  sizeBytes: number;
  modifiedAtUnix: number | null;
}

interface AiImageEncodingCacheEntry {
  key: string;
  result: AiImageDataUrlResult;
  encodedSizeBytes: number;
  lastUsedAt: number;
}

const AI_IMAGE_ENCODING_CACHE_VERSION = 'ai-jpeg-1600-q88-v1';
const AI_IMAGE_ENCODING_MAX_CACHE_ITEMS = 30;
const AI_IMAGE_ENCODING_MAX_CACHE_BYTES = 180 * 1024 * 1024;

function makeAiImageEncodingCacheKey(fingerprint: AiImageFileFingerprintResult): string {
  return [
    fingerprint.sourcePath,
    fingerprint.sizeBytes,
    fingerprint.modifiedAtUnix ?? 0,
    AI_IMAGE_ENCODING_CACHE_VERSION,
  ].join('|');
}

export interface AiImageEncodingService {
  encodeImageForAi(imagePath: string): Promise<AiImageDataUrlResult>;
}

export class TauriAiImageEncodingService implements AiImageEncodingService {
  private readonly cache = new Map<string, AiImageEncodingCacheEntry>();
  private readonly inFlight = new Map<string, Promise<AiImageDataUrlResult>>();
  private totalCacheBytes = 0;

  async encodeImageForAi(imagePath: string): Promise<AiImageDataUrlResult> {
    const safePath = imagePath.trim();
    if (!safePath) {
      throw new Error(localeText("main", "ui.AiImageEncodingService.488eda733a2c97f7"));
    }

    if (!isTauriRuntime()) {
      throw new Error(localeText("main", "ui.AiImageEncodingService.1930c27fd7fb2762"));
    }

    const startedAt = performance.now();

    let fingerprint: AiImageFileFingerprintResult | null = null;
    let cacheKey: string | null = null;

    try {
      fingerprint = await invoke<AiImageFileFingerprintResult>('get_ai_image_file_fingerprint', { imagePath: safePath });
      cacheKey = makeAiImageEncodingCacheKey(fingerprint);
    } catch (error) {
      appLogService.warning('task', localeText("main", "ui.AiImageEncodingService.5de2fb69abf40ceb"), {
        imagePath: safePath,
        error: error instanceof Error ? error.message : String(error),
      });
    }

    if (cacheKey) {
      const cached = this.cache.get(cacheKey);
      if (cached) {
        cached.lastUsedAt = Date.now();
        appLogService.info('task', localeText("main", "ui.AiImageEncodingService.19384ff1aff5029a"), {
          imagePath: safePath,
          cacheKey,
          encodedSizeBytes: cached.encodedSizeBytes,
          elapsedMs: Math.round(performance.now() - startedAt),
        });

        return {
          ...cached.result,
          cacheHit: true,
          cacheKey,
        };
      }

      const running = this.inFlight.get(cacheKey);
      if (running) {
        appLogService.info('task', localeText("main", "ui.AiImageEncodingService.15b5597955fc0956"), {
          imagePath: safePath,
          cacheKey,
        });
        const result = await running;
        return {
          ...result,
          cacheHit: true,
          cacheKey,
        };
      }
    }

    const encodePromise = invoke<AiImageDataUrlResult>('encode_image_for_ai', { imagePath: safePath })
      .then((result) => {
        const normalizedResult: AiImageDataUrlResult = {
          ...result,
          cacheHit: false,
          cacheKey: cacheKey ?? undefined,
        };

        if (cacheKey) {
          this.storeInCache(cacheKey, normalizedResult);
        }

        appLogService.info('task', localeText("main", "ui.AiImageEncodingService.f258581d847ce64e"), {
          imagePath: safePath,
          cacheKey,
          cacheHit: false,
          originalSizeBytes: result.originalSizeBytes,
          encodedSizeBytes: result.encodedSizeBytes,
          width: result.width,
          height: result.height,
          wasResized: result.wasResized,
          elapsedMs: Math.round(performance.now() - startedAt),
        });

        return normalizedResult;
      })
      .finally(() => {
        if (cacheKey) {
          this.inFlight.delete(cacheKey);
        }
      });

    if (cacheKey) {
      this.inFlight.set(cacheKey, encodePromise);
    }

    return encodePromise;
  }

  private storeInCache(key: string, result: AiImageDataUrlResult): void {
    const existing = this.cache.get(key);
    if (existing) {
      this.totalCacheBytes -= existing.encodedSizeBytes;
    }

    const encodedSizeBytes = result.encodedSizeBytes;
    this.cache.set(key, {
      key,
      result: { ...result, cacheHit: false, cacheKey: key },
      encodedSizeBytes,
      lastUsedAt: Date.now(),
    });
    this.totalCacheBytes += encodedSizeBytes;

    this.evictOldEntries();
  }

  private evictOldEntries(): void {
    while (
      this.cache.size > AI_IMAGE_ENCODING_MAX_CACHE_ITEMS
      || this.totalCacheBytes > AI_IMAGE_ENCODING_MAX_CACHE_BYTES
    ) {
      const oldest = Array.from(this.cache.values())
        .sort((left, right) => left.lastUsedAt - right.lastUsedAt)[0];

      if (!oldest) return;

      this.cache.delete(oldest.key);
      this.totalCacheBytes -= oldest.encodedSizeBytes;

      appLogService.info('task', localeText("main", "ui.AiImageEncodingService.b86039c856fd12c2"), {
        cacheKey: oldest.key,
        remainingItems: this.cache.size,
        totalCacheBytes: this.totalCacheBytes,
      });
    }
  }
}

export const aiImageEncodingService: AiImageEncodingService = new TauriAiImageEncodingService();
