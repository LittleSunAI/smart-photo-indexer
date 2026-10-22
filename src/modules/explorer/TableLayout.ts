import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useEffect,useState} from 'react';
import {tableColumns,defaultTableColumns,columnStorageKey,type TableColumnId} from './ExplorerViews';

export interface TableLayout {version:2;visibleColumnIds:TableColumnId[];widthById:Record<TableColumnId,number>}
export const tableLayoutKey='smart-photo-indexer.table-columns.v2';
const ranges:Record<TableColumnId,[number,number]>={name:[160,1200],size:[88,400],dimensions:[104,400],type:[64,240],taken:[132,480],modified:[132,480],analyzed:[132,480],status:[96,420],camera:[112,600],tags:[112,1000],path:[160,1600],favorite:[88,240]};
export function columnWidth(id:TableColumnId,value:number){const [min,max]=ranges[id];return Math.round(Math.max(min,Math.min(max,value)));}
export function columnRange(id:TableColumnId){return ranges[id];}
export function defaultTableLayout():TableLayout{return {version:2,visibleColumnIds:[...defaultTableColumns],widthById:Object.fromEntries(tableColumns.map(c=>[c.id,c.width])) as Record<TableColumnId,number>};}
export function loadTableLayout():TableLayout{
 const defaults=defaultTableLayout();try{
  const raw=JSON.parse(localStorage.getItem(tableLayoutKey)??'null');
  const legacy=raw===null?JSON.parse(localStorage.getItem(columnStorageKey)??'null'):null;
  const visible=raw?.version===2&&Array.isArray(raw.visibleColumnIds)?raw.visibleColumnIds:Array.isArray(legacy)?legacy:defaults.visibleColumnIds;
  defaults.visibleColumnIds=tableColumns.filter(c=>c.id==='name'||visible.includes(c.id)).map(c=>c.id);
  if(raw?.version===2&&raw.widthById&&typeof raw.widthById==='object')for(const c of tableColumns){const value=raw.widthById[c.id],[min,max]=ranges[c.id];if(typeof value==='number'&&Number.isFinite(value)&&value>=min&&value<=max)defaults.widthById[c.id]=Math.round(value);}
 }catch{/* Corrupt preferences never change file data. */}return defaults;
}
const layoutEvent='smart-photo-indexer:table-layout';
export function useTableLayout(){
  useCurrentLocale();
 const [layout,setLayout]=useState(loadTableLayout),[error,setError]=useState('');
 useEffect(()=>{const updated=(event:Event)=>{if(event instanceof CustomEvent)setLayout(event.detail as TableLayout);else setLayout(loadTableLayout());};window.addEventListener(layoutEvent,updated);window.addEventListener('storage',updated);return()=>{window.removeEventListener(layoutEvent,updated);window.removeEventListener('storage',updated);};},[]);
 const persist=(next:TableLayout)=>{setLayout(next);window.dispatchEvent(new CustomEvent(layoutEvent,{detail:next}));try{localStorage.setItem(tableLayoutKey,JSON.stringify(next));setError('');}catch{setError(localeText("main", "ui.TableLayout.391adb57e8e2817f"));}};
 return {layout,setLayout,persist,error};
}
