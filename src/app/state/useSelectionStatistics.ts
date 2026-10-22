import {localeTag as displayLocaleTag} from "../../shared/i18n/Formatting";
import {plural,localeTag} from '../../shared/i18n/Formatting';
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useEffect,useRef,useState} from 'react';
import {invoke} from "../../shared/tauri/invoke";
import type {AppState} from '../types';
import {isQueryView,quickSearchView} from './QuickSearch';
import {selectSelectedItems,selectSelectionCounts,formatSelectionSummary} from './appSelectors';
import {isTauriRuntime} from '../../shared/tauri/runtime';

interface Statistics{files:number;bytes:number;unknownSizes:number;partial:boolean;formats:string[]}
export function formatSelectionBytes(bytes:number):string{const units=[localeText("main", "ui.useSelectionStatistics.e80cbf81c03c8ce2"),localeText("main", "ui.useSelectionStatistics.c1ebaaf081c98385"),localeText("main", "ui.useSelectionStatistics.49cf3fdb1238a92f"),localeText("main", "ui.useSelectionStatistics.5389cfedcf4ed120"),localeText("main", "ui.useSelectionStatistics.4c5cccee4a8f82dc")];let n=0;while(bytes>=1024&&n<units.length-1){bytes/=1024;n++;}return `${new Intl.NumberFormat(displayLocaleTag(),{maximumFractionDigits:n?1:0}).format(bytes)} ${units[n]}`;}
export function statisticsLabel(stats:Statistics,includeCount=true):string{
 if(stats.partial&&stats.files===0)return localeText("main", "ui.useSelectionStatistics.8bf2b320233d5a22");
 const size=stats.partial||stats.unknownSizes?stats.files>stats.unknownSizes?localeText("main", "ui.useSelectionStatistics.85dc11ae84ccf29e", {"value1": (formatSelectionBytes(stats.bytes))}):localeText("main", "ui.useSelectionStatistics.75335d99b92638e0"):formatSelectionBytes(stats.bytes);
 const formats=stats.formats.length>0?` · ${stats.formats.slice(0,3).join(', ')}${stats.formats.length>3?localeText("main", "ui.useSelectionStatistics.c42f6492a317118e"):''}`:'';
 return `${includeCount?`${stats.partial?localeText("main","ui.useSelectionStatistics.5de2b69d3b415e51"):''}${plural('files',stats.files)} · `:''}${size}${formats}${stats.partial?localeText("main","ui.useSelectionStatistics.d3b9789b7d5cd4a1"):''}`;
}
export function useSelectionStatistics(base:AppState):string{
  useCurrentLocale();
 const state=quickSearchView(base),items=selectSelectedItems(state),summary=formatSelectionSummary(selectSelectionCounts(state),false);
 const snapshot=isQueryView(state)?state.search.selectionSnapshot:undefined;
 const paths=items.map(i=>i.path),folders=items.some(i=>i.kind==='folder');
 const queryId=snapshot?.input.queryId,excluded=snapshot?.excludedPaths??[];
 const remote=Boolean(queryId||folders),key=remote?JSON.stringify(queryId?[queryId,excluded]:[...new Set(paths)].sort()):'';
 const [result,setResult]=useState<{key:string;stats?:Statistics;error?:string}|null>(null),usedRemote=useRef(false);
 useEffect(()=>{
  if(!isTauriRuntime())return;
  if(!remote){if(usedRemote.current){usedRemote.current=false;void invoke('get_selection_statistics',{paths:[]}).catch(()=>{});}return;}
  usedRemote.current=true;let alive=true;setResult(null);
  void invoke<Statistics>('get_selection_statistics',{paths:queryId?[]:paths,queryId,excludedPaths:excluded}).then(stats=>{if(alive)setResult({key,stats});}).catch(error=>{if(alive)setResult({key,error:String(error)});});
  return()=>{alive=false;};
 },[key]);
 useEffect(()=>()=>{if(usedRemote.current)void invoke('get_selection_statistics',{paths:[]}).catch(()=>{});},[]);
 if(remote){if(!isTauriRuntime())return localeText("main", "ui.useSelectionStatistics.ba50b46664288078", {"value1": (summary)});if(result?.key!==key)return localeText("main", "ui.useSelectionStatistics.aa3b1bd9cf10fcbd", {"value1": (summary)});return `${summary} · ${result.stats?statisticsLabel(result.stats,!queryId):localeText("main", "ui.useSelectionStatistics.882d83bc81e0c891")}`;}
 if(!items.length)return summary;
 const unique=[...new Map(items.map(i=>[i.path.replaceAll('/','\\').toLowerCase(),i])).values()];
 let bytes=0,unknown=0;const formats=new Set<string>();for(const item of unique){if(item.kind==='folder')continue;const size=item.sizeBytes??(item.kind==='image'?item.fileMetadata?.sizeBytes:undefined);if(size==null)unknown++;else bytes+=size;const ext=item.extension??item.name.split('.').slice(1).at(-1);if(ext)formats.add(ext.toUpperCase());}
 if(unique.length===1&&unique[0].kind==='image'){const photo=unique[0],w=photo.fileMetadata?.width,h=photo.fileMetadata?.height,dimensions=w&&h?`${w} × ${h}`:photo.dimensions;return [[...formats].join(', '),dimensions,unknown?localeText("main", "ui.useSelectionStatistics.69e4b4bd60095560"):formatSelectionBytes(bytes)].filter(Boolean).join(' · ');}
 return `${summary} · ${statisticsLabel({files:unique.length,bytes,unknownSizes:unknown,partial:false,formats:[...formats]},false)}`;
}
