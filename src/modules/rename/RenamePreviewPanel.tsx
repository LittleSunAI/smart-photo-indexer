import {nativeDisplayText} from "../../shared/i18n/NativeMessages";
import {plural} from '../../shared/i18n/Formatting';
import {text as localeText} from "../../shared/i18n/Runtime";
import {useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import type { RenamePreviewState } from '../../app/types';
import type { TaskQueueState } from '../taskQueue/types';
import { messages, pluralizeRu } from '../../shared/i18n';

interface RenamePreviewPanelProps {
  renamePreview: RenamePreviewState;
  taskQueue: TaskQueueState;
  onConfirm(): void;
  onCancel(): void;
}

export function RenamePreviewPanel({ renamePreview, taskQueue, onConfirm, onCancel }: RenamePreviewPanelProps) {
  useCurrentLocale();
  const result = renamePreview.result;
  const copy=renamePreview.config?.analysisCopy?.enabled;
  const hasBlockingProblems = Boolean(result && (result.errors > 0 || result.conflicts > 0));
  const canConfirm = Boolean(result && !hasBlockingProblems && !renamePreview.isPreparing && taskQueue.runState !== 'running');

  if (!renamePreview.isPreparing && !result && !renamePreview.errorMessage) return null;

  return (
    <div className={hasBlockingProblems ? 'rename-preview-card rename-preview-card--blocked' : 'rename-preview-card'}>
      <div className="section-header">
        <strong>{messages().rename.title}</strong>
        <span>{renamePreview.isPreparing ? messages().rename.preparing : result ? plural('items',result.total) : messages().rename.notReady}</span>
      </div>

      <p className="metadata-edit-note">
        {copy?localeText("rename", "ui.RenamePreviewPanel.49f3f488652b11f4"):messages().rename.smartRenameEnabled}
      </p>

      {renamePreview.errorMessage ? <p className="danger">{messages().rename.previewFailed}: {nativeDisplayText(renamePreview.errorMessage??'')}</p> : null}

      {result ? (
        <>
          <div className="rename-preview-summary">
            <span><strong>{result.ready}</strong><small>{copy?localeText("rename", "ui.RenamePreviewPanel.4bb71ddd32bea1cf"):messages().rename.willRename}</small></span>
            <span><strong>{result.unchanged}</strong><small>{messages().rename.alreadyOK}</small></span>
            <span><strong>{result.conflicts}</strong><small>{messages().rename.conflicts}</small></span>
            <span><strong>{result.errors}</strong><small>{messages().rename.errors}</small></span>
          </div>

          <div className="rename-preview-list">
            {result.items.slice(0, 10).map((item) => (
              <div key={`${item.oldPath}-${item.newFileName}`} className={`rename-preview-row rename-preview-row--${item.status}`}>
                <span className="rename-preview-row__status">{item.status}</span>
                <span className="rename-preview-row__old" title={item.oldPath}>{item.oldFileName}</span>
                <span className="rename-preview-row__arrow">→</span>
                <span className="rename-preview-row__new" title={item.newPath}>{item.newFileName}</span>
                {item.candidateCollisions > 0 ? <span className="rename-preview-row__note">{item.candidateCollisions} {messages().rename.occupiedSkipped}</span> : null}
                <span className="rename-preview-row__note" title={item.message}>{item.message}</span>
              </div>
            ))}
          </div>

          {result.items.length > 10 ? <p className="metadata-edit-note">{messages().rename.showingFirstRows(10, result.items.length)}</p> : null}
          {hasBlockingProblems ? <p className="danger">{messages().rename.blockedByProblems}</p> : null}
        </>
      ) : null}

      <div className="metadata-edit-actions">
        <Hint message={{id:"rename.preview-cancel"}}><button type="button" className="secondary-action" disabled={renamePreview.isPreparing || taskQueue.runState === 'running'} onClick={onCancel}>{messages().rename.cancel}</button></Hint>
        <Hint message={{id:"rename.preview-confirm"}}><button type="button" className="primary-action" disabled={!canConfirm} onClick={onConfirm}>{messages().rename.confirm}</button></Hint>
      </div>
    </div>
  );
}
