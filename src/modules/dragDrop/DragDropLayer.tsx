import {nativeDisplayText} from "../../shared/i18n/NativeMessages";
import {useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {Icon} from '../../shared/ui/Icon';
import { useEffect, useMemo, useState, type DragEvent } from 'react';
import { listen, TauriEvent, type UnlistenFn } from '@tauri-apps/api/event';
import { useAppActions, useAppState } from '../../app/state/AppStateContext';
import type { ExplorerItem } from '../explorer/types';
import type { PhotoCollection } from '../collections/types';
import { messages, pluralizeRu } from '../../shared/i18n';

interface TauriDragDropPayload {
  paths?: string[];
}

export function DragDropLayer() {
  useCurrentLocale();
  const state = useAppState();
  const actions = useAppActions();
  const [collectionId, setCollectionId] = useState('');

  useEffect(() => {
    let isDisposed = false;
    const unlisteners: UnlistenFn[] = [];

    async function subscribe<T>(event: string, handler: (event: { payload: T }) => void) {
      if (isDisposed) return;
      const unlisten = await listen<T>(event, (value) => {
        if (!isDisposed) handler(value);
      });
      if (isDisposed) unlisten();
      else unlisteners.push(unlisten);
    }

    async function attachTauriDragEvents() {
      try {
        await subscribe(TauriEvent.DRAG_ENTER, () => actions.setDragOver(true));
        await subscribe(TauriEvent.DRAG_OVER, () => actions.setDragOver(true));
        await subscribe(TauriEvent.DRAG_LEAVE, () => actions.setDragOver(false));
        await subscribe<TauriDragDropPayload | string[] | unknown>(TauriEvent.DRAG_DROP, (event) => {
          const paths = extractPathsFromTauriPayload(event.payload);
          if (paths.length === 0) {
            actions.setStatusMessage(messages().dragDrop.noFilePaths);
            actions.setDragOver(false);
            return;
          }
          void actions.handleDroppedPaths(paths);
        });
      } catch (error) {
        if (!isDisposed) {
          actions.setStatusMessage(messages().dragDrop.attachFailed(getErrorMessage(error)), 'error');
        }
      }
    }

    void attachTauriDragEvents();

    return () => {
      isDisposed = true;
      for (const unlisten of unlisteners) {
        try {
          unlisten();
        } catch {
          // Listener cleanup should never break the app shell.
        }
      }
    };
  }, [actions]);

  const hasDropPanel = state.dragDrop.isInspecting || state.dragDrop.items.length > 0 || Boolean(state.dragDrop.errorMessage);
  const droppedImages = useMemo(
    () => state.dragDrop.items.filter((item) => item.kind === 'image'),
    [state.dragDrop.items],
  );
  const firstFolder = useMemo(
    () => state.dragDrop.items.find((item) => item.kind === 'folder') ?? null,
    [state.dragDrop.items],
  );

  const handleDomDragOver = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'copy';
    actions.setDragOver(true);
  };

  const handleDomDrop = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    const paths = extractPathsFromDataTransfer(event.dataTransfer);
    if (paths.length === 0) {
      actions.setStatusMessage(messages().dragDrop.noReadablePaths);
      actions.setDragOver(false);
      return;
    }
    void actions.handleDroppedPaths(paths);
  };

  return (
    <div
      className="drag-drop-root"
      onDragOver={handleDomDragOver}
      onDragEnter={() => actions.setDragOver(true)}
      onDragLeave={(event) => {
        if (event.currentTarget === event.target) {
          actions.setDragOver(false);
        }
      }}
      onDrop={handleDomDrop}
      aria-live="polite"
    >
      {state.dragDrop.isDraggingOver ? (
        <div className="drag-drop-overlay" role="status">
          <div className="drag-drop-overlay__card">
            <strong>{messages().dragDrop.overlayTitle}</strong>
            <span>{messages().dragDrop.overlayText}</span>
          </div>
        </div>
      ) : null}

      {hasDropPanel ? (
        <aside className="drag-drop-panel" aria-label={messages().dragDrop.panelAria}>
          <div className="drag-drop-panel__header">
            <div>
              <strong>{messages().dragDrop.panelTitle}</strong>
              <span>{messages().dragDrop.summary(state.dragDrop.folderCount, state.dragDrop.imageCount, state.dragDrop.fileCount, state.dragDrop.unsupportedCount)}</span>
            </div>
            <Hint message={{id:"common.close-dialog"}}><button type="button" onClick={actions.clearDroppedPaths} aria-label={messages().dragDrop.closePanel}><Icon name="close"/></button></Hint>
          </div>

          {state.dragDrop.isInspecting ? (
            <p className="drag-drop-panel__muted">{messages().dragDrop.inspecting}</p>
          ) : null}

          {state.dragDrop.errorMessage ? (
            <p className="drag-drop-panel__error">{nativeDisplayText(state.dragDrop.errorMessage??'')}</p>
          ) : null}

          {state.dragDrop.items.length > 0 ? (
            <>
              <div className="drag-drop-panel__actions">
                <Hint message={{id:"drop.open-folder"}}><button type="button" disabled={!firstFolder} onClick={() => void actions.openFirstDroppedFolder()}>
                  {messages().dragDrop.openFirstFolder}
                </button></Hint>
                <Hint message={{id:"drop.select-images"}}><button type="button" disabled={droppedImages.length === 0} onClick={actions.selectDroppedImages}>
                  {messages().dragDrop.selectImages}
                </button></Hint>
                <Hint message={{id:"drop.process"}}><button type="button" disabled={droppedImages.length === 0} onClick={actions.queueDroppedImages}>
                  {messages().dragDrop.processDropped}
                </button></Hint>
              </div>

              <div className="drag-drop-panel__collection-row">
                <Hint message={{id:"drop.collection"}}><select value={collectionId} onChange={(event) => setCollectionId(event.target.value)} disabled={state.collections.items.length === 0 || droppedImages.length === 0}>
                  <option value="">{messages().dragDrop.addToCollection}</option>
                  {state.collections.items.map((collection: PhotoCollection) => (
                    <option key={collection.id} value={collection.id}>{collection.name}</option>
                  ))}
                </select></Hint>
                <Hint message={{id:"drop.add"}}><button
                  type="button"
                  disabled={!collectionId || droppedImages.length === 0}
                  onClick={() => { void actions.addDroppedImagesToCollection(collectionId); }}
                >
                  {messages().dragDrop.add}
                </button></Hint>
              </div>

              <ul className="drag-drop-panel__list">
                {state.dragDrop.items.slice(0, 8).map((item) => (
                  <li key={item.id} title={item.path}>
                    <span className={`drag-drop-panel__kind drag-drop-panel__kind--${item.kind}`}>{getKindLabel(item)}</span>
                    <span>{item.name}</span>
                  </li>
                ))}
                {state.dragDrop.items.length > 8 ? <li className="drag-drop-panel__muted">{messages().dragDrop.more(state.dragDrop.items.length - 8)}</li> : null}
              </ul>
            </>
          ) : null}
        </aside>
      ) : null}
    </div>
  );
}

function extractPathsFromTauriPayload(payload: unknown): string[] {
  if (Array.isArray(payload)) {
    return payload.filter((item): item is string => typeof item === 'string' && item.trim().length > 0);
  }

  if (payload && typeof payload === 'object' && 'paths' in payload) {
    const paths = (payload as TauriDragDropPayload).paths;
    if (Array.isArray(paths)) {
      return paths.filter((item): item is string => typeof item === 'string' && item.trim().length > 0);
    }
  }

  return [];
}

function extractPathsFromDataTransfer(dataTransfer: DataTransfer): string[] {
  const paths: string[] = [];
  for (const file of Array.from(dataTransfer.files)) {
    const path = (file as File & { path?: string }).path || file.webkitRelativePath || file.name;
    if (path && path.trim()) paths.push(path.trim());
  }
  return Array.from(new Set(paths));
}

function buildDropSummary(imageCount: number, folderCount: number, fileCount: number, unsupportedCount: number): string {
  return messages().dragDrop.summary(folderCount, imageCount, fileCount, unsupportedCount);
}

function getKindLabel(item: ExplorerItem): string {
  if (item.kind === 'folder') return messages().dragDrop.kind.folder;
  if (item.kind === 'image') return messages().dragDrop.kind.image;
  return messages().dragDrop.kind.file;
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return messages().dragDrop.unknownError;
}
