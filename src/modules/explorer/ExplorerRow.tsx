import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {Icon} from '../../shared/ui/Icon';
import type { MouseEvent } from 'react';
import type { ExplorerItem } from './types';
import {CachedImage} from '../thumbnail/CachedImage';
import {tableValue,type TableColumnId} from './ExplorerViews';

interface Props{
 item:ExplorerItem;columns?:TableColumnId[];template?:string;isSelected:boolean;isFocused:boolean;
 onSelect(event:MouseEvent):void;onOpen():void;onContextMenu(event:MouseEvent):void;onToggleFavorite?(id:string):void;
}
export function ExplorerRow({item,columns,template,isSelected,isFocused,onSelect,onOpen,onContextMenu,onToggleFavorite}:Props){
  useCurrentLocale();
 const title=<><span className="explorer-row__preview">{item.kind==='image'?<CachedImage imagePath={item.path} revision={item.sourceRevision??`${item.modifiedAtUnix}:${item.sizeBytes}`} originalPreviewUrl={item.previewUrl} variant="card" width={64} height={64} className="explorer-row__image"/>:<Icon name={item.kind==='folder'?'folder':'file'}/>}</span><span className="explorer-row__name" title={item.name}>{item.name}</span></>;
 const row=<article role={columns?'row':undefined} className={`file-card explorer-row ${columns?'explorer-row--table':'explorer-row--list'} ${item.kind==='image'?'image-card':item.kind==='folder'?'folder-card':'generic-file-card'} ${isSelected?'selected':''} ${isFocused?'focused':''}`}
  style={columns?{gridTemplateColumns:template}:undefined} data-explorer-item-id={item.id} aria-selected={isSelected} aria-label={item.name} tabIndex={0} draggable={false}
  onClick={onSelect} onDoubleClick={onOpen} onContextMenu={onContextMenu} onKeyDown={event=>{if(event.key==='Enter'){event.preventDefault();onOpen();}}}>
  {columns?columns.map(id=><span className={`explorer-row__cell explorer-row__cell--${id}`} role="cell" key={id} title={tableValue(item,id)}>{id==='name'?title:tableValue(item,id)}</span>):<span className="explorer-row__cell explorer-row__cell--name">{title}</span>}
  {columns?<span aria-hidden="true"/>:null}
  {item.kind==='image'?<Hint message={{id:item.isFavorite?'common.favorite-remove':'common.favorite-add'}}><button type="button" className={`explorer-row__favorite ${item.isFavorite?'is-favorite':''}`} aria-label={item.isFavorite?localeText("main", "ui.ExplorerRow.f0315034a7ef4d6d"):localeText("main", "ui.ExplorerRow.020e60b72f14fc65")} onClick={event=>{event.stopPropagation();onToggleFavorite?.(item.id);}}><Icon name="star" filled={Boolean(item.isFavorite)}/></button></Hint>:<span/>}
 </article>;
 return item.kind==='image'?row:<Hint message={{id:item.kind==='folder'?'main.folder-open':'main.open-file'}}>{row}</Hint>;
}
