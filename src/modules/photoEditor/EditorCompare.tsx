import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useRef,useState} from 'react';
import {convertFileSrc} from '@tauri-apps/api/core';
import {Icon} from '../../shared/ui/Icon';
import type {EditorAsset,EditorDraft} from './EditorContracts';
import {orientationTransform,type WorkingOrientation} from './EditorOrientation';
export interface CompareFrame {stepId:string;asset:EditorAsset;orientation:WorkingOrientation;title:string}
export function comparisonPlane(before:CompareFrame,after:CompareFrame){
 const height=after.orientation.rotation%2?after.asset.width:after.asset.height;
 const aspect=(frame:CompareFrame)=>frame.orientation.rotation%2?frame.asset.height/frame.asset.width:frame.asset.width/frame.asset.height;
 return {width:height*Math.max(aspect(before),aspect(after)),height};
}
export function EditorCompare({before,after,viewport,onClose}:{before:CompareFrame;after:CompareFrame;viewport:EditorDraft['viewport'];onClose():void}){
  useCurrentLocale();
 const [divider,setDivider]=useState(50),[swapped,setSwapped]=useState(false),surface=useRef<HTMLDivElement>(null),pointer=useRef<{id:number;offset:number}|null>(null);const a=swapped?after:before,b=swapped?before:after;
 // The same plane also belongs to the editor's Fit action and initial pair view.
 const commonHeight=comparisonPlane(before,after).height;
 const image=(frame:CompareFrame)=>{const asset=frame.asset,oh=frame.orientation.rotation%2?asset.width:asset.height,factor=commonHeight/oh;return <img src={convertFileSrc(asset.previewPath)} alt={frame.title} draggable={false} style={{width:asset.width*factor,height:asset.height*factor,transform:'translate(-50%,-50%) translate('+viewport.panX+'px,'+viewport.panY+'px) scale('+viewport.zoom+') '+orientationTransform(frame.orientation)}}/>;};
 const move=(clientX:number)=>{const box=surface.current?.getBoundingClientRect();if(box)setDivider(Math.max(0,Math.min(100,(clientX-(pointer.current?.offset??0)-box.left)/box.width*100)));};
 const finish=(el:HTMLElement,id:number)=>{if(pointer.current?.id!==id)return;pointer.current=null;if(el.hasPointerCapture(id))el.releasePointerCapture(id);};
 return <div ref={surface} className="editor-compare" aria-label={localeText("editor", "ui.EditorCompare.3efc8f37a880efff")} data-before-step={before.stepId} data-after-step={after.stepId}><div className="editor-compare-image">{image(b)}</div><div className="editor-compare-image" style={{clipPath:'inset(0 '+(100-divider)+'% 0 0)'}}>{image(a)}</div><div className="editor-compare-line" style={{left:divider+'%'}}/><Hint message={{id:"compare.divider"}}><button type="button" className="editor-compare-handle" role="slider" aria-label={localeText("editor", "ui.EditorCompare.3117ad3aa79244ba")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(divider)} style={{left:divider+'%'}} onPointerDown={e=>{if(e.button!==0||pointer.current!==null)return;e.preventDefault();e.stopPropagation();const box=surface.current?.getBoundingClientRect();pointer.current={id:e.pointerId,offset:box?e.clientX-box.left-box.width*divider/100:0};e.currentTarget.setPointerCapture(e.pointerId);}} onPointerMove={e=>{if(pointer.current?.id===e.pointerId){e.preventDefault();e.stopPropagation();move(e.clientX);}}} onPointerUp={e=>{if(pointer.current?.id===e.pointerId){move(e.clientX);finish(e.currentTarget,e.pointerId);}}} onPointerCancel={e=>finish(e.currentTarget,e.pointerId)} onLostPointerCapture={e=>finish(e.currentTarget,e.pointerId)} onKeyDown={e=>{const values:Record<string,number>={ArrowLeft:divider-(e.shiftKey?10:1),ArrowRight:divider+(e.shiftKey?10:1),Home:0,End:100};if(e.key in values){e.preventDefault();e.stopPropagation();setDivider(Math.max(0,Math.min(100,values[e.key])));}}}>↔</button></Hint><div className="editor-compare-controls"><Hint message={{id:"editor.compare-swap"}}><button type="button" aria-label={localeText("editor", "ui.EditorCompare.e6bce4f803c4411e")} title={localeText("editor", "ui.EditorCompare.d88926e76714f5a0")} onClick={()=>setSwapped(v=>!v)}><Icon name="refresh"/></button></Hint><Hint message={{id:"common.close-dialog"}}><button type="button" aria-label={localeText("editor", "ui.EditorCompare.fd4e7e81c096ed48")} onClick={onClose}><Icon name="close"/></button></Hint></div></div>;
}
