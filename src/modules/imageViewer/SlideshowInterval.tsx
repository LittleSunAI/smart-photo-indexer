import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,useState} from 'react';

export function SlideshowInterval({value,onChange}:{value:number;onChange(value:number):void}){
  useCurrentLocale();
 const [open,setOpen]=useState(false),root=useRef<HTMLDivElement>(null),trigger=useRef<HTMLButtonElement>(null);
 useEffect(()=>{if(!open)return;const outside=(e:PointerEvent)=>{if(!root.current?.contains(e.target as Node))setOpen(false);},key=(e:KeyboardEvent)=>{if(e.key==='Escape'){e.preventDefault();e.stopImmediatePropagation();setOpen(false);trigger.current?.focus();}};document.addEventListener('pointerdown',outside,true);document.addEventListener('keydown',key,true);return()=>{document.removeEventListener('pointerdown',outside,true);document.removeEventListener('keydown',key,true);};},[open]);
 return <div data-tour-id="viewer.bottom.interval" ref={root} className="viewer-interval"><Hint message={{id:"viewer.slideshow-interval"}}><button ref={trigger} type="button" aria-label={localeText("viewer", "ui.SlideshowInterval.2e181c643f06675b")} aria-expanded={open} aria-haspopup="dialog" title={localeText("viewer", "ui.SlideshowInterval.f151072e64c533a8")} onClick={()=>setOpen(!open)}>{value} {' '}{localeText("viewer", "ui.SlideshowInterval.a40ad433927f0515")}</button></Hint>{open?<div className="viewer-interval__choices" role="dialog" aria-label={localeText("viewer", "ui.SlideshowInterval.8c3d31be5f9c9932")}>{Array.from({length:10},(_,i)=>i+1).map(seconds=><Hint key={seconds} message={{id:"viewer.interval-choice",args:{секунды:seconds}}}><button key={seconds} type="button" aria-pressed={value===seconds} onClick={()=>{onChange(seconds);setOpen(false);trigger.current?.focus();}}>{seconds}</button></Hint>)}</div>:null}</div>;
}
