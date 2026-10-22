import {nativeDisplayText} from "../../shared/i18n/NativeMessages";
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import type { TaskSummary } from '../../modules/taskQueue/types';
import { messages } from '../../shared/i18n';
import type { ViewSize } from '../types';

interface AppStatusBarProps {
  summary: TaskSummary;
  statusMessage: string;
  statusSeverity?:'info'|'warning'|'error'|'critical';
  selectionSummary: string;
  viewSize?: ViewSize;
  onViewSizeChange?(value: ViewSize): void;
}

const viewSizes: ViewSize[] = ['table', 'list', 'small', 'medium', 'large', 'extraLarge'];
export function AppStatusBar({ summary, statusMessage, statusSeverity, selectionSummary, viewSize, onViewSizeChange }: AppStatusBarProps) {
  useCurrentLocale();
  const important=statusSeverity==='error'||statusSeverity==='critical';
  const message=important?statusMessage:'';
  return (
    <footer data-tour-id="main.footer.status" className="app-status-bar" aria-label={messages().statusBar.aria}>
      <span className="app-status-bar__entry" title={messages().statusBar.items(summary.totalItems)}>{messages().statusBar.items(summary.totalItems)}</span>
      <span className="app-status-bar__entry"><i className="dot processed" /> {summary.processed} {messages().statusBar.processed}</span>
      <span className="app-status-bar__entry"><i className="dot queued" /> {summary.inQueue} {messages().statusBar.inQueue}</span>
      <span className="app-status-bar__entry"><i className="dot notProcessed" /> {summary.notProcessed} {messages().statusBar.notProcessed}</span>
      <span className="app-status-bar__entry"><i className="dot error" /> {summary.errors} {nativeDisplayText(messages().statusBar.error??'')}</span>
      <span className="app-status-bar__selection" title={selectionSummary}>{selectionSummary}</span>
      {message?<span className="app-status-bar__message" title={message} data-important={important}>{message}</span>:null}

      {viewSize && onViewSizeChange ? <Hint message={{id:"main.thumbnail-size"}}><label data-tour-id="main.footer.view" className="app-status-bar__zoom">
        <span title={messages().statusBar.thumbnailSize}>{localeText("main", "ui.AppStatusBar.8835f399b05b5cc4")}</span>
        <input type="range" min="1" max="6" step="1" value={viewSizes.indexOf(viewSize) + 1} aria-label={messages().statusBar.thumbnailSize} aria-valuetext={({table:localeText("main", "ui.AppStatusBar.8195b6dc27e9d8f5"),list:localeText("main", "ui.AppStatusBar.5dfc2d28918ca4f0"),small:localeText("main", "ui.AppStatusBar.60a8417afe42850e"),medium:localeText("main", "ui.AppStatusBar.d927f09757069ff1"),large:localeText("main", "ui.AppStatusBar.3e7c1a30c3b56e9e"),extraLarge:localeText("main", "ui.AppStatusBar.d91ab0a1465f02a5")})[viewSize]} onChange={event => onViewSizeChange(viewSizes[Number(event.target.value) - 1])} />
      </label></Hint> : null}
    </footer>
  );
}
