import {text as localeText} from "../../shared/i18n/Runtime";
export type AiProviderErrorCode =
  | 'missing_api_key'
  | 'invalid_api_key'
  | 'insufficient_credits'
  | 'rate_limited'
  | 'provider_overloaded'
  | 'request_timeout'
  | 'network_error'
  | 'invalid_request'
  | 'invalid_response'
  | 'provider_error'
  | 'unknown_error';

export interface AiProviderErrorOptions {
  code: AiProviderErrorCode;
  message: string;
  userMessage?: string;
  retryable?: boolean;
  statusCode?: number;
  retryAfterMs?: number;
  provider?: string;
  rawMessage?: string;
}

export class AiProviderError extends Error {
  readonly code: AiProviderErrorCode;
  readonly userMessage: string;
  readonly retryable: boolean;
  readonly statusCode?: number;
  readonly retryAfterMs?: number;
  readonly provider?: string;
  readonly rawMessage?: string;

  constructor(options: AiProviderErrorOptions) {
    super(options.message);
    this.name = 'AiProviderError';
    this.code = options.code;
    this.userMessage = options.userMessage ?? options.message;
    this.retryable = options.retryable ?? false;
    this.statusCode = options.statusCode;
    this.retryAfterMs = options.retryAfterMs;
    this.provider = options.provider;
    this.rawMessage = options.rawMessage;
  }
}

export interface NormalizedAiError {
  message: string;
  code: AiProviderErrorCode;
  retryable: boolean;
  retryAfterMs?: number;
  statusCode?: number;
}

export function normalizeAiError(error: unknown): NormalizedAiError {
  if (error instanceof AiProviderError) {
    return {
      message: error.userMessage,
      code: error.code,
      retryable: error.retryable,
      retryAfterMs: error.retryAfterMs,
      statusCode: error.statusCode,
    };
  }

  if (error instanceof DOMException && error.name === 'AbortError') {
    return {
      message: localeText("main", "ui.AiProviderError.840ee82a164118f7"),
      code: 'request_timeout',
      retryable: true,
    };
  }

  if (error instanceof TypeError) {
    return {
      message: localeText("main", "ui.AiProviderError.9517084cfc6edd2d", {"value1": (error.message)}),
      code: 'network_error',
      retryable: true,
    };
  }

  if (error instanceof Error) {
    return {
      message: error.message,
      code: 'unknown_error',
      retryable: false,
    };
  }

  if (typeof error === 'string') {
    return {
      message: error,
      code: 'unknown_error',
      retryable: false,
    };
  }

  return {
    message: localeText("main", "ui.AiProviderError.2eb593a21ffe5be2"),
    code: 'unknown_error',
    retryable: false,
  };
}
