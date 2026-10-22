import {text as localeText} from "../../shared/i18n/Runtime";
import {invoke} from "../../shared/tauri/invoke";
import { isTauriRuntime } from '../../shared/tauri/runtime';
import type { ActionJournalInput, ActionJournalResult, CreateActionLogEntryInput, CreateActionLogEntryResult } from './ActionLog.types';

export interface ActionLogService {
  getActionJournal(input?: ActionJournalInput): Promise<ActionJournalResult>;
  createActionLogEntry(input: CreateActionLogEntryInput): Promise<CreateActionLogEntryResult>;
}

function browserJournal(input: ActionJournalInput = {}): ActionJournalResult {
  const now = Math.floor(Date.now() / 1000);
  const entries = [
    {
      id: 'browser-preview:journal',
      source: 'browser_preview',
      actionType: 'browser_preview',
      imageId: input.imageId ?? null,
      imageFileName: null,
      imagePath: null,
      oldValue: null,
      newValue: null,
      message: localeText("journal", "ui.ActionLogService.270622d1f92a6d3c"),
      createdAtUnix: now,
    },
  ];

  return {
    totalCount: entries.length,
    returnedCount: entries.length,
    entries,
    loadedAtUnix: now,
  };
}

class TauriActionLogService implements ActionLogService {
  async getActionJournal(input: ActionJournalInput = {}): Promise<ActionJournalResult> {
    if (!isTauriRuntime()) {
      return browserJournal(input);
    }

    return invoke<ActionJournalResult>('get_action_journal', { input });
  }

  async createActionLogEntry(input: CreateActionLogEntryInput): Promise<CreateActionLogEntryResult> {
    if (!isTauriRuntime()) {
      return { actionLogId: Date.now(), createdAtUnix: Math.floor(Date.now() / 1000) };
    }

    return invoke<CreateActionLogEntryResult>('create_action_log_entry', { input });
  }
}

export const actionLogService: ActionLogService = new TauriActionLogService();
