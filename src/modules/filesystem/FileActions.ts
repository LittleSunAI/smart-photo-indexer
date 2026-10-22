import {text as localeText} from "../../shared/i18n/Runtime";
import type {AppState} from '../../app/types';
import {isQueryView,quickSearchView} from '../../app/state/QuickSearch';
import {selectSelectedImageItems} from '../../app/state/appSelectors';

export type FileActionKind='copy'|'move'|'trash';
export type FileActionSource={type:'explicit';paths:string[]}|{type:'query';queryId:string;excludedPaths:string[]}|{type:'clipboard';selectionId:string;revision:number};
export type FileCommand='copy'|'cut'|'paste'|'copyTo'|'moveTo'|'trash';
export interface FileSelection {selectionId:string|null;revision:number;kind:FileActionKind|null;count:number;ready:boolean}
export interface FileInfo {name:string;path:string;sizeBytes:number;modifiedAt:number|null}
export interface FileConflict {conflictId:string;ordinal:number;source:FileInfo;target:FileInfo;replaceAllowed:boolean;requiresFreshDecision:boolean}
export interface FileOutcome {ordinal:number;oldPath:string;newPath:string|null;imageId:string|null;status:string;message:string|null;filesystemApplied:boolean;indexApplied:boolean;editorApplied:boolean}
export interface FileOperation {operationId:string;revision:number;state:string;counts:Record<'copied'|'moved'|'trashed'|'skipped'|'failed'|'partial',number>;total:number;processed:number;conflict:FileConflict|null;changedPaths:FileOutcome[];warnings:string[];hasMore:boolean}
export type FileIntent={command:FileCommand;source?:FileActionSource;destination?:string};
export const fileCommandEvent='smart-photo-indexer:file-command';
export function requestFileCommand(command:FileCommand,source?:FileActionSource,destination?:string){window.dispatchEvent(new CustomEvent<FileIntent>(fileCommandEvent,{detail:{command,source,destination}}));}
export function selectedFileSource(state:AppState):FileActionSource {
 const s=quickSearchView(state),selection=isQueryView(s)?s.search.selectionSnapshot:undefined;
 if(selection?.input.queryId)return {type:'query',queryId:selection.input.queryId,excludedPaths:[...selection.excludedPaths]};
 return {type:'explicit',paths:[...new Set(selectSelectedImageItems(s).map(item=>item.path))]};
}
export function fileSelectionCount(state:AppState):number {
 const s=quickSearchView(state),selection=isQueryView(s)?s.search.selectionSnapshot:undefined;
 return selection?Math.max(0,selection.totalCount-selection.excludedPaths.length):selectSelectedImageItems(s).length;
}
export function pasteFolder(state:AppState):string|undefined{return !isQueryView(state)&&!state.isScanningFolder?state.currentPath??undefined:undefined;}
export function fileShortcut(event:Pick<KeyboardEvent,'key'|'ctrlKey'|'metaKey'|'altKey'|'shiftKey'|'repeat'|'defaultPrevented'|'target'>):FileCommand|undefined{
 if(event.repeat||event.defaultPrevented||event.altKey||event.shiftKey)return;
 if(event.target instanceof Element&&event.target.closest('input,textarea,select,[contenteditable]:not([contenteditable=false]),[role=textbox],dialog,[role=separator]'))return;
 if(event.ctrlKey||event.metaKey)return ({c:'copy',x:'cut',v:'paste'} as const)[event.key.toLowerCase() as 'c'|'x'|'v'];
 if(event.key==='Delete')return 'trash';
}
export function operationSummary(operation:FileOperation):string{
 const labels={copied:localeText("main", "ui.FileActions.f3816aa2726e8b33"),moved:localeText("main", "ui.FileActions.1794521033005681"),trashed:localeText("main", "ui.FileActions.1f153add0b77c6d8"),skipped:localeText("main", "ui.FileActions.4e78dd39df7fd62c"),failed:localeText("main", "ui.FileActions.a9cccc23c39b1e9f"),partial:localeText("main", "ui.FileActions.5497102f9d205303")};
 const counts=(Object.keys(labels) as (keyof typeof labels)[]).filter(key=>operation.counts[key]).map(key=>`${labels[key]}: ${operation.counts[key]}`).join(' · ');
 const first=operation.warnings[0]??operation.changedPaths.find(row=>row.message)?.message;
 return [counts,first].filter(Boolean).join('. ')||localeText("main", "ui.FileActions.2026acde5944f27c");
}
