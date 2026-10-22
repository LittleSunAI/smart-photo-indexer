import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,useId,type PointerEvent as ReactPointerEvent} from 'react';
import type {EditorAsset,EditorDraft} from './EditorContracts';

import {outpaintGeometry,outpaintFrameDraft} from './EditorOutpaintGeometry';
export {outpaintGeometry} from './EditorOutpaintGeometry';

export type OutpaintHandle='left'|'right'|'top'|'bottom'|'top-left'|'top-right'|'bottom-left'|'bottom-right';
type Geometry=NonNullable<ReturnType<typeof outpaintGeometry>>;
export const outpaintHandles:OutpaintHandle[]=['top-left','top','top-right','right','bottom-right','bottom','bottom-left','left'];
export function resizedOutpaint(frame:Geometry,handle:OutpaintHandle,dx:number,dy:number){
 const clamp=(v:number,min:number,max:number)=>Math.max(min,Math.min(max,Math.round(v)));
 const next={left:frame.left,right:frame.right,top:frame.top,bottom:frame.bottom};
 if(handle.includes('left'))next.left=clamp(frame.left-dx,Math.max(1-frame.w-frame.right,1-frame.w),Math.min(1024,2048-frame.w-frame.right));
 if(handle.includes('right'))next.right=clamp(frame.right+dx,Math.max(1-frame.w-frame.left,1-frame.w),Math.min(1024,2048-frame.w-frame.left));
 if(handle.includes('top'))next.top=clamp(frame.top-dy,Math.max(1-frame.h-frame.bottom,1-frame.h),Math.min(1024,2048-frame.h-frame.bottom));
 if(handle.includes('bottom'))next.bottom=clamp(frame.bottom+dy,Math.max(1-frame.h-frame.top,1-frame.h),Math.min(1024,2048-frame.h-frame.top));
 return next;
}
export function translatedOutpaint(frame:Geometry,dx:number,dy:number){
 const bounds=(origin:number,length:number,source:number,delta:number)=>{
  const min=length>=source?Math.max(source-length,-1024):0,max=length>=source?Math.min(0,source+1024-length):source-length;
  return Math.max(min,Math.min(max,origin+Math.round(delta)))-origin;
 };
 const x=bounds(frame.x,frame.width,frame.w,dx),y=bounds(frame.y,frame.height,frame.h,dy);
 return {left:frame.left-x,right:frame.right+x,top:frame.top-y,bottom:frame.bottom+y};
}
function expansionDraft(d:EditorDraft,values:{left:number;right:number;top:number;bottom:number;w:number;h:number}):EditorDraft{
 return outpaintFrameDraft(d,{version:1,x:-values.left,y:-values.top,width:values.w+values.left+values.right,height:values.h+values.top+values.bottom});
}
export function EditorOutpaintFrame({draft,asset,onChange,locked=false}:{draft:EditorDraft;asset:EditorAsset;onChange?(change:(d:EditorDraft)=>EditorDraft):void;locked?:boolean}){
  useCurrentLocale();
 const frame=outpaintGeometry(draft,asset),z=draft.viewport.zoom,patternId=useId();
 const owner=JSON.stringify([draft.assetId,draft.operation,draft.orientation,draft.tool]);
 const latest=useRef({draft,onChange,owner});latest.current={draft,onChange,owner};
 const drag=useRef<{pointerId:number;x:number;y:number;zoom:number;frame:Geometry;handle:OutpaintHandle|'move';owner:string;node:HTMLElement}|null>(null);
 const clear=()=>{const g=drag.current;drag.current=null;if(g?.node.hasPointerCapture(g.pointerId))g.node.releasePointerCapture(g.pointerId);};
 useEffect(()=>{clear();},[owner,locked,z,draft.viewport.panX,draft.viewport.panY]);
 useEffect(()=>{const escape=(e:KeyboardEvent)=>{const g=drag.current;if(e.key==='Escape'&&g){e.preventDefault();e.stopPropagation();if(g.owner===latest.current.owner)latest.current.onChange?.(d=>expansionDraft(d,g.frame));clear();}};const stop=()=>clear();window.addEventListener('keydown',escape,true);window.addEventListener('blur',stop);window.addEventListener('editor-flush-gestures',stop);return()=>{window.removeEventListener('keydown',escape,true);window.removeEventListener('blur',stop);window.removeEventListener('editor-flush-gestures',stop);clear();};},[]);
 if(!frame)return null;
 const c=frame.crop,hole={x:(c.x-frame.x)*z,y:(c.y-frame.y)*z,width:c.width*z,height:c.height*z};
 const start=(e:ReactPointerEvent<HTMLElement>,handle:OutpaintHandle|'move')=>{if(e.button!==0||locked||!onChange)return;e.preventDefault();e.stopPropagation();clear();e.currentTarget.setPointerCapture(e.pointerId);drag.current={pointerId:e.pointerId,x:e.clientX,y:e.clientY,zoom:z,frame,handle,owner,node:e.currentTarget};};
 const move=(e:ReactPointerEvent<HTMLElement>)=>{const g=drag.current;if(!g||g.pointerId!==e.pointerId||g.owner!==latest.current.owner||locked)return;e.preventDefault();e.stopPropagation();const dx=(e.clientX-g.x)/g.zoom,dy=(e.clientY-g.y)/g.zoom,next=g.handle==='move'?translatedOutpaint(g.frame,dx,dy):resizedOutpaint(g.frame,g.handle,dx,dy);onChange?.(d=>expansionDraft(d,{...next,w:g.frame.w,h:g.frame.h}));};
 const labels:Record<OutpaintHandle,string>={left:localeText("editor", "ui.EditorOutpaintFrame.a296ab6e05dcead1"),right:localeText("editor", "ui.EditorOutpaintFrame.c2e6bc1fc77da7aa"),top:localeText("editor", "ui.EditorOutpaintFrame.cdda5cad933e780b"),bottom:localeText("editor", "ui.EditorOutpaintFrame.2adadc6f7c9a6698"),'top-left':localeText("editor", "ui.EditorOutpaintFrame.b9567551596f83dc"),'top-right':localeText("editor", "ui.EditorOutpaintFrame.74821d6e087b1589"),'bottom-left':localeText("editor", "ui.EditorOutpaintFrame.0a7e4e7104601a68"),'bottom-right':localeText("editor", "ui.EditorOutpaintFrame.ab40d8a12a4a2a1d")};
 return <>{frame.requiresCrop?<svg className="editor-outpaint-crop-shade" width={frame.w*z} height={frame.h*z} style={{transform:`translate(${draft.viewport.panX-frame.w*z/2}px,${draft.viewport.panY-frame.h*z/2}px)`}} aria-hidden="true"><path fill="rgba(10,18,30,.64)" fillRule="evenodd" d={`M0 0H${frame.w*z}V${frame.h*z}H0ZM${c.x*z} ${c.y*z}H${(c.x+c.width)*z}V${(c.y+c.height)*z}H${c.x*z}Z`}/></svg>:null}<div data-tour-id="editor.canvas.outpaint-frame" className={`editor-outpaint-frame${frame.bounded&&(frame.requiresCrop||frame.valid||!frame.hasExpansion)?'':' is-invalid'}${locked||!onChange?' is-locked':''}`} aria-label={localeText("editor", "ui.EditorOutpaintFrame.8de83f833062a727", {"value1": (frame.width), "value2": (frame.height)})} data-width={frame.width} data-height={frame.height} style={{width:frame.width*z,height:frame.height*z,transform:`translate(${draft.viewport.panX-(frame.w/2+frame.left)*z}px,${draft.viewport.panY-(frame.h/2+frame.top)*z}px)`}} onPointerDown={e=>start(e,'move')} onPointerMove={move} onPointerUp={e=>{move(e);clear();}} onPointerCancel={clear} onLostPointerCapture={clear}>
  <svg className="editor-outpaint-padding" width="100%" height="100%" aria-hidden="true"><defs><pattern id={patternId} patternUnits="userSpaceOnUse" width="16" height="16"><rect width="16" height="16" fill="var(--theme-surface-soft)"/><path d="M0 0H8V8H0ZM8 8H16V16H8Z" fill="var(--theme-selection)"/></pattern></defs><path fill={`url(#${patternId})`} fillRule="evenodd" d={`M0 0H${frame.width*z}V${frame.height*z}H0ZM${hole.x} ${hole.y}H${hole.x+hole.width}V${hole.y+hole.height}H${hole.x}Z`}/></svg>
  <span>{frame.width} × {frame.height}</span>
  {onChange&&!locked?outpaintHandles.map(handle=><Hint key={handle} message={{id:"outpaint.handle"}}><button key={handle} type="button" data-outpaint-handle={handle} className={`editor-outpaint-handle editor-outpaint-handle--${handle}`} aria-label={labels[handle]} title={labels[handle]} onPointerDown={e=>start(e,handle)} onKeyDown={e=>{const delta={ArrowLeft:[-1,0],ArrowRight:[1,0],ArrowUp:[0,-1],ArrowDown:[0,1]}[e.key];if(!delta)return;e.preventDefault();e.stopPropagation();const scale=e.shiftKey?10:1;onChange(d=>expansionDraft(d,{...resizedOutpaint(frame,handle,delta[0]*scale,delta[1]*scale),w:frame.w,h:frame.h}));}}/></Hint>):null}
 </div></>;
}
