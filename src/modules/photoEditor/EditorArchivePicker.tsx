import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,useState} from 'react';
import {invoke} from "../../shared/tauri/invoke";
import {indexService} from '../index/IndexService';
import type {IndexSearchResultItem} from '../index/types';
import {CachedImage} from '../thumbnail/CachedImage';
import {EditorModal} from './EditorModal';
import {EditorVirtualList} from './EditorVirtualList';
export function EditorArchivePicker({onClose,onImport,onError,locked=false}:{onClose():void;onImport(path:string):Promise<void>;onError(message:string):void;locked?:boolean}){
  useCurrentLocale();
 const [query,setQuery]=useState(''),[items,setItems]=useState<IndexSearchResultItem[]>([]),[total,setTotal]=useState(0),[busy,setBusy]=useState(false);
 const generation=useRef(0),loading=useRef(false),count=useRef(0),knownTotal=useRef(0),snapshot=useRef<string|undefined>(undefined);
 const closeSnapshot=(queryId?:string)=>{if(queryId)void invoke('close_search_snapshot',{queryId}).catch(()=>{});};
 const load=async(reset=false)=>{if(loading.current&&!reset)return;const current=reset?++generation.current:generation.current;loading.current=true;try{const result=await indexService.searchIndexedImages({queryId:reset?undefined:snapshot.current,query,statuses:['processed','queued','notProcessed','error','relocated','missing'],limit:128,offset:reset?0:count.current,sortOrder:'nameAsc'});if(current!==generation.current){closeSnapshot(result.queryId);return;}snapshot.current=result.queryId;count.current=(reset?0:count.current)+result.items.length;knownTotal.current=result.totalCount;setItems(old=>reset?result.items:[...old,...result.items]);setTotal(result.totalCount);}catch(e){if(current===generation.current)onError(String(e));}finally{if(current===generation.current)loading.current=false;}};
 useEffect(()=>{count.current=0;knownTotal.current=0;loading.current=false;setItems([]);setTotal(0);const timer=setTimeout(()=>void load(true),180);return()=>{clearTimeout(timer);generation.current++;closeSnapshot(snapshot.current);snapshot.current=undefined;};},[query]);
 return <EditorModal title={localeText("editor", "ui.EditorArchivePicker.390842c5bf93928f")} busy={busy} onClose={onClose}><Hint message={{id:"source.archive-search"}}><input aria-label={localeText("editor", "ui.EditorArchivePicker.5d008981b163bec4")} value={query} onChange={e=>setQuery(e.target.value)} placeholder={localeText("editor", "ui.EditorArchivePicker.3352e0e3c14e9800")}/></Hint><span className="editor-muted">{total} {' '}{localeText("editor", "ui.EditorArchivePicker.8e12f696c36922a0")}</span><div className="editor-archive-picker"><EditorVirtualList items={items} rowSize={64} label={localeText("editor", "ui.EditorArchivePicker.189eb19c2ca33d2a")} itemKey={a=>a.path} onEnd={()=>{if(count.current<knownTotal.current)void load();}} render={a=><Hint message={{id:"source.archive-pick"}}><button type="button" className="editor-archive-photo" disabled={busy||locked||a.status==='missing'} title={a.path} onClick={()=>{setBusy(true);void onImport(a.path).then(onClose).catch(e=>onError(String(e))).finally(()=>setBusy(false));}}><CachedImage imagePath={a.path} width={80} height={56}/><span>{a.fileName}</span></button></Hint>}/></div></EditorModal>;
}
