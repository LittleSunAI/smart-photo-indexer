import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,useState} from 'react';
import {convertFileSrc} from '@tauri-apps/api/core';
import type {EditorAsset} from './EditorContracts';
import {EditorVirtualList} from './EditorVirtualList';
import {Icon} from '../../shared/ui/Icon';

export function EditorProjectReferenceGallery({assets,selected,busy,onSelect}:{assets:EditorAsset[];selected:string[];busy:boolean;onSelect(id:string):void}){
  useCurrentLocale();
 const root=useRef<HTMLDivElement>(null),[columns,setColumns]=useState(3);
 useEffect(()=>{const el=root.current;if(!el)return;const update=()=>setColumns(Math.max(1,Math.min(6,Math.floor((el.clientWidth-12)/112))));const observer=new ResizeObserver(update);observer.observe(el);update();return()=>observer.disconnect();},[]);
 const rows:EditorAsset[][]=[];for(let i=0;i<assets.length;i+=columns)rows.push(assets.slice(i,i+columns));
 return <div ref={root} className="editor-reference-gallery" style={{height:Math.min(430,rows.length*130),maxHeight:'50vh'}}><EditorVirtualList items={rows} itemKey={row=>row[0].id} rowSize={130} label={localeText("editor", "ui.EditorProjectReferenceGallery.2499906c68b04f42")} render={row=><div className="editor-reference-gallery-row" style={{gridTemplateColumns:`repeat(${columns},minmax(0,1fr))`}}>{row.map(a=><Hint key={a.id} message={{id:"reference.gallery"}}><button key={a.id} type="button" className="editor-reference-gallery-item" aria-label={localeText("editor", "ui.EditorProjectReferenceGallery.5775be1153688f1d", {"value1": (a.width), "value2": (a.height), "value3": (a.id)})} aria-pressed={selected.includes(a.id)} disabled={busy} onClick={()=>onSelect(a.id)}><img src={convertFileSrc(a.previewPath)} alt=""/><span>{a.width} × {a.height}</span>{selected.includes(a.id)?<span className="editor-reference-gallery-check"><Icon name="check"/></span>:null}</button></Hint>)}</div>}/></div>;
}
