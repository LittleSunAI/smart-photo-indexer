import {useEffect,useLayoutEffect,useRef,useState,useSyncExternalStore} from 'react';
import {createPortal} from 'react-dom';
import {hints} from './HintController';
import {HelpText} from './HelpText';
import './hint.css';

export function HintHost(){
 const visible=useSyncExternalStore(hints.subscribe,hints.snapshot,()=>null);
 const note=useRef<HTMLDivElement>(null),[position,setPosition]=useState({left:8,top:8});
 useEffect(()=>hints.mount(),[]);
 useLayoutEffect(()=>{
  hints.note=note.current;if(!visible)return;
  const place=()=>{
   if(!visible.anchor.isConnected||!visible.anchor.getClientRects().length||getComputedStyle(visible.anchor).visibility==='hidden'){hints.close();return;}
   const box=note.current?.getBoundingClientRect(),anchor=visible.anchor.getBoundingClientRect();if(!box)return;
   const left=Math.max(8,Math.min(anchor.left,innerWidth-box.width-8));
   const clamp=(top:number)=>Math.max(8,Math.min(top,innerHeight-box.height-8));
   let top=clamp(anchor.bottom+8+box.height<=innerHeight-8?anchor.bottom+8:anchor.top-box.height-8);
   // A compact modal can place its footer immediately below the hovered field.
   // Prefer another vertical position over covering its neighbouring actions.
   if(visible.portal instanceof HTMLDialogElement){
    const modal=visible.portal.getBoundingClientRect();
    const controls=[...visible.portal.querySelectorAll<HTMLElement>('button,input,select,textarea,summary,[role="switch"]')]
     .filter(node=>!visible.anchor.contains(node)&&!note.current?.contains(node)&&node.getClientRects().length)
     .map(node=>node.getBoundingClientRect());
    const overlap=(y:number)=>controls.reduce((sum,r)=>sum+Math.max(0,Math.min(left+box.width,r.right)-Math.max(left,r.left))*Math.max(0,Math.min(y+box.height,r.bottom)-Math.max(y,r.top)),0);
    for(const candidate of [clamp(anchor.top-box.height-8),clamp(modal.bottom+8),clamp(modal.top-box.height-8)]){
     if(overlap(candidate)<overlap(top))top=candidate;
     if(overlap(top)===0)break;
    }
   }
   setPosition(previous=>previous.left===left&&previous.top===top?previous:{left,top});
  };
  place();const observer=new ResizeObserver(place);if(note.current)observer.observe(note.current);
  window.addEventListener('resize',place);document.addEventListener('scroll',place,true);
  return()=>{observer.disconnect();window.removeEventListener('resize',place);document.removeEventListener('scroll',place,true);hints.note=null;};
 },[visible]);
 if(!visible)return null;
 return createPortal(<div ref={note} id={hints.tooltipId} role="tooltip" data-hint-mode={visible.pinned?'pinned':'automatic'} className="app-hint" style={position} tabIndex={visible.pinned?0:undefined}>
  {visible.options.message?<HelpText message={visible.options.message}/>:visible.options.content}
  {visible.options.supplement?<div className="app-hint__supplement"><HelpText message={visible.options.supplement}/></div>:null}
  {visible.options.details?<div className="app-hint__supplement">{visible.options.details}</div>:null}
  {visible.options.reason?<p className="app-hint__reason">{visible.options.reason}</p>:null}
 </div>,visible.portal);
}
