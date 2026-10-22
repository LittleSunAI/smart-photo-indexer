import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,useState} from 'react';
import {convertFileSrc} from "@tauri-apps/api/core";
import {invoke} from "../../shared/tauri/invoke";
import {CachedImage} from '../thumbnail/CachedImage';
import type {ImageItem} from '../explorer/types';
import {Icon} from '../../shared/ui/Icon';

export function ZoomPreview({item}:{item:ImageItem}){
  useCurrentLocale();
 const root=useRef<HTMLDivElement>(null),image=useRef<HTMLImageElement>(null),drag=useRef<{id:number;x:number;y:number;panX:number;panY:number}|null>(null);
 const [detail,setDetail]=useState<{path:string;width:number;height:number}|null>(null),[view,setView]=useState({zoom:1,x:0,y:0}),[detailError,setDetailError]=useState(''),current=useRef(view);current.current=view;
 const key=`${item.path}|${item.sourceRevision??`${item.modifiedAtUnix}:${item.sizeBytes}`}`;
 useEffect(()=>{let disposed=false;setDetail(null);setDetailError('');setView({zoom:1,x:0,y:0});drag.current=null;const timer=setTimeout(()=>void invoke<{path:string;width:number;height:number}>('get_selected_photo_preview',{imagePath:item.path}).then(v=>{if(!disposed){const preload=new Image();preload.onload=()=>{if(!disposed)setDetail(v);};preload.onerror=()=>{if(!disposed)setDetailError(localeText("details", "ui.ZoomPreview.8fe9031221189f63"));};preload.src=convertFileSrc(v.path);}}).catch(error=>{if(!disposed)setDetailError(String(error));}),160);return()=>{disposed=true;clearTimeout(timer);};},[key]);
 const constrain=(v:typeof view)=>{const box=root.current?.getBoundingClientRect();if(!box||!detail)return v;const fit=Math.min(box.width/detail.width,box.height/detail.height),mx=Math.max(0,(detail.width*fit*v.zoom-box.width)/2),my=Math.max(0,(detail.height*fit*v.zoom-box.height)/2);return {...v,x:Math.min(mx,Math.max(-mx,v.x)),y:Math.min(my,Math.max(-my,v.y))};};
 useEffect(()=>{if(!root.current)return;const obs=new ResizeObserver(()=>setView(v=>constrain(v)));obs.observe(root.current);return()=>obs.disconnect();},[detail]);
 useEffect(()=>{const node=root.current;if(!node)return;const prevent=(e:WheelEvent)=>{if(detail)e.preventDefault();};node.addEventListener('wheel',prevent,{passive:false});return()=>node.removeEventListener('wheel',prevent);},[detail]);
 const end=()=>{drag.current=null;};
 return <div ref={root} className="details-zoom-preview" title={detailError?localeText("details", "ui.ZoomPreview.0eba07845f50e042")+detailError:localeText("details", "ui.ZoomPreview.7e1dc5bab51affd7")} aria-label={localeText("details", "ui.ZoomPreview.bac003dbfad3953d")} tabIndex={0} onWheel={e=>{if(!detail)return;e.preventDefault();const box=e.currentTarget.getBoundingClientRect(),previous=current.current,zoom=Math.max(1,Math.min(8,previous.zoom*Math.exp(-e.deltaY*.0015))),ratio=zoom/previous.zoom,x=e.clientX-box.left-box.width/2,y=e.clientY-box.top-box.height/2;setView(constrain({zoom,x:x-(x-previous.x)*ratio,y:y-(y-previous.y)*ratio}));}} onPointerDown={e=>{if(e.button!==0||current.current.zoom<=1||!detail)return;e.preventDefault();e.currentTarget.setPointerCapture(e.pointerId);drag.current={id:e.pointerId,x:e.clientX,y:e.clientY,panX:current.current.x,panY:current.current.y};}} onPointerMove={e=>{const g=drag.current;if(g?.id===e.pointerId)setView(constrain({...current.current,x:g.panX+e.clientX-g.x,y:g.panY+e.clientY-g.y}));}} onPointerUp={end} onPointerCancel={end} onLostPointerCapture={end} onDoubleClick={()=>setView({zoom:1,x:0,y:0})} onKeyDown={e=>{if(e.key==='Escape'||e.key==='0'){e.preventDefault();setView({zoom:1,x:0,y:0});}}}>
  {!detail?<CachedImage className="details-zoom-base" imagePath={item.path} revision={item.sourceRevision??`${item.modifiedAtUnix}:${item.sizeBytes}`} originalPreviewUrl={item.previewUrl} variant="details" width={720} height={720}/>:null}
  {detail?<img ref={image} draggable={false} className="details-zoom-image" src={convertFileSrc(detail.path)} alt={localeText("details", "ui.ZoomPreview.57ba02bb9fb13975")} style={{transform:`translate(${view.x}px,${view.y}px) scale(${view.zoom})`}}/>:null}
  <Hint message={{id:"viewer.fit"}} supplement={{id:"preview.gestures"}}><button type="button" className="details-preview-fit" aria-label={localeText("details", "ui.ZoomPreview.20925efb246ad3fe")} title={localeText("details", "ui.ZoomPreview.4b9cafdcba708aa1")} onPointerDown={e=>e.stopPropagation()} onDoubleClick={e=>e.stopPropagation()} onClick={e=>{e.stopPropagation();drag.current=null;setView({zoom:1,x:0,y:0});}}><Icon name="fit"/></button></Hint>
  {view.zoom>1?<span className="details-zoom-indicator">{Math.round(view.zoom*100)}{localeText("details", "ui.ZoomPreview.b97acf11b739695b")}</span>:null}
 </div>;
}
