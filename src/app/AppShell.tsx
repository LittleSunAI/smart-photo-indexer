import {text as localeText, useCurrentLocale} from "../shared/i18n/Runtime";
import { useEffect, useState } from 'react';
import { MainToolbar } from './layout/MainToolbar';
import { WorkspaceLayout } from './layout/WorkspaceLayout';
import { AppStatusBar } from './layout/AppStatusBar';
import { useAppActions, useAppState } from './state/AppStateContext';
import { AiReadinessController } from './state/AiReadinessController';
import {selectWorkspaceSummary} from './state/appSelectors';
import {useSelectionStatistics} from './state/useSelectionStatistics';
import{SettingsHost}from'../modules/settings/SettingsHost';
import { DragDropLayer } from '../modules/dragDrop/DragDropLayer';
import { ErrorCenter } from '../modules/errors/ErrorCenter';
import { LogPanel } from '../modules/logs/LogPanel';
import { appLogService, type AppLogEntry } from '../modules/logs/AppLogService';
import { SmartImageViewerHost } from '../modules/imageViewer';
import type { ImageItem } from '../modules/explorer/types';
import { isTauriRuntime } from '../shared/tauri/runtime';
import {BatchRenameHost} from '../modules/rename/BatchRenameHost';
import {FileActionsHost} from '../modules/filesystem/FileActionsHost';

export function AppShell() {
  useCurrentLocale();
  const state = useAppState();
  const actions = useAppActions();
  const selectionSummary = useSelectionStatistics(state);
  const imageViewerImages = isTauriRuntime() ? [] : [...state.explorerItems, ...state.search.results, ...state.dragDrop.items].filter((item): item is ImageItem => item.kind === 'image');
  const [logPanelOpen, setLogPanelOpen] = useState(false);
  const [logEntries, setLogEntries] = useState<AppLogEntry[]>(() => appLogService.getEntries());

  useEffect(() => {
    const unsubscribe = appLogService.subscribe(setLogEntries);
    const openLogs = () => {
      appLogService.info('ui', localeText("main", "ui.AppShell.2beac7d448ecf71c"));
      setLogPanelOpen(true);
    };
    const recordWindowError = (event: ErrorEvent) => {
      appLogService.error('app', event.message || localeText("main", "ui.AppShell.835b670c89b00f7a"), {
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
        stack: event.error instanceof Error ? event.error.stack : null,
      });
    };
    const recordUnhandledRejection = (event: PromiseRejectionEvent) => {
      appLogService.error('app', localeText("main", "ui.AppShell.3160cec5cf39ca47"), event.reason);
    };

    window.addEventListener('smart-photo-indexer:open-log-panel', openLogs);
    window.addEventListener('error', recordWindowError);
    window.addEventListener('unhandledrejection', recordUnhandledRejection);
    appLogService.info('app', localeText("main", "ui.AppShell.c88364f87ccd4af1"));

    return () => {
      window.removeEventListener('smart-photo-indexer:open-log-panel', openLogs);
      window.removeEventListener('error', recordWindowError);
      window.removeEventListener('unhandledrejection', recordUnhandledRejection);
      unsubscribe();
    };
  }, []);

  return (
    <div className="app-shell">
      <AiReadinessController />
      <BatchRenameHost/>
      <FileActionsHost/>
      <MainToolbar />

      <SettingsHost/>

      {logPanelOpen ? <LogPanel entries={logEntries} onClose={() => setLogPanelOpen(false)} /> : null}

      <ErrorCenter issues={state.appIssues} onDismiss={actions.dismissAppIssue} onClearAll={actions.clearAppIssues} />
      <WorkspaceLayout />
      {!isTauriRuntime() ? (
        <SmartImageViewerHost
          images={imageViewerImages}
          selectedImageId={state.selectedItemId}
          onToggleFavorite={(itemId) => {
            void actions.toggleFavoriteForItem(itemId);
          }}
        />
      ) : null}
      <DragDropLayer />
      <AppStatusBar summary={selectWorkspaceSummary(state)} statusMessage={state.statusMessage} statusSeverity={state.statusSeverity} selectionSummary={selectionSummary} viewSize={state.viewSize} onViewSizeChange={actions.setViewSize} />
    </div>
  );
}
