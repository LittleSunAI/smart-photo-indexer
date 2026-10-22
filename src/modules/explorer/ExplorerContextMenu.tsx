import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useAppSelector} from '../../app/state/AppStateContext';
import { useEffect, useLayoutEffect, useRef } from 'react';
import type { ExplorerContextMenuState } from '../../app/types';
import type { ExplorerItem } from './types';
import { messages } from '../../shared/i18n';
import {openBatchRename} from '../rename/BatchRenameService';
import {fileSelectionCount,pasteFolder,requestFileCommand,type FileCommand} from '../filesystem/FileActions';
import {useFileActions} from '../filesystem/FileActionsService';

interface ExplorerContextMenuProps {
  menu: ExplorerContextMenuState;
  targetItem: ExplorerItem | null;
  selectedCount: number;
  selectedImageCount: number;
  visibleItemIds: string[];
  onOpenTarget(): void;
  onRevealTarget(): void;
  onSelectAll(visibleItemIds: string[]): void;
  onClearSelection(): void;
  onQueueSelected(): void;
  onClose(): void;
  onEditSelected(): void;
  onCompare?():void;
}

function getOpenLabel(targetItem: ExplorerItem | null): string {
  if (!targetItem) return messages().explorer.open.generic;
  if (targetItem.kind === 'folder') return messages().explorer.open.folder;
  if (targetItem.kind === 'image') return messages().explorer.open.image;
  return messages().explorer.open.file;
}

export function ExplorerContextMenu({
  menu,
  targetItem,
  selectedCount,
  selectedImageCount,
  visibleItemIds,
  onOpenTarget,
  onRevealTarget,
  onSelectAll,
  onClearSelection,
  onQueueSelected,
  onClose,
  onEditSelected,
  onCompare,
}: ExplorerContextMenuProps) {
  useCurrentLocale();
  const menuRef = useRef<HTMLDivElement>(null);
  const manual=useAppSelector(state=>state.settings.processingLimits.manualQueue);
  const fileCount=useAppSelector(fileSelectionCount),destination=useAppSelector(pasteFolder);
  const {clipboard,operation,preparing}=useFileActions();
  const fileBusy=preparing||Boolean(operation&&['running','awaitingConflict'].includes(operation.state));
  const fileCommand=(command:FileCommand)=>{onClose();requestFileCommand(command);};
  useLayoutEffect(() => {
    const element = menuRef.current;
    if (!menu.isOpen || !element) return;
    const margin = 8;
    const width = document.documentElement.clientWidth;
    const height = document.documentElement.clientHeight;
    element.style.maxWidth = `${Math.max(0, width - margin * 2)}px`;
    element.style.maxHeight = `${Math.max(0, height - margin * 2)}px`;
    element.style.overflowY = 'auto';
    const bounds = element.getBoundingClientRect();
    element.style.left = `${Math.max(margin, Math.min(menu.x, width - bounds.width - margin))}px`;
    element.style.top = `${Math.max(margin, Math.min(menu.y, height - bounds.height - margin))}px`;
  }, [menu.isOpen, menu.x, menu.y, selectedCount, selectedImageCount]);
  useEffect(() => {
    if (!menu.isOpen) return undefined;

    const closeFromDocument = () => onClose();
    const closeFromEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };

    window.addEventListener('click', closeFromDocument);
    window.addEventListener('contextmenu', closeFromDocument);
    window.addEventListener('keydown', closeFromEscape);
    window.addEventListener('resize', closeFromDocument);

    return () => {
      window.removeEventListener('click', closeFromDocument);
      window.removeEventListener('contextmenu', closeFromDocument);
      window.removeEventListener('keydown', closeFromEscape);
      window.removeEventListener('resize', closeFromDocument);
    };
  }, [menu.isOpen, onClose]);

  if (!menu.isOpen) return null;

  const hasTarget = Boolean(targetItem);
  const canQueue = selectedImageCount > 0;
  const canReveal = hasTarget && targetItem?.kind !== 'folder';

  return (
    <div
      ref={menuRef}
      className="explorer-context-menu"
      style={{ left: menu.x, top: menu.y }}
      role="menu"
      aria-label={messages().explorer.contextMenuAria}
      onClick={(event) => event.stopPropagation()}
      onContextMenu={(event) => event.preventDefault()}
    >
      <div className="explorer-context-menu__header">
        <strong>{hasTarget ? targetItem?.name : messages().workspace.mode}</strong>
        <span>{selectedCount > 0 ? messages().workspace.selected(selectedCount) : messages().workspace.noSelection}</span>
      </div>

      <Hint message={{id:"main.open-file"}}><button type="button" role="menuitem" disabled={!hasTarget} onClick={onOpenTarget}>{getOpenLabel(targetItem)}</button></Hint>
      <Hint message={{id:"common.reveal-file"}}><button type="button" role="menuitem" disabled={!canReveal} onClick={onRevealTarget}>{messages().explorer.open.reveal}</button></Hint>
      <Hint message={{id:"main.rename"}}><button type="button" role="menuitem" disabled={!hasTarget&&!selectedCount} onClick={()=>{const persistent=document.querySelector<HTMLButtonElement>('.toolbar-batch-rename');persistent?.focus();openBatchRename(selectedCount?undefined:targetItem?[targetItem.path]:undefined);onClose();}}>{localeText("main", "ui.ExplorerContextMenu.c43fa2f0e6e58b90")}</button></Hint>
      {selectedImageCount===2&&onCompare?<Hint message={{id:"main.compare"}}><button type="button" role="menuitem" onClick={onCompare}>{localeText("main", "ui.ExplorerContextMenu.c2b009cb356b964d")}</button></Hint>:null}
      <div className="explorer-context-menu__separator" />
      <Hint message={{id:'files.copy'}}><button type="button" role="menuitem" disabled={!fileCount||preparing} onClick={()=>fileCommand('copy')}>{localeText("main", "ui.ExplorerContextMenu.fe143cf9ad6a34fc")}<span className="file-menu-key">{localeText("main", "ui.ExplorerContextMenu.a39ff1a48c65e915")}</span></button></Hint>
      <Hint message={{id:'files.cut'}}><button type="button" role="menuitem" disabled={!fileCount||preparing} onClick={()=>fileCommand('cut')}>{localeText("main", "ui.ExplorerContextMenu.10237513cb7cdf9f")}<span className="file-menu-key">{localeText("main", "ui.ExplorerContextMenu.2ee503549395564b")}</span></button></Hint>
      <Hint message={{id:clipboard.kind==='move'?'files.paste-move':'files.paste-copy'}}><button type="button" role="menuitem" disabled={!clipboard.ready||!destination||fileBusy} onClick={()=>fileCommand('paste')}>{localeText("main", "ui.ExplorerContextMenu.ae5cd7c2157b64c4")}<span className="file-menu-key">{localeText("main", "ui.ExplorerContextMenu.29d0cf5a57fe0046")}</span></button></Hint>
      {!destination?<span className="explorer-context-menu__note">{localeText("main", "ui.ExplorerContextMenu.68178b38f9446ecc")}</span>:null}
      <Hint message={{id:'files.copy-to'}}><button type="button" role="menuitem" disabled={!fileCount||fileBusy} onClick={()=>fileCommand('copyTo')}>{localeText("main", "ui.ExplorerContextMenu.9b50ba2f78994302")}</button></Hint>
      <Hint message={{id:'files.move-to'}}><button type="button" role="menuitem" disabled={!fileCount||fileBusy} onClick={()=>fileCommand('moveTo')}>{localeText("main", "ui.ExplorerContextMenu.30282c8325fff283")}</button></Hint>
      <Hint message={{id:'files.trash'}}><button type="button" role="menuitem" disabled={!fileCount||fileBusy} onClick={()=>fileCommand('trash')}>{localeText("main", "ui.ExplorerContextMenu.8eca318080b9d979")}<span className="file-menu-key">{localeText("main", "ui.ExplorerContextMenu.1369440a8d9d49e2")}</span></button></Hint>
      <div className="explorer-context-menu__separator" />
      <Hint message={{id:"main.select-all"}}><button type="button" role="menuitem" onClick={() => onSelectAll(visibleItemIds)}>{messages().explorer.selectAllVisible}</button></Hint>
      <Hint message={{id:"main.clear-selection"}}><button type="button" role="menuitem" disabled={selectedCount === 0} onClick={onClearSelection}>{messages().explorer.clearSelection}</button></Hint>
      <div className="explorer-context-menu__separator" />
      <Hint message={{id:manual?"main.process-manual":"main.process-direct"}}><button type="button" role="menuitem" disabled={!canQueue} onClick={onQueueSelected}>{messages().explorer.queueSelected}</button></Hint>
      <Hint message={{id:"main.edit-photo"}} supplement={selectedCount!==1||selectedImageCount!==1?{id:"main.edit-one-required"}:undefined}><button data-testid="explorer-edit-in-editor" type="button" role="menuitem" disabled={selectedCount!==1||selectedImageCount!==1} onClick={onEditSelected}>{localeText("main", "explorer.editInEditor")}</button></Hint>
      {selectedCount > 0 && selectedImageCount === 0 ? <span className="explorer-context-menu__note">{messages().explorer.selectedItemsDoNotContainImages}</span> : null}
    </div>
  );
}
