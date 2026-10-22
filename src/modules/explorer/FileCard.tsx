import {plural} from '../../shared/i18n/Formatting';
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {Icon} from '../../shared/ui/Icon';
import type { MouseEvent } from 'react';
import type { ExplorerItem, ImageItem, ProcessingStatus } from './types';
import { CachedImage } from '../thumbnail/CachedImage';
import { messages, pluralizeRu } from '../../shared/i18n';

const statusLabel: Record<ProcessingStatus, string> = {
  get processed() { return messages().explorer.statusLabels.processed; },
  get queued() { return messages().explorer.statusLabels.queued; },
  get notProcessed() { return messages().explorer.statusLabels.notProcessed; },
  get error() { return messages().explorer.statusLabels.error; },
  get relocated() { return messages().explorer.statusLabels.relocated; },
  get missing() { return messages().explorer.statusLabels.missing; },
};

interface FileCardProps {
  item: ExplorerItem;
  isSelected: boolean;
  isFocused: boolean;
  onSelect(event: MouseEvent): void;
  onOpen(): void;
  onContextMenu(event: MouseEvent): void;
  onToggleFavorite?(itemId: string): void;
}

function ImageThumbnail({ item }: { item: ImageItem }) {
  useCurrentLocale();
  return (
    <CachedImage
      className="real-photo-thumbnail"
      imagePath={item.path}
      revision={item.sourceRevision ?? `${item.modifiedAtUnix}:${item.sizeBytes}`}
      originalPreviewUrl={item.previewUrl}
      fallbackClassName={`mock-photo mock-photo-${item.theme ?? 'birthday'}`}
      fallbackLabel={item.theme ?? localeText("main", "ui.FileCard.0c76c4c78b859df8")}
      variant="card"
      width={360}
      height={260}
    />
  );
}

function SelectionBadge({ isSelected }: { isSelected: boolean }) {
  return <span className={`selection-check ${isSelected ? 'selection-check--selected' : ''}`} aria-hidden="true">{isSelected ? <Icon name="check"/> : null}</span>;
}

export function FileCard({ item, isSelected, isFocused, onSelect, onOpen, onContextMenu, onToggleFavorite }: FileCardProps) {
  useCurrentLocale();
  const selectionClass = `${isSelected ? 'selected' : ''} ${isFocused ? 'focused' : ''}`.trim();

  if (item.kind === 'folder') {
    return (
      <Hint message={{id:"main.folder-open"}}><article
        className={`file-card folder-card ${selectionClass}`}
        data-explorer-item-id={item.id}
        draggable={false}
        aria-label={localeText("main", "ui.FileCard.c541a0d6d8bcdb61", {"value1": (item.name), "value2": (isSelected ? localeText("main", "ui.FileCard.0f8d44ab9f89612e") : '')})}
        aria-selected={isSelected}
        onClick={onSelect}
        onDoubleClick={onOpen}
        onContextMenu={onContextMenu}
        tabIndex={0}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault();
            onOpen();
          }
        }}
      >
        <SelectionBadge isSelected={isSelected} />
        <Hint message={{id:"main.folder-menu"}}><button type="button" className="card-menu" aria-label={messages().explorer.folderMenu} onClick={(event) => {event.stopPropagation();onContextMenu(event);}}><Icon name="more"/></button></Hint>
        <div className="folder-icon" aria-hidden="true"><Icon name="folder"/></div>
        <strong title={item.name}>{item.name}</strong>
        {item.childCount > 0 && <small>{plural('items',item.childCount)}</small>}
      </article></Hint>
    );
  }

  if (item.kind === 'file') {
    return (
      <Hint message={{id:"main.open-file"}}><article
        className={`file-card generic-file-card ${selectionClass}`}
        data-explorer-item-id={item.id}
        draggable={false}
        aria-label={localeText("main", "ui.FileCard.cccfd70427dc2409", {"value1": (item.name), "value2": (item.fileType), "value3": (isSelected ? localeText("main", "ui.FileCard.0f8d44ab9f89612e") : '')})}
        aria-selected={isSelected}
        onClick={onSelect}
        onDoubleClick={onOpen}
        onContextMenu={onContextMenu}
        tabIndex={0}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.preventDefault();
            onOpen();
          }
        }}
      >
        <SelectionBadge isSelected={isSelected} />
        <div className="generic-file-icon" aria-hidden="true">{item.fileType.slice(0, 4)}</div>
        <strong>{item.name}</strong>
        <small>{item.sizeLabel}</small>
      </article></Hint>
    );
  }

  return (
    <article
      className={`file-card image-card status-${item.status} ${selectionClass}`}
      data-explorer-item-id={item.id}
      draggable={false}
      aria-label={localeText("main", "ui.FileCard.f2eb3577fe738220", {"value1": (item.name), "value2": (statusLabel[item.status]), "value3": (isSelected ? ', selected' : '')})}
      aria-selected={isSelected}
      onClick={onSelect}
      onDoubleClick={onOpen}
      onContextMenu={onContextMenu}
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          event.preventDefault();
          onOpen();
        }
      }}
    >
      <SelectionBadge isSelected={isSelected} />
      <Hint message={{id:item.isFavorite?'common.favorite-remove':'common.favorite-add'}}><button
        type="button"
        className={item.isFavorite ? 'favorite-toggle favorite-toggle--active' : 'favorite-toggle'}
        title={item.isFavorite ? messages().explorer.removeFromFavorites : messages().explorer.addToFavorites}
        aria-label={item.isFavorite ? messages().explorer.removeFromFavorites : messages().explorer.addToFavorites}
        onClick={(event) => {
          event.stopPropagation();
          onToggleFavorite?.(item.id);
        }}
      >
        <Icon name="star" filled={Boolean(item.isFavorite)}/>
      </button></Hint>
      <ImageThumbnail item={item} />
      {['processed','queued','error','missing'].includes(item.status)||item.collections?.length?<div className="file-card-state-badges">
        {['processed','queued','error','missing'].includes(item.status) ? <span className={`processing-state-badge processing-state-badge--${item.status}`} title={statusLabel[item.status]}>{statusLabel[item.status]}</span> : null}
        {item.collections?.length ? <span className="collection-count-badge" title={item.collections.join(', ')}>{plural('collections',item.collections.length)}</span> : null}
      </div>:null}
      <div className="file-name-row">
        {!['processed','queued','error','missing'].includes(item.status)?<i className={`dot ${item.status}`} title={statusLabel[item.status]} />:null}
        <span>{item.name}</span>
      </div>
    </article>
  );
}
