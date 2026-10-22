import {useLayoutEffect,useRef,useState,type ReactNode,type RefObject} from 'react';
import {createPortal} from 'react-dom';
export function AnchoredPanel({anchor,onClose,label,children,width=260}:{anchor:RefObject<HTMLElement|null>;onClose():void;label:string;children:ReactNode;width?:number}){
 const panel=useRef<HTMLDivElement>(null),[position,setPosition]=useState({left:8,top:8});
 const close=useRef(onClose);close.current=onClose;
 useLayoutEffect(()=>{
  const place=()=>{const rect=anchor.current?.getBoundingClientRect();if(!rect)return;const height=panel.current?.offsetHeight??150;setPosition({left:Math.max(8,Math.min(rect.right-width,window.innerWidth-width-8)),top:rect.bottom+height+6<=window.innerHeight?rect.bottom+4:Math.max(8,rect.top-height-4)});};
  const outside=(event:PointerEvent)=>{if(!anchor.current?.contains(event.target as Node)&&!panel.current?.contains(event.target as Node))close.current();};
  const key=(event:KeyboardEvent)=>{if(event.key==='Escape'){event.preventDefault();event.stopPropagation();close.current();anchor.current?.focus();}};
  place();panel.current?.querySelector<HTMLElement>('input,button')?.focus();window.addEventListener('resize',place);window.addEventListener('scroll',place,true);document.addEventListener('pointerdown',outside);window.addEventListener('keydown',key,true);
  return()=>{window.removeEventListener('resize',place);window.removeEventListener('scroll',place,true);document.removeEventListener('pointerdown',outside);window.removeEventListener('keydown',key,true);};
 },[anchor,width]);
 return createPortal(<div ref={panel} role="dialog" aria-label={label} className="anchored-panel" style={{...position,width,maxWidth:'calc(100vw - 16px)'}}>{children}</div>,document.body);
}
