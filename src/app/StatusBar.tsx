import {text as localeText, useCurrentLocale} from "../shared/i18n/Runtime";
import { AppStatusBar } from './layout/AppStatusBar';
import type { TaskSummary } from '../modules/taskQueue/types';

interface StatusBarProps {
  summary: TaskSummary;
  statusMessage: string;
  selectionSummary?: string;
}

export function StatusBar({ summary, statusMessage, selectionSummary = localeText("main", "ui.StatusBar.6aa046832b01bd06") }: StatusBarProps) {
  useCurrentLocale();
  return <AppStatusBar summary={summary} statusMessage={statusMessage} selectionSummary={selectionSummary} />;
}
