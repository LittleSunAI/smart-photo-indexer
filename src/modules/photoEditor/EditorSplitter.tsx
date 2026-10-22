import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef} from 'react';
export function EditorSplitter({label,value,min,max,axis,reverse=false,className='',onChange,onCommit}:{label:string;value:number;min:number;max:number;axis:'x'|'y';reverse?:boolean;className?:string;onChange(value:number):void;onCommit?(value:number):void}){
  const drag=useRef<{coordinate:number;value:number;pointerId:number;owner:HTMLElement}|null>(null),pending=useRef<number|null>(null),frame=useRef<number|null>(null);
  const latest=useRef(onChange);latest.current=onChange;
  const finishCallback=useRef(onCommit);finishCallback.current=onCommit;const last=useRef(value);if(!drag.current)last.current=value;
  const commit=()=>{if(frame.current!==null){cancelAnimationFrame(frame.current);frame.current=null;}const next=pending.current;pending.current=null;if(next!==null){last.current=next;latest.current(next);}};
  const finish=()=>{const active=drag.current;commit();drag.current=null;if(active?.owner.hasPointerCapture(active.pointerId))active.owner.releasePointerCapture(active.pointerId);if(active)finishCallback.current?.(last.current);};
  useEffect(()=>{window.addEventListener('editor-flush-gestures',finish);window.addEventListener('blur',finish);return()=>{window.removeEventListener('editor-flush-gestures',finish);window.removeEventListener('blur',finish);if(frame.current!==null)cancelAnimationFrame(frame.current);const active=drag.current;drag.current=null;if(active?.owner.hasPointerCapture(active.pointerId))active.owner.releasePointerCapture(active.pointerId);};},[]);
  return <Hint message={{id:"layout.editor-split"}}><div className={`editor-splitter ${className}`} role="separator" tabIndex={0} aria-label={label} aria-orientation={axis==='x'?'vertical':'horizontal'} aria-valuenow={Math.round(value)} aria-valuemin={min} aria-valuemax={max}
    onKeyDown={e=>{if(['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End'].includes(e.key)){e.preventDefault();const direction=(e.key==='ArrowLeft'||e.key==='ArrowUp'?-1:1)*(reverse?-1:1),next=e.key==='Home'?min:e.key==='End'?max:Math.max(min,Math.min(max,value+direction*(e.shiftKey?20:5)));onChange(next);onCommit?.(next);}}}
    onPointerDown={e=>{if(e.button!==0||drag.current)return;e.preventDefault();e.currentTarget.setPointerCapture(e.pointerId);drag.current={coordinate:axis==='x'?e.clientX:e.clientY,value,pointerId:e.pointerId,owner:e.currentTarget};}}
    onPointerMove={e=>{if(!drag.current||drag.current.pointerId!==e.pointerId)return;pending.current=Math.max(min,Math.min(max,drag.current.value+((axis==='x'?e.clientX:e.clientY)-drag.current.coordinate)*(reverse?-1:1)));if(frame.current===null)frame.current=requestAnimationFrame(commit);}}
    onPointerUp={e=>{if(drag.current?.pointerId===e.pointerId)finish();}} onPointerCancel={e=>{if(drag.current?.pointerId===e.pointerId)finish();}} onLostPointerCapture={e=>{if(drag.current?.pointerId===e.pointerId)finish();}}/></Hint>
}
