import {localeTag as displayLocaleTag} from "../../shared/i18n/Formatting";
import {text as localeText} from "../../shared/i18n/Runtime";
import type { SortMode } from '../../app/types';
import type { ExplorerItem } from './types';
export const tableColumns=[
  {id:'name',get label() { return localeText("main", "ui.ExplorerViews.fc33c4290826b579"); },width:260,asc:'nameAsc',desc:'nameDesc'},
  {id:'size',get label() { return localeText("main", "ui.ExplorerViews.ec18d59a01a346e1"); },width:104,asc:'sizeAsc',desc:'sizeDesc'},
  {id:'dimensions',get label() { return localeText("main", "ui.ExplorerViews.172fde042ef877be"); },width:124,asc:'dimensionsAsc',desc:'dimensionsDesc'},
  {id:'type',get label() { return localeText("main", "ui.ExplorerViews.462755cec7525531"); },width:80,asc:'type',desc:'typeDesc'},
  {id:'taken',get label() { return localeText("main", "ui.ExplorerViews.83f9e74a6e27c5b8"); },width:168,asc:'dateTakenOldest',desc:'dateTakenNewest'},
  {id:'status',get label() { return localeText("main", "ui.ExplorerViews.04986a13dfd82f4b"); },width:126,asc:'status',desc:'statusDesc'},
  {id:'analyzed',get label() { return localeText("main", "ui.ExplorerViews.e0a9e540e510b19c"); },width:168,asc:'analyzedAsc',desc:'analyzedDesc'},
  {id:'modified',get label() { return localeText("main", "ui.ExplorerViews.d38d20dae2ab988d"); },width:168,asc:'modifiedAsc',desc:'modifiedDesc'},
  {id:'camera',get label() { return localeText("main", "ui.ExplorerViews.0f7de12432b0504d"); },width:180,asc:'cameraAsc',desc:'cameraDesc'},
  {id:'tags',get label() { return localeText("main", "ui.ExplorerViews.63a84c9de3d07feb"); },width:200,asc:'tagsAsc',desc:'tagsDesc'},
  {id:'path',get label() { return localeText("main", "ui.ExplorerViews.78c9a0ae6e9abd42"); },width:320,asc:'pathAsc',desc:'pathDesc'},
  {id:'favorite',get label() { return localeText("main", "ui.ExplorerViews.fffbc48fb466bf7d"); },width:108,asc:'favoriteAsc',desc:'favoriteDesc'},
] as const;
export type TableColumnId=typeof tableColumns[number]['id'];
export const defaultTableColumns:TableColumnId[]=['name','size','dimensions','type','taken','status'];
export const columnStorageKey='smart-photo-indexer.table-columns.v1';
export function loadTableColumns():TableColumnId[]{try{const raw=JSON.parse(localStorage.getItem(columnStorageKey)??'null');if(Array.isArray(raw))return tableColumns.filter(column=>column.id==='name'||raw.includes(column.id)).map(column=>column.id);}catch{/* Defaults remain available. */}return [...defaultTableColumns];}
const statuses={get processed() { return localeText("main", "ui.ExplorerViews.00f48e6a0da79a1b"); },get queued() { return localeText("main", "ui.ExplorerViews.046ac4c84606710c"); },get notProcessed() { return localeText("main", "ui.ExplorerViews.a51245c14d5b992c"); },get relocated() { return localeText("main", "ui.ExplorerViews.5b1eab8f801c268c"); },get missing() { return localeText("main", "ui.ExplorerViews.792452362f3ea56f"); },get error() { return localeText("main", "ui.ExplorerViews.054754833afb97a0"); }};
const date=(seconds?:number|null)=>seconds==null?'—':new Intl.DateTimeFormat(displayLocaleTag(),{dateStyle:'short',timeStyle:'short'}).format(new Date(seconds*1000));
export function tableValue(item:ExplorerItem,column:TableColumnId):string{
  const image=item.kind==='image'?item:null;
  switch(column){
    case 'name':return item.name;
    case 'path':return item.path;
    case 'size':return item.kind==='folder'?'—':item.sizeBytes==null?'—':`${new Intl.NumberFormat(displayLocaleTag(),{maximumFractionDigits:1}).format(item.sizeBytes/(item.sizeBytes>=1048576?1048576:1024))} ${item.sizeBytes>=1048576?localeText("main", "ui.ExplorerViews.901114f34283f837"):localeText("main", "ui.ExplorerViews.0d6fe3cd4f2d4992")}`;
    case 'dimensions':return image?.fileMetadata?.width&&image.fileMetadata.height?`${image.fileMetadata.width} × ${image.fileMetadata.height}`:'—';
    case 'type':return item.kind==='folder'?localeText("main", "ui.ExplorerViews.581031e5935b7ecf"):item.extension?.toUpperCase()??localeText("main", "ui.ExplorerViews.91f4d61941959331");
    case 'taken':return image?.fileMetadata?.dateTakenLocal?.replace('T',' ')??'—';
    case 'analyzed':return date(image?.aiProcessedAtUnix);
    case 'modified':return date(item.kind==='folder'?null:item.modifiedAtUnix);
    case 'camera':return `${image?.fileMetadata?.cameraMake??''} ${image?.fileMetadata?.cameraModel??''}`.trim()||'—';
    case 'tags':return image?.tags.length?image.tags.join(', '):'—';
    case 'favorite':return image?.isFavorite?localeText("main", "ui.ExplorerViews.378e6db7fcda53c2"):'—';
    case 'status':return image?statuses[image.status]:'—';
  }
}
export function searchOrderFor(sort:SortMode):string{return sort==='dateTakenNewest'?'takenDesc':sort==='dateTakenOldest'?'takenAsc':sort==='type'?'typeAsc':sort==='status'?'statusAsc':sort;}
export function sortForSearchOrder(order:string):SortMode|null{for(const column of tableColumns){if(searchOrderFor(column.asc)===order)return column.asc;if(searchOrderFor(column.desc)===order)return column.desc;}return null;}
export function sortLabel(sort:SortMode):string{
  const column=tableColumns.find(column=>column.asc===sort||column.desc===sort);
  return column?`${column.label} ${column.desc===sort?'↓':'↑'}`:sort;
}
