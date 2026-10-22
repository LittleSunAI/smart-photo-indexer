import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useUiPreferences} from '../../shared/useUiPreferences';
import {Icon} from '../../shared/ui/Icon';
import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type MouseEvent } from 'react';
import type { ExplorerItem } from './types';
import { FileCard } from './FileCard';
import { useVirtualGridItems } from '../performance/useVirtualGridItems';
import { messages } from '../../shared/i18n';
import type {SortMode,ViewSize} from '../../app/types';
import {ExplorerRow} from './ExplorerRow';
import {useTableLayout,defaultTableLayout} from './TableLayout';
import {TableResizeHandle} from './TableResizeHandle';
import {InfoHint} from '../../shared/ui/InfoHint';
import {tableColumns,type TableColumnId} from './ExplorerViews';
import {useMarqueeSelection} from './useMarqueeSelection';
import {fileShortcut,requestFileCommand} from '../filesystem/FileActions';
import {useFileMoveDrag} from './useFileMoveDrag';

interface ExplorerGridProps {
  hasMore?:boolean;
  loadingMore?:boolean;
  pageError?:string|null;
  onReachEnd?():void;
  scrollContext?:string;
  items: ExplorerItem[];
  selectedItemIds: string[];
  focusedItemId: string | null;
  viewSize: ViewSize;
  sortMode?:SortMode|null;
  onSort?(sort:SortMode):void;
  isLoading?: boolean;
  errorMessage?: string | null;
  onSelect(itemId: string | null): void;
  onToggleSelection(itemId: string): void;
  onRangeSelect(itemId: string, visibleItemIds: string[]): void;
  onSelectAll(visibleItemIds: string[]): void;
  onSelectAllItems?(visibleItemIds: string[]): void;
  onOpenFolder(folderId: string): void;
  onOpenFile(itemId: string): void;
  onContextMenu(itemId: string | null, x: number, y: number): void;
  onToggleFavorite?(itemId: string): void;
  emptyTitle?: string;
  emptyMessage?: string;
}

export function ExplorerGrid({
  hasMore=false,loadingMore=false,pageError=null,onReachEnd,
  scrollContext,
  items,
  selectedItemIds,
  focusedItemId,
  viewSize,
  sortMode,onSort,
  isLoading = false,
  errorMessage = null,
  onSelect,
  onToggleSelection,
  onRangeSelect,
  onSelectAll,
  onSelectAllItems,
  onOpenFolder,
  onOpenFile,
  onContextMenu,
  onToggleFavorite,
  emptyTitle = messages().workspace.explorerEmptyTitle,
  emptyMessage = messages().workspace.explorerEmptyMessage,
}: ExplorerGridProps) {
  useCurrentLocale();
  const uiPreferences=useUiPreferences();
  const {layout,setLayout,persist,error:layoutError}=useTableLayout();
  const columns=layout.visibleColumnIds;
  const [picker,setPicker]=useState(layout);
  const draftColumns=picker.visibleColumnIds;
  const setDraftColumns=(update:(current:TableColumnId[])=>TableColumnId[])=>setPicker(current=>({...current,visibleColumnIds:update(current.visibleColumnIds)}));
  const dialogRef=useRef<HTMLDialogElement>(null);
  const shownColumns=tableColumns.filter(column=>columns.includes(column.id));
  const template=shownColumns.map(column=>`${layout.widthById[column.id]}px`).concat('minmax(0,1fr)','64px').join(' ');
  const tableWidth=shownColumns.reduce((total,column)=>total+layout.widthById[column.id],64);
  const orderedItemIds = useMemo(() => items.map(item => item.id), [items]);
  const selectedIds = useMemo(() => new Set(selectedItemIds), [selectedItemIds]);
  const suppressNextCardClickRef = useRef(false);
  const suppressDoubleClickRef = useRef(false);
  const marqueeActiveRef=useRef(false);
  const {
    containerRef,
    visibleItems,
    canvasStyle,
    gridStyle,geometry,
  } = useVirtualGridItems(items, viewSize, focusedItemId,scrollContext,marqueeActiveRef);

  const handleDragSelectionPointerDown=useMarqueeSelection({root:containerRef,items,geometry:viewSize==='table'?{...geometry,width:Math.max(geometry.width,tableWidth)}:geometry,context:scrollContext,view:viewSize,blockedFocus:marqueeActiveRef,suppressedClick:suppressNextCardClickRef,onSelect:onSelectAll});
  const handleFileMovePointerDown=useFileMoveDrag({root:containerRef,items,selectedItemIds,context:scrollContext,view:viewSize,blockedFocus:marqueeActiveRef,suppressedClick:suppressNextCardClickRef});

  useEffect(()=>{
    const node=containerRef.current;if(!node||!onReachEnd||!hasMore||loadingMore||pageError)return;
    let frame=0;const check=()=>{frame=0;if(node.scrollHeight-node.scrollTop-node.clientHeight<600)onReachEnd();};
    const schedule=()=>{if(!frame)frame=requestAnimationFrame(check);};
    node.addEventListener('scroll',schedule,{passive:true});schedule();
    return()=>{node.removeEventListener('scroll',schedule);if(frame)cancelAnimationFrame(frame);};
  },[hasMore,loadingMore,pageError,onReachEnd,items.length,viewSize]);

  const handleGridKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const command=fileShortcut(event.nativeEvent);
    if(command){event.preventDefault();event.stopPropagation();requestFileCommand(command);return;}
    if(event.defaultPrevented||event.target instanceof Element&&event.target.closest('input,textarea,select,[contenteditable]:not([contenteditable=false]),dialog,[role=separator]'))return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') {
      event.preventDefault();(onSelectAllItems ?? onSelectAll)(orderedItemIds);
    }
    if (event.key === 'Escape') {event.preventDefault();onSelect(null);}
  };
  const emptyContextMenu=(event:MouseEvent<HTMLDivElement>)=>{event.preventDefault();onContextMenu(null,event.clientX,event.clientY);};
  if (errorMessage) {
    return (
      <div className="explorer-grid explorer-grid--empty">
        <div className="empty-state empty-state--error">
          <strong>{messages().explorer.folderCouldNotOpen}</strong>
          <span>{errorMessage}</span>
        </div>
      </div>
    );
  }

  if (isLoading && items.length===0) {
    return (
      <div className="explorer-grid explorer-grid--empty">
        <div className="empty-state">
          <strong>{messages().explorer.scanningFolder}</strong>
          <span>{messages().explorer.scanningDescription}</span>
        </div>
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div ref={containerRef} className="explorer-grid explorer-grid--empty" tabIndex={0} onKeyDown={handleGridKeyDown} onContextMenu={emptyContextMenu}>
        <div className="empty-state">
          <strong>{emptyTitle}</strong>
          <span>{emptyMessage}</span>
        </div>
      </div>
    );
  }

  const handleCardSelect = (itemId: string, event: MouseEvent) => {
    if (suppressNextCardClickRef.current) {
      event.preventDefault();
      event.stopPropagation();
      suppressNextCardClickRef.current = false;
      return;
    }

    if (event.shiftKey) {
      onRangeSelect(itemId, orderedItemIds);
      return;
    }

    if (event.ctrlKey || event.metaKey) {
      onToggleSelection(itemId);
      return;
    }

    onSelect(itemId);
  };

  return (<>
    {loadingMore?<span className="grid-page-status" role="status">{localeText("main", "ui.ExplorerGrid.91c3220d29733bb2")}</span>:null}
    {pageError?<span className="grid-page-status" role="alert">{localeText("main", "ui.ExplorerGrid.fb9e31c1ed688db0")}{' '}<Hint message={{id:"common.retry-read",args:{раздел:localeText("main", "ui.ExplorerGrid.9cc566a0f4714808")}}}><button type="button" onClick={onReachEnd}>{localeText("main", "ui.ExplorerGrid.4e85ebb1c3051188")}</button></Hint><span title={pageError}>{localeText("main", "ui.ExplorerGrid.0deffebf281965da")}</span></span>:null}
    {layoutError?<span className="table-layout-error" role="status">{layoutError}</span>:null}
    <dialog ref={dialogRef} className="table-columns-dialog" aria-labelledby="table-columns-title">
      <form onSubmit={event=>{event.preventDefault();persist(picker);dialogRef.current?.close();}}>
        <div className="table-columns-dialog__heading"><h2 id="table-columns-title">{localeText("main", "ui.ExplorerGrid.8980cdbce22d141a")}</h2><Hint message={{id:"common.close-dialog"}}><button type="button" aria-label={localeText("main", "ui.ExplorerGrid.f98dd956f3956f17")} onClick={()=>dialogRef.current?.close()}><Icon name="close"/></button></Hint></div>
        <p>{localeText("main", "ui.ExplorerGrid.1e747df76ab600c2")}</p>
        <div className="table-columns-dialog__options">{tableColumns.map(column=><Hint key={column.id} message={{id:"main.column-visible",args:{название:column.label}}}><label key={column.id}><input type="checkbox" checked={draftColumns.includes(column.id)} disabled={column.id==='name'} onChange={event=>setDraftColumns(current=>event.target.checked?[...current,column.id]:current.filter(id=>id!==column.id))}/>{column.label}</label></Hint>)}</div>
        <div className="table-columns-dialog__actions"><Hint message={{id:"main.columns-default"}}><button type="button" onClick={()=>setPicker(defaultTableLayout())}>{localeText("main", "ui.ExplorerGrid.0499a1349afa97fe")}</button></Hint><Hint message={{id:"common.cancel-draft"}}><button type="button" onClick={()=>dialogRef.current?.close()}>{localeText("main", "ui.ExplorerGrid.9f621fdc8f2ca0dc")}</button></Hint><Hint message={{id:"main.columns-apply"}}><button type="submit" className="primary-action">{localeText("main", "ui.ExplorerGrid.cff64f5546ed0f1e")}</button></Hint></div>
      </form>
    </dialog>
    <div
      ref={containerRef}
      className={`explorer-grid explorer-grid--${viewSize} explorer-grid--fit-${uiPreferences.thumbnailFit}`}
      style={{ display: 'block', position: 'relative' }}
      aria-label={messages().explorer.gridAria}
      role={viewSize==='table'?'table':undefined}
      aria-rowcount={viewSize==='table'?items.length+1:undefined}
      tabIndex={0}
      onKeyDown={handleGridKeyDown}
      onPointerDownCapture={event=>{suppressNextCardClickRef.current=false;suppressDoubleClickRef.current=false;if(!handleFileMovePointerDown(event))handleDragSelectionPointerDown(event);}}
      onClickCapture={event=>{if(suppressNextCardClickRef.current){event.preventDefault();event.stopPropagation();suppressNextCardClickRef.current=false;suppressDoubleClickRef.current=true;}}}
      onDoubleClickCapture={event=>{if(suppressNextCardClickRef.current||suppressDoubleClickRef.current){event.preventDefault();event.stopPropagation();suppressNextCardClickRef.current=false;suppressDoubleClickRef.current=false;}}}
      onDragStart={(event) => event.preventDefault()}
      onContextMenu={(event) => {
        event.preventDefault();
        onContextMenu(null, event.clientX, event.clientY);
      }}
    >
      {viewSize==='table'?<div className="explorer-table__header" role="row" style={{gridTemplateColumns:template,minWidth:tableWidth}}>
        {shownColumns.map(column=><div role="columnheader" key={column.id} aria-sort={sortMode===column.asc?'ascending':sortMode===column.desc?'descending':'none'}><Hint message={{id:"main.column-sort",args:{название:column.label}}}><button type="button" title={column.id==='tags'?localeText("main", "ui.ExplorerGrid.c3309541c36571d8"):localeText("main", "ui.ExplorerGrid.20dfa784e7d2cf1d", {"value1": (column.label)})} onClick={()=>onSort?.(sortMode===column.asc?column.desc:column.asc)}>{column.label}<span aria-hidden="true">{sortMode===column.asc?'↑':sortMode===column.desc?'↓':'↕'}</span></button></Hint><TableResizeHandle id={column.id} label={column.label} width={layout.widthById[column.id]} defaultWidth={column.width} onPreview={value=>setLayout(current=>({...current,widthById:{...current.widthById,[column.id]:value}}))} onCommit={value=>persist({...layout,widthById:{...layout.widthById,[column.id]:value}})}/></div>)}
        <span aria-hidden="true"/>
        <div className="explorer-table__header-actions" data-testid="table-header-actions"><span data-testid="table-selection-help"><InfoHint label={localeText("main", "ui.WorkspaceLayout.fecab4bc4a4d8f41")} message={{id:'main.card-select'}} activationOnly/></span><Hint message={{id:"main.columns"}}><button type="button" data-tour-id="main.display.columns" data-help-id="table-columns" className="explorer-table__columns-button" aria-label={localeText("main", "ui.ExplorerGrid.cd6131b50054079c")} title={localeText("main", "ui.ExplorerGrid.75b8974e1769f818")} onClick={()=>{setPicker(structuredClone(layout));dialogRef.current?.showModal();}}><Icon name="plus"/></button></Hint></div>
      </div>:null}
      <div style={{...canvasStyle,...(viewSize==='table'?{minWidth:tableWidth}:{})}}><div style={gridStyle}>
      {visibleItems.map((item) => {
        const Card=viewSize==='list'||viewSize==='table'?ExplorerRow:FileCard;
        return <Card
          key={item.id}
          item={item}
          isSelected={selectedIds.has(item.id)}
          isFocused={item.id === focusedItemId}
          {...(viewSize==='table'?{columns,template}:{})}
          onSelect={(event) => handleCardSelect(item.id, event)}
          onOpen={() => {
            if (item.kind === 'folder') {
              onOpenFolder(item.id);
            } else {
              onOpenFile(item.id);
            }
          }}
          onToggleFavorite={onToggleFavorite}
          onContextMenu={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onContextMenu(item.id, event.clientX, event.clientY);
          }}
        />;
      })}
      </div></div>



    </div></>
  );
}
