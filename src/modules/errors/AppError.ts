import {technicalError,commandError,commandErrorMessage} from '../../shared/tauri/CommandError';
import {text as localeText} from "../../shared/i18n/Runtime";
export type AppErrorDomain =
  | 'filesystem'
  | 'sqlite'
  | 'thumbnail'
  | 'ai'
  | 'rename'
  | 'settings'
  | 'search'
  | 'collections'
  | 'dragDrop'
  | 'unknown';

export type AppErrorSeverity = 'info' | 'warning' | 'error' | 'critical';

export interface AppErrorIssue {
  id: string;
  domain: AppErrorDomain;
  severity: AppErrorSeverity;
  title: string;
  userMessage: string;
  technicalMessage: string;
  operation?: string;
  relatedPath?: string | null;
  retryable: boolean;
  suggestions: string[];
  createdAtUnix: number;
  isDismissed: boolean;
}

export interface AppErrorContext {
  domain?: AppErrorDomain;
  operation?: string;
  title?: string;
  relatedPath?: string | null;
  severity?: AppErrorSeverity;
  retryable?: boolean;
}

function rawErrorMessage(error:unknown):string{return technicalError(error);}

function lowerMessage(error: unknown): string {
  return rawErrorMessage(error).toLowerCase();
}

function classifyDomain(error: unknown, context?: AppErrorContext): AppErrorDomain {
  if (context?.domain) return context.domain;
  const text = lowerMessage(error);
  if (text.includes('sqlite') || text.includes('database') || text.includes('rusqlite')) return 'sqlite';
  if (text.includes('thumbnail') || text.includes('image') || text.includes('decode')) return 'thumbnail';
  if (text.includes('openrouter') || text.includes('ai provider') || text.includes('api key') || text.includes('rate limit')) return 'ai';
  if (text.includes('rename') || text.includes('file exists')) return 'rename';
  if (text.includes('permission') || text.includes('access') || text.includes('not found') || text.includes('path') || text.includes('directory') || text.includes('drive')) return 'filesystem';
  return 'unknown';
}

function domainTitle(domain: AppErrorDomain): string {
  return messages().errors.titles[domain];
}

function buildUserMessage(domain: AppErrorDomain, error: unknown): string {
  if(commandError(error)?.code==='queue.target-missing')return commandErrorMessage(error);
  const technical = rawErrorMessage(error);
  const text = technical.toLowerCase();

  if (domain === 'filesystem') {
    if (text.includes('permission') || text.includes('access is denied') || text.includes('отказано в доступе')) return localeText("errors", "ui.AppError.29f83006052b5b10");
    if (text.includes('not found') || text.includes('no such file') || text.includes('cannot find') || text.includes('не удается найти') || text.includes('не удаётся найти')) return localeText("errors", "ui.AppError.46d28e810af5f5ea");
    if (text.includes('not a directory')) return localeText("errors", "ui.AppError.23e24440b42d10c1");
    if (text.includes('drive') || text.includes('device') || text.includes('network')) return localeText("errors", "ui.AppError.576d464798b50712");
    return localeText("errors", "ui.AppError.c24fc80188dc9691");
  }

  if (domain === 'sqlite') {
    if (text.includes('locked')) return localeText("errors", "ui.AppError.91b74ac1a1a5825e");
    if (text.includes('readonly') || text.includes('read-only')) return localeText("errors", "ui.AppError.d6844044b6a04117");
    if (text.includes('disk') || text.includes('full')) return localeText("errors", "ui.AppError.1c5c9e082281b8eb");
    return localeText("errors", "ui.AppError.ecd9c960ce6caeca");
  }

  if (domain === 'thumbnail') {
    if (text.includes('unsupported') || text.includes('decode') || text.includes('format')) return localeText("errors", "ui.AppError.acf00cb4cc34ca3f");
    return localeText("errors", "ui.AppError.28e58ab63d673c1a");
  }

  if (domain === 'ai') {
    return technical || localeText("errors", "ui.AppError.6316bbafc80e8a33");
  }

  if (domain === 'rename') {
    if (text.includes('exists')) return localeText("errors", "ui.AppError.f84404f2ee48cdaf");
    return localeText("errors", "ui.AppError.69bac058432731b7");
  }

  return technical || localeText("errors", "ui.AppError.048861624f97f457");
}

function suggestionsFor(domain: AppErrorDomain, error: unknown): string[] {
  const text = lowerMessage(error);
  switch (domain) {
    case 'filesystem':
      return [
        localeText("errors", "ui.AppError.cd27a98a5d6823ff"),
        localeText("errors", "ui.AppError.21a0c187383b036f"),
        text.includes('network') || text.includes('drive') ? localeText("errors", "ui.AppError.d13d7e1381af9e32") : localeText("errors", "ui.AppError.ed80b7a66aa74edc"),
      ];
    case 'sqlite':
      return [
        localeText("errors", "ui.AppError.f152db5ec31dd66a"),
        localeText("errors", "ui.AppError.96682a1c2655ac38"),
        localeText("errors", "ui.AppError.fbe2f73011fb6292"),
      ];
    case 'thumbnail':
      return messages().errors.hints.thumbnail;
    case 'ai':
      return messages().errors.hints.ai;
    case 'rename':
      return messages().errors.hints.rename;
    default:
      return [localeText("errors", "ui.AppError.19d04d1b4b2ef391"), localeText("errors", "ui.AppError.dc734e48df6b7f46")];
  }
}

function defaultSeverity(domain: AppErrorDomain): AppErrorSeverity {
  if (domain === 'sqlite' || domain === 'rename') return 'error';
  if (domain === 'ai' || domain === 'filesystem') return 'warning';
  return 'warning';
}

function defaultRetryable(domain: AppErrorDomain, error: unknown): boolean {
  const text = lowerMessage(error);
  if (domain === 'sqlite' && text.includes('locked')) return true;
  if (domain === 'filesystem' && (text.includes('network') || text.includes('drive') || text.includes('device'))) return true;
  if (domain === 'ai' && (text.includes('rate') || text.includes('timeout') || text.includes('overload') || text.includes('network'))) return true;
  return false;
}

export class AppErrorService {
  createIssue(error: unknown, context: AppErrorContext = {}): AppErrorIssue {
    const domain = classifyDomain(error, context);
    const technicalMessage = rawErrorMessage(error);
    const now = Math.floor(Date.now() / 1000);

    return {
      id: `issue:${now}:${Math.random().toString(16).slice(2, 10)}`,
      domain,
      severity: context.severity ?? defaultSeverity(domain),
      title: context.title ?? domainTitle(domain),
      userMessage: buildUserMessage(domain, error),
      technicalMessage,
      operation: context.operation,
      relatedPath: context.relatedPath ?? null,
      retryable: context.retryable ?? commandError(error)?.retryable ?? defaultRetryable(domain, error),
      suggestions: suggestionsFor(domain, error),
      createdAtUnix: now,
      isDismissed: false,
    };
  }

  toUserMessage(error: unknown, context: AppErrorContext = {}): string {
    return this.createIssue(error, context).userMessage;
  }
}

export const appErrorService = new AppErrorService();
import { messages } from '../../shared/i18n';
