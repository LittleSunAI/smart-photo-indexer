import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,type KeyboardEvent} from 'react';
import {columnRange,columnWidth} from './TableLayout';
import type {TableColumnId} from './ExplorerViews';
interface Props{id:TableColumnId;label:string;width:number;defaultWidth:number;onPreview(value:number):void;onCommit(value:number):void}
export function TableResizeHandle({id,label,width,defaultWidth,onPreview,onCommit}:Props){
  useCurrentLocale();
 const current=useRef({width,onPreview,onCommit});current.current={width,onPreview,onCommit};
 const gesture=useRef<{pointer:number;x:number;start:number;value:number;node:HTMLElement}|null>(null),keyboard=useRef<number|null>(null),frame=useRef<number|null>(null);
 const preview=()=>{if(frame.current!==null)cancelAnimationFrame(frame.current);frame.current=null;if(gesture.current)current.current.onPreview(gesture.current.value);};
 const cancel=()=>{const g=gesture.current;if(g){gesture.current=null;if(frame.current!==null)cancelAnimationFrame(frame.current);frame.current=null;current.current.onPreview(g.start);if(g.node.hasPointerCapture(g.pointer))g.node.releasePointerCapture(g.pointer);}if(keyboard.current!==null){current.current.onPreview(keyboard.current);keyboard.current=null;}};
 useEffect(()=>()=>cancel(),[]);
 const key=(e:KeyboardEvent)=>{e.stopPropagation();if(e.key==='Escape'){e.preventDefault();cancel();return;}if(e.key==='Enter'){e.preventDefault();if(keyboard.current!==null){keyboard.current=null;current.current.onCommit(current.current.width);}return;}if(e.key!=='ArrowLeft'&&e.key!=='ArrowRight')return;e.preventDefault();keyboard.current??=current.current.width;current.current.onPreview(columnWidth(id,current.current.width+(e.key==='ArrowLeft'?-1:1)*(e.shiftKey?32:8)));};
 const [min,max]=columnRange(id);
 return <Hint message={{id:"layout.column-width"}}><span className="explorer-table__resize" role="separator" aria-orientation="vertical" aria-label={localeText("main", "ui.TableResizeHandle.a9283a28880fa0d0", {"value1": (label)})} aria-valuemin={min} aria-valuemax={max} aria-valuenow={width} tabIndex={0} title={localeText("main", "ui.TableResizeHandle.3dd89e4352686f0a")} onKeyDown={key} onBlur={()=>{if(keyboard.current!==null){keyboard.current=null;current.current.onCommit(current.current.width);}}} onClick={e=>{e.stopPropagation();e.preventDefault();}} onDoubleClick={e=>{e.stopPropagation();e.preventDefault();cancel();current.current.onCommit(defaultWidth);}} onPointerDown={e=>{if(e.button!==0)return;e.preventDefault();e.stopPropagation();e.currentTarget.focus();e.currentTarget.setPointerCapture(e.pointerId);gesture.current={pointer:e.pointerId,x:e.clientX,start:width,value:width,node:e.currentTarget};}} onPointerMove={e=>{const g=gesture.current;if(!g||g.pointer!==e.pointerId)return;g.value=columnWidth(id,g.start+e.clientX-g.x);if(frame.current===null)frame.current=requestAnimationFrame(preview);}} onPointerUp={e=>{const g=gesture.current;if(!g||g.pointer!==e.pointerId)return;preview();gesture.current=null;current.current.onCommit(g.value);if(e.currentTarget.hasPointerCapture(e.pointerId))e.currentTarget.releasePointerCapture(e.pointerId);}} onPointerCancel={cancel} onLostPointerCapture={()=>{if(gesture.current)cancel();}}/></Hint>;
}
